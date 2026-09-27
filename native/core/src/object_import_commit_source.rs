//! Source-specific preparation and capture for the shared durable import engine.
use super::{files, plan};
use crate::{
    object_catalog::ObjectRecord, object_file_import_preparation as file,
    object_import_preparation as project, project_runtime::ProjectRuntime,
    project_storage_router::ProjectStorageRouter,
};
use anyhow::{ensure, Context, Result};

#[path = "object_project_import_plan.rs"]
mod project_plan;

pub(super) enum Source {
    Files(file::Preparation),
    Project(project::Preparation),
}

pub(super) fn kind(id: &str) -> &'static str {
    if id.starts_with("file-import-") {
        "file_object_import"
    } else {
        "object_import"
    }
}

impl Source {
    pub(super) fn load(runtime: &ProjectRuntime, id: &str) -> Result<Self> {
        let store = runtime.store();
        let store = store
            .lock()
            .map_err(|_| anyhow::anyhow!("IMPORT_STORE_LOCK"))?;
        let source = if id.starts_with("file-import-") {
            Self::Files(file::get(&store, id)?.context("IMPORT_PREPARATION_NOT_FOUND")?)
        } else {
            Self::Project(project::get(&store, id)?.context("IMPORT_PREPARATION_NOT_FOUND")?)
        };
        let target = match &source {
            Self::Files(receipt) => &receipt.target_project_id,
            Self::Project(receipt) => &receipt.target_project_id,
        };
        ensure!(target == runtime.project_id(), "IMPORT_TARGET_MISMATCH");
        Ok(source)
    }

    pub(super) fn objects(&self) -> Result<Vec<ObjectRecord>> {
        match self {
            Self::Files(receipt) => plan::objects(receipt),
            Self::Project(receipt) => project_plan::objects(receipt),
        }
    }

    pub(super) fn freeze(
        &self,
        runtime: &ProjectRuntime,
        router: Option<&ProjectStorageRouter>,
    ) -> Result<()> {
        let receipt = match self {
            Self::Files(receipt) => return files::freeze(runtime, receipt),
            Self::Project(receipt) => receipt,
        };
        let router = router.context("IMPORT_SOURCE_ROUTER_REQUIRED")?;
        let source =
            router.object_import_source(&receipt.source_path, &receipt.source_project_id)?;
        let target = std::fs::canonicalize(runtime.project_root())?;
        let root = std::fs::canonicalize(source.root())?;
        ensure!(
            !target.starts_with(&root) && !root.starts_with(&target),
            "IMPORT_SOURCE_TARGET_OVERLAP"
        );
        let snapshot = source.snapshot()?;
        let selected = crate::object_import_snapshot::select(
            &snapshot.objects,
            &receipt.source_project_id,
            &receipt.source_object_id,
            &receipt.accepted_version_id,
        )?;
        ensure!(
            selected.versions == receipt.versions && selected.digest == receipt.source_digest,
            "IMPORT_SOURCE_CHANGED"
        );
        crate::object_import_content::verify(source.root(), &receipt.versions)?;
        let mut copied = std::collections::BTreeSet::new();
        for version in &receipt.versions {
            for file in &version.files {
                if !copied.insert(&file.sha256) {
                    continue;
                }
                let path = format!(".beaver/content/blobs/{}", file.sha256);
                let blob = crate::files::safe_path(source.root(), &path)?;
                let parent = blob.parent().context("IMPORT_SOURCE_PARENT_MISSING")?;
                let captured = runtime.files().capture_paths(
                    parent,
                    vec![file.sha256.clone()],
                    Some(file.bytes),
                )?;
                ensure!(
                    captured.get(&file.sha256) == Some(&file.sha256),
                    "IMPORT_SOURCE_CHANGED"
                );
            }
        }
        crate::object_import_content::verify(source.root(), &receipt.versions)
    }

    pub(super) fn verify_frozen(&self, runtime: &ProjectRuntime) -> Result<()> {
        if let Self::Project(receipt) = self {
            crate::object_import_content::verify(runtime.project_root(), &receipt.versions)?;
        }
        Ok(())
    }

    pub(super) fn provenance(&self, db: &rusqlite::Connection) -> Result<()> {
        if let Self::Project(receipt) = self {
            for (source, target) in &receipt.identity_map.objects {
                db.execute("INSERT INTO entities(kind,id,value) VALUES('object_import_origin',?,?)",
                    rusqlite::params![target, serde_json::to_string(&serde_json::json!({
                        "preparationId": receipt.preparation_id, "sourceProjectId": receipt.source_project_id,
                        "sourceObjectId": source, "targetObjectId": target,
                        "sourceDigest": receipt.source_digest
                    }))?])?;
            }
        }
        Ok(())
    }
}
