use super::run_fixture::{accept, capture, claim, fixture};
use crate::{
    object_catalog::ObjectReference,
    object_framework::Baseline,
    object_run_preparation::{self, PreparationState},
};
use anyhow::Result;
use std::fs;

#[test]
fn reference_closure_uses_frozen_accepted_content() -> Result<()> {
    let fixture = fixture()?;
    let reference = capture(&fixture, "rival", "reference", "reference", vec![])?;
    accept(&fixture, "rival", &reference)?;
    let hero = capture(
        &fixture,
        "hero",
        "hero",
        "hero",
        vec![ObjectReference {
            project_id: "project-1".into(),
            object_id: "rival".into(),
            version_id: Some(reference.clone()),
        }],
    )?;
    accept(&fixture, "hero", &hero)?;
    let claimed = claim(&fixture, None)?;
    assert_eq!(
        claimed.record().baseline.as_ref().unwrap().versions.len(),
        2
    );
    let replacement = capture(&fixture, "rival", "replacement", "replacement", vec![])?;
    accept(&fixture, "rival", &replacement)?;
    let ready = object_run_preparation::prepare(&fixture.runtime, claimed)?;
    assert_eq!(ready.state, PreparationState::Ready);
    let workspace = fixture.runtime.files().workspace(&ready.id)?;
    assert_eq!(fs::read_to_string(workspace.join("hero.tscn"))?, "hero");
    assert_eq!(
        fs::read_to_string(workspace.join("rival.tscn"))?,
        "reference"
    );
    assert_eq!(
        fs::read_to_string(fixture.temp.path().join("rival.tscn"))?,
        "replacement"
    );
    Ok(())
}

#[test]
fn missing_or_corrupt_content_keeps_failure_and_writer_without_partial_restore() -> Result<()> {
    for damage in ["missing", "size", "hash"] {
        let fixture = fixture()?;
        let version = capture(&fixture, "hero", "hero", "original", vec![])?;
        accept(&fixture, "hero", &version)?;
        let claimed = claim(&fixture, None)?;
        let blob = &claimed.record().baseline.as_ref().unwrap().versions[0].files[0].sha256;
        let path = fixture.temp.path().join(".beaver/content/blobs").join(blob);
        let expected = match damage {
            "missing" => {
                fs::remove_file(path)?;
                "IMPORT_CONTENT_MISSING"
            }
            "size" => {
                fs::write(path, "short")?;
                "IMPORT_CONTENT_SIZE_MISMATCH"
            }
            _ => {
                fs::write(path, "tampered")?;
                "IMPORT_CONTENT_HASH_MISMATCH"
            }
        };
        let failed = object_run_preparation::prepare(&fixture.runtime, claimed)?;
        assert_eq!(failed.state, PreparationState::Failed, "{damage}");
        assert!(
            failed.error.as_ref().unwrap().contains(expected),
            "{damage}"
        );
        assert!(!fixture.runtime.files().workspace(&failed.id)?.exists());
        assert_eq!(
            fs::read_to_string(fixture.temp.path().join("hero.tscn"))?,
            "original"
        );
        assert_eq!(
            object_run_preparation::get(&fixture.runtime, "project-1", &failed.id)?,
            Some(failed)
        );
        super::queue_fixture::enqueue(&fixture, &["head"])?;
        assert!(
            object_run_preparation::claim_next(&fixture.runtime, "project-1", "retry")?.is_none()
        );
    }
    Ok(())
}

#[test]
fn an_existing_workspace_is_preserved_and_never_overwritten() -> Result<()> {
    let fixture = fixture()?;
    let claimed = claim(&fixture, Some(Baseline::Empty {}))?;
    let workspace = fixture.runtime.files().workspace(&claimed.record().id)?;
    fs::create_dir_all(&workspace)?;
    fs::write(workspace.join("retained.txt"), "retained work")?;
    let failed = object_run_preparation::prepare(&fixture.runtime, claimed)?;
    assert_eq!(failed.state, PreparationState::Failed);
    assert!(failed
        .error
        .unwrap()
        .contains("OBJECT_RUN_WORKSPACE_ALREADY_EXISTS"));
    assert_eq!(
        fs::read_to_string(workspace.join("retained.txt"))?,
        "retained work"
    );
    Ok(())
}
