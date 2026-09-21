use super::{
    fixture::png,
    project_references::{capture, project},
};
use anyhow::{ensure, Result};
use beaver_core::asset_reference;
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
        // Remove the junction itself before TempDir cleanup.
        let _ = fs::remove_dir(&self.0);
    }
}

#[test]
fn project_observer_rejects_linked_evidence_task_and_reference_directories() -> Result<()> {
    for level in ["evidence", "task", "references"] {
        let temp = tempfile::tempdir()?;
        let (runtime, id) = project(temp.path())?;
        let reference = capture(&runtime, &id)?;
        let image = runtime.project_root().join(&reference.image_path);
        let directory = match level {
            "evidence" => runtime.project_root().join(".beaver/evidence"),
            "task" => image.parent().unwrap().parent().unwrap().to_owned(),
            _ => image.parent().unwrap().to_owned(),
        };
        let external = temp.path().join("external");
        fs::rename(&directory, &external)?;
        let junction = Junction::create(&directory, &external)?;
        assert!(asset_reference::read(&runtime.files(), &reference).is_err());
        assert!(capture(&runtime, &id).is_err());
        let relative = image.strip_prefix(&directory)?;
        assert_eq!(fs::read(external.join(relative))?, png()?);
        assert_eq!(
            runtime
                .store()
                .lock()
                .unwrap()
                .list::<beaver_core::asset_task::Reference>("asset-reference")?
                .len(),
            1
        );
        drop(junction);
    }
    Ok(())
}

#[tokio::test]
async fn checkpoint_capture_restore_and_pruning_reject_linked_directories() -> Result<()> {
    for level in ["home", "checkpoints"] {
        let temp = tempfile::tempdir()?;
        let (runtime, id) = project(temp.path())?;
        let agent = super::project_checkpoints::context(&runtime, &id, 0)?;
        for index in 0..5 {
            fs::write(
                agent.client.checkpoints.join(format!("{index}.blend")),
                b"BLENDER outside scene",
            )?;
        }
        let recorded = format!(".beaver/workspaces/.codex/{id}/asset-checkpoints/0.blend");
        let directory = if level == "home" {
            agent.files.codex_home(&id)?
        } else {
            agent.client.checkpoints.clone()
        };
        let external = temp.path().join("external");
        fs::rename(&directory, &external)?;
        let junction = Junction::create(&directory, &external)?;
        assert!(agent.files.resolve_checkpoint(&recorded).is_err());
        assert!(agent.checkpoint().await.is_err());
        let outside_checkpoints = external.join(agent.client.checkpoints.strip_prefix(&directory)?);
        assert_eq!(fs::read_dir(&outside_checkpoints)?.count(), 5);
        for index in 0..5 {
            assert_eq!(
                fs::read(outside_checkpoints.join(format!("{index}.blend")))?,
                b"BLENDER outside scene"
            );
        }
        drop(junction);
    }
    Ok(())
}
