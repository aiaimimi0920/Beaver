use super::{file, inspect, issue, record, source, task, Fixture};
use crate::{asset_work_inputs, files::Files};
use anyhow::Result;
use serde_json::{json, Value};
use std::fs;

fn content_issue(report: &Value, id: &str, field: &str, reason: &str) {
    assert!(
        report["contentReferences"]["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| {
                issue["sourceId"] == id && issue["field"] == field && issue["reason"] == reason
            }),
        "missing issue {id} {field} {reason}: {}",
        report["contentReferences"]
    );
}

fn reference(fixture: &Fixture, task: &str, project: &str) -> Result<Value> {
    let path = format!("asset-observer/{task}/references/view.png");
    Ok(json!({"id":"view","taskId":task,"projectId":project,
        "imagePath":source(fixture, &path),"sha256":file(fixture, &path)?}))
}

#[test]
fn offline_followup_seeds_and_attempts_keep_earlier_task_evidence() -> Result<()> {
    let fixture = Fixture::new()?;
    let project = &fixture.projects[0];
    task(&fixture, "task-0", project)?;
    task(&fixture, "task-1", &fixture.projects[1])?;
    let mut followup = task(&fixture, "followup", project)?;
    let reference = reference(&fixture, "task-0", project)?;
    let checkpoint = "codex/task-0/asset-checkpoints/earlier.blend";
    file(&fixture, checkpoint)?;
    let checkpoint = source(&fixture, checkpoint);
    let narrative = "invalid-value-must-not-escape";
    let seed = json!({"reference":reference,"checkpoint":checkpoint,
        "history":[{"evidence":narrative}],"text":narrative,"imageObservation":narrative});
    followup["assetFeedbackSeed"] = seed.clone();
    fixture.store.put("task", "followup", &followup)?;
    fixture.store.put(
        "operation",
        "pending",
        &json!({"taskId":"followup",
        "projectId":project,"taskAfter":followup}),
    )?;
    fixture.store.put(
        "asset-task",
        "followup",
        &json!({"taskId":"followup",
        "projectId":project,"feedback":[seed],"work":{"attempts":[
            {"checkpoint":checkpoint,"endCheckpoint":checkpoint,
             "inputs":{"imagePath":narrative},"outputs":{"sha256":narrative},
             "tools":[{"checkpoint":narrative}]},
            {"checkpoint":null,"endCheckpoint":null,"inputFiles":null}
        ]}}),
    )?;
    let report = inspect(&fixture)?;
    assert_eq!(report["fileReferences"]["issues"], json!([]));
    assert_eq!(report["contentReferences"]["issues"], json!([]));
    assert_eq!(report["contentReferences"]["checked"], 0);
    assert_eq!(report["fileReferences"]["checked"], 12);
    Ok(())
}

#[test]
fn nested_seed_and_attempt_paths_reject_missing_cross_project_and_malformed_values() -> Result<()> {
    let fixture = Fixture::new()?;
    let [project, other] = &fixture.projects;
    let mut value = task(&fixture, "task-0", project)?;
    task(&fixture, "task-1", other)?;
    let foreign = reference(&fixture, "task-1", other)?;
    let checkpoint = "codex/task-1/asset-checkpoints/foreign.blend";
    file(&fixture, checkpoint)?;
    value["assetFeedbackSeed"] = json!({"reference":foreign,
        "checkpoint":source(&fixture, checkpoint)});
    fixture.store.put("task", "task-0", &value)?;
    value["assetFeedbackSeed"]["checkpoint"] = json!(source(
        &fixture,
        "codex/task-0/asset-checkpoints/missing.blend"
    ));
    fixture.store.put(
        "operation",
        "pending",
        &json!({"taskId":"task-0",
        "projectId":project,"taskAfter":value}),
    )?;
    fixture.store.put(
        "asset-task",
        "task-0",
        &json!({"taskId":"task-0",
        "projectId":project,"feedback":[false],"work":{"attempts":[
            {"checkpoint":source(&fixture, checkpoint),
             "endCheckpoint":source(&fixture, "codex/task-0/asset-checkpoints/missing.blend")},
            {"checkpoint":{"private":"invalid-value-must-not-escape"}},false
        ]}}),
    )?;
    let mut invalid = task(&fixture, "invalid", project)?;
    invalid["assetFeedbackSeed"] = json!("invalid-value-must-not-escape");
    fixture.store.put("task", "invalid", &invalid)?;
    let report = inspect(&fixture)?;
    for (id, field, reason) in [
        (
            "task-0",
            "/assetFeedbackSeed/reference/imagePath",
            "FILE_PROJECT_MISMATCH",
        ),
        (
            "task-0",
            "/assetFeedbackSeed/checkpoint",
            "FILE_PROJECT_MISMATCH",
        ),
        (
            "pending",
            "/taskAfter/assetFeedbackSeed/reference/imagePath",
            "FILE_PROJECT_MISMATCH",
        ),
        (
            "pending",
            "/taskAfter/assetFeedbackSeed/checkpoint",
            "FILE_NOT_FOUND",
        ),
        ("invalid", "/assetFeedbackSeed", "INVALID_FILE_REFERENCE"),
        ("task-0", "/feedback/0", "INVALID_FILE_REFERENCE"),
        (
            "task-0",
            "/work/attempts/0/checkpoint",
            "FILE_PROJECT_MISMATCH",
        ),
        ("task-0", "/work/attempts/0/endCheckpoint", "FILE_NOT_FOUND"),
        (
            "task-0",
            "/work/attempts/1/checkpoint",
            "INVALID_FILE_REFERENCE",
        ),
        ("task-0", "/work/attempts/2", "INVALID_FILE_REFERENCE"),
    ] {
        issue(&report, id, field, reason);
    }
    Ok(())
}

#[test]
fn attempt_inputs_retain_every_frozen_version_after_workspace_changes() -> Result<()> {
    let fixture = Fixture::new()?;
    let project = &fixture.projects[0];
    task(&fixture, "task-0", project)?;
    let workspace = fixture.data.join("workspaces/task-0");
    let files = Files::new(fixture.data.clone());
    let mut attempts = Vec::new();
    let mut hashes = Vec::new();
    for (role, bytes) in [
        (asset_work_inputs::Role::Source, "first model"),
        (asset_work_inputs::Role::Dependency, "second model"),
    ] {
        fs::write(workspace.join("model.blend"), bytes)?;
        let captured = asset_work_inputs::capture(
            &files,
            &workspace,
            &[asset_work_inputs::Declaration {
                path: "model.blend".into(),
                role,
                expected_sha256: None,
            }],
        )?;
        hashes.push(captured[0].sha256.clone());
        attempts.push(json!({"inputFiles":captured}));
    }
    // Neither input version remains at the current workspace path.
    fs::write(workspace.join("model.blend"), "latest unsaved changes")?;
    fixture.store.put(
        "asset-task",
        "task-0",
        &json!({"taskId":"task-0",
        "projectId":project,"work":{"attempts":attempts}}),
    )?;
    let report = inspect(&fixture)?;
    assert_eq!(report["contentReferences"]["issues"], json!([]));
    assert_eq!(report["contentReferences"]["checked"], 2);
    assert_eq!(
        report["contentReferences"]["blobs"]
            .as_object()
            .unwrap()
            .len(),
        2
    );
    for hash in hashes {
        assert_eq!(
            report["contentReferences"]["blobs"][&hash],
            json!([project])
        );
        let blob = record(&report, &format!("application/data/blobs/{hash}"));
        assert_eq!(blob["projectIds"], json!([project]));
    }
    Ok(())
}

#[test]
fn attempts_report_broken_input_blobs_and_malformed_collections_without_raw_values() -> Result<()> {
    let fixture = Fixture::new()?;
    let project = &fixture.projects[0];
    task(&fixture, "task-0", project)?;
    let wrong = "b".repeat(64);
    file(&fixture, &format!("blobs/{wrong}"))?;
    fixture.store.put(
        "asset-task",
        "task-0",
        &json!({"taskId":"task-0",
        "projectId":project,"work":{"attempts":[
            {"inputFiles":[
                {"path":"model.blend","role":"source","sha256":"a".repeat(64)},
                {"path":"model.blend","role":"dependency","sha256":wrong},
                {"sha256":"invalid-value-must-not-escape"},false,{}
            ]},
            {"inputFiles":"invalid-value-must-not-escape"},false,
            {"inputFiles":null},{"inputFiles":[]},{}
        ]}}),
    )?;
    for (id, work) in [
        ("null-work", Value::Null),
        (
            "wrong-attempts",
            json!({"attempts":"invalid-value-must-not-escape"}),
        ),
        ("missing-attempts", json!({})),
    ] {
        task(&fixture, id, project)?;
        fixture.store.put(
            "asset-task",
            id,
            &json!({"taskId":id,
            "projectId":project,"work":work}),
        )?;
    }
    let report = inspect(&fixture)?;
    for (field, reason) in [
        ("/work/attempts/0/inputFiles/0/sha256", "CONTENT_NOT_FOUND"),
        (
            "/work/attempts/0/inputFiles/1/sha256",
            "CONTENT_HASH_MISMATCH",
        ),
        (
            "/work/attempts/0/inputFiles/2/sha256",
            "INVALID_CONTENT_HASH",
        ),
        ("/work/attempts/0/inputFiles/3", "INVALID_INPUT_FILE"),
        (
            "/work/attempts/0/inputFiles/4/sha256",
            "INVALID_CONTENT_HASH",
        ),
        ("/work/attempts/1/inputFiles", "INVALID_INPUT_FILES"),
        ("/work/attempts/2", "INVALID_ATTEMPT"),
    ] {
        content_issue(&report, "task-0", field, reason);
    }
    for (id, field, reason) in [
        ("null-work", "/work", "INVALID_ASSET_WORK"),
        ("wrong-attempts", "/work/attempts", "INVALID_ATTEMPTS"),
        ("missing-attempts", "/work/attempts", "INVALID_ATTEMPTS"),
    ] {
        content_issue(&report, id, field, reason);
        issue(&report, id, field, "INVALID_FILE_REFERENCE");
    }
    assert_eq!(report["contentReferences"]["blobs"], json!({}));
    Ok(())
}
