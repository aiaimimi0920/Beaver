use super::*;
use beaver_core::{object_tasks, scheduler_runtime::TaskRuntime};
use std::time::Duration;

pub(super) fn scheduler(f: &Fixture, project: &str, calls: Arc<AtomicUsize>) -> Result<Scheduler> {
    let runtime = TaskRuntime::from_project(f.router.runtime_for_project(project)?);
    Ok(Scheduler::start_with_objects(
        Arc::new(move || Ok(vec![runtime.clone()])),
        Arc::new(|_, _| panic!("object controls must not call legacy factory")),
        Arc::new(move |_, _| {
            calls.fetch_add(1, Ordering::SeqCst);
            Err("simulated launch failure".into())
        }),
        Arc::new(|| {}),
        Arc::new(|| Ok(1)),
    ))
}

pub(super) async fn synchronize(s: &Scheduler, project: &str, medium: &str) -> Result<()> {
    s.wake().map_err(anyhow::Error::msg)?;
    tokio::time::timeout(
        Duration::from_secs(10),
        s.synchronize(format!("object:{}:{project}{medium}", project.len())),
    )
    .await?
    .map_err(anyhow::Error::msg)
}

pub(super) async fn finished(
    s: &Scheduler,
    f: &Fixture,
    project: &str,
    medium: &str,
    run: &str,
    count: usize,
) -> Result<Value> {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let views = call(
                s,
                &f.router,
                "objectTask.attempts",
                json!({"projectId":project,"runId":run}),
            )
            .await?;
            if views.as_array().is_some_and(|items| {
                items.len() == count && items.iter().all(|item| item["availability"] == "finished")
            }) {
                synchronize(s, project, medium).await?;
                return Ok::<_, anyhow::Error>(views);
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await?
}

pub(super) fn unpause(
    f: &Fixture,
    project: &str,
    medium: &str,
    request: &str,
    paused: bool,
) -> Result<Value> {
    let snapshot = f.task_api("objectTask.snapshot", json!({"projectId":project}))?;
    let task = snapshot["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"] == medium)
        .unwrap();
    let control_revision = snapshot["dispatchControls"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["taskId"] == medium)
        .map_or(json!(0), |c| c["revision"].clone());
    f.task_api(
        "objectTask.setPaused",
        json!({
            "projectId":project,"taskId":medium,"objectId":task["objectId"],"runId":task["runId"],
            "requestId":request,"expectedTaskRevision":task["revision"],
            "expectedControlRevision":control_revision,"paused":paused
        }),
    )
}

pub(super) struct Source {
    pub run: String,
    pub attempt: Value,
    pub check: Value,
    pub unpaused: Value,
}

pub(super) async fn source_history(f: &Fixture) -> Result<Source> {
    source_history_with_successor(f, false).await
}

pub(super) async fn source_history_with_successor(f: &Fixture, successor: bool) -> Result<Source> {
    source_history_with_stages(f, if successor { 2 } else { 1 }).await
}

pub(super) async fn source_history_with_stages(f: &Fixture, stages: usize) -> Result<Source> {
    // This fixture's legacy queued task is unrelated to the object execution consumer.
    // Remove only its three temporary seed records, never production application data.
    let storage = ProjectStore::open(&f.source, "original")?;
    for status in ["running", "queued", "waitingChildren"] {
        storage.store().remove("task", status)?;
    }
    drop(storage);
    f.store.lock().unwrap().put(
        "project",
        "original",
        &json!({"id":"original","path":f.source}),
    )?;
    let runtime = f.router.open_registered("original")?;
    let mut draft = json!({
        "projectId":"original","draftId":"execution-plan","expectedRevision":0,"expectedPlanRevision":0,
        "plan":{"objects":[{"id":"object","name":"Execution object"}],"tasks":[
            {"id":"build","granularity":"medium","objectId":"object","title":"Build","prompt":"Preserved prompt","acceptance":"","baseline":{"basePolicy":"empty"}},
            {"id":"fine","granularity":"fine","parentTaskId":"build","objectId":"object","stageId":"files","title":"Files","prompt":"Keep files","acceptance":""}
        ]}
    });
    if stages >= 2 {
        draft["plan"]["tasks"].as_array_mut().unwrap().push(json!({
            "id":"later-fine","position":1,"granularity":"fine","parentTaskId":"build",
            "objectId":"object","stageId":"review","dependsOn":["fine"],
            "title":"Review","prompt":"Review files","acceptance":""
        }));
    }
    if stages == 3 {
        draft["plan"]["tasks"].as_array_mut().unwrap().push(json!({
            "id":"final-fine","position":2,"granularity":"fine","parentTaskId":"build",
            "objectId":"object","stageId":"final","dependsOn":["later-fine"],
            "title":"Final","prompt":"Finalize files","acceptance":""
        }));
    }
    f.task_api("objectTask.saveDraft", draft)?;
    f.task_api("objectTask.commit", json!({"projectId":"original","requestId":"execution-commit","draftId":"execution-plan","expectedDraftRevision":1,"expectedPlanRevision":0}))?;
    let run = object_tasks::snapshot(&runtime, "original")?.runs[0]
        .id
        .clone();
    object_tasks::enqueue(&runtime, "original", &["build".into()])?;
    unpause(f, "original", "build", "source-pause", true)?;
    let unpaused = unpause(f, "original", "build", "source-unpause", false)?;
    let calls = Arc::new(AtomicUsize::new(0));
    let s = scheduler(f, "original", calls.clone())?;
    let attempts = finished(&s, f, "original", "build", &run, 1).await?;
    assert_eq!(attempts[0]["attempt"]["state"], "failed");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let check = call(&s, &f.router, "objectTask.checkAttempt", json!({"projectId":"original","requestId":"source-check","target":attempts[0]["attempt"]["target"]})).await?;
    s.shutdown().await.map_err(anyhow::Error::msg)?;
    drop(s);
    drop(runtime);
    f.router.close("original")?;
    let other = f._temp.path().join("other");
    fs::create_dir(&other)?;
    fs::write(other.join("project.godot"), "config_version=5\n")?;
    drop(ProjectStore::initialize(&other, "other")?);
    f.store
        .lock()
        .unwrap()
        .put("project", "other", &json!({"id":"other","path":other}))?;
    f.router.open_registered("other")?;
    Ok(Source {
        run,
        attempt: attempts[0]["attempt"].clone(),
        check,
        unpaused,
    })
}

pub(super) fn mapped(prepared: &project_derivation_copy::Prepared, kind: &str, id: &str) -> String {
    let entry = prepared
        .identities
        .entities
        .iter()
        .find(|e| e.source.kind == kind && e.source.id == id)
        .unwrap();
    match &entry.target {
        beaver_core::project_derivation_identity::Target::Remap { key } => key.id.clone(),
        _ => panic!("unexpected archived execution identity"),
    }
}
