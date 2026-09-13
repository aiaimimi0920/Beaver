use super::{comparison, model::Run, repository, runner, service::State};
use anyhow::{anyhow, Result};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

fn save(state: &State, run: &Run) -> Result<()> {
    state
        .store
        .lock()
        .map_err(|_| anyhow!("Database lock unavailable"))?
        .put("validationRun", &run.id, run)?;
    (state.changed)();
    Ok(())
}

fn claim(state: &State, kind: &str) -> Result<Option<(Run, Arc<AtomicBool>)>> {
    let store = state
        .store
        .lock()
        .map_err(|_| anyhow!("Database lock unavailable"))?;
    if state.stop.load(Ordering::SeqCst) {
        return Ok(None);
    }
    let mut runs: Vec<Run> = store.list("validationRun")?;
    runs.retain(|run| run.managed && run.kind == kind && run.status == "queued");
    runs.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    let Some(mut run) = runs.into_iter().next() else {
        return Ok(None);
    };
    let token = Arc::new(AtomicBool::new(false));
    state
        .active
        .lock()
        .map_err(|_| anyhow!("Validation lock unavailable"))?
        .insert(run.id.clone(), token.clone());
    run.status = "running".into();
    run.phase = "preparing".into();
    store.put("validationRun", &run.id, &run)?;
    Ok(Some((run, token)))
}

fn execute(state: &State, run: &mut Run, cancelled: &AtomicBool) -> Result<()> {
    let tools = {
        let store = state
            .store
            .lock()
            .map_err(|_| anyhow!("Database lock unavailable"))?;
        (state.resolve)(&store, run)?
    };
    let persistence_failed = AtomicBool::new(false);
    runner::execute(
        &state.data,
        &tools.godot,
        tools.ffmpeg.as_deref(),
        run,
        cancelled,
        |update| {
            let mut update = update.clone();
            if update.status == "completed" && update.kind == "visual" {
                update.status = "running".into();
                update.phase = "comparing".into();
            }
            if save(state, &update).is_err() {
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
            let store = state
                .store
                .lock()
                .map_err(|_| anyhow!("Database lock unavailable"))?;
            comparison::baseline_run(&store, run)?
        };
        comparison::compare_with(&state.data, run, baseline.as_ref())?;
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
            let result = state
                .store
                .lock()
                .map_err(|_| anyhow!("Database lock unavailable"))
                .and_then(|mut store| super::coordinator::refresh(&mut store, &state.data));
            match result {
                Ok(true) => (state.changed)(),
                Ok(false) => (),
                Err(error) => eprintln!("Validation coordination: {error}"),
            }
        }
        match claim(&state, kind) {
            Ok(Some((mut run, token))) => {
                if let Err(error) = execute(&state, &mut run, &token) {
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
                if let Err(error) = save(&state, &run) {
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
