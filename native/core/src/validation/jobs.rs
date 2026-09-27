use super::{
    comparison,
    model::Run,
    repository, runner,
    service::{State, Storage, ToolContext},
    settings,
};
use anyhow::{anyhow, Result};
use std::{
    collections::BTreeSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

fn save(state: &State, storage: &Storage, run: &Run) -> Result<()> {
    storage
        .store
        .lock()
        .map_err(|_| anyhow!("Database lock unavailable"))?
        .put("validationRun", &run.id, run)?;
    (state.changed)();
    Ok(())
}

fn owned_projects(
    storage: &Storage,
    store: &crate::store::Store,
    local: &BTreeSet<String>,
) -> Result<BTreeSet<String>> {
    if let Some(id) = &storage.project_id {
        return Ok(BTreeSet::from([id.clone()]));
    }
    Ok(store
        .list_with_ids::<serde_json::Value>("project")?
        .into_iter()
        .map(|(id, _)| id)
        .filter(|id| !local.contains(id))
        .collect())
}

fn claim(state: &State, kind: &str) -> Result<Option<(Run, Arc<AtomicBool>, Storage)>> {
    if state.stop.load(Ordering::SeqCst) {
        return Ok(None);
    }
    let storages = (state.enumerate)()?;
    let project_ids: BTreeSet<String> = storages
        .iter()
        .filter_map(|storage| storage.project_id.clone())
        .collect();
    let mut candidates = vec![];
    for storage in storages {
        if storage.draining {
            continue;
        }
        let Some(_permit) = storage.work_gate.acquire()? else {
            continue;
        };
        let store = storage
            .store
            .lock()
            .map_err(|_| anyhow!("Database lock unavailable"))?;
        let owned = owned_projects(&storage, &store, &project_ids)?;
        for run in store.list::<Run>("validationRun")? {
            let owned_by_storage = owned.contains(&run.project_id);
            if owned_by_storage
                && run.managed
                && matches_lane(&run.kind, kind)
                && run.status == "queued"
            {
                candidates.push((run, storage.clone()));
            }
        }
    }
    candidates.sort_by(|(left, _), (right, _)| {
        left.created_at
            .cmp(&right.created_at)
            .then_with(|| left.id.cmp(&right.id))
    });
    let Some((mut run, storage)) = candidates.into_iter().next() else {
        return Ok(None);
    };
    let Some(permit) = storage.work_gate.acquire()? else {
        return Ok(None);
    };
    let store = storage
        .store
        .lock()
        .map_err(|_| anyhow!("Database lock unavailable"))?;
    let current: Run = repository::get(&store, "validationRun", &run.id)?;
    if !owned_projects(&storage, &store, &project_ids)?.contains(&current.project_id)
        || !current.managed
        || !matches_lane(&current.kind, kind)
        || current.status != "queued"
    {
        return Ok(None);
    }
    run = current;
    let token = Arc::new(AtomicBool::new(false));
    state
        .active
        .lock()
        .map_err(|_| anyhow!("Validation lock unavailable"))?
        .insert(run.id.clone(), token.clone());
    run.status = "running".into();
    run.phase = "preparing".into();
    store.put("validationRun", &run.id, &run)?;
    drop(store);
    drop(permit);
    Ok(Some((run, token, storage)))
}

fn coordinate(storage: &Storage, local: &BTreeSet<String>) -> Result<bool> {
    if storage.draining {
        return Ok(false);
    }
    let Some(_permit) = storage.work_gate.acquire()? else {
        return Ok(false);
    };
    let mut store = storage
        .store
        .lock()
        .map_err(|_| anyhow!("Database lock unavailable"))?;
    let owned = owned_projects(storage, &store, local)?;
    super::coordinator::refresh_matching(&mut store, &storage.files, |id| owned.contains(id))
}

fn matches_lane(run: &str, lane: &str) -> bool {
    run == lane || (lane == "visual" && run == "objectPreview")
}

fn execute(state: &State, storage: &Storage, run: &mut Run, cancelled: &AtomicBool) -> Result<()> {
    let context = {
        let store = storage
            .store
            .lock()
            .map_err(|_| anyhow!("Database lock unavailable"))?;
        ToolContext {
            engine: super::blender_preview::engine(run),
            project: repository::project(&store, &run.project_id)?,
            settings: settings::read(&store, &run.project_id)?,
        }
    };
    let tools = (state.resolve)(&context)?;
    let persistence_failed = AtomicBool::new(false);
    runner::execute(
        &storage.files,
        &tools.engine,
        tools.ffmpeg.as_deref(),
        run,
        cancelled,
        |update| {
            let mut update = update.clone();
            if update.status == "completed" && update.kind == "visual" {
                update.status = "running".into();
                update.phase = "comparing".into();
            }
            if save(state, storage, &update).is_err() {
                persistence_failed.store(true, Ordering::SeqCst);
                cancelled.store(true, Ordering::SeqCst);
            }
        },
    );
    anyhow::ensure!(
        !persistence_failed.load(Ordering::SeqCst),
        "Cannot persist validation progress"
    );
    if run.status == "completed" && run.kind == "visual" {
        // Freeze the comparison input once, then release the store during image processing.
        let baseline = {
            let store = storage
                .store
                .lock()
                .map_err(|_| anyhow!("Database lock unavailable"))?;
            comparison::baseline_run(&store, run)?
        };
        comparison::compare_with(&storage.files, run, baseline.as_ref())?;
    }
    if cancelled.load(Ordering::SeqCst) {
        run.status = "cancelled".into();
        run.verdict = "needsReview".into();
        run.error = Some("Run cancelled; partial evidence retained".into());
    }
    Ok(())
}

pub(crate) fn work(state: Arc<State>, kind: &str) {
    while !state.stop.load(Ordering::SeqCst) {
        if kind == "code" {
            match (state.enumerate)() {
                Ok(storages) => {
                    let mut changed = false;
                    let local = storages
                        .iter()
                        .filter_map(|s| s.project_id.clone())
                        .collect();
                    for storage in storages {
                        let result = coordinate(&storage, &local);
                        match result {
                            Ok(value) => changed |= value,
                            Err(error) => eprintln!("Validation coordination: {error}"),
                        }
                    }
                    if changed {
                        (state.changed)();
                    }
                }
                Err(error) => eprintln!("Validation coordination: {error}"),
            }
        }
        match claim(&state, kind) {
            Ok(Some((mut run, token, storage))) => {
                if let Err(error) = execute(&state, &storage, &mut run, &token) {
                    run.status = if token.load(Ordering::SeqCst) {
                        "cancelled"
                    } else {
                        "failed"
                    }
                    .into();
                    run.verdict = "needsReview".into();
                    run.error = Some(error.to_string());
                }
                run.phase = run.status.clone();
                run.finished_at = Some(repository::now());
                if let Err(error) = save(&state, &storage, &run) {
                    eprintln!("Validation result {}: {error}", run.id);
                }
                if let Ok(mut active) = state.active.lock() {
                    active.remove(&run.id);
                }
            }
            Ok(None) => thread::sleep(Duration::from_millis(500)),
            Err(error) => {
                eprintln!("Validation queue: {error}");
                thread::sleep(Duration::from_secs(2));
            }
        }
    }
}

#[cfg(test)]
#[path = "jobs_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "jobs_lifecycle_tests.rs"]
mod lifecycle_tests;

#[cfg(test)]
#[path = "jobs_registration_tests.rs"]
mod registration_tests;
