//! Resolve source ownership before reading; never open a runtime for inspection.
use super::ProjectStorageRouter;
use crate::{
    object_external_snapshot::ExternalSource, object_source_snapshot::ObjectSourceSnapshot,
    project_runtime::ProjectRuntime, project_storage_layout as layout,
};
use anyhow::{ensure, Context, Result};
use rusqlite::TransactionBehavior;
use serde_json::Value;
use std::path::{Path, PathBuf};

pub(crate) enum ObjectImportSource {
    Managed(ProjectRuntime),
    External(ExternalSource),
}

impl ObjectImportSource {
    pub(crate) fn root(&self) -> &Path {
        match self {
            Self::Managed(runtime) => runtime.project_root(),
            Self::External(source) => source.root(),
        }
    }

    pub(crate) fn snapshot(&self) -> Result<ObjectSourceSnapshot> {
        match self {
            Self::External(source) => source.snapshot(),
            Self::Managed(runtime) => {
                let handle = runtime.store();
                let mut store = handle
                    .lock()
                    .map_err(|_| anyhow::anyhow!("项目数据库锁不可用"))?;
                let transaction = store
                    .connection
                    .transaction_with_behavior(TransactionBehavior::Deferred)?;
                let snapshot = ObjectSourceSnapshot::read(&transaction, runtime.project_id())?;
                transaction.commit()?;
                Ok(snapshot)
            }
        }
    }
}

impl ProjectStorageRouter {
    pub fn inspect_object_source(
        &self,
        root: &Path,
        expected_project_id: Option<&str>,
        query: Option<&str>,
    ) -> Result<ObjectSourceSnapshot> {
        let manifest = layout::read_manifest(root, expected_project_id)?;
        let project_id = manifest.project_id.as_str();
        Ok(self
            .object_import_source(root, project_id)?
            .snapshot()?
            .inspect(project_id, query))
    }

    pub(crate) fn object_import_source(
        &self,
        root: &Path,
        project_id: &str,
    ) -> Result<ObjectImportSource> {
        layout::valid_id(project_id)?;
        let root = layout::root(root)?;
        let projects = self
            .projects
            .lock()
            .map_err(|_| anyhow::anyhow!("项目注册表锁不可用"))?;
        let runtime = projects.get(project_id);
        let registered = self
            .host
            .lock()
            .map_err(|_| anyhow::anyhow!("宿主数据库锁不可用"))?
            .list_with_ids::<Value>("project")?;
        for (key, project) in registered {
            if key == project_id || project["id"] == project_id {
                ensure!(
                    key == project_id && project["id"] == project_id,
                    "IMPORT_SOURCE_REGISTRATION_ID_MISMATCH"
                );
                let path = PathBuf::from(project["path"].as_str().context("项目登记缺少路径")?);
                ensure!(path.is_absolute(), "项目登记路径必须是绝对路径");
                // An open runtime can outlive a stale host path, but never a replacement identity.
                if runtime.is_none() || path.exists() {
                    ensure!(
                        layout::root(&path)? == root,
                        "IMPORT_SOURCE_REGISTERED_PATH_MISMATCH"
                    );
                }
            } else if let Some(path) = project["path"].as_str().map(Path::new) {
                if path.is_absolute() {
                    if let Ok(registered_root) = std::fs::canonicalize(path) {
                        ensure!(
                            registered_root != root,
                            "IMPORT_SOURCE_REGISTRATION_ID_MISMATCH"
                        );
                    }
                }
            }
        }
        for opened in projects.runtimes() {
            ensure!(
                opened.project_root() != root || opened.project_id() == project_id,
                "IMPORT_SOURCE_RUNTIME_ID_MISMATCH"
            );
        }
        if let Some(runtime) = runtime {
            ensure!(
                runtime.project_root() == root,
                "IMPORT_SOURCE_RUNTIME_PATH_MISMATCH"
            );
            layout::manifest_in(&root, Some(project_id))?;
            layout::validate_files(&root)?;
            Ok(ObjectImportSource::Managed(runtime))
        } else {
            // Acquire ownership before releasing the registry, so a local open cannot race us.
            Ok(ObjectImportSource::External(ExternalSource::open(
                &root, project_id,
            )?))
        }
    }
}
