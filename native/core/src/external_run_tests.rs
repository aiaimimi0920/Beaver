use super::*;
use crate::external_run_contract::RunRequest;
use std::fs;

struct Fixture {
    _temp: tempfile::TempDir,
    store: Arc<Mutex<Store>>,
    files: Arc<Files>,
    task: Value,
    registry: ExternalRegistry,
    live: Arc<Live>,
    outcome: oneshot::Receiver<Outcome>,
}
impl Fixture {
    fn new() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let project = temp.path().join("project");
        fs::create_dir(&project)?;
        fs::write(project.join("before.txt"), "original")?;
        let root = temp.path().join("data");
        let mut store = Store::open(&root)?;
        let files = Arc::new(Files::new(root));
        store.put(
            "project",
            "p",
            &json!({"id":"p","name":"test","path":project}),
        )?;
        let mut task = crate::task_create::create(
            &mut store,
            &files,
            json!({"projectId":"p","prompt":"Build a character","executionMode":"external-agent","decompose":false}),
            &Value::Null,
            &Value::Null,
        )?;
        task["status"] = json!("running");
        store.put("task", task["id"].as_str().unwrap(), &task)?;
        let store = Arc::new(Mutex::new(store));
        let registry = ExternalRegistry::new();
        let (live, outcome) = registry.register(
            &task,
            store.clone(),
            files.clone(),
            "external brief".into(),
            None,
            Arc::new(AtomicBool::new(false)),
        )?;
        Ok(Self {
            _temp: temp,
            store,
            files,
            task,
            registry,
            live,
            outcome,
        })
    }
    fn request(&self, id: &str, revision: u64, path: &str, content: &str) -> RunRequest {
        RunRequest {
            task_id: self.live.task_id.clone(),
            run_id: self.live.run_id.clone(),
            revision,
            request_id: id.into(),
            arguments: json!({"tool":"file.write","arguments":{"path":path,"content":content,"expectedSha256":null}}),
        }
    }
}

#[tokio::test]
async fn successful_request_is_replayed_without_side_effects_and_conflicts_are_rejected(
) -> Result<()> {
    let f = Fixture::new()?;
    let request = f.request("write", 0, "new.txt", "once");
    let (a, b) = tokio::join!(
        f.registry.tool(request.clone()),
        f.registry.tool(request.clone())
    );
    let first = a?;
    assert_eq!(first["status"], "succeeded");
    assert_eq!(first, b?);
    fs::write(f.live.workspace.join("new.txt"), "human changed")?;
    assert_eq!(f.registry.tool(request.clone()).await?, first);
    assert_eq!(
        fs::read_to_string(f.live.workspace.join("new.txt"))?,
        "human changed"
    );
    let mut changed = request;
    changed.arguments["arguments"]["content"] = json!("different");
    assert!(f
        .registry
        .tool(changed)
        .await
        .unwrap_err()
        .to_string()
        .contains("conflict"));
    assert!(f
        .registry
        .tool(f.request("stale", 0, "other.txt", "no"))
        .await
        .is_err());
    assert!(!f.live.workspace.join("other.txt").exists());
    assert_eq!(f.registry.context(&f.live.task_id)?["revision"], 1);
    Ok(())
}

#[tokio::test]
async fn concurrent_new_requests_consume_a_revision_only_once() -> Result<()> {
    let f = Fixture::new()?;
    let (a, b) = tokio::join!(
        f.registry.tool(f.request("a", 0, "a.txt", "a")),
        f.registry.tool(f.request("b", 0, "b.txt", "b"))
    );
    assert_ne!(a.is_ok(), b.is_ok());
    assert_ne!(
        f.live.workspace.join("a.txt").exists(),
        f.live.workspace.join("b.txt").exists()
    );
    let other = ExternalRegistry::new();
    assert!(other.context(&f.live.task_id).is_err());
    assert!(other
        .register(
            &f.task,
            f.store.clone(),
            f.files.clone(),
            String::new(),
            None,
            Arc::new(AtomicBool::new(false))
        )
        .is_err());
    Ok(())
}

#[tokio::test]
async fn cancelled_old_runs_reject_calls_and_preserve_the_workspace() -> Result<()> {
    let f = Fixture::new()?;
    f.registry
        .tool(f.request("before", 0, "kept.txt", "keep"))
        .await?;
    f.registry.cancel_all()?;
    assert!(f
        .registry
        .tool(f.request("after", 1, "missing.txt", "no"))
        .await
        .is_err());
    f.live.close("revoked").await?;
    assert_eq!(
        fs::read_to_string(f.live.workspace.join("kept.txt"))?,
        "keep"
    );
    assert!(!f.live.workspace.join("missing.txt").exists());
    assert_eq!(f.registry.pending()?, json!([]));
    assert!(f
        .registry
        .register(
            &f.task,
            f.store.clone(),
            f.files.clone(),
            String::new(),
            None,
            Arc::new(AtomicBool::new(false))
        )
        .is_err());
    Ok(())
}

#[tokio::test]
async fn failed_requests_are_receipts_and_do_not_automatically_retry() -> Result<()> {
    let f = Fixture::new()?;
    let request = f.request("bad", 0, "../escape.txt", "blocked");
    let receipt = f.registry.tool(request.clone()).await?;
    assert_eq!(receipt["status"], "failed");
    assert_eq!(f.registry.tool(request).await?, receipt);
    assert_eq!(f.registry.context(&f.live.task_id)?["revision"], 1);
    Ok(())
}

#[tokio::test]
async fn external_plan_uses_shared_validation_and_inherits_execution_mode() -> Result<()> {
    let mut f = Fixture::new()?;
    f.task["decompose"] = json!(true);
    f.store
        .lock()
        .unwrap()
        .put("task", &f.live.task_id, &f.task)?;
    let plan = json!({"summary":"Two stages","steps":[
        {"title":"Create","prompt":"Create","direction":"visual","acceptance":"Present"},
        {"title":"Review","prompt":"Review","direction":"review","acceptance":"Checked"}]});
    let request = RunRequest {
        arguments: plan,
        ..f.request("plan", 0, "unused", "unused")
    };
    assert_eq!(
        f.registry.submit_plan(request).await?["status"],
        "succeeded"
    );
    assert_eq!(f.outcome.await?, Outcome::Completed);
    f.live.close("finished").await?;
    let mut store = f.store.lock().unwrap();
    let parent = crate::task_finish::finish(
        &mut store,
        &f.files,
        &f.live.task_id,
        Outcome::Completed,
        &AtomicBool::new(false),
    )?;
    assert_eq!(parent["status"], "waitingChildren");
    assert!(parent["threadId"].is_null() && parent["turnId"].is_null());
    let children: Vec<Value> = store
        .list::<Value>("task")?
        .into_iter()
        .filter(|task| task["parentTaskId"] == f.live.task_id)
        .collect();
    assert_eq!(children.len(), 2);
    assert!(children
        .iter()
        .all(|task| task["executionMode"] == "external-agent"));
    assert!(crate::task_plan::submit(
        &store,
        &f.files,
        &f.live.task_id,
        &json!({"threadId":"fake","arguments":{}})
    )
    .is_err());
    Ok(())
}

#[test]
fn caller_cannot_supply_host_or_codex_identity() {
    for field in ["owner", "threadId", "turnId"] {
        let mut input = json!({"taskId":"task","runId":"run","requestId":"request","revision":0,"arguments":{}});
        input[field] = json!("forged");
        assert!(serde_json::from_value::<RunRequest>(input).is_err());
    }
}

#[tokio::test]
async fn review_and_planning_runs_cannot_mutate_their_workspaces() -> Result<()> {
    for mode in ["review", "planning"] {
        let mut f = Fixture::new()?;
        if mode == "review" {
            f.task["capability"] = json!("review");
        } else {
            f.task["decompose"] = json!(true);
        }
        f.store
            .lock()
            .unwrap()
            .put("task", &f.live.task_id, &f.task)?;
        let receipt = f
            .registry
            .tool(f.request("blocked", 0, "blocked.txt", "blocked"))
            .await?;
        assert_eq!(receipt["status"], "failed");
        assert!(!f.live.workspace.join("blocked.txt").exists());
        let context = f.registry.context(&f.live.task_id)?;
        let tools = context["tools"]["tools"].as_array().unwrap();
        assert!(tools
            .iter()
            .all(|tool| ["file.read", "workflow.list", "workflow.run"]
                .contains(&tool["name"].as_str().unwrap())));
        let workflow = tools
            .iter()
            .find(|tool| tool["name"] == "workflow.run")
            .unwrap();
        assert_eq!(
            workflow["inputSchema"]["properties"]["action"]["enum"],
            json!(["inspect"])
        );
        assert!(!crate::external_run_permissions::allowed(
            &f.task,
            "workflow.run",
            &json!({"action":"validate"})
        ));
        assert!(crate::external_run_permissions::allowed(
            &f.task,
            "workflow.run",
            &json!({"action":"inspect"})
        ));
        let read = RunRequest {
            arguments: json!({"tool":"file.read","arguments":{"path":"before.txt"}}),
            ..f.request("read", 1, "unused", "unused")
        };
        assert_eq!(f.registry.tool(read).await?["status"], "succeeded");
    }
    Ok(())
}

#[path = "external_run_recovery_tests.rs"]
mod recovery_tests;
