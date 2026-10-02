//! Map owned filesystem namespaces, never arbitrary names inside work copies or sessions.
use crate::{
    data_backup::Entry, project_derivation_identity::IdentityMap,
    project_derivation_validation_records::Rewrite, project_storage_layout,
};
use anyhow::{ensure, Context, Result};
use std::collections::BTreeSet;

pub(crate) struct Paths<'a>(pub(crate) &'a IdentityMap);

pub(crate) fn valid_relative_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(['\\', ':', '\0'])
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

impl Paths<'_> {
    fn workspace_identity(&self, id: &str, codex: bool) -> Result<String> {
        let kinds = [
            "task",
            if codex {
                "object_attempt"
            } else {
                "object_run"
            },
        ];
        let matches: Vec<_> = kinds
            .into_iter()
            .filter(|kind| {
                self.0
                    .entities
                    .iter()
                    .any(|e| e.source.kind == *kind && e.source.id == id)
            })
            .collect();
        ensure!(
            matches.len() == 1,
            "ambiguous or unknown workspace identity"
        );
        self.identity(matches[0], id)
    }

    fn identity(&self, kind: &str, id: &str) -> Result<String> {
        let key = Rewrite(self.0).key(kind, id)?;
        ensure!(key.kind == kind, "filesystem identity kind mismatch");
        project_storage_layout::valid_id(&key.id)?;
        Ok(key.id)
    }

    pub(crate) fn relative(&self, path: &str) -> Result<String> {
        ensure!(
            valid_relative_path(path),
            "invalid derivation relative path"
        );
        let mut parts: Vec<String> = path.split('/').map(str::to_owned).collect();
        if parts.first().map(String::as_str) != Some(".beaver") {
            return Ok(path.into());
        }
        match parts.get(1).map(String::as_str) {
            Some("workspaces") if parts.len() >= 3 => {
                let slot = if parts[2] == ".codex" { 3 } else { 2 };
                if parts.len() > slot {
                    parts[slot] = self.workspace_identity(&parts[slot], slot == 3)?;
                }
            }
            Some("evidence") if parts.len() >= 3 => {
                if parts[2] == "asset-observer" {
                    if parts.len() >= 4 {
                        parts[3] = self.identity("task", &parts[3])?;
                    }
                    if parts.len() >= 6 && parts[4] == "references" {
                        ensure!(parts.len() == 6, "reference image cannot contain children");
                        let id = parts[5]
                            .strip_suffix(".png")
                            .context("invalid reference filename")?;
                        parts[5] = format!("{}.png", self.identity("asset-reference", id)?);
                    }
                } else {
                    parts[2] = self.identity("validationRun", &parts[2])?;
                }
            }
            _ => {}
        }
        Ok(parts.join("/"))
    }

    /// Same bytes and entry types, new namespace only. Reject aliases even on case-sensitive hosts.
    pub(crate) fn entries(&self, source: &[Entry]) -> Result<Vec<Entry>> {
        ensure!(
            self.0.format == "beaver-project-derivation-identities-v1",
            "unsupported identity map"
        );
        let mut names = BTreeSet::new();
        let mut result = Vec::with_capacity(source.len());
        for entry in source {
            let path = self.relative(&entry.path)?;
            ensure!(
                names.insert(path.to_lowercase()),
                "derived filesystem path collision"
            );
            result.push(Entry {
                path,
                ..entry.clone()
            });
        }
        result.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(result)
    }
}
