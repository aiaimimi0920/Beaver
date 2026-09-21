use super::{safe_path, Files};
use anyhow::{Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

impl Files {
    pub fn workspace(&self, task_id: &str) -> Result<PathBuf> {
        anyhow::ensure!(
            !task_id.contains('/') && task_id != ".codex",
            "非法任务工作区标识"
        );
        match &self.project {
            Some((root, _)) => safe_path(root, &format!(".beaver/workspaces/{task_id}")),
            None => safe_path(&self.root, &format!("workspaces/{task_id}")),
        }
    }

    /// Persistent location; resolve against the current project root before file access.
    pub fn workspace_location(&self, task_id: &str) -> Result<String> {
        let workspace = self.workspace(task_id)?;
        if self.project.is_some() {
            Ok(format!(".beaver/workspaces/{task_id}"))
        } else {
            Ok(workspace
                .to_str()
                .context("任务工作副本路径不是 UTF-8")?
                .to_owned())
        }
    }

    pub fn resolve_workspace(&self, task_id: &str, recorded: &Path) -> Result<PathBuf> {
        // Legacy tasks can have nonstandard work copies. Project storage owns its layout.
        if self.project.is_none() {
            return Ok(recorded.to_owned());
        }
        let expected = self.workspace(task_id)?;
        // Compare raw spelling: Path equality would accept aliases such as extra separators.
        anyhow::ensure!(
            recorded.to_str() == Some(format!(".beaver/workspaces/{task_id}").as_str()),
            "任务工作副本不属于当前项目或任务"
        );
        anyhow::ensure!(expected.is_dir(), "任务工作副本不存在");
        // Use the checked storage path, never an alias supplied by the task record.
        Ok(expected)
    }

    pub fn codex_home(&self, task_id: &str) -> Result<PathBuf> {
        anyhow::ensure!(
            !task_id.is_empty()
                && task_id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
            "任务标识无效"
        );
        // Keep sessions outside task work copies, so restoring a copy cannot replace its HOME.
        match &self.project {
            Some((root, _)) => safe_path(root, &format!(".beaver/workspaces/.codex/{task_id}")),
            None => safe_path(&self.root, &format!("codex/{task_id}")),
        }
    }

    pub fn checkpoints(&self, task_id: &str) -> Result<PathBuf> {
        let home = self.codex_home(task_id)?;
        // Check from an existing root even before HOME has been created.
        match &self.project {
            Some((root, _)) => safe_path(
                root,
                &format!(".beaver/workspaces/.codex/{task_id}/asset-checkpoints"),
            ),
            None => Ok(home.join("asset-checkpoints")),
        }
    }

    pub(crate) fn check_checkpoint_directory(&self, task_id: &str, path: &Path) -> Result<()> {
        if self.project.is_some() {
            anyhow::ensure!(
                path == self.checkpoints(task_id)?,
                "Checkpoint directory does not belong to this task"
            );
        }
        Ok(())
    }

    pub(crate) fn checkpoint_location(&self, task_id: &str, path: &Path) -> Result<String> {
        if self.project.is_none() {
            return Ok(path.to_string_lossy().into_owned());
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .context("Invalid checkpoint name")?;
        let relative = format!(".beaver/workspaces/.codex/{task_id}/asset-checkpoints/{name}");
        let expected = self.resolve_checkpoint(&relative)?;
        anyhow::ensure!(
            path.is_absolute() && fs::canonicalize(path)? == fs::canonicalize(&expected)?,
            "Checkpoint does not belong to this task"
        );
        Ok(relative)
    }

    /// A follow-up may restore an earlier task's checkpoint within the same project.
    pub fn resolve_checkpoint(&self, recorded: &str) -> Result<PathBuf> {
        let Some((root, _)) = &self.project else {
            return Ok(PathBuf::from(recorded));
        };
        let parts: Vec<_> = recorded.split('/').collect();
        anyhow::ensure!(
            parts.len() == 6
                && parts[..3] == [".beaver", "workspaces", ".codex"]
                && parts[4] == "asset-checkpoints"
                && Path::new(parts[5])
                    .extension()
                    .is_some_and(|ext| ext == "blend"),
            "Invalid project checkpoint location"
        );
        self.codex_home(parts[3])?;
        safe_path(root, recorded)
    }

    pub(crate) fn delivery_exports(&self) -> Result<PathBuf> {
        match &self.project {
            Some((root, _)) => safe_path(root, ".beaver/cache/delivery-exports"),
            None => safe_path(&self.root, "delivery-exports"),
        }
    }

    pub(crate) fn validation_evidence(&self, run_id: &str) -> Result<PathBuf> {
        uuid::Uuid::parse_str(run_id).context("Invalid run ID")?;
        match &self.project {
            Some((root, _)) => safe_path(root, &format!(".beaver/evidence/{run_id}")),
            None => safe_path(&self.root, &format!("validation/{run_id}")),
        }
    }

    pub(crate) fn observer_reference_location(&self, task_id: &str, id: &str) -> Result<String> {
        crate::asset_task::validate_id(task_id)?;
        crate::asset_task::validate_id(id)?;
        let relative = format!("asset-observer/{task_id}/references/{id}.png");
        match &self.project {
            Some(_) => Ok(format!(".beaver/evidence/{relative}")),
            None => Ok(safe_path(&self.root, &relative)?
                .to_string_lossy()
                .into_owned()),
        }
    }

    pub(crate) fn resolve_observer_reference(
        &self,
        task_id: &str,
        id: &str,
        recorded: &str,
    ) -> Result<PathBuf> {
        match &self.project {
            Some((root, _)) => {
                let expected = self.observer_reference_location(task_id, id)?;
                anyhow::ensure!(
                    recorded == expected,
                    "Reference image does not belong to this task"
                );
                safe_path(root, &expected)
            }
            // Historical references recorded an absolute path, including archived images.
            None => Ok(PathBuf::from(recorded)),
        }
    }
}
