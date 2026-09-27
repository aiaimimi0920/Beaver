use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum State {
    Pending,
    Running,
    Completed,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Journal {
    pub request_id: String,
    pub owner_id: String,
    pub workspace: String,
    pub state: State,
    pub error: Option<String>,
}

impl Journal {
    pub fn new(
        request_id: impl Into<String>,
        owner_id: impl Into<String>,
        workspace: impl Into<String>,
    ) -> Result<Self> {
        let request_id = request_id.into();
        let owner_id = owner_id.into();
        let workspace = workspace.into();
        ensure!(
            !request_id.is_empty() && !owner_id.is_empty(),
            "INVALID_RESOURCE_JOURNAL_ID"
        );
        ensure!(!workspace.is_empty(), "INVALID_RESOURCE_JOURNAL_WORKSPACE");
        Ok(Self {
            request_id,
            owner_id,
            workspace,
            state: State::Pending,
            error: None,
        })
    }
    pub fn start(&mut self, owner_id: &str) -> Result<()> {
        ensure!(self.owner_id == owner_id, "RESOURCE_JOURNAL_OWNER_MISMATCH");
        ensure!(
            matches!(self.state, State::Pending | State::Blocked),
            "RESOURCE_JOURNAL_NOT_RETRYABLE"
        );
        self.state = State::Running;
        self.error = None;
        Ok(())
    }
    pub fn complete(&mut self, owner_id: &str) -> Result<()> {
        ensure!(self.owner_id == owner_id, "RESOURCE_JOURNAL_OWNER_MISMATCH");
        ensure!(self.state == State::Running, "RESOURCE_JOURNAL_NOT_RUNNING");
        self.state = State::Completed;
        Ok(())
    }
    pub fn block(&mut self, owner_id: &str, error: impl Into<String>) -> Result<()> {
        ensure!(self.owner_id == owner_id, "RESOURCE_JOURNAL_OWNER_MISMATCH");
        ensure!(self.state == State::Running, "RESOURCE_JOURNAL_NOT_RUNNING");
        self.state = State::Blocked;
        self.error = Some(error.into());
        Ok(())
    }
    pub fn resolve_workspace(&self, project_root: &Path) -> Result<PathBuf> {
        crate::files::safe_path(project_root, &self.workspace)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    #[test]
    fn retry_and_owner_are_enforced() -> Result<()> {
        let mut j = Journal::new("req-1", "owner-a", ".beaver/workspaces/run-1")?;
        j.start("owner-a")?;
        j.block("owner-a", "child still running")?;
        assert!(j.start("owner-b").is_err());
        j.start("owner-a")?;
        j.complete("owner-a")?;
        assert!(j.start("owner-a").is_err());
        Ok(())
    }
    #[test]
    fn workspace_escape_is_rejected() -> Result<()> {
        let root = tempdir()?;
        assert!(Journal::new("r", "o", ".beaver/workspaces/run")
            .unwrap()
            .resolve_workspace(root.path())
            .is_ok());
        assert!(Journal::new("r", "o", "../outside")
            .unwrap()
            .resolve_workspace(root.path())
            .is_err());
        assert!(Journal::new("r", "o", "missing/../../outside")?
            .resolve_workspace(root.path())
            .is_err());
        Ok(())
    }
}
