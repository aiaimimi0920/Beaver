use crate::{
    files::Files, store::Store, task_actions, task_plan, task_supplement,
    validation::task_completion,
};
use anyhow::Result;
use serde_json::{json, Value};
use std::fs;

fn fixture() -> Result<(tempfile::TempDir, Store, Files)> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("data");
    let store = Store::open(&root)?;
    let files = Files::new(root);
    let workspace = files.workspace("supplement")?;
    fs::create_dir_all(&workspace)?;
    fs::write(workspace.join("retained.txt"), "accepted model")?;
    let baseline = files.capture(&workspace)?;
    store.put(
        "project",
        "project",
        &json!({"id":"project","path":temp.path()}),
    )?;
    store.put("task", "parent", &json!({"id":"parent","projectId":"project","status":"waitingChildren","integrationValidation":true,"validationVersion":1,"plan":{"summary":"Keep accepted models","steps":[{}]},"subtaskIds":["original","supplement"],"codeValidation":{"status":"cancelled","runId":"old"}}))?;
    store.put("task", "original", &json!({"id":"original","projectId":"project","parentTaskId":"parent","status":"completed","accepted":true}))?;
    store.put("task", "supplement", &json!({"id":"supplement","projectId":"project","parentTaskId":"parent","origin":"user","title":"落实集成验收期间的补充要求","prompt":"Resume validation only","status":"failed","error":"请先配置服务地址","workspacePrepared":true,"workspace":workspace,"baseline":baseline,"changes":[],"conflicts":[]}))?;
    store.event(
        "parent",
        "2026-10-21",
        "validationSteering",
        "supplement saved",
    )?;
    Ok((temp, store, files))
}

#[test]
fn withdrawal_preserves_history_and_requires_real_integration() -> Result<()> {
    let (_temp, mut store, files) = fixture()?;
    let record = task_supplement::withdraw(
        &mut store,
        &files,
        "supplement",
        "parent",
        "Unintended redundant instruction",
    )?;
    let child: Value = store.get("task", "supplement")?.unwrap();
    assert_eq!(child["status"], "failed");
    assert_eq!(child["error"], "请先配置服务地址");
    assert!(child["accepted"].is_null());
    assert_eq!(store.events("supplement")?.len(), 1);
    assert_eq!(
        task_supplement::withdraw(
            &mut store,
            &files,
            "supplement",
            "parent",
            "Unintended redundant instruction"
        )?,
        record
    );
    assert_eq!(store.events("supplement")?.len(), 1);
    task_plan::reconcile(&mut store, &files)?;
    let parent: Value = store.get("task", "parent")?.unwrap();
    assert_eq!(parent["status"], "queued");
    assert_eq!(parent["validationOnly"], true);
    assert_eq!(parent["codeValidation"]["status"], "cancelled");
    assert_eq!(parent["subtaskIds"], json!(["original", "supplement"]));
    assert!(task_actions::continue_task(&mut store, "supplement", "", false).is_err());
    assert!(!task_plan::eligible(&child, &[parent, child.clone()]));
    Ok(())
}

#[test]
fn unsafe_supplements_reject_without_mutation() -> Result<()> {
    for patch in [
        json!({"status":"running"}),
        json!({"accepted":true}),
        json!({"sessionHistory":[{"threadId":"old"}]}),
        json!({"restartContext":{}}),
        json!({"migrationRetained":{}}),
        json!({"threadId":"started"}),
        json!({"externalRunId":"run"}),
        json!({"codeValidation":{"status":"cancelled"}}),
        json!({"origin":"validationRepair"}),
        json!({"projectId":"foreign"}),
        json!({"parentTaskId":"foreign"}),
        json!({"changes":[{}]}),
        json!({"conflicts":[{}]}),
        json!({"externalRecoveryRequired":{"runId":"unknown"}}),
    ] {
        let (_temp, mut store, files) = fixture()?;
        let mut child: Value = store.get("task", "supplement")?.unwrap();
        for (key, value) in patch.as_object().unwrap() {
            child[key] = value.clone();
        }
        store.put("task", "supplement", &child)?;
        assert!(
            task_supplement::withdraw(&mut store, &files, "supplement", "parent", "cleanup")
                .is_err(),
            "{patch}"
        );
        assert_eq!(store.get::<Value>("task", "supplement")?.unwrap(), child);
        assert!(store.events("supplement")?.is_empty());
    }
    Ok(())
}

#[test]
fn dirty_workspace_dependents_and_original_step_are_protected() -> Result<()> {
    for scenario in ["dirty", "dependent", "original", "provenance"] {
        let (_temp, mut store, files) = fixture()?;
        match scenario {
            "dirty" => fs::write(
                files.workspace("supplement")?.join("retained.txt"),
                "changed",
            )?,
            "dependent" => store.put(
                "task",
                "later",
                &json!({"id":"later","dependsOn":["supplement"]}),
            )?,
            "original" => {
                let mut p: Value = store.get("task", "parent")?.unwrap();
                p["plan"]["steps"] = json!([{}, {}]);
                store.put("task", "parent", &p)?;
            }
            _ => {
                let mut c: Value = store.get("task", "supplement")?.unwrap();
                c["title"] = json!("Unrelated task");
                store.put("task", "supplement", &c)?;
            }
        }
        assert!(
            task_supplement::withdraw(&mut store, &files, "supplement", "parent", "cleanup")
                .is_err(),
            "{scenario}"
        );
    }
    Ok(())
}

#[test]
fn paused_parent_and_other_failed_children_remain_blocked() -> Result<()> {
    let (_temp, mut store, files) = fixture()?;
    let mut parent: Value = store.get("task", "parent")?.unwrap();
    parent["planPaused"] = json!(true);
    store.put("task", "parent", &parent)?;
    task_supplement::withdraw(&mut store, &files, "supplement", "parent", "cleanup")?;
    task_plan::reconcile(&mut store, &files)?;
    assert_eq!(
        store.get::<Value>("task", "parent")?.unwrap()["status"],
        "waitingChildren"
    );
    task_actions::continue_task(&mut store, "parent", "", false)?;
    assert_eq!(
        store.get::<Value>("task", "supplement")?.unwrap()["status"],
        "failed"
    );
    let mut original: Value = store.get("task", "original")?.unwrap();
    original["accepted"] = json!(false);
    store.put("task", "original", &original)?;
    task_plan::reconcile(&mut store, &files)?;
    parent = store.get("task", "parent")?.unwrap();
    assert_eq!(parent["status"], "waitingChildren");
    assert!(task_completion::integrated(&store, &mut parent).is_err());
    Ok(())
}

#[test]
fn steering_keeps_external_mode_and_empty_resume_is_not_steering() -> Result<()> {
    let (_temp, mut store, files) = fixture()?;
    let mut parent: Value = store.get("task", "parent")?.unwrap();
    parent["executionMode"] = json!("external-agent");
    parent["status"] = json!("running");
    store.put("task", "parent", &parent)?;
    assert!(!task_actions::continue_task(
        &mut store, "parent", "", false
    )?);
    task_completion::steered(&mut store, &files, &mut parent, "Real new instruction")?;
    let id = parent["subtaskIds"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()
        .as_str()
        .unwrap();
    let child: Value = store.get("task", id)?.unwrap();
    assert_eq!(child["executionMode"], "external-agent");
    assert!(child["externalRunId"].is_null());
    assert!(child["threadId"].is_null());
    Ok(())
}

#[test]
fn withdrawn_dependency_never_replaces_gut_receipt() -> Result<()> {
    use crate::validation::{repository, task_gate, test_support::Fixture};
    let fixture = Fixture::new()?;
    let mut parent = json!({"id":"parent","projectId":"p","status":"running","capability":"code","validationVersion":1,"integrationValidation":true,"subtaskIds":["original","withdrawn"],"plan":{"summary":"Accepted children"}});
    fixture.store.put("task", "original", &json!({"id":"original","projectId":"p","parentTaskId":"parent","status":"completed","accepted":true}))?;
    fixture.store.put("task", "withdrawn", &json!({"id":"withdrawn","projectId":"p","parentTaskId":"parent","origin":"user","status":"failed","supplementWithdrawal":{"schemaVersion":1,"parentId":"parent"}}))?;
    for state in ["cancelled", "failed"] {
        let mut run = repository::new_run(
            &fixture.store,
            "p",
            fixture.files.capture(&fixture.project)?,
            None,
            Some("parent".into()),
            None,
        )?;
        run.status = state.into();
        fixture.save(&run)?;
        parent["codeValidation"] = json!({"runId":run.id,"status":state});
        assert!(!task_gate::ready(
            &fixture.store,
            &fixture.files,
            &mut parent,
            &[]
        )?);
        assert_ne!(parent["status"], "completed");
    }
    let mut fresh = repository::new_run(
        &fixture.store,
        "p",
        fixture.files.capture(&fixture.project)?,
        None,
        Some("parent".into()),
        None,
    )?;
    fixture.passing_code(&mut fresh)?;
    parent["codeValidation"] = json!({"runId":fresh.id,"status":"completed"});
    assert!(task_gate::ready(
        &fixture.store,
        &fixture.files,
        &mut parent,
        &[]
    )?);
    task_completion::integrated(&fixture.store, &mut parent)?;
    assert_eq!(parent["status"], "completed");
    assert!(parent["report"].as_str().unwrap().contains("1 个子任务"));
    Ok(())
}
