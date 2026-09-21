use anyhow::Result;
use beaver_core::{object_framework_status::status, store::Store};
use serde_json::{json, Value};
use std::fs;

#[test]
fn registration_diagnosis_is_read_only_and_independent_of_runtime() -> Result<()> {
    use beaver_core::object_framework_status::registration_status;
    let temp = tempfile::tempdir()?;
    let store = Store::open(&temp.path().join("host"))?;
    let root = temp.path().join("offline");
    let record = json!({"id":"p", "path":root});
    store.put("project", "p", &record)?;
    assert_eq!(registration_status(&store, "p")?["state"], "offline");
    assert!(!root.exists());
    fs::create_dir(&root)?;
    assert_eq!(registration_status(&store, "p")?["state"], "legacy");
    assert!(!root.join(".beaver").exists());
    fs::create_dir(root.join(".beaver"))?;
    let manifest = root.join(".beaver/project.json");
    let bytes = br#"{"schemaVersion":1,"storageVersion":1,"projectId":"p"}"#;
    fs::write(&manifest, bytes)?;
    let diagnosis = registration_status(&store, "p")?;
    assert_eq!(diagnosis["state"], "detected");
    assert_eq!(diagnosis["action"], "none");
    assert_eq!(diagnosis["registeredPath"], root.to_string_lossy().as_ref());
    assert!(diagnosis["conflictProjectId"].is_null());
    assert!(diagnosis["message"]
        .as_str()
        .unwrap()
        .contains("尚未检查数据库"));
    assert_eq!(fs::read(&manifest)?, bytes);
    assert_eq!(fs::read_dir(root.join(".beaver"))?.count(), 1);
    fs::write(&manifest, b"invalid")?;
    assert_eq!(registration_status(&store, "p")?["state"], "invalid");
    assert_eq!(fs::read(&manifest)?, b"invalid");
    assert_eq!(
        registration_status(&store, "p")?["action"],
        "repair_storage"
    );
    assert_eq!(store.get::<Value>("project", "p")?, Some(record));
    assert!(registration_status(&store, "missing").is_err());
    Ok(())
}

#[test]
fn registration_diagnosis_reports_same_path_conflict_without_opening_projects() -> Result<()> {
    use beaver_core::object_framework_status::registration_status;
    let temp = tempfile::tempdir()?;
    let store = Store::open(&temp.path().join("host"))?;
    let project = temp.path().join("moved");
    store.put("project", "first", &json!({"id":"first","path":project}))?;
    store.put(
        "project",
        "second",
        &json!({"id":"second","path":project.to_string_lossy().to_uppercase()}),
    )?;
    let diagnosis = registration_status(&store, "first")?;
    assert_eq!(diagnosis["state"], "offline");
    assert_eq!(diagnosis["action"], "resolve_registration_conflict");
    assert_eq!(diagnosis["conflictProjectId"], "second");
    assert!(!project.exists());
    Ok(())
}

#[test]
fn readiness_reads_project_manifest_without_initializing_storage() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let store = Store::open(&temp.path().join("host"))?;
    let project = temp.path().join("project");
    fs::create_dir(&project)?;
    let record = json!({"id":"project","path":project});
    store.put("project", "project", &record)?;
    assert_eq!(status(&store, "project")?["storage"]["state"], "legacy");
    assert!(!project.join(".beaver").exists());
    let control = project.join(".beaver");
    fs::create_dir(&control)?;
    let manifest =
        br#"{"schemaVersion":1,"storageVersion":1,"projectId":"project","history":{"keep":true}}"#;
    fs::write(control.join("project.json"), manifest)?;
    let state = status(&store, "project")?;
    assert_eq!(state["storage"]["state"], "detected");
    assert_eq!(
        state["capabilities"],
        json!({"objectsRead":false,"manufactureRead":false,"execution":false})
    );
    assert_eq!(state["storage"]["databasePath"], ".beaver/project.sqlite");
    assert_eq!(state["storage"]["routed"], false);
    assert!(state["blockers"]
        .as_array()
        .unwrap()
        .iter()
        .any(|blocker| blocker["code"] == "PROJECT_STORAGE_NOT_ROUTED"));
    let routed =
        beaver_core::object_framework_status::status_with_routing(&store, "project", true)?;
    assert_eq!(routed["storage"]["routed"], true);
    assert!(routed["blockers"]
        .as_array()
        .unwrap()
        .iter()
        .all(|blocker| blocker["code"] != "PROJECT_STORAGE_NOT_ROUTED"));
    assert_eq!(routed["capabilities"]["objectsRead"], false);
    assert_eq!(fs::read(control.join("project.json"))?, manifest);
    assert_eq!(fs::read_dir(&control)?.count(), 1);
    assert_eq!(store.get::<Value>("project", "project")?, Some(record));
    assert!(store.list::<Value>("task")?.is_empty());
    Ok(())
}

#[test]
fn incompatible_missing_and_offline_storage_stays_unavailable_and_untouched() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let store = Store::open(&temp.path().join("host"))?;
    let project = temp.path().join("project");
    let control = project.join(".beaver");
    fs::create_dir_all(&control)?;
    store.put(
        "project",
        "project",
        &json!({"id":"project","path":project}),
    )?;
    assert_eq!(status(&store, "project")?["storage"]["state"], "invalid");
    for data in [
        b"not-json".to_vec(),
        br#"{"schemaVersion":2,"storageVersion":1,"projectId":"project"}"#.to_vec(),
        br#"{"schemaVersion":1,"storageVersion":1,"projectId":"other"}"#.to_vec(),
        vec![b' '; 65537],
    ] {
        fs::write(control.join("project.json"), &data)?;
        assert_eq!(status(&store, "project")?["storage"]["state"], "invalid");
        assert_eq!(fs::read(control.join("project.json"))?, data);
        assert!(!control.join("project.sqlite").exists());
    }
    store.put(
        "project",
        "offline",
        &json!({"id":"offline","path":temp.path().join("missing")}),
    )?;
    assert_eq!(status(&store, "offline")?["storage"]["state"], "offline");
    let offline_routed =
        beaver_core::object_framework_status::status_with_routing(&store, "offline", true)?;
    assert_eq!(offline_routed["storage"]["routed"], false);
    assert!(!temp.path().join("missing").exists());
    assert!(status(&store, "unknown").is_err());
    Ok(())
}
