use super::*;
use crate::{
    files::Files, project_derivation_validation_records::Rewrite, project_storage::ProjectStore,
};
use serde_json::json;
use std::sync::Arc;

fn prepare(root: &Path) -> Result<PathBuf> {
    let source = root.join("source");
    fs::create_dir(&source)?;
    fs::write(source.join("project.godot"), "config_version=5\n")?;
    let project = ProjectStore::initialize(&source, "original")?;
    project.store().put(
        "project",
        "original",
        &json!({"id":"original","path":source}),
    )?;
    project
        .store()
        .put("task", "task", &json!({"id":"task","projectId":"original"}))?;
    project.store().put(
        "asset-reference",
        "reference",
        &json!({"id":"reference","taskId":"task","projectId":"original"}),
    )?;
    project.store().put(
        "validationRun",
        "run",
        &json!({"id":"run","projectId":"original"}),
    )?;
    drop(project);
    for (path, bytes) in [
        (
            ".beaver/workspaces/task/task.txt",
            b"task original".as_slice(),
        ),
        (
            ".beaver/workspaces/.codex/task/sessions/task.jsonl",
            b"{\"cwd\":\"original/task\"}\n",
        ),
        (
            ".beaver/workspaces/.codex/task/asset-checkpoints/attempt.blend",
            b"blend",
        ),
        (
            ".beaver/evidence/asset-observer/task/references/reference.png",
            b"image",
        ),
        (".beaver/evidence/run/report.json", b"{\"runId\":\"run\"}"),
        (".beaver/custom/task", b"unknown preserved"),
    ] {
        let file = source.join(path);
        fs::create_dir_all(file.parent().unwrap())?;
        fs::write(file, bytes)?;
    }
    let destination = root.join("prepared");
    copy::prepare(
        copy::Request {
            request_id: "copy".into(),
            source,
            source_project_id: "original".into(),
            target_project_id: "derived".into(),
        },
        &destination,
    )?;
    Ok(destination)
}

#[test]
fn mapped_files_follow_runtime_paths_and_preserve_opaque_content_offline() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let preparation = prepare(temp.path())?;
    let original = data_backup::inventory(&temp.path().join("source"))?;
    let prepared = copy::inspect(&preparation)?;
    let task = Rewrite(&prepared.identities).key("task", "task")?.id;
    let reference = Rewrite(&prepared.identities)
        .key("asset-reference", "reference")?
        .id;
    let run = Rewrite(&prepared.identities)
        .key("validationRun", "run")?
        .id;
    let receipt = create(&preparation, "one")?;
    assert_eq!(
        original,
        data_backup::inventory(&temp.path().join("source"))?
    );
    fs::rename(temp.path().join("source"), temp.path().join("offline"))?;
    let moved = temp.path().join("moved");
    fs::rename(&preparation, &moved)?;
    assert_eq!(receipt.entries, inspect(&moved, "one")?.entries);
    let stage = directory(&moved, "one")?;
    let project = stage.join("project");
    assert!(ProjectStore::open(&project, "original").is_err());
    let files = Files::project(
        project.clone(),
        Arc::new(File::open(stage.join(".beaver-migration-pending"))?),
    );
    let workspace = files.resolve_workspace(&task, Path::new(&files.workspace_location(&task)?))?;
    assert_eq!(fs::read(workspace.join("task.txt"))?, b"task original");
    assert_eq!(
        fs::read(files.codex_home(&task)?.join("sessions/task.jsonl"))?,
        b"{\"cwd\":\"original/task\"}\n"
    );
    let checkpoint = format!(".beaver/workspaces/.codex/{task}/asset-checkpoints/attempt.blend");
    assert_eq!(fs::read(files.resolve_checkpoint(&checkpoint)?)?, b"blend");
    let image = files.observer_reference_location(&task, &reference)?;
    assert_eq!(
        fs::read(files.resolve_observer_reference(&task, &reference, &image)?)?,
        b"image"
    );
    assert_eq!(
        fs::read(files.validation_evidence(&run)?.join("report.json"))?,
        b"{\"runId\":\"run\"}"
    );
    assert_eq!(
        fs::read(project.join(".beaver/custom/task"))?,
        b"unknown preserved"
    );
    assert!(!project.join(".beaver/workspaces/task").exists());
    assert_eq!(
        fs::read(project.join(".beaver/project.sqlite"))?,
        fs::read(moved.join("project/.beaver/project.sqlite"))?
    );
    copy::inspect(&moved)?;
    drop(files);
    let standalone = temp.path().join("standalone");
    fs::rename(stage, &standalone)?;
    assert!(ProjectStore::open(&standalone.join("project"), "original").is_err());
    Ok(())
}

#[test]
fn corrupted_and_incomplete_generations_cannot_be_reused_or_rebound() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let preparation = prepare(temp.path())?;
    create(&preparation, "one")?;
    let stage = directory(&preparation, "one")?;
    fs::remove_file(stage.join(RECEIPT))?;
    let partial = data_backup::inventory(&stage)?;
    assert!(inspect(&preparation, "one").is_err());
    assert!(create(&preparation, "one").is_err());
    assert_eq!(partial, data_backup::inventory(&stage)?);
    create(&preparation, "retry")?;
    let retry = directory(&preparation, "retry")?;
    fs::write(retry.join("project/project.godot"), "changed")?;
    assert!(inspect(&preparation, "retry").is_err());
    assert!(create(&preparation, "../escape").is_err());
    create(&preparation, "good")?;
    let other = tempfile::tempdir()?;
    let second = prepare(other.path())?;
    create(&second, "good")?;
    fs::copy(
        directory(&preparation, "good")?.join(RECEIPT),
        directory(&second, "good")?.join(RECEIPT),
    )?;
    assert!(inspect(&second, "good").is_err());
    inspect(&preparation, "good")?;
    copy::inspect(&preparation)?;
    Ok(())
}

#[test]
fn missing_owners_aliases_and_target_collisions_fail_before_file_creation() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let preparation = prepare(temp.path())?;
    let prepared = copy::inspect(&preparation)?;
    let paths = Paths(&prepared.identities);
    for path in [
        "../escape",
        "C:/outside",
        ".beaver/workspaces/missing/file",
        ".beaver/evidence/missing/file",
        ".beaver/evidence/asset-observer/task/references/missing.png",
    ] {
        assert!(paths.relative(path).is_err(), "{path}");
    }
    let entries = ["game.png", "GAME.png"].map(|path| Entry {
        path: path.into(),
        bytes: 1,
        sha256: Some("hash".into()),
    });
    assert!(paths.entries(&entries).is_err());
    let source = temp.path().join("source");
    fs::create_dir_all(source.join(".beaver/workspaces/orphan"))?;
    let before = data_backup::inventory(&source)?;
    let request = copy::Request {
        request_id: "again".into(),
        source: source.clone(),
        source_project_id: "original".into(),
        target_project_id: "derived".into(),
    };
    let orphan = temp.path().join("orphan-preparation");
    for error in [
        copy::inspect_source(&source).unwrap_err(),
        copy::prepare(request, &orphan).unwrap_err(),
    ] {
        assert!(
            error
                .to_string()
                .contains("ambiguous or unknown workspace identity"),
            "{error:#}"
        );
    }
    assert!(!orphan.exists());
    assert_eq!(data_backup::inventory(&source)?, before);
    Ok(())
}
