use super::{code, model::Run, repository, runner};
use crate::{
    executor::Outcome,
    files::{Change, Files, Snapshot},
    store::Store,
};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

#[cfg(test)]
#[path = "task_gate_asset_tests.rs"]
mod asset_tests;
#[cfg(test)]
#[path = "task_gate_scope_tests.rs"]
mod scope_tests;

fn asset_only(change: &Change, path: &str) -> bool {
    // Deletions and changes to existing validation policy still need the code gate.
    change.after.is_some()
        && (path.ends_with(".blend")
            || path.ends_with(".glb")
            || path.ends_with(".png")
            || (path == "beaver.validation.json" && change.before.is_none()))
}

pub fn required(task: &Value, changes: &[Change]) -> bool {
    task["integrationValidation"] == true
        || changes.iter().any(|c| {
            let path = c.path.to_lowercase();
            !path.starts_with("docs/")
                && !path.ends_with(".md")
                && !path.ends_with(".txt")
                && !(task["assetTask"] == true && asset_only(c, &path))
        })
}

pub fn validation_only(task: &Value) -> bool {
    task["validationOnly"] == true || task["integrationValidation"] == true
}

pub fn candidate(current: &Snapshot, changes: &[Change]) -> Result<Snapshot> {
    let mut result = current.clone();
    for change in changes {
        let old = current.get(&change.path).cloned();
        anyhow::ensure!(
            old == change.before || old == change.after,
            "Merge conflict: {}",
            change.path
        );
        if let Some(hash) = &change.after {
            result.insert(change.path.clone(), hash.clone());
        } else {
            result.remove(&change.path);
        }
    }
    Ok(result)
}

pub fn changes(files: &Files, task: &Value) -> Result<Vec<Change>> {
    if task["integrationValidation"] == true {
        return Ok(vec![]);
    }
    if task["validationOnly"] == true {
        return Ok(serde_json::from_value(task["changes"].clone())?);
    }
    let baseline: Snapshot = serde_json::from_value(task["baseline"].clone())?;
    let workspace = files.resolve_workspace(
        task["id"].as_str().context("Task ID missing")?,
        Path::new(
            task["workspace"]
                .as_str()
                .context("Task workspace missing")?,
        ),
    )?;
    crate::source_encoding::normalize(&workspace, &baseline)?;
    Ok(Files::changes(&baseline, &files.capture(&workspace)?))
}

fn prepare(store: &Store, files: &Files, id: &str) -> Result<Option<Run>> {
    let mut task: Value = repository::get(store, "task", id)?;
    if task["validationVersion"] != 1
        || task["capability"] == "review"
        || task["clarifications"]
            .as_array()
            .is_some_and(|qs| qs.iter().any(|q| q["answers"].is_null()))
        || store
            .get::<crate::asset_task::State>("asset-task", id)?
            .is_some_and(|asset| !asset.ready())
    {
        return Ok(None);
    }
    let changes = changes(files, &task)?;
    let baseline: Snapshot = serde_json::from_value(task["baseline"].clone())?;
    crate::code_structure::check_changes(files, &baseline, &changes)?.ensure_ok()?;
    let project = task["projectId"]
        .as_str()
        .context("Task project missing")?
        .to_owned();
    let current = repository::snapshot(store, files, &project)?;
    if task["integrationValidation"] != true {
        task["changes"] = json!(changes);
    }
    task["validationPrepared"] = json!(true);
    store.put("task", id, &task)?;
    let Ok(snapshot) = candidate(&current, &changes) else {
        return Ok(None);
    };
    if !required(&task, &changes) {
        task["codeValidation"] = json!({"status":"notRequired","reason":"Documentation or asset-only changes; GUT not required"});
        store.put("task", id, &task)?;
        return Ok(None);
    }
    let run = repository::new_run(store, &project, snapshot, None, Some(id.into()), None)?;
    task["codeValidation"] =
        json!({"status":"running","runId":run.id,"snapshotId":run.snapshot_id});
    store.put("task", id, &task)?;
    store.event(
        id,
        &repository::now(),
        "codeValidation",
        &format!("GUT candidate {}", run.snapshot_id),
    )?;
    Ok(Some(run))
}

pub fn execute(
    store: Arc<Mutex<Store>>,
    files: &Files,
    id: &str,
    engine: Option<&Path>,
    cancelled: &AtomicBool,
    changed: &dyn Fn(),
) -> Outcome {
    let result = (|| -> Result<()> {
        let run = {
            let guard = store
                .lock()
                .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
            prepare(&guard, files, id)?
        };
        let Some(mut run) = run else {
            return Ok(());
        };
        changed();
        if let Some(engine) = engine.filter(|path| path.is_file()) {
            runner::execute(files, engine, None, &mut run, cancelled, |run| {
                if let Ok(store) = store.lock() {
                    let _ = store.put("validationRun", &run.id, run);
                }
                changed();
            });
        } else {
            run.status = "failed".into();
            run.phase = "failed".into();
            run.verdict = "needsReview".into();
            run.error = Some("请在设置中配置 Godot 4.4 或更新的 4.x 编辑器后重试代码验收".into());
            run.finished_at = Some(repository::now());
        }
        let store = store
            .lock()
            .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
        store.put("validationRun", &run.id, &run)?;
        let mut task: Value = repository::get(&store, "task", id)?;
        task["codeValidation"] = json!({"status":run.status,"runId":run.id,"snapshotId":run.snapshot_id,"error":run.error});
        store.put("task", id, &task)?;
        Ok(())
    })();
    if cancelled.load(Ordering::SeqCst) {
        Outcome::Interrupted
    } else {
        result.map_or_else(|e| Outcome::Failed(e.to_string()), |_| Outcome::Completed)
    }
}

/// Recompute the exact integration candidate under the finalizer's merge lock.
pub fn ready(store: &Store, files: &Files, task: &mut Value, changes: &[Change]) -> Result<bool> {
    if task["validationVersion"] != 1 || task["capability"] == "review" || !required(task, changes)
    {
        return Ok(true);
    }
    let id = task["id"].as_str().context("Task ID missing")?.to_owned();
    let project = task["projectId"].as_str().context("Project ID missing")?;
    let snapshot = candidate(&repository::snapshot(store, files, project)?, changes)?;
    let run = task["codeValidation"]["runId"]
        .as_str()
        .map(|id| repository::get::<Run>(store, "validationRun", id))
        .transpose()?;
    let Some(run) = run else {
        task["validationOnly"] = json!(true);
        task["status"] = json!("queued");
        task["error"] = json!("Recorded output requires GUT validation before merge");
        return Ok(false);
    };
    anyhow::ensure!(
        run.task_id.as_deref() == Some(&id) && run.project_id == project && run.kind == "code",
        "Wrong code validation receipt"
    );
    if run.snapshot != snapshot {
        let count = task["validationRechecks"].as_u64().unwrap_or(0);
        task["validationRechecks"] = json!(count + 1);
        task["validationOnly"] = json!(true);
        task["status"] = json!(if count < 3 { "queued" } else { "failed" });
        task["error"] =
            json!("Integrated candidate changed; rechecking recorded output before merge");
    } else if let Err(error) = code::validate(files, &run) {
        task["status"] = json!("failed");
        let error = run.error.clone().unwrap_or_else(|| error.to_string());
        task["error"] = json!(error);
        task["validationRepair"] = json!({"runId":run.id,"snapshotId":run.snapshot_id,"engineVersion":run.engine_version,"error":error,"code":run.code,"log":run.log.chars().take(12000).collect::<String>()});
    } else {
        task["validationOnly"] = json!(false);
        task["validationRechecks"] = json!(0);
        task.as_object_mut()
            .context("Invalid task")?
            .remove("validationRepair");
        return Ok(true);
    }
    store.event(
        &id,
        &repository::now(),
        "codeValidation",
        task["error"].as_str().unwrap_or("Validation pending"),
    )?;
    Ok(false)
}
