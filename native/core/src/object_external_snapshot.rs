//! Read-only object catalog access for an unregistered Beaver project.
use crate::{
    object_catalog::ObjectRecord, project_storage, project_storage_database,
    project_storage_layout as layout,
};
use anyhow::{ensure, Result};
use serde::Serialize;
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalObjectSnapshot {
    pub project: Value,
    pub objects: Vec<ObjectRecord>,
}

pub fn read(
    project_root: &Path,
    project_id: &str,
    query: Option<&str>,
) -> Result<ExternalObjectSnapshot> {
    let root = layout::root(project_root)?;
    let manifest = layout::manifest_in(&root, Some(project_id))?;
    layout::validate_files(&root)?;
    let directory = root.join(layout::CONTROL_DIR);
    let _lock = project_storage::lock(&directory, false)?;
    let snapshot = project_storage_database::snapshot(&directory, &manifest)?;
    let project = snapshot
        .project(project_id)?
        .ok_or_else(|| anyhow::anyhow!("项目本地存储缺少项目实体"))?;
    ensure!(project["id"] == project_id, "项目本地实体 ID 与清单不一致");
    let objects = filter(snapshot.objects()?, project_id, query);
    Ok(ExternalObjectSnapshot { project, objects })
}

fn filter(objects: Vec<ObjectRecord>, project_id: &str, query: Option<&str>) -> Vec<ObjectRecord> {
    let query = query
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase);
    objects
        .into_iter()
        .filter(|object| {
            object.project_id == project_id
                && query.as_ref().map_or(true, |query| {
                    object.id.to_ascii_lowercase().contains(query)
                        || object.name.to_ascii_lowercase().contains(query)
                        || object.components.iter().any(|component| {
                            component.name.to_ascii_lowercase().contains(query)
                                || component.kind.to_ascii_lowercase().contains(query)
                        })
                        || object
                            .files
                            .iter()
                            .any(|file| file.path.to_ascii_lowercase().contains(query))
                })
        })
        .collect()
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
