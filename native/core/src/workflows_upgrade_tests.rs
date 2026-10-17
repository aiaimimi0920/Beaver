use super::*;
use crate::{files::file_hash, project_storage::ProjectStore};
use base64::Engine;
use std::fs;

struct Fixture {
    _temp: tempfile::TempDir,
    runtime: ProjectRuntime,
}

fn old_package(root: &Path, commit: &str) -> Result<()> {
    let old = trust::old_hashes(commit)?;
    let bundled = trust::bundled()?;
    let fixtures: Value = serde_json::from_str(include_str!("../../../tests/fixtures/npr-upgrade-legacy.json"))?;
    for name in old.keys() {
        let short = name.strip_prefix(&format!("{}/", package::PACKAGE_ROOT)).unwrap();
        let bytes = if short == "beaver-provenance.json" { trust::old_manifest(commit)?.to_vec() }
            else if let Some(encoded) = fixtures[commit][short].as_str() {
                base64::engine::general_purpose::STANDARD.decode(encoded)?
            } else { bundled[name].clone() };
        assert_eq!(trust::hash(&bytes), old[name]);
        super::super::write_new(root, name, &bytes)?;
    }
    let config = json!({"schemaVersion":1,"godot":"retained-engine","engineVersion":"retained-version",
        "unrelated":{"retain":true},"packages":{"other":{"status":"custom"},"npr-characters":{
        "status":"ready","sourcePath":package::PACKAGE_ROOT,"sourceCommit":commit,
        "contractVersion":"1.1.0","pluginVersion":"1.3.0","customMetadata":"keep"}}});
    fs::write(root.join(super::super::RUNTIME_FILE), serde_json::to_vec_pretty(&config)?)?;
    Ok(())
}

impl Fixture {
    fn new(commit: &str) -> Result<Self> {
        let temp = tempfile::tempdir()?;
        fs::write(temp.path().join("project.godot"), "config_version=5\n")?;
        old_package(temp.path(), commit)?;
        let runtime = ProjectStore::initialize(temp.path(), "project-a")?.into_runtime();
        runtime.store().lock().unwrap().put("project", "project-a", &json!({
            "id":"project-a","path":temp.path(),"npr":{"status":"ready","preserve":"metadata"}}))?;
        Ok(Self { _temp: temp, runtime })
    }
    fn inspect(&self, task: Option<&str>) -> Result<Value> {
        let mut input = json!({"id":"project-a"});
        if let Some(task) = task { input["taskId"] = json!(task); }
        call(&self.runtime, "project.npr.inspectUpgrade", input)
    }
    fn upgrade(&self, task: Option<&str>) -> Result<Value> {
        let inspected = self.inspect(task)?;
        let mut input = json!({"id":"project-a","expectedOldCommit":inspected["sourceCommit"],
            "expectedRuntimeSha256":inspected["runtimeSha256"]});
        if let Some(task) = task { input["taskId"] = json!(task); }
        call(&self.runtime, "project.npr.upgrade", input)
    }
    fn task(&self, commit: &str) -> Result<PathBuf> {
        let root = self.runtime.files().workspace("task-a")?;
        fs::create_dir_all(&root)?;
        fs::write(root.join("project.godot"), "config_version=5\n")?;
        old_package(&root, commit)?;
        self.runtime.store().lock().unwrap().put("task", "task-a", &json!({
            "id":"task-a","projectId":"project-a","status":"interrupted","accepted":false,
            "workspace":".beaver/workspaces/task-a","prompt":"Preserve face-only acceptance scope",
            "productionWorkflow":{"id":"npr-character","sourceCommit":commit}}))?;
        Ok(root)
    }
}

#[test]
fn supported_upgrades_preserve_engine_metadata_and_have_exact_rollback() -> Result<()> {
    for commit in [trust::OLD_COMMIT, trust::EARLIER_COMMIT] {
        let f = Fixture::new(commit)?;
        let original = fs::read(f.runtime.project_root().join(super::super::RUNTIME_FILE))?;
        let inspected = f.inspect(None)?;
        assert_eq!(inspected["eligible"], true, "{inspected}");
        let result = f.upgrade(None)?;
        assert_eq!(result["status"], "completed");
        assert_eq!(result["runtime"]["godot"], "retained-engine");
        assert_eq!(result["runtime"]["unrelated"]["retain"], true);
        assert_eq!(result["runtime"]["packages"]["other"]["status"], "custom");
        package::require_current(f.runtime.project_root(), &result["runtime"])?;
        call(&f.runtime, "project.npr.rollbackUpgrade", json!({"id":"project-a","operationId":result["operationId"]}))?;
        assert_eq!(fs::read(f.runtime.project_root().join(super::super::RUNTIME_FILE))?, original);
        trust::plan(f.runtime.project_root())?;
    }
    Ok(())
}

#[test]
fn customized_dependencies_and_stale_runtime_cas_never_write() -> Result<()> {
    let f = Fixture::new(trust::OLD_COMMIT)?;
    let inspected = f.inspect(None)?;
    assert!(call(&f.runtime, "project.npr.upgrade", json!({"id":"project-a",
        "expectedOldCommit":trust::OLD_COMMIT,"expectedRuntimeSha256":"0".repeat(64)})).is_err());
    let name = trust::old_hashes(trust::OLD_COMMIT)?.keys().next().unwrap().clone();
    fs::write(f.runtime.project_root().join(&name), "customized")?;
    assert_eq!(f.inspect(None)?["eligible"], false);
    assert!(f.upgrade(None).is_err());
    assert_eq!(trust::identity(f.runtime.project_root())?.1, inspected["runtimeSha256"].as_str().unwrap());
    assert_eq!(fs::read_to_string(f.runtime.project_root().join(name))?, "customized");
    Ok(())
}

#[test]
fn task_upgrade_requires_quiescence_and_keeps_project_and_frozen_plan_unchanged() -> Result<()> {
    let f = Fixture::new(trust::EARLIER_COMMIT)?;
    let root = f.task(trust::EARLIER_COMMIT)?;
    let handle = f.runtime.store();
    let mut task: Value = handle.lock().unwrap().get("task", "task-a")?.unwrap();
    let original = task.clone();
    task["status"] = json!("running");
    handle.lock().unwrap().put("task", "task-a", &task)?;
    assert_eq!(f.inspect(Some("task-a"))?["eligible"], false);
    assert!(f.upgrade(Some("task-a")).is_err());
    task["status"] = json!("interrupted");
    task["externalRecoveryRequired"] = json!({"reason":"uncertain process"});
    handle.lock().unwrap().put("task", "task-a", &task)?;
    assert!(f.upgrade(Some("task-a")).is_err());
    handle.lock().unwrap().put("task", "task-a", &original)?;
    let result = f.upgrade(Some("task-a"))?;
    assert_eq!(result["reviewRequired"], true);
    assert_eq!(trust::identity(f.runtime.project_root())?.0["packages"]["npr-characters"]["sourceCommit"], trust::EARLIER_COMMIT);
    assert_eq!(handle.lock().unwrap().get::<Value>("task", "task-a")?.unwrap(), original);
    assert!(crate::task_workflows::verify_for_execution(&root, &original, &handle.lock().unwrap()).is_err());
    let inspected = f.inspect(Some("task-a"))?;
    let review = json!({"id":"project-a","taskId":"task-a","operationId":result["operationId"],
        "expectedRuntimeSha256":result["runtimeSha256"],"expectedFrozenWorkflow":original["productionWorkflow"],
        "reviewedWorkflow":inspected["currentWorkflow"],"reviewReason":"Reviewed the verified dependency fix",
        "acceptance":"Preserve face-only acceptance scope"});
    let mut forged = review.clone();
    forged["operationId"] = json!(uuid::Uuid::new_v4().to_string());
    assert!(call(&f.runtime, "project.npr.reviewUpgrade", forged).is_err());
    call(&f.runtime, "project.npr.reviewUpgrade", review)?;
    assert_eq!(f.inspect(Some("task-a"))?["reviewRequired"], false);
    crate::task_workflows::verify_for_execution(&root, &original, &handle.lock().unwrap())?;
    assert_eq!(handle.lock().unwrap().get::<Value>("task", "task-a")?.unwrap(), original);
    crate::task_actions::continue_task(&mut handle.lock().unwrap(), "task-a", "", false)?;
    let mut continued: Value = handle.lock().unwrap().get("task", "task-a")?.unwrap();
    crate::task_workflows::verify_for_execution(&root, &continued, &handle.lock().unwrap())?;
    continued["status"] = json!("interrupted");
    handle.lock().unwrap().put("task", "task-a", &continued)?;
    call(&f.runtime, "project.npr.rollbackUpgrade", json!({"id":"project-a","operationId":result["operationId"]}))?;
    assert!(super::review::verify(&handle.lock().unwrap(), &root, &original).is_err());
    Ok(())
}

#[test]
fn crash_receipt_fences_project_and_rolls_back_without_overwriting_conflicts() -> Result<()> {
    let f = Fixture::new(trust::OLD_COMMIT)?;
    let root = f.runtime.project_root();
    let plan = trust::plan(root)?;
    let handle = f.runtime.store();
    let mut store = handle.lock().unwrap();
    let project: Value = store.get("project", "project-a")?.unwrap();
    let mut receipt = transaction::prepare(&f.runtime.files(), root, &project, None, &plan)?;
    store.put(KIND, &receipt.operation_id, &receipt)?;
    f.runtime.files().write(root, super::super::RUNTIME_FILE, Some(&receipt.transitional_runtime))?;
    let changed = receipt.changes[0].clone();
    f.runtime.files().write(root, &changed.path, changed.after.as_deref())?;
    assert!(Journal::new(&mut store, &f.runtime.files()).blocked("project-a")?);
    for binding in [Value::Null, json!({"id":"general"})] {
        let task = json!({"id":"prepared","projectId":"project-a","workspacePrepared":true,"productionWorkflow":binding});
        assert!(crate::task_workflows::verify_for_execution(root, &task, &store).is_err());
    }
    assert!(crate::object_task_queue::claim_next_in(&store.connection, "project-a", "test")?.is_none());
    assert!(super::super::runtime(root).is_err());
    fs::write(root.join(&changed.path), "concurrent edit")?;
    assert!(transaction::rollback(&store, &f.runtime.files(), root, &mut receipt).is_err());
    assert_eq!(fs::read_to_string(root.join(&changed.path))?, "concurrent edit");
    f.runtime.files().write(root, &changed.path, changed.after.as_deref())?;
    transaction::rollback(&store, &f.runtime.files(), root, &mut receipt)?;
    assert!(!Journal::new(&mut store, &f.runtime.files()).blocked("project-a")?);
    assert_eq!(file_hash(&root.join(super::super::RUNTIME_FILE))?, receipt.changes.last().unwrap().before);
    trust::plan(root)?;
    Ok(())
}

#[test]
fn task_ownership_and_accepted_workspace_are_never_overwritten() -> Result<()> {
    let f = Fixture::new(trust::OLD_COMMIT)?;
    f.task(trust::OLD_COMMIT)?;
    let result = f.upgrade(Some("task-a"))?;
    let handle = f.runtime.store();
    let mut task: Value = handle.lock().unwrap().get("task", "task-a")?.unwrap();
    task["accepted"] = json!(true);
    task["status"] = json!("completed");
    handle.lock().unwrap().put("task", "task-a", &task)?;
    assert!(call(&f.runtime, "project.npr.rollbackUpgrade", json!({"id":"project-a","operationId":result["operationId"]})).is_err());
    task["projectId"] = json!("other-project");
    handle.lock().unwrap().put("task", "task-a", &task)?;
    assert!(f.inspect(Some("task-a")).is_err());
    Ok(())
}

#[test]
fn paused_parent_is_quiescent_but_queued_object_work_is_not() -> Result<()> {
    let f = Fixture::new(trust::OLD_COMMIT)?;
    f.task(trust::OLD_COMMIT)?;
    let handle = f.runtime.store();
    handle.lock().unwrap().put("task", "parent", &json!({"id":"parent","projectId":"project-a",
        "status":"waitingChildren","planPaused":true,"subtaskIds":["task-a"]}))?;
    assert_eq!(f.inspect(Some("task-a"))?["eligible"], true);
    handle.lock().unwrap().put("object_task_queue", "queue-a", &json!({"projectId":"project-a","state":"queued"}))?;
    assert_eq!(f.inspect(Some("task-a"))?["eligible"], false);
    handle.lock().unwrap().remove("object_task_queue", "queue-a")?;
    let result = f.upgrade(Some("task-a"))?;
    assert_eq!(result["status"], "completed");
    Ok(())
}
