#![allow(dead_code)]
use anyhow::Result;
use beaver_core::{
    asset_delivery_review as review, asset_task, files::Files, project_storage::ProjectStore,
    store::Store, task_callback, task_callback_runtime,
};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

pub struct Fixture {
    pub temp: tempfile::TempDir,
    pub workspace: PathBuf,
    pub store: Arc<Mutex<Store>>,
    files: Arc<Files>,
}

impl Fixture {
    pub fn new() -> Result<Self> {
        Self::with_storage(false)
    }
    pub fn new_project() -> Result<Self> {
        Self::with_storage(true)
    }
    fn with_storage(project_backed: bool) -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let project = temp.path().join("project");
        std::fs::create_dir_all(&project)?;
        let (store, files, workspace) = if project_backed {
            std::fs::write(project.join("project.godot"), "config_version=5\n")?;
            let runtime = ProjectStore::initialize(&project, "project")?.into_runtime();
            let files = runtime.files();
            let workspace = files.workspace("task")?;
            (runtime.store(), files, workspace)
        } else {
            (
                Arc::new(Mutex::new(Store::open(temp.path())?)),
                Arc::new(Files::new(temp.path().into())),
                temp.path().join("workspace"),
            )
        };
        std::fs::create_dir_all(&workspace)?;
        let location = if project_backed {
            json!(files.workspace_location("task")?)
        } else {
            json!(workspace)
        };
        let task = json!({"id":"task","projectId":"project","status":"running","threadId":"thread","turnId":"turn","prompt":"Two reviewed stages","workspace":location,"baseline":{},"capability":"code","assetTask":true});
        let db = store.lock().unwrap();
        db.put(
            "project",
            "project",
            &json!({"id":"project","path":project}),
        )?;
        db.put("task", "task", &task)?;
        asset_task::enable(&db, &task)?;
        drop(db);
        Ok(Self {
            temp,
            workspace,
            store,
            files,
        })
    }
    pub fn files(&self) -> Arc<Files> {
        self.files.clone()
    }
    pub fn state(&self) -> Result<asset_task::State> {
        asset_task::get(&self.store.lock().unwrap(), "task")
    }
    pub fn task(&self) -> Result<Value> {
        Ok(self.store.lock().unwrap().get("task", "task")?.unwrap())
    }
    pub fn resume(&self, turn: &str) -> Result<()> {
        let mut task = self.task()?;
        task["status"] = json!("running");
        task["turnId"] = json!(turn);
        self.store.lock().unwrap().put("task", "task", &task)
    }
    pub async fn call(&self, request: Value) -> Result<Value> {
        task_callback_runtime::call(
            self.store.clone(),
            self.files(),
            "task".into(),
            "thread".into(),
            self.task()?["turnId"].as_str().unwrap_or("").into(),
            request,
        )
        .await
    }
    pub async fn plan(&self) -> Result<Value> {
        self.call(json!({"operation":"deliveryPlan","requestId":"plan","expectedRevision":0,"assetRevision":0,
            "template":{"id":"two-stage","version":1,"stages":[{"id":"design","name":"Design"},{"id":"model","name":"Model"}]}})).await
    }
    pub fn request(&self, request: &str, paths: &[&str]) -> Result<Value> {
        let state = self.state()?;
        let flow = state.delivery.as_ref().unwrap();
        Ok(json!({"operation":"submitDelivery","requestId":request,
            "expectedRevision":task_callback::inspect(&self.store.lock().unwrap(), "task", None)?["revision"],
            "assetRevision":state.revision,"stageId":state.stages[flow.approved.len()].id,
            "inputCandidates":flow.approved,"paths":paths,"summary":"Saved candidate"}))
    }
    pub fn decision(&self, candidate: &str, decision: &str) -> Result<review::Decision> {
        Ok(review::Decision {
            id: "task".into(),
            request_id: uuid::Uuid::new_v4().to_string(),
            expected_revision: self.state()?.revision,
            candidate_id: candidate.into(),
            decision: decision.into(),
            note: "Reviewed fixture".into(),
        })
    }
    pub fn decide(&self, input: review::Decision) -> Result<Value> {
        let prepared = review::prepare(&self.store.lock().unwrap(), input)?;
        let verified = prepared.verify(&self.files())?;
        review::decide(&mut self.store.lock().unwrap(), verified)
    }
}
