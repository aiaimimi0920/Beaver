use crate::{
    external_execution::ExternalLaunch,
    external_run_contract::RunRequest,
    external_runs::ExternalRegistry,
    files::Files,
    scheduler::{Launch, Scheduler},
    store::Store,
};
use anyhow::Result;
use serde_json::{json, Value};
use std::{
    fs,
    sync::{Arc, Mutex},
    time::Duration,
};

struct Fixture {
    _temp: tempfile::TempDir,
    store: Arc<Mutex<Store>>,
    registry: ExternalRegistry,
    scheduler: Scheduler,
    id: String,
    project: std::path::PathBuf,
}
impl Fixture {
    fn new() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("data");
        let project = temp.path().join("project");
        fs::create_dir(&project)?;
        fs::write(project.join("before.txt"), "original")?;
        let files = Arc::new(Files::new(root.clone()));
        let mut store = Store::open(&root)?;
        store.put("project", "p", &json!({"id":"p","path":project}))?;
        let task = crate::task_create::create(
            &mut store,
            &files,
            json!({"projectId":"p","prompt":"test","executionMode":"external-agent","decompose":false}),
            &Value::Null,
            &Value::Null,
        )?;
        let id = task["id"].as_str().unwrap().to_owned();
        let store = Arc::new(Mutex::new(store));
        let registry = ExternalRegistry::new();
        let factory_registry = registry.clone();
        let scheduler = Scheduler::start(
            store.clone(),
            files,
            Arc::new(move |_| {
                Ok(Launch {
                    external: Some(ExternalLaunch {
                        registry: factory_registry.clone(),
                        blender_path: None,
                    }),
                    command: None,
                    model: String::new(),
                    prompt: "test".into(),
                    ask_user_tool: Value::Null,
                    secrets: vec![],
                    max_minutes: 0,
                    blender: None,
                    godot: None,
                })
            }),
            Arc::new(|| {}),
            Arc::new(|| Ok(1)),
        );
        Ok(Self {
            _temp: temp,
            store,
            registry,
            scheduler,
            id,
            project,
        })
    }
    async fn context(&self) -> Value {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Ok(context) = self.registry.context(&self.id) {
                    break context;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("scheduler did not register external run")
    }
    fn request(&self, context: &Value, id: &str, arguments: Value) -> RunRequest {
        RunRequest {
            task_id: self.id.clone(),
            run_id: context["runId"].as_str().unwrap().into(),
            revision: context["revision"].as_u64().unwrap(),
            request_id: id.into(),
            arguments,
        }
    }
    async fn terminal(&self) -> Value {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let task = self
                    .store
                    .lock()
                    .unwrap()
                    .get::<Value>("task", &self.id)
                    .unwrap()
                    .unwrap();
                if task["status"] != "running" && task["status"] != "queued" {
                    break task;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("scheduler did not finish external run")
    }
}

#[tokio::test]
async fn completed_external_output_still_passes_validation_and_approval_before_merge() -> Result<()>
{
    let f = Fixture::new()?;
    let context = f.context().await;
    assert!(context["task"]["threadId"].is_null() && context["task"]["turnId"].is_null());
    let write = f.request(&context,"write",json!({"tool":"file.write","arguments":{"path":"result.txt","content":"validated","expectedSha256":null}}));
    assert_eq!(f.registry.tool(write).await?["status"], "succeeded");
    let context = f.registry.context(&f.id)?;
    assert_eq!(
        f.registry
            .finish(f.request(&context, "finish", json!({"outcome":"completed"})))
            .await?["status"],
        "succeeded"
    );
    let task = f.terminal().await;
    assert_eq!(task["status"], "completed");
    assert_eq!(task["codeValidation"]["status"], "notRequired");
    assert_eq!(task["accepted"], true);
    assert_eq!(
        fs::read_to_string(f.project.join("result.txt"))?,
        "validated"
    );
    f.registry.cancel_all()?;
    f.scheduler.shutdown().await.map_err(anyhow::Error::msg)?;
    f.registry.release_closed()?;
    Ok(())
}

#[tokio::test]
async fn externally_claimed_success_cannot_bypass_code_structure_or_forge_validation() -> Result<()>
{
    let f = Fixture::new()?;
    let context = f.context().await;
    let forged = f.request(
        &context,
        "forged",
        json!({"outcome":"completed","validationPassed":true}),
    );
    assert_eq!(f.registry.finish(forged).await?["status"], "failed");
    let context = f.registry.context(&f.id)?;
    let code = (0..710)
        .map(|index| format!("var value_{index} = {index}\n"))
        .collect::<String>();
    let write = f.request(&context,"oversized",json!({"tool":"file.write","arguments":{"path":"oversized.gd","content":code,"expectedSha256":null}}));
    assert_eq!(f.registry.tool(write).await?["status"], "succeeded");
    let context = f.registry.context(&f.id)?;
    f.registry
        .finish(f.request(&context, "finish", json!({"outcome":"completed"})))
        .await?;
    let task = f.terminal().await;
    assert_eq!(task["status"], "failed");
    assert!(!f.project.join("oversized.gd").exists());
    assert!(std::path::Path::new(task["workspace"].as_str().unwrap())
        .join("oversized.gd")
        .exists());
    f.registry.cancel_all()?;
    f.scheduler.shutdown().await.map_err(anyhow::Error::msg)?;
    f.registry.release_closed()?;
    Ok(())
}

#[tokio::test]
async fn scheduler_cancel_revokes_run_and_never_merges_retained_output() -> Result<()> {
    let f = Fixture::new()?;
    let context = f.context().await;
    let write = f.request(&context,"write",json!({"tool":"file.write","arguments":{"path":"kept.txt","content":"kept","expectedSha256":null}}));
    f.registry.tool(write).await?;
    f.scheduler
        .interrupt(f.id.clone())
        .await
        .map_err(anyhow::Error::msg)?;
    assert_eq!(f.terminal().await["status"], "interrupted");
    assert!(!f.project.join("kept.txt").exists());
    assert!(f
        .registry
        .tool(f.request(
            &context,
            "old",
            json!({"tool":"file.read","arguments":{"path":"before.txt"}})
        ))
        .await
        .is_err());
    f.registry.cancel_all()?;
    f.scheduler.shutdown().await.map_err(anyhow::Error::msg)?;
    f.registry.release_closed()?;
    Ok(())
}
