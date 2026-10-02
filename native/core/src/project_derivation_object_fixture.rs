use crate::{
    object_catalog::{ObjectRecord, ObjectVersion},
    object_registration::{self, CommandResult},
    object_version_acceptance, object_version_capture, project_derivation_copy as copy,
    project_runtime::ProjectRuntime,
    project_storage::ProjectStore,
};
use anyhow::Result;
use serde_json::json;
use std::{fs, path::PathBuf};

pub(super) struct Fixture {
    pub temp: tempfile::TempDir,
    pub source: PathBuf,
    pub parent: ObjectRecord,
    pub child: ObjectRecord,
    pub first: ObjectVersion,
    pub second: ObjectVersion,
}

pub(super) fn capture(
    runtime: &ProjectRuntime,
    object: &ObjectRecord,
    request: &str,
) -> Result<CommandResult> {
    object_version_capture::capture(
        runtime,
        &object_version_capture::CaptureRequest {
            project_id: runtime.project_id().into(),
            request_id: request.into(),
            object_id: object.id.clone(),
            expected_revision: object.revision,
        },
    )
}

pub(super) fn accept(
    runtime: &ProjectRuntime,
    object: &ObjectRecord,
    version: &str,
    request: &str,
) -> Result<CommandResult> {
    object_version_acceptance::accept(
        runtime,
        &object_version_acceptance::AcceptanceRequest {
            project_id: runtime.project_id().into(),
            request_id: request.into(),
            object_id: object.id.clone(),
            expected_revision: object.revision,
            version_id: version.into(),
        },
    )
}

pub(super) fn update(
    runtime: &ProjectRuntime,
    object: &ObjectRecord,
    request: &str,
) -> Result<CommandResult> {
    object_registration::update(
        runtime,
        &object_registration::UpdateRequest {
            project_id: runtime.project_id().into(),
            request_id: request.into(),
            object_id: object.id.clone(),
            expected_revision: object.revision,
            name: object.name.clone(),
            category: object.category.clone(),
            tags: object.tags.clone(),
            thumbnail_path: object.thumbnail_path.clone(),
            parent_object_id: object.parent_object_id.clone(),
            components: object.components.clone(),
            files: object.files.clone(),
            references: object.references.clone(),
        },
    )
}

impl Fixture {
    pub fn new() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let source = temp.path().join("source");
        fs::create_dir(&source)?;
        fs::write(source.join("project.godot"), "config_version=5\n")?;
        fs::write(
            source.join("parent.txt"),
            "frozen one: original component-parent",
        )?;
        let storage = ProjectStore::initialize(&source, "original")?;
        storage.store().put(
            "project",
            "original",
            &json!({"id":"original","name":"Original","path":source}),
        )?;
        let runtime = storage.into_runtime();
        let parent = object_registration::register(
            &runtime,
            &serde_json::from_value(json!({
                "projectId":"original","requestId":"register-parent","name":"Parent",
                "components":[{"id":"component-parent","kind":"mesh","name":"Original mesh"}],
                "files":[{"path":"parent.txt","role":"source"}]
            }))?,
        )?
        .object;
        let first_capture = capture(&runtime, &parent, "capture-one")?;
        fs::write(source.join("parent.txt"), "frozen two")?;
        let second_capture = capture(&runtime, &first_capture.object, "capture-two")?;
        let accepted_second = accept(
            &runtime,
            &second_capture.object,
            second_capture.version_id.as_deref().unwrap(),
            "accept-two",
        )?;
        let parent = accept(
            &runtime,
            &accepted_second.object,
            first_capture.version_id.as_deref().unwrap(),
            "accept-one",
        )?
        .object;
        let first = parent.versions[0].clone();
        let second = parent.versions[1].clone();
        let child = object_registration::register(&runtime, &serde_json::from_value(json!({
            "projectId":"original","requestId":"register-child","name":"Child","category":"old category",
            "parentObjectId":parent.id,
            "components":[{"id":"component-old","kind":"mesh","name":"Old mesh"}],
            "references":[{"projectId":"original","objectId":parent.id,"versionId":first.version_id}]
        }))?)?.object;
        let child_capture = capture(&runtime, &child, "capture-child")?;
        let mut child = accept(
            &runtime,
            &child_capture.object,
            child_capture.version_id.as_deref().unwrap(),
            "accept-child",
        )?
        .object;
        child.category = "new category".into();
        child.components[0].id = "component-new".into();
        child.tags = vec!["new tag".into()];
        let child = update(&runtime, &child, "update-child")?.object;
        // Metadata may reserve a future path without any working file yet.
        object_registration::register(
            &runtime,
            &serde_json::from_value(json!({
                "projectId":"original","requestId":"register-empty","name":"Empty",
                "files":[{"path":"not-created-yet.txt","role":"source"}]
            }))?,
        )?;
        // Working bytes intentionally differ from both historical snapshots.
        fs::write(
            source.join("parent.txt"),
            "working original component-parent",
        )?;
        drop(runtime);
        Ok(Self {
            temp,
            source,
            parent,
            child,
            first,
            second,
        })
    }

    pub fn request(&self) -> copy::Request {
        copy::Request {
            request_id: "derive".into(),
            source: self.source.clone(),
            source_project_id: "original".into(),
            target_project_id: "derived".into(),
        }
    }
}
