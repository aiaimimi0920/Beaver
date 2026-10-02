use super::*;
use beaver_core::{
    object_run_preparation as preparation, object_tasks, project_derivation_copy,
    project_derivation_identity::Target,
};
use sha2::{Digest, Sha256};

#[path = "migration_derivation_queue_fixture.rs"]
mod fixture;

#[test]
fn migration_derivation_queue_api_preserves_replay_and_requires_explicit_resume_after_reopen(
) -> Result<()> {
    let _operation = super::super::super::TEST_OPERATION.lock().unwrap();
    let f = Fixture::new()?;
    let history = fixture::source_history(&f)?;
    let source_before = data_backup::inventory(&f.source)?;
    let inspection = f.api(
        "migration.inspectDerivationSource",
        json!({"source":f.source}),
    )?;
    let project = inspection["request"]["targetProjectId"].as_str().unwrap();
    let mut request = inspection["request"].clone();
    request["preparation"] = json!(f.preparation);
    f.api("migration.prepareDerivation", request)?;
    let prepared = project_derivation_copy::inspect(&f.preparation)?;
    let mapped = |kind: &str, id: &str| -> String {
        let entry = prepared
            .identities
            .entities
            .iter()
            .find(|e| e.source.kind == kind && e.source.id == id)
            .unwrap();
        match &entry.target {
            Target::Remap { key } => key.id.clone(),
            _ => panic!("unexpected archived queue"),
        }
    };
    let build = mapped("object_task", "build");
    let free = mapped("object_task", "free");
    let second = mapped("object_task", "second");
    let prepared_before = data_backup::inventory(&f.preparation)?;
    f.api("migration.assembleDerivation", f.paths())?;
    f.api("migration.activateAssembly", f.paths())?;
    f.api("migration.registerAssembly", f.paths())?;
    let input = json!({"projectId":project});
    let snapshot = f.task_api("objectTask.snapshot", input.clone())?;
    let view = f.task_api("objectTask.queueView", input.clone())?;
    assert_eq!(snapshot["planRevision"], 3);
    assert_eq!(
        view["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["taskId"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [free.as_str(), build.as_str(), second.as_str()]
    );
    assert_eq!(view["items"][1]["title"], "Revised build");
    assert_eq!(view["items"][2]["state"], "cancelled");
    assert!(view["items"].as_array().unwrap()[..2]
        .iter()
        .all(|i| i["blockers"].as_array().unwrap().contains(&json!("paused"))));
    let runtime = f.router.runtime_for_project(project)?;
    for (kind, source) in history {
        let source_request = source["request"]["requestId"].as_str().unwrap();
        let source_key = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&("original", source_request))?)
        );
        let receipt = runtime
            .store()
            .lock()
            .unwrap()
            .get::<Value>(kind, &mapped(kind, &source_key))?
            .unwrap();
        let method = match kind {
            "object_task_queue_reorder_receipt" => "objectTask.reorder",
            "object_task_coarse_dispatch_receipt" => "objectTask.setCoarsePaused",
            _ => "objectTask.setPaused",
        };
        assert_eq!(receipt["request"]["projectId"], project);
        assert_ne!(receipt["request"]["requestId"], source_request);
        if kind == "object_task_queue_reorder_receipt" {
            assert_eq!(receipt["result"]["items"][1]["title"], "Build");
            assert_ne!(receipt["result"]["version"], source["result"]["version"]);
            assert_ne!(receipt["result"]["version"], view["version"]);
        }
        assert_eq!(f.task_api(method, receipt["request"].clone())?, receipt);
        let mut conflict = receipt["request"].clone();
        if kind == "object_task_queue_reorder_receipt" {
            conflict["expectedVersion"] = view["version"].clone();
        } else {
            conflict["paused"] = json!(true);
        }
        assert!(f
            .task_api(method, conflict)
            .unwrap_err()
            .to_string()
            .contains("REQUEST_CONFLICT"));
    }
    assert_eq!(f.task_api("objectTask.snapshot", input.clone())?, snapshot);
    assert_eq!(f.task_api("objectTask.queueView", input.clone())?, view);
    assert_eq!(
        f.task_api(
            "objectTask.claim",
            json!({"projectId":project,"owner":"desktop-direct"})
        )?,
        Value::Null
    );
    assert!(preparation::claim_next(&runtime, project, "desktop-scheduler")?.is_none());
    for run in snapshot["runs"].as_array().unwrap() {
        assert!(preparation::get(&runtime, project, run["id"].as_str().unwrap())?.is_none());
    }
    drop(runtime);
    f.router.close(project)?;
    f.api("migration.registerAssembly", f.paths())?;
    f.router.open_registered(project)?;
    assert_eq!(f.task_api("objectTask.queueView", input.clone())?, view);
    let runtime = f.router.runtime_for_project(project)?;
    assert!(preparation::claim_next(&runtime, project, "desktop-reopen")?.is_none());
    let control = snapshot["dispatchControls"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["taskId"] == build)
        .unwrap();
    let task = snapshot["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"] == build)
        .unwrap();
    let resume = json!({
        "projectId":project,"taskId":build,"objectId":control["objectId"],"runId":control["runId"],
        "requestId":"explicit-resume-copy","expectedTaskRevision":task["revision"],
        "expectedControlRevision":control["revision"],"paused":false
    });
    let resumed = f.task_api("objectTask.setPaused", resume.clone())?;
    assert_eq!(resumed["result"]["revision"], 4);
    assert_eq!(f.task_api("objectTask.setPaused", resume)?, resumed);
    let claim = preparation::claim_next(&runtime, project, "desktop-authorized")?.unwrap();
    assert_eq!(claim.record().medium.id, build);
    assert_eq!(claim.record().generation, 1);
    assert_eq!(data_backup::inventory(&f.source)?, source_before);
    assert_eq!(data_backup::inventory(&f.preparation)?, prepared_before);
    assert!(f
        .store
        .lock()
        .unwrap()
        .list::<Value>("object_task_queue")?
        .is_empty());
    Ok(())
}
