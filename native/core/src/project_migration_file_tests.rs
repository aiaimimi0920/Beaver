use super::Fixture;
use crate::{data_backup, files::file_hash, migration_bundle};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::{collections::BTreeSet, fs, path::PathBuf};

#[path = "project_migration_activation_tests.rs"]
mod activation;
#[path = "project_migration_history_tests.rs"]
mod history;
#[path = "project_migration_partition_tests.rs"]
mod partition;
#[path = "project_migration_session_tests.rs"]
mod sessions;
#[path = "project_migration_source_tests.rs"]
mod sources;

fn source(fixture: &Fixture, path: &str) -> PathBuf {
    // The fixture freezes host into offline-host before creating the real archive.
    fixture.temp.path().join("offline-host").join(path)
}

fn file(fixture: &Fixture, path: &str) -> Result<String> {
    let path = fixture.data.join(path);
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(&path, "file-payload-must-not-escape")?;
    file_hash(&path)?.context("fixture file missing")
}

fn task(fixture: &Fixture, id: &str, project: &str) -> Result<Value> {
    let relative = format!("workspaces/{id}");
    fs::create_dir_all(fixture.data.join(&relative))?;
    let value = json!({"id":id,"projectId":project,"workspace":source(fixture, &relative)});
    fixture.store.put("task", id, &value)?;
    Ok(value)
}

fn inspect(fixture: &Fixture) -> Result<Value> {
    let backup = fixture.bundle()?;
    let before = data_backup::inventory(&backup)?;
    let manifest = migration_bundle::verify(&backup)?;
    let app = data_backup::verify(&backup.join("application"))?;
    // All source paths become unavailable; only the archive may be read.
    fs::rename(
        fixture.temp.path().join("offline-host"),
        fixture.temp.path().join("unavailable-host"),
    )?;
    for index in 0..2 {
        fs::rename(
            fixture.temp.path().join(format!("game-{index}")),
            fixture
                .temp
                .path()
                .join(format!("unavailable-game-{index}")),
        )?;
    }
    let report = migration_bundle::command(vec![
        "inspect-projects".into(),
        backup.clone().into_os_string(),
    ])?;
    assert_eq!(data_backup::inventory(&backup)?, before);
    assert_eq!(report["readyToActivate"], false);
    let expected: BTreeSet<_> = app
        .entries
        .iter()
        .map(|entry| format!("application/data/{}", entry.path))
        .chain(manifest.projects.iter().flat_map(|project| {
            project
                .entries
                .iter()
                .map(|entry| format!("projects/{}/{}", project.id, entry.path))
        }))
        .collect();
    let actual: BTreeSet<_> = report["files"]["records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|record| record["archivePath"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(actual, expected);
    assert!(!report.to_string().contains("file-payload-must-not-escape"));
    assert!(!report.to_string().contains("invalid-value-must-not-escape"));
    Ok(report)
}

fn record<'a>(report: &'a Value, path: &str) -> &'a Value {
    report["files"]["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|record| record["archivePath"] == path)
        .unwrap()
}

fn issue(report: &Value, id: &str, field: &str, reason: &str) {
    assert!(
        report["fileReferences"]["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| {
                issue["sourceId"] == id && issue["field"] == field && issue["reason"] == reason
            }),
        "missing issue {id} {field} {reason}: {}",
        report["fileReferences"]
    );
}

#[test]
fn archive_files_have_explicit_owners_and_unknowns_are_retained() -> Result<()> {
    let fixture = Fixture::new()?;
    let hash = file(&fixture, "content.tmp")?;
    file(&fixture, &format!("blobs/{hash}"))?;
    for (index, project) in fixture.projects.iter().enumerate() {
        let id = format!("task-{index}");
        let mut value = task(&fixture, &id, project)?;
        value["baseline"] = json!({"mesh.dat":hash});
        fixture.store.put("task", &id, &value)?;
        file(
            &fixture,
            &format!("workspaces/{id}/.beaver-context/project/brief.md"),
        )?;
        file(&fixture, &format!("codex/{id}/config.toml"))?;
        file(
            &fixture,
            &format!("asset-observer/{id}/references/view.png"),
        )?;
        fixture.store.put(
            "validationRun",
            &format!("run-{index}"),
            &json!({"id":format!("run-{index}"),"projectId":project}),
        )?;
        file(&fixture, &format!("validation/run-{index}/run.log"))?;
    }
    for path in [
        "codex/orphan/session.jsonl",
        "validation/orphan/run.log",
        "delivery-exports/candidate-temp/model.glb",
        "blobs/orphan",
    ] {
        file(&fixture, path)?;
    }
    fs::write(
        fixture.temp.path().join("game-0/beaver.project.json"),
        "file-payload-must-not-escape",
    )?;
    let report = inspect(&fixture)?;
    assert_eq!(
        record(&report, "application/data/beaver.sqlite")["scope"],
        "container"
    );
    for (index, project) in fixture.projects.iter().enumerate() {
        for path in [
            format!("workspaces/task-{index}/.beaver-context/project/brief.md"),
            format!("codex/task-{index}/config.toml"),
            format!("asset-observer/task-{index}/references/view.png"),
            format!("validation/run-{index}/run.log"),
        ] {
            let value = record(&report, &format!("application/data/{path}"));
            assert_eq!(value["scope"], "project");
            assert_eq!(value["projectId"], *project);
        }
    }
    let shared = record(&report, &format!("application/data/blobs/{hash}"));
    assert_eq!(shared["scope"], "shared");
    assert_eq!(shared["projectIds"].as_array().unwrap().len(), 2);
    assert_eq!(
        record(
            &report,
            &format!("projects/{}/beaver.project.json", fixture.projects[0])
        )["projectId"],
        fixture.projects[0]
    );
    for path in [
        "content.tmp",
        "codex/orphan/session.jsonl",
        "validation/orphan/run.log",
        "delivery-exports/candidate-temp/model.glb",
        "blobs/orphan",
    ] {
        assert_eq!(
            record(&report, &format!("application/data/{path}"))["scope"],
            "unresolved"
        );
    }
    assert!(report["files"]["unresolved"].as_u64().unwrap() >= 5);
    Ok(())
}

#[test]
fn file_references_survive_offline_paths_and_same_project_followups() -> Result<()> {
    let fixture = Fixture::new()?;
    let [project, other] = &fixture.projects;
    let original = task(&fixture, "task-0", project)?;
    task(&fixture, "task-1", other)?;
    let mut followup = task(&fixture, "followup", project)?;
    let checkpoint = source(&fixture, "codex/task-0/asset-checkpoints/saved.blend");
    file(&fixture, "codex/task-0/asset-checkpoints/saved.blend")?;
    followup["assetRestore"] = json!(checkpoint);
    fixture.store.put("task", "followup", &followup)?;
    fixture.store.put(
        "operation",
        "pending",
        &json!({"taskId":"task-0","projectId":project,"taskAfter":original}),
    )?;
    let hash = file(&fixture, "asset-observer/task-0/references/view.png")?;
    let reference = json!({"id":"view","taskId":"task-0","projectId":project,
        "imagePath":source(&fixture, "asset-observer/task-0/references/view.png"),"sha256":hash});
    fixture.store.put("asset-reference", "view", &reference)?;
    fixture.store.put(
        "asset-task",
        "followup",
        &json!({"taskId":"followup","projectId":project,
        "checkpoint":checkpoint,"lastFrame":reference,
        "feedback":[{"reference":reference,"checkpoint":checkpoint}]}),
    )?;
    let hash = file(&fixture, "validation/run/screens/front.png")?;
    fixture.store.put(
        "validationRun",
        "run",
        &json!({"id":"run","projectId":project,
        "evidence":[{"file":"screens/front.png","sha256":hash}]}),
    )?;
    // Windows path spelling and the extended prefix must not require live canonicalization.
    let mut value = original.clone();
    let workspace = source(&fixture, "workspaces/task-0")
        .to_string_lossy()
        .to_uppercase();
    value["workspace"] = json!(if workspace.starts_with("\\\\?\\") {
        workspace
    } else {
        format!("\\\\?\\{workspace}")
    });
    fixture.store.put("task", "task-0", &value)?;
    let report = inspect(&fixture)?;
    assert_eq!(report["fileReferences"]["issues"], json!([]));
    assert!(report["fileReferences"]["checked"].as_u64().unwrap() >= 10);
    Ok(())
}

#[test]
fn missing_wrong_type_cross_project_and_unsafe_references_are_reported_without_payloads(
) -> Result<()> {
    let fixture = Fixture::new()?;
    let [project, other] = &fixture.projects;
    let mut first = task(&fixture, "task-0", project)?;
    task(&fixture, "task-1", other)?;
    file(&fixture, "codex/task-1/asset-checkpoints/other.blend")?;
    first["assetRestore"] = json!(source(
        &fixture,
        "codex/task-1/asset-checkpoints/other.blend"
    ));
    fixture.store.put("task", "task-0", &first)?;
    let hash = file(&fixture, "validation/run/view.png")?;
    fs::create_dir_all(fixture.data.join("validation/run/directory.png"))?;
    fixture.store.put(
        "validationRun",
        "run",
        &json!({"id":"run","projectId":project,"evidence":[
            {"file":"missing.png","sha256":hash},
            {"file":"directory.png","sha256":hash},
            {"file":"view.png","sha256":"0".repeat(64)},
            {"file":"../invalid-value-must-not-escape","sha256":hash},
            {"file":"view.png","sha256":{"private":"invalid-value-must-not-escape"}},
            {"file":"view.png:stream","sha256":hash}
        ]}),
    )?;
    fixture.store.put(
        "task",
        "outside",
        &json!({"id":"outside","projectId":project,
        "workspace":fixture.temp.path().join("offline-host-sibling/workspaces/outside")}),
    )?;
    fixture.store.put(
        "task",
        "wrong",
        &json!({"id":"wrong","projectId":project,
        "workspace":source(&fixture, "workspaces/task-0")}),
    )?;
    fixture.store.put(
        "task",
        "directory",
        &json!({"id":"directory","projectId":project,
        "workspace":source(&fixture, "workspaces/directory")}),
    )?;
    file(&fixture, "workspaces/directory")?;
    let hash = file(&fixture, "asset-observer/task-1/references/foreign.png")?;
    fixture.store.put("asset-task", "task-0", &json!({"taskId":"task-0","projectId":project,
        "lastFrame":{"id":"foreign","taskId":"task-1","projectId":other,
            "imagePath":source(&fixture, "asset-observer/task-1/references/foreign.png"),"sha256":hash},
        "feedback":"invalid-value-must-not-escape"}))?;
    let report = inspect(&fixture)?;
    for (index, reason) in [
        "FILE_NOT_FOUND",
        "FILE_TYPE_MISMATCH",
        "FILE_HASH_MISMATCH",
        "INVALID_FILE_REFERENCE",
        "INVALID_FILE_HASH",
        "INVALID_FILE_REFERENCE",
    ]
    .iter()
    .enumerate()
    {
        issue(&report, "run", &format!("/evidence/{index}/file"), reason);
    }
    issue(&report, "task-0", "/assetRestore", "FILE_PROJECT_MISMATCH");
    issue(
        &report,
        "task-0",
        "/lastFrame/imagePath",
        "FILE_PROJECT_MISMATCH",
    );
    issue(&report, "task-0", "/feedback", "INVALID_FILE_REFERENCE");
    issue(&report, "outside", "/workspace", "FILE_OUTSIDE_APPLICATION");
    issue(
        &report,
        "wrong",
        "/workspace",
        "FILE_REFERENCE_PATH_MISMATCH",
    );
    issue(&report, "directory", "/workspace", "FILE_TYPE_MISMATCH");
    Ok(())
}

#[test]
fn mismatched_run_identity_and_missing_task_owners_do_not_acquire_file_ownership() -> Result<()> {
    let fixture = Fixture::new()?;
    let project = &fixture.projects[0];
    let hash = file(&fixture, "validation/run/view.png")?;
    fixture.store.put(
        "validationRun",
        "run",
        &json!({"id":"different","projectId":project,
        "evidence":[{"file":"view.png","sha256":hash}]}),
    )?;
    let hash = file(&fixture, "asset-observer/missing/references/view.png")?;
    fixture.store.put(
        "asset-reference",
        "view",
        &json!({"id":"view","taskId":"missing","projectId":project,
        "imagePath":source(&fixture, "asset-observer/missing/references/view.png"),"sha256":hash}),
    )?;
    let report = inspect(&fixture)?;
    assert_eq!(
        record(&report, "application/data/validation/run/view.png")["reason"],
        "ENTITY_ID_MISMATCH"
    );
    assert_eq!(
        record(
            &report,
            "application/data/asset-observer/missing/references/view.png"
        )["reason"],
        "TASK_NOT_FOUND"
    );
    issue(&report, "run", "/evidence/0/file", "FILE_OWNER_UNRESOLVED");
    issue(&report, "view", "/imagePath", "FILE_OWNER_UNRESOLVED");
    Ok(())
}
