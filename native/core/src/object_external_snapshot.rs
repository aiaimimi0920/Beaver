//! Lease-backed catalog access without opening or modifying the source database.
use crate::{
    object_source_snapshot::ObjectSourceSnapshot, project_storage, project_storage_database,
    project_storage_layout as layout,
};
use anyhow::Result;
use std::{
    fs::File,
    path::{Path, PathBuf},
};

pub fn read(
    project_root: &Path,
    project_id: &str,
    query: Option<&str>,
) -> Result<ObjectSourceSnapshot> {
    Ok(ExternalSource::open(project_root, project_id)?
        .snapshot()?
        .inspect(project_id, query))
}

pub(crate) struct ExternalSource {
    root: PathBuf,
    manifest: layout::Manifest,
    _lock: File,
}

impl ExternalSource {
    pub(crate) fn open(project_root: &Path, project_id: &str) -> Result<Self> {
        let root = layout::root(project_root)?;
        let directory = root.join(layout::CONTROL_DIR);
        layout::ordinary(&directory, true)?;
        layout::ordinary(&directory.join(layout::LOCK), false)?;
        let lock = project_storage::lock(&directory, false)?;
        let manifest = layout::manifest_in(&root, Some(project_id))?;
        layout::validate_files(&root)?;
        Ok(Self {
            root,
            manifest,
            _lock: lock,
        })
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn snapshot(&self) -> Result<ObjectSourceSnapshot> {
        project_storage_database::snapshot(&self.root.join(layout::CONTROL_DIR), &self.manifest)?
            .object_catalog(&self.manifest.project_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{object_catalog, project_storage::ProjectStore, project_storage_layout};
    use std::fs;

    #[test]
    fn reads_unregistered_project_without_writing_source() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let project = temp.path().join("project");
        fs::create_dir_all(&project)?;
        fs::write(
            project.join("project.godot"),
            "[application]\nconfig/name=External\n",
        )?;
        let mut store = ProjectStore::initialize(&project, "external-1")?;
        store.store_mut().put(
            "project",
            "external-1",
            &serde_json::json!({"id":"external-1"}),
        )?;
        drop(store);
        let root = project_storage_layout::root(&project)?;
        let mut opened = ProjectStore::open(&project, "external-1")?;
        object_catalog::register(
            opened.store_mut(),
            &object_catalog::ObjectRecord {
                id: "object-1".into(),
                project_id: "external-1".into(),
                name: "Hero".into(),
                category: "其他".into(),
                tags: vec![],
                thumbnail_path: None,
                parent_object_id: None,
                revision: 0,
                components: vec![],
                files: vec![],
                references: vec![],
                versions: vec![],
            },
        )?;
        drop(opened);
        let before = fs::read(root.join(".beaver/project.sqlite"))?;
        let snapshot = read(&project, "external-1", Some("hero"))?;
        assert_eq!(snapshot.objects.len(), 1);
        assert_eq!(fs::read(root.join(".beaver/project.sqlite"))?, before);
        Ok(())
    }

    #[test]
    fn rejects_wrong_identity() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let project = temp.path().join("project");
        fs::create_dir_all(&project)?;
        fs::write(
            project.join("project.godot"),
            "[application]\nconfig/name=External\n",
        )?;
        let store = ProjectStore::initialize(&project, "external-1")?;
        drop(store);
        assert!(read(&project, "other", None).is_err());
        Ok(())
    }
}
