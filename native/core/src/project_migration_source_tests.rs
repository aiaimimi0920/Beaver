use super::{file, inspect, issue, source, task, Fixture};
use crate::files::Files;
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
        "missing {id} {field} {reason}: {}",
        report["contentReferences"]
    );
}

#[test]
fn task_source_references_use_the_archived_task_copy_including_pending_operations() -> Result<()> {
    let fixture = Fixture::new()?;
    let project = &fixture.projects[0];
    let mut value = task(&fixture, "task-0", project)?;
    task(&fixture, "task-1", &fixture.projects[1])?;
    task(&fixture, "same-project", project)?;
    file(&fixture, "workspaces/task-0/art/texture.png")?;
    file(&fixture, "workspaces/same-project/other.png")?;
    fs::write(
        fixture.temp.path().join("game-0/project-only.png"),
        "current project",
    )?;
    value["references"] = json!([
        {"path":"art/texture.png","note":"invalid-value-must-not-escape"},
        {"path":"project-only.png"}, {"path":"other.png"}
    ]);
    value["context"] = json!({"references":[{"path":"invalid-value-must-not-escape"}]});
    fixture.store.put("task", "task-0", &value)?;
    fixture.store.put(
        "operation",
        "pending",
        &json!({
            "taskId":"task-0","projectId":project,"taskAfter":value
        }),
    )?;
    let report = inspect(&fixture)?;
    for (id, prefix) in [("task-0", ""), ("pending", "/taskAfter")] {
        for index in [1, 2] {
            issue(
                &report,
                id,
                &format!("{prefix}/references/{index}/path"),
                "TASK_REFERENCE_UNAVAILABLE",
            );
        }
    }
    assert_eq!(
        report["fileReferences"]["issues"].as_array().unwrap().len(),
        4
    );
    Ok(())
}

#[test]
fn task_source_references_report_unsafe_paths_wrong_types_and_untrusted_workspaces() -> Result<()> {
    let fixture = Fixture::new()?;
    let project = &fixture.projects[0];
    let mut value = task(&fixture, "task-0", project)?;
    task(&fixture, "task-1", &fixture.projects[1])?;
    file(&fixture, "workspaces/task-1/private.png")?;
    fs::create_dir(fixture.data.join("workspaces/task-0/folder"))?;
    let invalid = [
        "",
        "../task-1/private.png",
        "/private.png",
        "art//texture.png",
        "./texture.png",
        "art/../texture.png",
        "art\\texture.png",
        "texture.png:stream",
        "res://texture.png",
        "texture.png.",
        "texture.png ",
        "invalid-value-must-not-escape\0",
    ];
    let mut references: Vec<Value> = invalid.iter().map(|path| json!({"path":path})).collect();
    references.extend([json!({"path":17}), json!({}), Value::Null]);
    let invalid_count = references.len();
    references.push(json!({"path":"folder"}));
    value["references"] = json!(references);
    fixture.store.put("task", "task-0", &value)?;
    for (id, refs) in [
        ("null-refs", Value::Null),
        (
            "wrong-refs",
            json!({"private":"invalid-value-must-not-escape"}),
        ),
    ] {
        let mut value = task(&fixture, id, project)?;
        value["references"] = refs;
        fixture.store.put("task", id, &value)?;
    }
    let mut value = task(&fixture, "wrong-workspace", project)?;
    value["workspace"] = json!(source(&fixture, "workspaces/task-1"));
    value["references"] = json!([{"path":"private.png"}]);
    fixture.store.put("task", "wrong-workspace", &value)?;
    let report = inspect(&fixture)?;
    for index in 0..invalid_count {
        issue(
            &report,
            "task-0",
            &format!("/references/{index}/path"),
            "INVALID_FILE_REFERENCE",
        );
    }
    issue(
        &report,
        "task-0",
        &format!("/references/{invalid_count}/path"),
        "FILE_TYPE_MISMATCH",
    );
    for id in ["null-refs", "wrong-refs"] {
        issue(&report, id, "/references", "INVALID_FILE_REFERENCE");
    }
    issue(
        &report,
        "wrong-workspace",
        "/references/0/path",
        "FILE_REFERENCE_PATH_MISMATCH",
    );
    Ok(())
}

fn evidence(fixture: &Fixture, run: &str, references: Value) -> Result<Value> {
    let hash = file(fixture, &format!("validation/{run}/view.png"))?;
    Ok(json!({"file":"view.png","sha256":hash,"references":references}))
}

#[test]
fn validation_references_resolve_frozen_sources_after_the_live_files_change() -> Result<()> {
    let fixture = Fixture::new()?;
    let project = &fixture.projects[0];
    task(&fixture, "task-0", project)?;
    task(&fixture, "task-1", &fixture.projects[1])?;
    let root = fixture.temp.path().join("game-0");
    fs::create_dir(root.join("scripts"))?;
    fs::write(root.join("game.gd"), "historical source")?;
    fs::write(root.join("scripts/part.gd"), "historical step source")?;
    let snapshot = Files::new(fixture.data.clone()).capture(&root)?;
    let definition = json!({
        "references":[{"path":"res://game.gd","node":"invalid-value-must-not-escape"}],
        "steps":[{"references":[{"path":"scripts/part.gd"}]}],
        "unknown":{"references":[{"path":"invalid-value-must-not-escape"}]}
    });
    let flow = json!({"id":"flow","projectId":project,"revision":1,"definition":definition});
    fixture
        .store
        .put("validationFlowRevision", "flow:1", &flow)?;
    let evidence = evidence(&fixture, "run", json!([{"path":"game.gd"}]))?;
    fixture.store.put(
        "validationRun",
        "run",
        &json!({
            "id":"run","projectId":project,"snapshot":snapshot,"flow":flow,"evidence":[evidence],
            "unknown":{"references":[{"path":"invalid-value-must-not-escape"}]}
        }),
    )?;
    fs::write(root.join("game.gd"), "current project source")?;
    fs::remove_file(root.join("scripts/part.gd"))?;
    file(&fixture, "workspaces/task-0/game.gd")?;
    let report = inspect(&fixture)?;
    assert_eq!(report["contentReferences"]["issues"], json!([]));
    assert_eq!(report["fileReferences"]["issues"], json!([]));
    for hash in snapshot.values() {
        assert_eq!(report["contentReferences"]["blobs"][hash], json!([project]));
    }
    Ok(())
}

#[test]
fn validation_references_keep_missing_sources_distinct_from_invalid_declarations() -> Result<()> {
    let fixture = Fixture::new()?;
    let project = &fixture.projects[0];
    task(&fixture, "task-0", project)?;
    task(&fixture, "task-1", &fixture.projects[1])?;
    let root = fixture.temp.path().join("game-0");
    fs::write(root.join("game.gd"), "historical source")?;
    let snapshot = Files::new(fixture.data.clone()).capture(&root)?;
    // A later project/workspace file cannot satisfy a historical source declaration.
    fs::write(root.join("later.gd"), "created after the run")?;
    file(&fixture, "workspaces/task-0/later.gd")?;
    let evidence = evidence(
        &fixture,
        "run",
        json!([
            {"path":"later.gd"}, {"path":"GAME.gd"}, {"path":"../game.gd"},
            {"path":"res://"}, {"path":23}, false
        ]),
    )?;
    fixture.store.put(
        "validationRun",
        "run",
        &json!({
            "id":"run","projectId":project,"snapshot":snapshot,"evidence":[evidence],
            "flow":{"definition":{"references":null,"steps":[{"references":{}},false]}}
        }),
    )?;
    for (id, extra) in [
        ("no-snapshot", json!({})),
        ("null-snapshot", json!({"snapshot":null})),
        (
            "broken-blob",
            json!({"snapshot":{"game.gd":"a".repeat(64)}}),
        ),
    ] {
        let mut run = json!({"id":id,"projectId":project,"flow":{"definition":{
            "references":[{"path":"game.gd"}]}}});
        run.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        fixture.store.put("validationRun", id, &run)?;
    }
    let report = inspect(&fixture)?;
    for index in 0..6 {
        content_issue(
            &report,
            "run",
            &format!("/evidence/0/references/{index}/path"),
            if index < 2 {
                "SOURCE_REFERENCE_UNAVAILABLE"
            } else {
                "INVALID_SOURCE_REFERENCE"
            },
        );
    }
    for field in [
        "/flow/definition/references",
        "/flow/definition/steps/0/references",
        "/flow/definition/steps/1",
    ] {
        content_issue(&report, "run", field, "INVALID_SOURCE_REFERENCE");
    }
    for id in ["no-snapshot", "null-snapshot"] {
        content_issue(
            &report,
            id,
            "/flow/definition/references/0/path",
            "SOURCE_SNAPSHOT_UNAVAILABLE",
        );
    }
    content_issue(
        &report,
        "broken-blob",
        "/snapshot/game.gd",
        "CONTENT_NOT_FOUND",
    );
    assert!(report["contentReferences"]["blobs"]
        .get("a".repeat(64))
        .is_none());
    Ok(())
}
