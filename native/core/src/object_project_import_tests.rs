use super::*;
use crate::{
    object_catalog::{ObjectRecord, ObjectVersion},
    object_catalog_test_fixture::Fixture,
    object_import_preparation::{self as preparation, Baseline, Preparation, Request},
    project_storage::ProjectStore,
    store::Store,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    fs,
    sync::{Arc, Mutex},
};

struct SourceFixture {
    router: ProjectStorageRouter,
    host: Arc<Mutex<Store>>,
    temp: tempfile::TempDir,
}

impl SourceFixture {
    fn new() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("source");
        fs::create_dir(&root)?;
        fs::write(root.join("project.godot"), "config_version=5\n")?;
        let project = ProjectStore::initialize(&root, "source-1")?;
        project
            .store()
            .put("project", "source-1", &json!({"id":"source-1"}))?;
        for (id, versions) in [
            ("hero", vec![("hero-v1", "res://dep.txt")]),
            ("dep", vec![("dep-v1", "first"), ("dep-v2", "second")]),
        ] {
            let mut record: ObjectRecord = serde_json::from_value(json!({
                "id":id,"projectId":"source-1","name":id,"components":[],"files":[],
                "references":[],"versions":[]
            }))?;
            for (version, content) in versions {
                let hash = format!("{:x}", Sha256::digest(content.as_bytes()));
                fs::create_dir_all(root.join(".beaver/content/blobs"))?;
                fs::write(root.join(".beaver/content/blobs").join(&hash), content)?;
                let references = if id == "hero" {
                    json!([
                        {"projectId":"source-1","objectId":"dep","versionId":"dep-v1"},
                        {"projectId":"source-1","objectId":"dep","versionId":"dep-v2"}
                    ])
                } else {
                    json!([])
                };
                let value = ObjectVersion {
                    version_id: version.into(),
                    manifest: json!({
                        "schemaVersion":1,"projectId":"source-1","objectId":id,"versionId":version,
                        "status":"accepted","name":id,"category":"其他","tags":[],
                        "thumbnailPath":null,"parentObjectId":null,
                        "components":[{"id":format!("{id}-component"),"kind":"scene","name":id}],
                        "files":[{"path":format!("{id}.txt"),"role":"source","bytes":content.len(),"sha256":hash}],
                        "references":references
                    }),
                };
                project
                    .store()
                    .put("object_version", version, &(id, &value))?;
                record.versions.push(value);
            }
            project.store().put("object", id, &record)?;
        }
        drop(project);
        let host = Arc::new(Mutex::new(Store::open(&temp.path().join("host"))?));
        Ok(Self {
            router: ProjectStorageRouter::new(host.clone()),
            host,
            temp,
        })
    }

    fn prepare(&self, target: &Fixture) -> Result<Preparation> {
        let root = self.temp.path().join("source");
        let snapshot = self
            .router
            .inspect_object_source(&root, Some("source-1"), None)?;
        let digest = snapshot
            .import_versions
            .iter()
            .find(|v| v.version_id == "hero-v1")
            .unwrap()
            .source_digest
            .clone()
            .unwrap();
        preparation::prepare(
            &target.runtime.store(),
            &self.router,
            &Request {
                request_id: "one".into(),
                target_project_id: "project-1".into(),
                source_path: root,
                source_project_id: "source-1".into(),
                object_id: "hero".into(),
                baseline: Baseline::PinnedVersion("hero-v1".into()),
                source_digest: digest,
            },
        )
    }

    fn offline(&self) -> Result<()> {
        fs::rename(
            self.temp.path().join("source"),
            self.temp.path().join("offline"),
        )?;
        Ok(())
    }
}

#[test]
fn imports_registered_and_external_closures_once_without_accepting_or_changing_source() -> Result<()>
{
    for registered in [false, true] {
        let target = Fixture::new()?;
        let source = SourceFixture::new()?;
        if registered {
            source.host.lock().unwrap().put(
                "project",
                "source-1",
                &json!({
                    "id":"source-1","path":source.temp.path().join("source")
                }),
            )?;
            source.router.open_registered("source-1")?;
        }
        let receipt = source.prepare(&target)?;
        let source_db = source.temp.path().join("source/.beaver");
        let before = crate::object_import_snapshot::digest(&receipt.versions)?;
        let blobs = receipt
            .versions
            .iter()
            .flat_map(|version| &version.files)
            .map(|file| {
                let path = source_db.join("content/blobs").join(&file.sha256);
                Ok((
                    path.clone(),
                    fs::read(&path)?,
                    fs::metadata(&path)?.modified()?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        let imported = execute_project(&target.runtime, &source.router, &receipt.preparation_id)?;
        assert_eq!(
            imported.state,
            State::ImportedPendingValidation,
            "{:?}",
            imported.error
        );
        assert_eq!(target.count("object")?, 2);
        assert_eq!(target.count("object_version")?, 3);
        assert_eq!(target.count("object_import_origin")?, 2);
        assert_eq!(
            fs::read_to_string(target.temp.path().join("hero.txt"))?,
            "res://dep.txt"
        );
        assert_eq!(
            fs::read_to_string(target.temp.path().join("dep.txt"))?,
            "first"
        );
        let hero = target.object(&receipt.identity_map.objects["hero"])?;
        assert_eq!(
            hero.references[0].object_id,
            receipt.identity_map.objects["dep"]
        );
        assert_eq!(
            hero.references[1].version_id.as_ref(),
            Some(&receipt.identity_map.versions["dep-v2"])
        );
        assert_ne!(hero.components[0].id, "hero-component");
        assert!(crate::object_version_manifest::read(&hero, &hero.versions[0]).is_err());
        let dep = target.object(&receipt.identity_map.objects["dep"])?;
        let hash = dep.versions[1].manifest["files"][0]["sha256"]
            .as_str()
            .unwrap();
        assert_eq!(
            fs::read_to_string(target.runtime.files().blob(hash)?)?,
            "second"
        );
        assert!(source_db.exists());
        for (path, bytes, modified) in blobs {
            assert_eq!(fs::read(&path)?, bytes);
            assert_eq!(fs::metadata(&path)?.modified()?, modified);
        }
        let snapshot = source.router.inspect_object_source(
            &source.temp.path().join("source"),
            Some("source-1"),
            None,
        )?;
        assert_eq!(
            crate::object_import_snapshot::select(
                &snapshot.objects,
                "source-1",
                "hero",
                "hero-v1"
            )?
            .digest,
            before
        );
        assert_eq!(
            crate::object_import_snapshot::digest(&source.prepare(&target)?.versions)?,
            before
        );
        assert!(!source.temp.path().join("source/hero.txt").exists());
        assert_eq!(
            execute_project(&target.runtime, &source.router, &receipt.preparation_id)?,
            imported
        );
        assert!(abort(&target.runtime, &receipt.preparation_id).is_err());
    }
    Ok(())
}

#[test]
fn every_journal_boundary_can_resume_offline_after_runtime_reopen() -> Result<()> {
    for boundary in ["prepared", "intent", "written", "beforeCommit"] {
        let target = Fixture::new()?;
        let source = SourceFixture::new()?;
        let receipt = source.prepare(&target)?;
        let failed = execute_source(
            &target.runtime,
            &receipt.preparation_id,
            Some(&source.router),
            &mut |point| {
                ensure!(point != boundary, "injected crash");
                Ok(())
            },
        )?;
        assert_eq!(failed.state, State::Applying);
        assert!(failed.error.as_deref().unwrap().contains("injected crash"));
        assert_eq!(target.count("object")?, 0);
        source.offline()?;
        let Fixture { runtime, temp } = target;
        drop(runtime);
        let reopened = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
        assert_eq!(get(&reopened, &receipt.preparation_id)?, Some(failed));
        let completed = execute_project(&reopened, &source.router, &receipt.preparation_id)?;
        assert_eq!(
            completed.state,
            State::ImportedPendingValidation,
            "{:?}",
            completed.error
        );
        assert_eq!(
            execute_project(&reopened, &source.router, &receipt.preparation_id)?,
            completed
        );
    }
    Ok(())
}

#[test]
fn abort_preserves_external_edits_and_conflicts_never_overwrite_target() -> Result<()> {
    let target = Fixture::new()?;
    let source = SourceFixture::new()?;
    let receipt = source.prepare(&target)?;
    fs::write(target.temp.path().join("hero.txt"), "existing")?;
    let failed = execute_project(&target.runtime, &source.router, &receipt.preparation_id)?;
    assert_eq!(failed.state, State::Applying);
    assert!(failed.writes.is_empty());
    assert_eq!(
        fs::read_to_string(target.temp.path().join("hero.txt"))?,
        "existing"
    );
    fs::remove_file(target.temp.path().join("hero.txt"))?;
    let failed = execute_source(
        &target.runtime,
        &receipt.preparation_id,
        None,
        &mut |point| {
            ensure!(point != "beforeCommit", "stop");
            Ok(())
        },
    )?;
    assert_eq!(failed.writes.len(), 2);
    source.offline()?;
    fs::write(target.temp.path().join("hero.txt"), "external edits")?;
    let blocked = abort(&target.runtime, &receipt.preparation_id)?;
    assert_eq!(blocked.state, State::Aborting);
    assert!(!target.temp.path().join("dep.txt").exists());
    assert_eq!(
        fs::read_to_string(target.temp.path().join("hero.txt"))?,
        "external edits"
    );
    assert!(execute_project(&target.runtime, &source.router, &receipt.preparation_id).is_err());
    fs::remove_file(target.temp.path().join("hero.txt"))?;
    assert_eq!(
        abort(&target.runtime, &receipt.preparation_id)?.state,
        State::Aborted
    );
    assert_eq!(target.count("object")?, 0);
    Ok(())
}

#[test]
fn corrupt_source_or_receipt_and_transaction_failure_never_partially_register() -> Result<()> {
    let target = Fixture::new()?;
    let source = SourceFixture::new()?;
    let receipt = source.prepare(&target)?;
    let file = &receipt.versions[0].files[0];
    let blob = source
        .temp
        .path()
        .join("source/.beaver/content/blobs")
        .join(&file.sha256);
    let original = fs::read(&blob)?;
    fs::write(&blob, "corrupt")?;
    assert!(execute_project(&target.runtime, &source.router, &receipt.preparation_id).is_err());
    assert!(get(&target.runtime, &receipt.preparation_id)?.is_none());
    fs::write(&blob, original)?;
    target.runtime.store().lock().unwrap().connection.execute_batch(
        "CREATE TRIGGER reject_import BEFORE INSERT ON entities WHEN NEW.kind='object_import_origin' BEGIN SELECT RAISE(ABORT,'injected transaction failure'); END;"
    )?;
    let failed = execute_project(&target.runtime, &source.router, &receipt.preparation_id)?;
    assert_eq!(failed.state, State::Applying);
    assert_eq!(target.count("object")?, 0);
    assert_eq!(target.count("object_version")?, 0);
    target
        .runtime
        .store()
        .lock()
        .unwrap()
        .connection
        .execute_batch("DROP TRIGGER reject_import")?;
    let mut corrupt = receipt.clone();
    corrupt
        .identity_map
        .objects
        .insert("hero".into(), "hijacked".into());
    target.runtime.store().lock().unwrap().put(
        preparation::PREPARATION_KIND,
        &receipt.preparation_id,
        &corrupt,
    )?;
    assert!(execute_project(&target.runtime, &source.router, &receipt.preparation_id).is_err());
    target.runtime.store().lock().unwrap().put(
        preparation::PREPARATION_KIND,
        &receipt.preparation_id,
        &receipt,
    )?;
    source.offline()?;
    assert_eq!(
        execute_project(&target.runtime, &source.router, &receipt.preparation_id)?.state,
        State::ImportedPendingValidation
    );
    Ok(())
}
