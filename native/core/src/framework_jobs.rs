use crate::{
    asset_delivery_files::{self as artifacts, Candidate},
    asset_task,
    files::{Files, Snapshot},
    framework_contract::{Configuration, Job},
    framework_operations::{self as operations, Operation},
    store::Store,
};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc, Mutex},
};

pub struct Prepared {
    task: Value,
    state: asset_task::State,
    configuration: Configuration,
    workspace: PathBuf,
    context: Value,
    candidate: Option<Candidate>,
    snapshot: Snapshot,
    inputs: Vec<(String, Snapshot)>,
}

pub fn prepare(
    store: &Store,
    files: &Files,
    id: &str,
    job: &Job,
    identity: Option<&(String, String)>,
) -> Result<Prepared> {
    let task: Value = store.get("task", id)?.context("Missing task")?;
    crate::object_framework::require_legacy(&task)?;
    let state = asset_task::get(store, id)?;
    let configuration = crate::framework::configuration(store, id)?;
    let workspace = files.resolve_workspace(
        id,
        Path::new(
            task["workspace"]
                .as_str()
                .context("Missing task workspace")?,
        ),
    )?;
    let mut candidate = None;
    let mut snapshot = Snapshot::new();
    let mut inputs = Vec::new();
    match job {
        Job::Callback { request } => {
            ensure!(
                identity.is_some(),
                "Use the current executor identity for asynchronous callbacks"
            );
            let request = serde_json::from_value(request.clone())?;
            ensure!(
                crate::task_callback_prepare::required(&request),
                "Only file callbacks need a durable operation"
            );
        }
        Job::Plugin { plugin, action } => {
            ensure!(
                configuration.plugins.iter().any(|p| &p.id == plugin),
                "Plugin is not registered; owner must configure its pinned adapter first"
            );
            ensure!(
                ["probe", "install", "enable", "reload"].contains(&action.as_str()),
                "Unsupported plugin action"
            );
            ensure!(
                identity.is_some() || task["status"] != "running",
                "Interrupt the executor before an owner plugin operation"
            );
        }
        Job::Check { candidate_id } => {
            let value = artifacts::get(store, id, candidate_id)?;
            for input in &value.input_candidates {
                snapshot.extend(artifacts::get(store, id, input)?.files);
            }
            snapshot.extend(value.files.clone());
            for id in &value.attempt_ids {
                let attempt = state
                    .work
                    .attempts
                    .iter()
                    .find(|a| &a.id == id)
                    .context("Missing candidate attempt")?;
                if let Some(files) = &attempt.input_files {
                    inputs.push((
                        id.clone(),
                        files
                            .iter()
                            .map(|f| (f.path.clone(), f.sha256.clone()))
                            .collect(),
                    ));
                }
            }
            candidate = Some(value);
        }
        Job::InputExport { attempt_id } => {
            let attempt = state
                .work
                .attempts
                .iter()
                .find(|a| &a.id == attempt_id)
                .context("Unknown task attempt")?;
            let inputs = attempt
                .input_files
                .as_ref()
                .context("This historical attempt has no captured input manifest")?;
            ensure!(!inputs.is_empty(), "Attempt has no file inputs to export");
            snapshot.extend(inputs.iter().map(|i| (i.path.clone(), i.sha256.clone())));
        }
        Job::Dependencies { command, paths } => {
            command.validate()?;
            ensure!(
                !paths.is_empty() && paths.len() <= 32,
                "Scan 1 to 32 source files"
            );
        }
    }
    let context = json!({"protocolVersion":1,"taskId":id,"workspace":workspace,"dataRoot":files.root(),"threadId":task["threadId"],"turnId":task["turnId"],"sessionId":state.session_id,"checkpoint":state.checkpoint,"attemptId":state.work.attempts.iter().find(|a|a.status=="running").map(|a|&a.id),"configurationRevision":configuration.revision});
    Ok(Prepared {
        task,
        state,
        configuration,
        workspace,
        context,
        candidate,
        snapshot,
        inputs,
    })
}

pub fn execute(
    store: Arc<Mutex<Store>>,
    files: Arc<Files>,
    operation: &Operation,
    prepared: Prepared,
    cancelled: &AtomicBool,
) -> Result<()> {
    if let Job::Callback { request } = &operation.job {
        return crate::task_callback_runtime::operation(store, files, operation, request.clone());
    }
    let result = match &operation.job {
        Job::Plugin { plugin, action } => {
            let plugin = prepared
                .configuration
                .plugins
                .iter()
                .find(|p| &p.id == plugin)
                .context("Missing plugin")?;
            crate::framework_plugins::execute(
                plugin,
                action,
                &prepared.workspace,
                &prepared.context,
                cancelled,
            )?
        }
        Job::Check { .. } => crate::framework_checks::run(
            &files,
            prepared.candidate.as_ref().unwrap(),
            &prepared.snapshot,
            &prepared.inputs,
            &prepared.configuration,
            cancelled,
        )?,
        Job::InputExport { attempt_id } => {
            json!({"attemptId":attempt_id,"manifest":prepared.state.work.attempts.iter().find(|a|&a.id==attempt_id).unwrap().input_files,"path":artifacts::export_snapshot(&files,&prepared.snapshot)?,"source":"frozen-attempt-inputs"})
        }
        Job::Dependencies { command, paths } => crate::framework_inputs::scan(
            &files,
            &prepared.workspace,
            command,
            paths,
            &prepared.context,
            cancelled,
        )?,
        Job::Callback { .. } => unreachable!(),
    };
    let mut db = store
        .lock()
        .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
    operations::require_running(&db, &operation.task_id, &operation.id)?;
    let task: Value = db
        .get("task", &operation.task_id)?
        .context("Task disappeared")?;
    let state = asset_task::get(&db, &operation.task_id)?;
    let current = crate::framework::configuration(&db, &operation.task_id)?;
    let stale = task["threadId"] != prepared.task["threadId"]
        || task["turnId"] != prepared.task["turnId"]
        || task["status"] != prepared.task["status"]
        || state.revision != prepared.state.revision
        || state.session_id != prepared.state.session_id
        || current != prepared.configuration;
    let mut saved = operation.clone();
    saved.status = if stale { "stale" } else { "succeeded" }.into();
    saved.ended_at = Some(asset_task::now());
    saved.result = Some(json!({"context":prepared.context,"value":result}));
    db.transaction(|db| {
        if !stale {
            if let Job::Check { candidate_id } = &operation.job {
                crate::task_callback::put(
                    db,
                    &format!("framework-check/{}", operation.task_id),
                    candidate_id,
                    &result,
                )?;
            }
        }
        crate::task_callback::put(db, operations::KIND, &operation.id, &saved)
    })?;
    Ok(())
}
