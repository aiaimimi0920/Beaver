use super::Fixture;
use crate::{data_backup, files::Files, migration_bundle};
use anyhow::Result;
use serde_json::{json, Value};
use std::fs;

fn inspect(fixture: &Fixture) -> Result<Value> {
    let backup = fixture.bundle()?;
    let before = data_backup::inventory(&backup)?;
    let report =
        migration_bundle::command(vec!["inspect-projects".into(), backup.into_os_string()])?;
    assert_eq!(
        data_backup::inventory(&fixture.temp.path().join("bundle"))?,
        before
    );
    assert_eq!(report["readyToActivate"], false);
    Ok(report)
}

fn has_issue(report: &Value, section: &str, source: &str, field: &str, reason: &str) -> bool {
    report[section]["issues"]
        .as_array()
        .unwrap()
        .iter()
        .any(|issue| {
            issue["sourceId"] == source && issue["field"] == field && issue["reason"] == reason
        })
}

fn content(fixture: &Fixture) -> Result<Value> {
    let source = fixture.temp.path().join("content-source");
    fs::create_dir(&source)?;
    fs::write(source.join("mesh.dat"), b"retained immutable model")?;
    Ok(serde_json::to_value(
        Files::new(fixture.data.clone()).capture(&source)?,
    )?)
}

#[test]
fn archived_task_history_retains_shared_blobs_and_frozen_retired_flows() -> Result<()> {
    let fixture = Fixture::new()?;
    let [project, other] = &fixture.projects;
    let snapshot = content(&fixture)?;
    let hash = snapshot["mesh.dat"].as_str().unwrap();
    for (task, owner) in [("task-0", project), ("task-1", other)] {
        fixture.store.put(
            "task",
            task,
            &json!({"id":task,"projectId":owner,"baseline":snapshot}),
        )?;
    }
    fixture.store.put(
        "task",
        "child",
        &json!({"id":"child","projectId":project,
        "parentTaskId":"task-0","dependsOn":["task-0"],"baseline":{},
        "changes":[{"path":"mesh.dat","before":null,"after":hash}]}),
    )?;
    let old_flow = json!({"id":"flow","projectId":project,"revision":1,
        "definition":{"taskIds":["child"],"status":"active"}});
    let new_flow = json!({"id":"flow","projectId":project,"revision":2,
        "definition":{"taskIds":["child"],"status":"retired"}});
    fixture.store.put("validationFlow", "flow", &new_flow)?;
    fixture
        .store
        .put("validationFlowRevision", "flow:1", &old_flow)?;
    fixture
        .store
        .put("validationFlowRevision", "flow:2", &new_flow)?;
    fixture.store.put(
        "validationRun",
        "run",
        &json!({"id":"run","projectId":project,
        "taskId":"child","flow":old_flow,"snapshot":snapshot,
        "snapshotId":"not-a-blob","evidence":[{"sha256":"evidence-is-a-file-not-a-blob"}]}),
    )?;
    fixture.store.put(
        "validationBaseline",
        "baseline-1",
        &json!({"id":"baseline-1",
        "projectId":project,"flowId":"flow","runId":"run","previousId":null}),
    )?;
    fixture.store.put(
        "validationBaseline",
        "baseline-2",
        &json!({"id":"baseline-2",
        "projectId":project,"flowId":"flow","runId":"run","previousId":"baseline-1"}),
    )?;
    fixture.store.put(
        "validationRelease",
        "release",
        &json!({"id":"release","projectId":project,
        "flows":[old_flow],"flowIds":["flow"],"runIds":["run"],"snapshot":snapshot,
        "scopeId":"not-an-entity","excluded":[{"flowId":"flow","revision":2}]}),
    )?;
    fixture.store.put(
        "operation",
        "pending",
        &json!({"id":"pending","taskId":"child",
        "projectId":project,"state":"applying","changes":[{"before":hash,"after":null}],
        "taskAfter":{"id":"child","projectId":project,"baseline":snapshot}}),
    )?;
    fixture.store.put(
        "asset-delivery/task-0",
        "candidate",
        &json!({"id":"candidate",
        "taskId":"task-0","files":snapshot,"inputCandidates":[]}),
    )?;
    let report = inspect(&fixture)?;
    assert_eq!(report["entityReferences"]["issues"], json!([]));
    assert!(report["entityReferences"]["checked"].as_u64().unwrap() >= 16);
    assert_eq!(report["contentReferences"]["issues"], json!([]));
    let mut owners = vec![project.clone(), other.clone()];
    owners.sort();
    assert_eq!(report["contentReferences"]["blobs"][hash], json!(owners));
    assert_eq!(
        report["contentReferences"]["blobs"]
            .as_object()
            .unwrap()
            .len(),
        1
    );
    Ok(())
}

#[test]
fn partition_scan_reports_dangling_cross_project_and_malformed_edges() -> Result<()> {
    let fixture = Fixture::new()?;
    let project = &fixture.projects[0];
    fixture.store.put("task", "task-0", &json!({"id":"task-0","projectId":project,
        "parentTaskId":"gone","dependsOn":["task-1",42,null],"subtaskIds":"private-reference-sentinel",
        "validationFeedbackId":"feedback"}))?;
    fixture.store.put(
        "validationFeedback",
        "feedback",
        &json!({"id":"feedback",
        "projectId":"unregistered","runId":"missing"}),
    )?;
    fixture.store.put(
        "validationRepairDecision",
        "orphan-run",
        &json!({"id":"orphan-run",
        "projectId":project,"decision":{"result":{"taskId":"gone","feedbackId":"feedback"}}}),
    )?;
    fixture.store.put(
        "asset-delivery/task-0",
        "candidate",
        &json!({"id":"candidate",
        "inputCandidates":["missing-candidate"]}),
    )?;
    let report = inspect(&fixture)?;
    for (source, field, reason) in [
        ("task-0", "/parentTaskId", "REFERENCE_NOT_FOUND"),
        ("task-0", "/dependsOn/0", "CROSS_PROJECT_REFERENCE"),
        ("task-0", "/dependsOn/1", "INVALID_REFERENCE_FIELD"),
        ("task-0", "/dependsOn/2", "INVALID_REFERENCE_FIELD"),
        ("task-0", "/subtaskIds", "INVALID_REFERENCE_FIELD"),
        ("task-0", "/validationFeedbackId", "TARGET_OWNER_UNRESOLVED"),
        ("feedback", "/runId", "SOURCE_OWNER_UNRESOLVED"),
        ("orphan-run", "$key", "REFERENCE_NOT_FOUND"),
        ("candidate", "/inputCandidates/0", "REFERENCE_NOT_FOUND"),
    ] {
        assert!(
            has_issue(&report, "entityReferences", source, field, reason),
            "{source} {field}: {report}"
        );
    }
    assert!(!report.to_string().contains("private-reference-sentinel"));
    Ok(())
}

#[test]
fn historical_runs_require_the_matching_archived_flow_revision() -> Result<()> {
    let fixture = Fixture::new()?;
    let project = &fixture.projects[0];
    let flow = json!({"id":"flow","projectId":project,"revision":1,"definition":{"taskIds":[]}});
    fixture.store.put("validationFlow", "flow", &flow)?;
    fixture
        .store
        .put("validationFlowRevision", "flow:1", &flow)?;
    let mut changed = flow.clone();
    changed["definition"]["taskIds"] = json!(["task-0"]);
    fixture.store.put(
        "validationRun",
        "changed",
        &json!({"id":"changed","projectId":project,"flow":changed}),
    )?;
    let mut absent = flow;
    absent["revision"] = json!(2);
    fixture.store.put(
        "validationRelease",
        "lost-history",
        &json!({"id":"lost-history",
        "projectId":project,"flows":[absent]}),
    )?;
    let report = inspect(&fixture)?;
    assert!(has_issue(
        &report,
        "entityReferences",
        "changed",
        "/flow",
        "FROZEN_FLOW_MISMATCH"
    ));
    assert!(has_issue(
        &report,
        "entityReferences",
        "lost-history",
        "/flows/0",
        "REFERENCE_NOT_FOUND"
    ));
    Ok(())
}

#[test]
fn archive_hash_integrity_does_not_hide_missing_or_misnamed_history_blobs() -> Result<()> {
    let fixture = Fixture::new()?;
    let project = &fixture.projects[0];
    let snapshot = content(&fixture)?;
    let valid_hash = snapshot["mesh.dat"].as_str().unwrap();
    let missing = "a".repeat(64);
    let corrupt = "b".repeat(64);
    fs::write(
        fixture.data.join("blobs").join(&corrupt),
        b"wrong content under a valid digest name",
    )?;
    fixture.store.put(
        "task",
        "task-0",
        &json!({"id":"task-0","projectId":project,
        "baseline":{"missing":missing,"corrupt":corrupt,"invalid":"private-hash-sentinel"},
        "changes":[{"before":null,"after":false},null]}),
    )?;
    fixture.store.put(
        "task",
        "task-1",
        &json!({"id":"task-1","projectId":fixture.projects[1],
        "baseline":"private-snapshot-sentinel"}),
    )?;
    fixture.store.put(
        "task",
        "unknown",
        &json!({"id":"unknown","projectId":"unregistered",
        "baseline":snapshot}),
    )?;
    fixture.store.put(
        "operation",
        "pending",
        &json!({"taskId":"task-0","projectId":project,
        "taskAfter":{"id":"task-0","projectId":project,"baseline":{"old":missing}},
        "changes":[{"before":valid_hash,"after":missing}]}),
    )?;
    let report = inspect(&fixture)?;
    for (source, field, reason) in [
        ("task-0", "/baseline/missing", "CONTENT_NOT_FOUND"),
        ("task-0", "/baseline/corrupt", "CONTENT_HASH_MISMATCH"),
        ("task-0", "/baseline/invalid", "INVALID_CONTENT_HASH"),
        ("task-0", "/changes/0/after", "INVALID_CONTENT_HASH"),
        ("task-0", "/changes/1", "INVALID_CHANGE"),
        ("task-1", "/baseline", "INVALID_SNAPSHOT"),
        ("unknown", "/baseline/mesh.dat", "CONTENT_OWNER_UNRESOLVED"),
        ("pending", "/taskAfter/baseline/old", "CONTENT_NOT_FOUND"),
    ] {
        assert!(
            has_issue(&report, "contentReferences", source, field, reason),
            "{source} {field}: {report}"
        );
    }
    assert_eq!(
        report["contentReferences"]["blobs"][valid_hash],
        json!([project])
    );
    assert!(!report.to_string().contains("private-hash-sentinel"));
    assert!(!report.to_string().contains("private-snapshot-sentinel"));
    Ok(())
}
