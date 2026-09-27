use super::*;
use crate::{
    object_catalog_test_fixture::Fixture, object_file_import_preparation as preparation,
    object_import_file_source, project_storage::ProjectStore,
};
use std::fs;

fn prepare(f: &Fixture, source: &std::path::Path) -> Result<preparation::Preparation> {
    preparation::prepare(
        &f.runtime.store(),
        &preparation::Request {
            request_id: "import-one".into(),
            target_project_id: "project-1".into(),
            snapshot: object_import_file_source::inspect(&[source.to_path_buf()])?,
            groups: vec![],
        },
    )
}

#[test]
fn grouped_import_preserves_layout_and_source_and_is_not_accepted() -> Result<()> {
    let f = Fixture::new()?;
    let source = tempfile::tempdir()?;
    fs::create_dir(source.path().join("nested"))?;
    fs::write(source.path().join("main.txt"), "nested/asset.txt")?;
    fs::write(source.path().join("nested/asset.txt"), "asset")?;
    fs::write(source.path().join("other.txt"), "other")?;
    let snapshot = object_import_file_source::inspect(&[source.path().to_path_buf()])?;
    let paths = snapshot
        .files
        .iter()
        .filter(|file| file.relative_path != "other.txt")
        .map(|file| file.path.clone())
        .collect();
    let receipt = preparation::prepare(
        &f.runtime.store(),
        &preparation::Request {
            request_id: "grouped".into(),
            target_project_id: "project-1".into(),
            snapshot: snapshot.clone(),
            groups: vec![preparation::FileImportGroup {
                id: "group-one".into(),
                name: "Grouped assets".into(),
                paths,
            }],
        },
    )?;
    let imported = execute(&f.runtime, &receipt.preparation_id)?;
    assert_eq!(
        imported.state,
        State::ImportedPendingValidation,
        "{:?}",
        imported.error
    );
    assert_eq!(imported.object_ids.len(), 2);
    assert_eq!(f.count("object")?, 2);
    assert_eq!(f.count("object_version")?, 2);
    assert_eq!(
        object_import_file_source::inspect(&[source.path().to_path_buf()])?,
        snapshot
    );
    let grouped = f.object(&receipt.identity_map.groups["group-one"])?;
    assert_eq!(grouped.files.len(), 2);
    let main = grouped
        .files
        .iter()
        .find(|file| file.path.ends_with("/main.txt"))
        .unwrap();
    let parent = f
        .runtime
        .project_root()
        .join(&main.path)
        .parent()
        .unwrap()
        .to_path_buf();
    assert_eq!(
        fs::read_to_string(parent.join("nested/asset.txt"))?,
        "asset"
    );
    assert!(crate::object_version_manifest::read(&grouped, &grouped.versions[0]).is_err());
    assert!(crate::object_version_acceptance::accept(
        &f.runtime,
        &crate::object_version_acceptance::AcceptanceRequest {
            project_id: "project-1".into(),
            request_id: "must-not-accept".into(),
            object_id: grouped.id,
            version_id: grouped.versions[0].version_id.clone(),
            expected_revision: 1,
        }
    )
    .is_err());
    drop(source);
    assert_eq!(execute(&f.runtime, &receipt.preparation_id)?, imported);
    assert!(abort(&f.runtime, &receipt.preparation_id).is_err());
    Ok(())
}

#[test]
fn every_durable_boundary_recovers_after_reopen_without_source() -> Result<()> {
    for boundary in ["prepared", "intent", "written", "beforeCommit"] {
        let f = Fixture::new()?;
        let source = tempfile::tempdir()?;
        fs::write(source.path().join("a.txt"), "a")?;
        fs::write(source.path().join("b.txt"), "b")?;
        let receipt = prepare(&f, source.path())?;
        let failed = execute_with(&f.runtime, &receipt.preparation_id, &mut |point| {
            ensure!(point != boundary, "injected interruption");
            Ok(())
        })?;
        assert_eq!(failed.state, State::Applying);
        assert!(failed.error.is_some());
        assert_eq!(f.count("object")?, 0);
        drop(source);
        let Fixture { runtime, temp } = f;
        drop(runtime);
        let runtime = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
        assert_eq!(get(&runtime, &receipt.preparation_id)?, Some(failed));
        let imported = execute(&runtime, &receipt.preparation_id)?;
        assert_eq!(
            imported.state,
            State::ImportedPendingValidation,
            "{:?}",
            imported.error
        );
        assert_eq!(execute(&runtime, &receipt.preparation_id)?, imported);
        assert_eq!(
            runtime
                .store()
                .lock()
                .unwrap()
                .list::<serde_json::Value>("object")?
                .len(),
            2
        );
    }
    Ok(())
}

#[test]
fn source_drift_does_not_start_journal_or_create_objects() -> Result<()> {
    let f = Fixture::new()?;
    let source = tempfile::tempdir()?;
    fs::write(source.path().join("a.txt"), "original")?;
    let receipt = prepare(&f, source.path())?;
    fs::write(source.path().join("a.txt"), "changed")?;
    assert!(execute(&f.runtime, &receipt.preparation_id)
        .unwrap_err()
        .to_string()
        .contains("IMPORT_SOURCE_CHANGED"));
    assert!(get(&f.runtime, &receipt.preparation_id)?.is_none());
    assert_eq!(f.count("object")?, 0);
    Ok(())
}

#[test]
fn preexisting_target_is_never_overwritten_or_removed_by_abort() -> Result<()> {
    let f = Fixture::new()?;
    let source = tempfile::tempdir()?;
    fs::write(source.path().join("a.txt"), "a")?;
    let receipt = prepare(&f, source.path())?;
    let object = plan::objects(&receipt)?.remove(0);
    let path = f.runtime.project_root().join(&object.files[0].path);
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(&path, "a")?;
    let failed = execute(&f.runtime, &receipt.preparation_id)?;
    assert_eq!(failed.state, State::Applying);
    assert!(failed.writes.is_empty());
    assert_eq!(
        abort(&f.runtime, &receipt.preparation_id)?.state,
        State::Aborted
    );
    assert_eq!(fs::read_to_string(&path)?, "a");
    assert!(execute(&f.runtime, &receipt.preparation_id).is_err());
    Ok(())
}

#[test]
fn abort_retains_external_edits_and_can_finish_after_conflict_removed() -> Result<()> {
    let f = Fixture::new()?;
    let source = tempfile::tempdir()?;
    fs::write(source.path().join("a.txt"), "a")?;
    let receipt = prepare(&f, source.path())?;
    let failed = execute_with(&f.runtime, &receipt.preparation_id, &mut |point| {
        ensure!(point != "written", "interruption");
        Ok(())
    })?;
    let path = f.runtime.project_root().join(&failed.writes[0]);
    fs::write(&path, "external")?;
    let blocked = abort(&f.runtime, &receipt.preparation_id)?;
    assert_eq!(blocked.state, State::Aborting);
    assert_eq!(fs::read_to_string(&path)?, "external");
    assert!(execute(&f.runtime, &receipt.preparation_id).is_err());
    fs::remove_file(path)?;
    assert_eq!(
        abort(&f.runtime, &receipt.preparation_id)?.state,
        State::Aborted
    );
    assert_eq!(f.count("object")?, 0);
    Ok(())
}

#[test]
fn final_transaction_failure_rolls_back_all_objects_and_retries_once() -> Result<()> {
    let f = Fixture::new()?;
    let source = tempfile::tempdir()?;
    fs::write(source.path().join("a.txt"), "a")?;
    fs::write(source.path().join("b.txt"), "b")?;
    let receipt = prepare(&f, source.path())?;
    let second = &plan::objects(&receipt)?[1].id;
    f.runtime.store().lock().unwrap().connection.execute_batch(&format!(
        "CREATE TRIGGER fail_import BEFORE INSERT ON entities WHEN NEW.kind='object' AND NEW.id='{second}' BEGIN SELECT RAISE(ABORT,'injected transaction failure'); END;"))?;
    let failed = execute(&f.runtime, &receipt.preparation_id)?;
    assert_eq!(failed.state, State::Applying);
    assert_eq!(f.count("object")?, 0);
    assert_eq!(f.count("object_version")?, 0);
    f.runtime
        .store()
        .lock()
        .unwrap()
        .connection
        .execute_batch("DROP TRIGGER fail_import;")?;
    drop(source);
    assert_eq!(
        execute(&f.runtime, &receipt.preparation_id)?.state,
        State::ImportedPendingValidation
    );
    assert_eq!(f.count("object")?, 2);
    Ok(())
}
