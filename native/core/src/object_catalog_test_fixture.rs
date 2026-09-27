use crate::{
    object_catalog::{self, ObjectRecord, ObjectReference, ObjectVersion},
    object_registration::{RegisterRequest, UpdateRequest},
    object_version_capture::CaptureRequest,
    project_runtime::ProjectRuntime,
    project_storage::ProjectStore,
};
use anyhow::Result;
use serde_json::{json, Value};
use std::fs;

pub(crate) struct Fixture {
    pub runtime: ProjectRuntime,
    pub temp: tempfile::TempDir,
}

impl Fixture {
    pub fn new() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        fs::write(temp.path().join("project.godot"), "config_version=5\n")?;
        let project = ProjectStore::initialize(temp.path(), "project-1")?;
        project
            .store()
            .put("project", "project-1", &json!({"id":"project-1"}))?;
        Ok(Self {
            runtime: project.into_runtime(),
            temp,
        })
    }

    pub fn request(&self, request_id: &str) -> RegisterRequest {
        RegisterRequest {
            project_id: "project-1".into(),
            request_id: request_id.into(),
            name: "Empty object".into(),
            category: object_catalog::default_category(),
            tags: vec![],
            thumbnail_path: None,
            parent_object_id: None,
            components: vec![],
            files: vec![],
            references: vec![],
        }
    }

    pub fn object(&self, id: &str) -> Result<ObjectRecord> {
        Ok(object_catalog::get(&self.runtime.store().lock().unwrap(), id)?.unwrap())
    }

    pub fn count(&self, kind: &str) -> Result<usize> {
        Ok(self
            .runtime
            .store()
            .lock()
            .unwrap()
            .list::<Value>(kind)?
            .len())
    }

    /// Seed previously accepted history; public registration deliberately has no acceptance input.
    pub fn accepted(&self, id: &str, version_id: &str) -> Result<ObjectReference> {
        let version = ObjectVersion {
            version_id: version_id.into(),
            manifest: json!({
                "schemaVersion":1, "projectId":"project-1", "objectId":id, "versionId":version_id,
                "status":"accepted", "name":"Accepted history", "category":"其他", "tags":[],
                "thumbnailPath":null, "parentObjectId":null, "components":[], "files":[], "references":[],
            }),
        };
        let object = ObjectRecord {
            id: id.into(),
            project_id: "project-1".into(),
            name: "Working name".into(),
            category: object_catalog::default_category(),
            tags: vec![],
            thumbnail_path: None,
            parent_object_id: None,
            revision: 0,
            components: vec![],
            files: vec![],
            references: vec![],
            versions: vec![version.clone()],
        };
        let store = self.runtime.store();
        let store = store.lock().unwrap();
        store.put("object", id, &object)?;
        store.put("object_version", version_id, &(id, version))?;
        Ok(ObjectReference {
            project_id: "project-1".into(),
            object_id: id.into(),
            version_id: Some(version_id.into()),
        })
    }
}

pub(crate) fn update_request(object: &ObjectRecord, request_id: &str) -> UpdateRequest {
    UpdateRequest {
        project_id: object.project_id.clone(),
        object_id: object.id.clone(),
        request_id: request_id.into(),
        expected_revision: object.revision,
        name: object.name.clone(),
        category: object.category.clone(),
        tags: object.tags.clone(),
        thumbnail_path: object.thumbnail_path.clone(),
        parent_object_id: object.parent_object_id.clone(),
        components: object.components.clone(),
        files: object.files.clone(),
        references: object.references.clone(),
    }
}

pub(crate) fn capture_request(object: &ObjectRecord, request_id: &str) -> CaptureRequest {
    CaptureRequest {
        project_id: object.project_id.clone(),
        object_id: object.id.clone(),
        request_id: request_id.into(),
        expected_revision: object.revision,
    }
}
