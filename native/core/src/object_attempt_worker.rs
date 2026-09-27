use crate::{
    executor::Control,
    object_attempt::{self, Lease, State},
    object_attempt_launch::{Factory, Launch},
    object_attempt_rpc,
    object_run_preparation::PreparationClaim,
    project_runtime::ProjectRuntime,
    rpc::Rpc,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::mpsc;

pub(crate) async fn execute(
    runtime: ProjectRuntime,
    claim: PreparationClaim,
    factory: Factory,
    cancelled: Arc<AtomicBool>,
    controls: mpsc::Receiver<Control>,
    changed: Arc<dyn Fn() + Send + Sync>,
) -> Result<(), String> {
    let prepare_runtime = runtime.clone();
    let lease = tokio::task::spawn_blocking(move || object_attempt::start(&prepare_runtime, claim))
        .await
        .map_err(|_| "OBJECT_ATTEMPT_PREPARATION_WORKER_FAILED")?
        .map_err(|error| error.to_string())?;
    changed();
    let Some(lease) = lease else {
        return Ok(());
    };
    execute_lease(runtime, lease, factory, cancelled, controls).await
}

pub(crate) async fn execute_lease(
    runtime: ProjectRuntime,
    mut lease: Lease,
    factory: Factory,
    cancelled: Arc<AtomicBool>,
    mut controls: mpsc::Receiver<Control>,
) -> Result<(), String> {
    let launch_runtime = runtime.clone();
    let record = lease.record().clone();
    let stop = cancelled.clone();
    let launch = tokio::task::spawn_blocking(move || {
        if stop.load(Ordering::SeqCst) {
            return Ok(None);
        }
        factory(&record, &launch_runtime).map(Some)
    })
    .await
    .map_err(|_| "OBJECT_ATTEMPT_LAUNCH_WORKER_FAILED")?;
    let result = match launch {
        Ok(Some(launch)) if !cancelled.load(Ordering::SeqCst) => {
            run(&runtime, &mut lease, launch, &cancelled, &mut controls).await?
        }
        Ok(_) => (
            State::Interrupted,
            Some("OBJECT_ATTEMPT_INTERRUPTED".into()),
        ),
        Err(_) => (State::Failed, Some("OBJECT_ATTEMPT_LAUNCH_FAILED".into())),
    };
    // The RPC path returns only after close succeeds; a failed close deliberately
    // keeps the attempt running and stops the scheduler without a checkpoint.
    let (state, error) = result;
    tokio::task::spawn_blocking(move || {
        object_attempt::finish(&runtime, lease, state, error, &cancelled)
    })
    .await
    .map_err(|_| "OBJECT_ATTEMPT_CHECKPOINT_WORKER_FAILED")?
    .map_err(|error| error.to_string())
}

async fn run(
    runtime: &ProjectRuntime,
    lease: &mut Lease,
    launch: Launch,
    cancelled: &AtomicBool,
    controls: &mut mpsc::Receiver<Control>,
) -> Result<(State, Option<String>), String> {
    object_attempt::validate(runtime, lease).map_err(|error| error.to_string())?;
    let files = runtime.files();
    let record = lease.record().clone();
    let matches_input = tokio::task::spawn_blocking(move || {
        let path = files.resolve_workspace(
            &record.preparation.run.id,
            std::path::Path::new(&record.preparation.workspace),
        )?;
        Ok::<_, anyhow::Error>(files.capture(&path)? == record.input)
    })
    .await
    .map_err(|_| "OBJECT_ATTEMPT_INPUT_WORKER_FAILED")?
    .map_err(|error| error.to_string())?;
    if !matches_input {
        return Ok((State::Failed, Some("OBJECT_ATTEMPT_INPUT_CHANGED".into())));
    }
    let Launch {
        command,
        cwd,
        model,
        secrets,
        timeout,
        godot,
    } = launch;
    // A failed spawn/attach has no confirmed tree shutdown. Preserve ownership
    // without freezing output, even when the failure likely preceded execution.
    let (rpc, mut events) = Rpc::spawn_owned(command).map_err(|error| redact(&error, &secrets))?;
    let result = {
        let conversation =
            object_attempt_rpc::conversation(runtime, lease, &rpc, &mut events, &cwd, &model);
        tokio::pin!(conversation);
        let deadline = tokio::time::sleep(timeout);
        tokio::pin!(deadline);
        let root_exit = rpc.wait_for_exit();
        tokio::pin!(root_exit);
        loop {
            tokio::select! {
                biased;
                control = controls.recv() => match control {
                    None | Some(Control::Interrupt) => break (State::Interrupted, Some("OBJECT_ATTEMPT_INTERRUPTED".into())),
                    Some(Control::Steer { reply, .. }) => { let _ = reply.send(Err("OBJECT_ATTEMPT_DEFINITION_FROZEN".into())); }
                },
                _ = &mut deadline => break (State::Failed, Some("OBJECT_ATTEMPT_TIMEOUT".into())),
                result = &mut conversation => break match result {
                    Ok(()) => (State::AwaitingGate, None),
                    Err(error) => (State::Failed, Some(redact(&error, &secrets))),
                },
                result = &mut root_exit => break (State::Failed, Some(match result {
                    Ok(()) => "OBJECT_ATTEMPT_PROCESS_EXITED".into(),
                    Err(error) => redact(&error, &secrets),
                })),
            }
        }
    };
    rpc.close()
        .await
        .map_err(|error| redact(&error, &secrets))?;
    if cancelled.load(Ordering::SeqCst) {
        Ok((
            State::Interrupted,
            Some("OBJECT_ATTEMPT_INTERRUPTED".into()),
        ))
    } else if result.0 == State::AwaitingGate {
        if let Some(import) = godot {
            object_attempt::validate(runtime, lease).map_err(|error| error.to_string())?;
            let home = runtime
                .files()
                .codex_home(&lease.record().id)
                .map_err(|error| error.to_string())?;
            crate::object_attempt_godot::run(import, &cwd, &home, cancelled, controls).await
        } else {
            Ok(result)
        }
    } else {
        Ok(result)
    }
}

fn redact(message: &str, secrets: &[String]) -> String {
    secrets
        .iter()
        .filter(|secret| !secret.is_empty())
        .fold(message.into(), |text, secret| {
            text.replace(secret, "[REDACTED_SECRET]")
        })
}
