#![cfg(windows)]

mod project_storage_support;
use anyhow::{ensure, Result};
use beaver_core::{
    asset_delivery_files,
    object_framework_status::status,
    project_storage::ProjectStore,
    store::Store,
    validation::{repository, sandbox::Sandbox},
};
use project_storage_support::{copy, project, tree};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

struct Junction(PathBuf);

impl Junction {
    fn create(link: &Path, target: &Path) -> Result<Self> {
        let output = Command::new("cmd.exe")
            .args(["/D", "/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()?;
        ensure!(
            output.status.success(),
            "cannot create fixture junction: {output:?}"
        );
        Ok(Self(link.to_owned()))
    }
}

impl Drop for Junction {
    fn drop(&mut self) {
        // Remove only the junction itself before TempDir recursively cleans up the fixture.
        let _ = fs::remove_dir(&self.0);
    }
}

#[test]
fn linked_project_and_control_directories_are_rejected_without_touching_targets() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let host = Store::open(&temp.path().join("host"))?;
    let original = project(temp.path(), "original", "project")?;
    let linked = temp.path().join("linked");
    let _root_link = Junction::create(&linked, &original)?;
    host.put("project", "project", &json!({"id":"project","path":linked}))?;
    let before = tree(&original)?;
    assert_eq!(status(&host, "project")?["storage"]["state"], "invalid");
    assert!(ProjectStore::open(&linked, "project").is_err());
    assert!(ProjectStore::initialize(&linked, "project").is_err());
    assert_eq!(tree(&original)?, before);

    let root = temp.path().join("control-link");
    fs::create_dir(&root)?;
    fs::write(root.join("project.godot"), b"config_version=5\n")?;
    let _control_link = Junction::create(&root.join(".beaver"), &original.join(".beaver"))?;
    host.put("project", "project", &json!({"id":"project","path":root}))?;
    assert_eq!(status(&host, "project")?["storage"]["state"], "invalid");
    assert!(ProjectStore::open(&root, "project").is_err());
    assert!(ProjectStore::initialize(&root, "project").is_err());
    assert_eq!(tree(&original)?, before);
    Ok(())
}

#[test]
fn linked_content_directory_cannot_redirect_project_storage_writes() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let source = project(temp.path(), "source", "project")?;
    let root = temp.path().join("project");
    copy(&source, &root)?;
    let external = temp.path().join("external");
    fs::create_dir(&external)?;
    fs::write(external.join("keep.bin"), b"external content")?;
    let before = tree(&external)?;
    let content = root.join(".beaver").join("content");
    fs::remove_dir(&content)?;
    let junction = Junction::create(&content, &external)?;
    let error = ProjectStore::open(&root, "project").err().unwrap();
    assert!(format!("{error:#}").contains("目录联接"), "{error:#}");
    assert_eq!(tree(&external)?, before);
    drop(junction);
    fs::create_dir(&content)?;
    assert_eq!(tree(&root)?, tree(&source)?);
    Ok(())
}

#[test]
fn active_project_content_handles_reject_a_later_junction() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project(temp.path(), "project", "project")?;
    let runtime = ProjectStore::open(&root, "project")?.into_runtime();
    let external = temp.path().join("external");
    fs::create_dir(&external)?;
    fs::write(external.join("keep.txt"), "keep")?;
    let before = tree(&external)?;
    let content = root.join(".beaver").join("content").join("blobs");
    let junction = Junction::create(&content, &external)?;
    assert!(runtime.files().capture(&root).is_err());
    assert!(runtime.files().blob(&"a".repeat(64)).is_err());
    assert_eq!(tree(&external)?, before);
    drop(junction);
    assert!(runtime.files().capture(&root).is_ok());
    Ok(())
}

#[test]
fn active_workspace_handles_reject_linked_parent_and_task_directories() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project(temp.path(), "project", "project")?;
    let runtime = ProjectStore::open(&root, "project")?.into_runtime();
    let external = temp.path().join("external");
    fs::create_dir(&external)?;
    fs::write(external.join("keep.txt"), "keep")?;
    let before = tree(&external)?;
    let workspaces = root.join(".beaver").join("workspaces");
    fs::remove_dir(&workspaces)?;
    let parent_link = Junction::create(&workspaces, &external)?;
    assert!(runtime.files().workspace("task").is_err());
    assert!(runtime.files().codex_home("task").is_err());
    assert!(runtime
        .files()
        .resolve_workspace("task", std::path::Path::new(".beaver/workspaces/task"))
        .is_err());
    drop(parent_link);
    fs::create_dir(&workspaces)?;
    let task_link = Junction::create(&workspaces.join("task"), &external)?;
    assert!(runtime.files().workspace("task").is_err());
    assert!(runtime
        .files()
        .resolve_workspace("task", std::path::Path::new(".beaver/workspaces/task"))
        .is_err());
    assert_eq!(tree(&external)?, before);
    drop(task_link);
    assert!(!runtime.files().workspace("task")?.exists());
    Ok(())
}

#[test]
fn active_codex_home_handles_reject_linked_session_directories() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let root = project(temp.path(), "project", "project")?;
    let runtime = ProjectStore::open(&root, "project")?.into_runtime();
    let external = temp.path().join("external");
    fs::create_dir(&external)?;
    fs::write(external.join("keep.txt"), "keep")?;
    let before = tree(&external)?;
    let codex = root.join(".beaver").join("workspaces").join(".codex");
    let parent_link = Junction::create(&codex, &external)?;
    assert!(runtime.files().codex_home("task").is_err());
    drop(parent_link);
    fs::create_dir(&codex)?;
    let task_link = Junction::create(&codex.join("task"), &external)?;
    assert!(runtime.files().codex_home("task").is_err());
    assert_eq!(tree(&external)?, before);
    drop(task_link);
    assert!(!runtime.files().codex_home("task")?.exists());
    Ok(())
}

#[test]
fn project_exports_reject_linked_cache_and_export_directories() -> Result<()> {
    for relative in [".beaver/cache", ".beaver/cache/delivery-exports"] {
        let temp = tempfile::tempdir()?;
        let root = project(temp.path(), "project", "project")?;
        let runtime = ProjectStore::open(&root, "project")?.into_runtime();
        let files = runtime.files();
        let snapshot = files.capture(&root)?;
        let external = temp.path().join("external");
        fs::create_dir(&external)?;
        fs::write(external.join("keep.txt"), "keep")?;
        let before = tree(&external)?;
        let link = relative
            .split('/')
            .fold(root.clone(), |path, part| path.join(part));
        if link.exists() {
            fs::remove_dir(&link)?;
        }
        let junction = Junction::create(&link, &external)?;
        assert!(asset_delivery_files::export_snapshot(&files, &snapshot).is_err());
        assert_eq!(tree(&external)?, before);
        drop(junction);
    }
    Ok(())
}

#[test]
fn active_evidence_handles_reject_linked_parent_and_run_directories() -> Result<()> {
    for link_parent in [true, false] {
        let temp = tempfile::tempdir()?;
        let root = project(temp.path(), "project", "project")?;
        let runtime = ProjectStore::open(&root, "project")?.into_runtime();
        let files = runtime.files();
        let store = runtime.store();
        let store = store.lock().unwrap();
        store.put("project", "project", &json!({"id":"project", "path":root}))?;
        let run =
            repository::build_run(&store, "project", files.capture(&root)?, None, None, None)?;
        let external = temp.path().join("external");
        fs::create_dir(&external)?;
        fs::write(external.join("keep.txt"), "keep")?;
        let before = tree(&external)?;
        let evidence = root.join(".beaver").join("evidence");
        let link = if link_parent {
            fs::remove_dir(&evidence)?;
            evidence
        } else {
            evidence.join(&run.id)
        };
        let junction = Junction::create(&link, &external)?;
        assert!(repository::run_dir(&files, &run.id).is_err());
        assert!(Sandbox::prepare(&files, &run).is_err());
        assert_eq!(tree(&external)?, before);
        drop(junction);
    }
    Ok(())
}
