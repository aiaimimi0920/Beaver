use anyhow::Result;
use beaver_core::{
    project_storage::ProjectStore, project_storage_router::ProjectStorageRouter, store::Store,
};
use serde_json::{json, Value};
use std::{
    fs,
    sync::{Arc, Mutex},
};

pub(crate) struct Fixture {
    pub(crate) router: ProjectStorageRouter,
    pub(crate) host: Arc<Mutex<Store>>,
    pub(crate) temp: tempfile::TempDir,
}

impl Fixture {
    pub(crate) fn new() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let host = Arc::new(Mutex::new(Store::open(&temp.path().join("host"))?));
        let router = ProjectStorageRouter::new(host.clone());
        for id in ["p", "other", "closed"] {
            let root = temp.path().join(id);
            fs::create_dir(&root)?;
            fs::write(root.join("project.godot"), "config_version=5\n")?;
            let project = ProjectStore::initialize(&root, id)?;
            project.store().put("project", id, &json!({"id":id}))?;
            drop(project);
            host.lock()
                .unwrap()
                .put("project", id, &json!({"id":id,"path":root}))?;
            if id != "closed" {
                drop(router.open_registered(id)?);
            }
        }
        host.lock()
            .unwrap()
            .put("project", "legacy", &json!({"id":"legacy"}))?;
        Ok(Self { router, host, temp })
    }
}

pub(crate) fn draft_input() -> Value {
    json!({
        "projectId":"p","draftId":"draft","expectedRevision":0,"expectedPlanRevision":0,
        "plan":{
            "objects":[{"id":"hero","name":"Hero"}],
            "assumptions":[{
                "id":"lighting",
                "statement":"Use warm lighting indoors.",
                "basis":"The brief requests a welcoming interior.",
                "source":"automatic",
                "sourceDetail":"Initial plan synthesis"
            }],
            "tasks":[
                {"id":"coarse","granularity":"coarse","title":"Game","prompt":"Make game","acceptance":"Playable"},
                {"id":"medium","granularity":"medium","title":"Hero","prompt":"Make hero","acceptance":"Moves","objectId":"hero","parentTaskId":"coarse"},
                {"id":"fine","granularity":"fine","title":"Movement","prompt":"Add movement","acceptance":"Moves","objectId":"hero","parentTaskId":"medium","stageId":"movement"}
            ]
        }
    })
}

pub(crate) fn commit_input() -> Value {
    json!({
        "projectId":"p","requestId":"commit","draftId":"draft",
        "expectedDraftRevision":1,"expectedPlanRevision":0
    })
}
