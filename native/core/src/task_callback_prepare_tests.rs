use super::*;
use crate::{asset_task, store::Store, task_callback};
use serde_json::json;
use std::path::PathBuf;

struct Fixture {
    temp: tempfile::TempDir,
    workspace: PathBuf,
    store: Store,
    request: Value,
}

impl Fixture {
    fn new(operation: &str) -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("workspace");
        std::fs::create_dir(&workspace)?;
        std::fs::write(workspace.join("source.blend"), b"saved source")?;
        let mut store = Store::open(temp.path())?;
        let task = json!({"id":"task","projectId":"project","status":"running",
            "threadId":"thread","turnId":"turn","workspace":workspace});
        store.put("task", "task", &task)?;
        asset_task::enable(&store, &task)?;
        task_callback::call(
            &mut store,
            "task",
            "thread",
            "turn",
            &json!({
                "operation":"deliveryPlan","requestId":"plan","expectedRevision":0,
                "assetRevision":0,"template":{"id":"fixture","version":1,
                    "stages":[{"id":"design","name":"Design"},{"id":"model","name":"Model"}]}
            }),
        )?;
        let created = task_callback::call(
            &mut store,
            "task",
            "thread",
            "turn",
            &json!({
                "operation":"work","requestId":"create","expectedRevision":1,
                "assetRevision":1,"stageId":"design","change":{"action":"create",
                    "definition":{"title":"Brief","goal":"Save a brief","acceptance":"Reviewable"}}
            }),
        )?;
        let request = json!({"operation":"work","requestId":"begin","expectedRevision":2,
            "assetRevision":2,"stageId":"design","change":{"action":"begin",
                "subtaskId":created["subtaskId"],"inputs":{},
                "inputFiles":[{"path":"source.blend","role":"source"}]}});
        let mut fixture = Self {
            temp,
            workspace,
            store,
            request,
        };
        if operation != "begin" {
            let proof = fixture.prepare()?;
            let begun = fixture.commit(&proof, &fixture.request.clone())?;
            fixture.request = json!({"operation":"work","requestId":"finish","expectedRevision":3,
                "assetRevision":3,"stageId":"design","change":{"action":"finish",
                    "attemptId":begun["attemptId"],"outcome":"completed","summary":"Saved",
                    "outputs":{"path":"source.blend"}}});
        }
        if operation == "submit" {
            let proof = fixture.prepare()?;
            fixture.commit(&proof, &fixture.request.clone())?;
            fixture.request = json!({"operation":"submitDelivery","requestId":"submit",
                "expectedRevision":4,"assetRevision":4,"stageId":"design",
                "inputCandidates":[],"paths":["source.blend"],"summary":"Saved candidate"});
        }
        Ok(fixture)
    }

    fn prepare(&self) -> Result<Prepared> {
        let state = asset_task::get(&self.store, "task")?;
        let request = serde_json::from_value(self.request.clone())?;
        preflight(&state, &request)?;
        Prepared::capture(
            &Files::new(self.temp.path().into()),
            &self.workspace,
            &state,
            &request,
            &self.request,
        )
    }

    fn commit(&mut self, proof: &Prepared, request: &Value) -> Result<Value> {
        task_callback::call_prepared(
            &mut self.store,
            "task",
            "thread",
            "turn",
            request,
            Some(proof),
        )
    }

    fn snapshot(&self) -> Result<Value> {
        Ok(json!({"task":self.store.get::<Value>("task", "task")?,
            "asset":asset_task::get(&self.store, "task")?,
            "callback":task_callback::inspect(&self.store, "task", None)?}))
    }

    fn assert_rejected(&mut self, proof: &Prepared, request: &Value, message: &str) -> Result<()> {
        let before = self.snapshot()?;
        let error = self.commit(proof, request).unwrap_err().to_string();
        assert!(error.contains(message), "{error}");
        assert_eq!(self.snapshot()?, before);
        assert!(
            task_callback::inspect(&self.store, "task", request["requestId"].as_str())?["receipt"]
                .is_null()
        );
        Ok(())
    }
}

#[test]
fn prepared_commit_rechecks_revisions_and_identity_after_file_io() -> Result<()> {
    for operation in ["begin", "finish", "submit"] {
        for change in ["callback", "asset", "threadId", "turnId"] {
            let mut f = Fixture::new(operation)?;
            let proof = f.prepare()?;
            // Deterministically interleave a writer at the runtime's lock-free IO boundary.
            let message = match change {
                "callback" => {
                    task_callback::call(
                        &mut f.store,
                        "task",
                        "thread",
                        "turn",
                        &json!({
                            "operation":"report","requestId":"interleaved",
                            "expectedRevision":f.request["expectedRevision"],
                            "report":{"kind":"progress","summary":"Other callback","inputs":{},"outputs":{}}
                        }),
                    )?;
                    "Callback revision changed"
                }
                "asset" => {
                    let mut state = asset_task::get(&f.store, "task")?;
                    state.revision += 1;
                    asset_task::save(&f.store, &state)?;
                    "Asset revision changed"
                }
                field => {
                    let mut task = f.store.get::<Value>("task", "task")?.unwrap();
                    task[field] = json!("new-execution");
                    f.store.put("task", "task", &task)?;
                    "Stale execution"
                }
            };
            f.assert_rejected(&proof, &f.request.clone(), message)?;
        }
    }
    Ok(())
}

#[test]
fn prepared_evidence_cannot_authorize_a_different_request() -> Result<()> {
    for operation in ["begin", "finish", "submit"] {
        let mut f = Fixture::new(operation)?;
        let proof = f.prepare()?;
        let specific = match operation {
            "begin" => ("/change/inputFiles", json!([])),
            "finish" => ("/change/attemptId", json!("different-attempt")),
            _ => ("/paths", json!(["another.blend"])),
        };
        for (pointer, value) in [
            ("/requestId", json!("another-request")),
            ("/assetRevision", json!(999)),
            ("/stageId", json!("model")),
            specific,
        ] {
            let mut altered = f.request.clone();
            *altered.pointer_mut(pointer).unwrap() = value;
            f.assert_rejected(&proof, &altered, "File preparation does not match callback")?;
        }
        f.commit(&proof, &f.request.clone())?;
    }
    Ok(())
}
