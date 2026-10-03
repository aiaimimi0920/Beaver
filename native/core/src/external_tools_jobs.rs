use super::{blender, Context};
use anyhow::{ensure, Context as _, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, Mutex, OnceLock,
    },
    time::{Duration, Instant},
};

struct Job {
    run_id: String,
    cancelled: AtomicBool,
    done: AtomicBool,
    state: Mutex<Value>,
    changed: Condvar,
}
static JOBS: OnceLock<Mutex<BTreeMap<String, Arc<Job>>>> = OnceLock::new();
#[cfg(test)]
#[path = "external_tools_jobs_tests.rs"]
mod tests;
fn registry() -> &'static Mutex<BTreeMap<String, Arc<Job>>> {
    JOBS.get_or_init(Default::default)
}
fn lock_error() -> anyhow::Error {
    anyhow::anyhow!("Managed Blender job lock unavailable")
}

pub(super) fn start(
    context: &Context,
    arguments: &Value,
    cancelled: &AtomicBool,
    progress: &mut dyn FnMut(&Value) -> Result<()>,
) -> Result<Value> {
    let id = uuid::Uuid::new_v4().to_string();
    let job = Arc::new(Job {
        run_id: context.run_id.clone(),
        cancelled: AtomicBool::new(false),
        done: AtomicBool::new(false),
        state: Mutex::new(
            json!({"jobId":id,"runId":context.run_id,"requestId":context.request_id,"status":"preparing"}),
        ),
        changed: Condvar::new(),
    });
    {
        let mut jobs = registry().lock().map_err(|_| lock_error())?;
        ensure!(
            !jobs
                .values()
                .any(|item| item.run_id == context.run_id && !item.done.load(Ordering::SeqCst)),
            "This run already has an active Blender job"
        );
        ensure!(
            jobs.values()
                .filter(|item| item.run_id == context.run_id)
                .count()
                < 256,
            "External run reached its 256-job limit"
        );
        jobs.insert(id, job.clone());
    }
    let mut spawned = false;
    let started = (|| -> Result<_> {
        let prepared = blender::prepare(context, arguments)?;
        ensure!(
            !job.cancelled.load(Ordering::SeqCst) && !cancelled.load(Ordering::SeqCst),
            "Job cancelled before spawn"
        );
        let process = prepared.spawn()?;
        spawned = true;
        if cancelled.load(Ordering::SeqCst) {
            job.cancelled.store(true, Ordering::SeqCst);
        }
        let mut state = job.state.lock().map_err(|_| lock_error())?;
        state["status"] = json!("running");
        state["process"] = prepared.receipt(process.id());
        // Failure to persist the owned PID/command aborts and drops the owned tree.
        progress(&state)?;
        save(&state)?;
        drop(state);
        Ok((prepared, process))
    })();
    let (prepared, process) = match started {
        Ok(value) => value,
        Err(error) => {
            if spawned {
                complete(&job, Err(anyhow::anyhow!(error.to_string())));
            } else {
                let mut state = job.state.lock().map_err(|_| lock_error())?;
                state["status"] = json!("rejected");
                state["error"] = json!(error.to_string());
                state["retrySafe"] = json!(false);
                job.done.store(true, Ordering::SeqCst);
                job.changed.notify_all();
            }
            return Err(error);
        }
    };
    let worker = job.clone();
    if let Err(error) = std::thread::Builder::new()
        .name("beaver-external-blender".into())
        .spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                prepared.complete(process, &worker.cancelled)
            }))
            .unwrap_or_else(|_| {
                Err(anyhow::anyhow!(
                    "Managed Blender worker panicked; result uncertain"
                ))
            });
            complete(&worker, result);
        })
    {
        complete(&job, Err(anyhow::anyhow!(error.to_string())));
        return Err(error.into());
    }
    let state = job.state.lock().map_err(|_| lock_error())?.clone();
    Ok(state)
}

fn complete(job: &Job, result: Result<Value>) {
    let Ok(mut state) = job.state.lock() else {
        job.done.store(true, Ordering::SeqCst);
        job.changed.notify_all();
        return;
    };
    match result {
        Ok(result) => {
            state["status"] = json!(if result["success"] == true {
                "succeeded"
            } else if result["termination"] == "cancelled" {
                "cancelled"
            } else if result["termination"] == "timed_out" {
                "timed_out"
            } else {
                "failed"
            });
            state["result"] = result;
        }
        Err(error) => {
            state["status"] = json!("unknown");
            state["error"] = json!(error.to_string());
            state["retrySafe"] = json!(false);
        }
    }
    if let Err(error) = save(&state) {
        state["status"] = json!("unknown");
        state["receiptError"] = json!(error.to_string());
        state["retrySafe"] = json!(false);
    }
    job.done.store(true, Ordering::SeqCst);
    job.changed.notify_all();
}

fn save(state: &Value) -> Result<()> {
    let Some(directory) = state["process"]["jobDirectory"].as_str() else {
        return Ok(());
    };
    let directory = std::path::Path::new(directory);
    // This receipt supplements the host registry; it is not a task authorization source.
    let mut temporary = tempfile::Builder::new()
        .prefix(".receipt-")
        .tempfile_in(directory)?;
    std::io::Write::write_all(&mut temporary, &serde_json::to_vec(state)?)?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(directory.join("job.json"))
        .map_err(|error| error.error)?;
    Ok(())
}

/// The host serializes starts/close for a run. Revalidate under the registry lock
/// anyway: a newly admitted/uncertain job must prevent the entire reclamation.
fn reclaim(run_id: &str) -> Result<()> {
    let mut jobs = registry().lock().map_err(|_| lock_error())?;
    for job in jobs.values().filter(|job| job.run_id == run_id) {
        ensure!(
            job.done.load(Ordering::SeqCst),
            "Active Blender job cannot be reclaimed"
        );
        let state = job.state.lock().map_err(|_| lock_error())?;
        ensure!(
            matches!(
                state["status"].as_str(),
                Some("succeeded" | "failed" | "cancelled" | "timed_out" | "rejected")
            ),
            "Uncertain Blender job cannot be reclaimed"
        );
    }
    jobs.retain(|_, job| job.run_id != run_id);
    Ok(())
}

pub(super) fn poll(context: &Context, arguments: &Value, cancel: bool) -> Result<Value> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct Arguments {
        job_id: String,
    }
    let args: Arguments = serde_json::from_value(arguments.clone())?;
    let job = registry()
        .lock()
        .map_err(|_| lock_error())?
        .get(&args.job_id)
        .cloned()
        .context("Unknown job in this host process; do not replay an uncertain start")?;
    ensure!(
        job.run_id == context.run_id,
        "Blender job belongs to another external run"
    );
    if cancel && !job.done.load(Ordering::SeqCst) {
        job.cancelled.store(true, Ordering::SeqCst);
    }
    let mut state = job.state.lock().map_err(|_| lock_error())?.clone();
    state["cancelRequested"] = json!(job.cancelled.load(Ordering::SeqCst));
    Ok(state)
}

pub(super) fn ensure_idle(run_id: &str) -> Result<()> {
    let jobs = registry().lock().map_err(|_| lock_error())?;
    for job in jobs.values().filter(|job| job.run_id == run_id) {
        ensure!(
            job.done.load(Ordering::SeqCst),
            "Finish is blocked by an active Blender job; poll or cancel it first"
        );
        ensure!(
            job.state.lock().map_err(|_| lock_error())?["status"] != "unknown",
            "Blender outcome is uncertain; do not finalize or replay this run"
        );
    }
    Ok(())
}

pub(super) fn cleanup(run_id: &str) -> Result<()> {
    let jobs: Vec<_> = registry()
        .lock()
        .map_err(|_| lock_error())?
        .values()
        .filter(|job| job.run_id == run_id)
        .cloned()
        .collect();
    for job in &jobs {
        job.cancelled.store(true, Ordering::SeqCst);
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    for job in jobs {
        let mut state = job.state.lock().map_err(|_| lock_error())?;
        while !job.done.load(Ordering::SeqCst) {
            let remaining = deadline.saturating_duration_since(Instant::now());
            ensure!(
                !remaining.is_zero(),
                "Owned Blender cleanup unconfirmed; run remains blocked"
            );
            state = job
                .changed
                .wait_timeout(state, remaining)
                .map_err(|_| lock_error())?
                .0;
        }
        ensure!(
            state["status"] != "unknown",
            "Owned Blender outcome is uncertain; inspect its receipt before recovery"
        );
    }
    reclaim(run_id)
}
