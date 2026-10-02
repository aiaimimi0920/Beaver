use super::*;
use beaver_core::{
    object_catalog, object_registration, object_version_acceptance, object_version_capture,
};

#[test]
fn migration_derivation_api_preserves_object_versions_and_opens_editable_catalog() -> Result<()> {
    let _operation = super::super::super::TEST_OPERATION.lock().unwrap();
    let f = Fixture::new()?;
    let runtime = ProjectStore::open(&f.source, "original")?.into_runtime();
    let original = object_registration::register(
        &runtime,
        &serde_json::from_value(json!({
            "projectId":"original","requestId":"register-object","name":"Catalog object",
            "components":[{"id":"component-source","kind":"mesh","name":"Mesh"}]
        }))?,
    )?;
    let captured = object_version_capture::capture(
        &runtime,
        &object_version_capture::CaptureRequest {
            project_id: "original".into(),
            request_id: "capture".into(),
            object_id: original.object.id.clone(),
            expected_revision: 0,
        },
    )?;
    let accepted = object_version_acceptance::accept(
        &runtime,
        &object_version_acceptance::AcceptanceRequest {
            project_id: "original".into(),
            request_id: "accept".into(),
            object_id: original.object.id.clone(),
            expected_revision: 1,
            version_id: captured.version_id.unwrap(),
        },
    )?;
    drop(runtime);
    let before = data_backup::inventory(&f.source)?;
    let inspection = f.api(
        "migration.inspectDerivationSource",
        json!({"source":f.source}),
    )?;
    let id = inspection["request"]["targetProjectId"].as_str().unwrap();
    let mut request = inspection["request"].clone();
    request["preparation"] = json!(f.preparation);
    f.api("migration.prepareDerivation", request)?;
    f.api("migration.assembleDerivation", f.paths())?;
    f.api("migration.activateAssembly", f.paths())?;
    assert_eq!(
        f.api("migration.registerAssembly", f.paths())?["runtimeReady"],
        true
    );
    let runtime = f.router.open_registered(id)?;
    let objects = object_catalog::list(&runtime.store().lock().unwrap(), id)?;
    assert_eq!(objects.len(), 1);
    let object = &objects[0];
    assert_ne!(object.id, original.object.id);
    assert_ne!(object.components[0].id, original.object.components[0].id);
    assert_ne!(object.versions[0].version_id, accepted.version_id.unwrap());
    assert_eq!(object.versions[0].manifest["status"], "accepted");
    assert_eq!(object.versions[0].manifest["projectId"], id);
    let next = object_version_capture::capture(
        &runtime,
        &object_version_capture::CaptureRequest {
            project_id: id.into(),
            request_id: "capture-after-derivation".into(),
            object_id: object.id.clone(),
            expected_revision: object.revision,
        },
    )?;
    assert_eq!(next.object.versions.len(), 2);
    assert_eq!(
        f.api("migration.registerAssembly", f.paths())?["hostRegistrationChanged"],
        false
    );
    drop(runtime);
    f.router.close(id)?;
    let reopened = f.router.open_registered(id)?;
    assert_eq!(
        object_catalog::get(&reopened.store().lock().unwrap(), &object.id)?,
        Some(next.object)
    );
    assert_eq!(data_backup::inventory(&f.source)?, before);
    Ok(())
}
