use super::*;
use crate::{object_external_snapshot, project_storage::ProjectStore};
use serde_json::json;
use std::fs;

#[path = "object_import_concurrency_tests.rs"]
mod concurrency;
#[path = "object_import_discovery_tests.rs"]
mod discovery;
#[path = "object_import_test_fixture.rs"]
mod fixture;
#[path = "object_import_rejection_tests.rs"]
mod rejection;
#[path = "object_import_source_tests.rs"]
mod sources;
use fixture::*;

#[test]
fn baseline_requires_an_accepted_version() {
    assert_eq!(
        serde_json::to_value(Baseline::LatestAccepted).unwrap(),
        json!({"kind":"latestAccepted"})
    );
    assert_eq!(
        serde_json::to_value(Baseline::PinnedVersion("v1".into())).unwrap(),
        json!({"kind":"pinnedVersion","versionId":"v1"})
    );
    for invalid in [
        json!({"kind":"empty"}),
        json!({"kind":"latestAccepted","versionId":"v1"}),
    ] {
        assert!(serde_json::from_value::<Baseline>(invalid).is_err());
    }
}

#[test]
fn uses_frozen_blobs_even_when_working_files_change_or_disappear() -> Result<()> {
    let mut manifest = accepted("hero", "hero-v1");
    manifest["files"] = json!([file("hero.tscn", "accepted scene")]);
    let f = Fixture::new(vec![record("hero", vec![manifest.clone()])])?;
    f.blob("accepted scene")?;
    fs::write(f.source.join("hero.tscn"), "unaccepted working scene")?;
    let mut request = f.inspected("hero-v1")?;
    for missing in [false, true] {
        if missing {
            fs::remove_file(f.source.join("hero.tscn"))?;
            request.request_id = "request-2".into();
        }
        let before = inventory(&f.source)?;
        let receipt = f.prepare(&request)?;
        assert_eq!(serde_json::to_value(&receipt.versions[0])?, manifest);
        assert_eq!(receipt.accepted_version_id, "hero-v1");
        assert!(!receipt.ready_to_commit);
        assert_eq!(inventory(&f.source)?, before);
        f.assert_no_imported_entities()?;
    }
    Ok(())
}

#[test]
fn closes_frozen_references_and_maps_shared_objects_and_components_once() -> Result<()> {
    let mut hero = accepted("hero", "hero-v1");
    hero["references"] = json!([
        reference("dep", "dep-v1"),
        reference("dep", "dep-v2"),
        reference("branch", "branch-v1")
    ]);
    let mut branch = accepted("branch", "branch-v1");
    branch["references"] = json!([reference("dep", "dep-v1")]);
    let mut dep1 = accepted("dep", "dep-v1");
    dep1["references"] = json!([reference("hero", "hero-v1")]);
    dep1["files"] = json!([file("dep.txt", "first")]);
    let mut dep2 = accepted("dep", "dep-v2");
    dep2["files"] = json!([file("dep.txt", "second")]);
    let mut latest = accepted("hero", "hero-v2");
    latest["references"] = json!([reference("missing-current-object", "missing-v1")]);
    let mut working = record("hero", vec![hero, latest]);
    working.references = serde_json::from_value(json!([reference("live-only", "live-v1")]))?;
    let f = Fixture::new(vec![
        working,
        record("branch", vec![branch]),
        record("dep", vec![dep1, dep2]),
    ])?;
    f.blob("first")?;
    f.blob("second")?;
    let request = f.inspected("hero-v1")?;
    let receipt = f.prepare(&request)?;
    assert_eq!(
        receipt
            .versions
            .iter()
            .map(|v| v.version_id.as_str())
            .collect::<Vec<_>>(),
        ["branch-v1", "dep-v1", "dep-v2", "hero-v1"]
    );
    assert_eq!(receipt.identity_map.objects.len(), 3);
    assert_eq!(receipt.identity_map.components.len(), 3);
    assert_eq!(receipt.identity_map.versions.len(), 4);
    for map in [
        &receipt.identity_map.objects,
        &receipt.identity_map.components,
        &receipt.identity_map.versions,
    ] {
        assert!(map.iter().all(|(source, target)| source != target));
        assert_eq!(
            map.values()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            map.len()
        );
    }
    assert_eq!(get(&f.target(), &receipt.preparation_id)?, Some(receipt));
    f.assert_no_imported_entities()
}

#[test]
fn latest_skips_candidates_and_rejects_a_changed_selection() -> Result<()> {
    let first = accepted("hero", "hero-v1");
    let mut candidate = accepted("hero", "hero-candidate");
    candidate["status"] = json!("candidate");
    let mut source = record("hero", vec![first, candidate]);
    let f = Fixture::new(vec![source.clone()])?;
    let mut request = f.inspected("hero-v1")?;
    request.baseline = Baseline::LatestAccepted;
    assert_eq!(f.prepare(&request)?.accepted_version_id, "hero-v1");
    let mut opened = ProjectStore::open(&f.source, "source-1")?;
    let published = accepted("hero", "hero-v2");
    let version = crate::object_catalog::ObjectVersion {
        version_id: "hero-v2".into(),
        manifest: published,
    };
    opened
        .store_mut()
        .put("object_version", "hero-v2", &(source.id.clone(), &version))?;
    source.versions.push(version);
    opened.store_mut().put("object", "hero", &source)?;
    drop(opened);
    request.request_id = "new-operation".into();
    let error = f.prepare(&request).unwrap_err();
    assert!(error.to_string().contains("IMPORT_SOURCE_CHANGED"));
    assert_eq!(f.target().list::<Value>(PREPARATION_KIND)?.len(), 1);
    request.baseline = Baseline::PinnedVersion("hero-v1".into());
    assert_eq!(f.prepare(&request)?.accepted_version_id, "hero-v1");
    Ok(())
}

#[test]
fn retries_replay_offline_and_new_requests_get_independent_identities() -> Result<()> {
    let f = Fixture::new(vec![record("hero", vec![accepted("hero", "hero-v1")])])?;
    let request = f.inspected("hero-v1")?;
    let receipt = f.prepare(&request)?;
    let offline = f.temp.path().join("offline-source");
    fs::rename(&f.source, &offline)?;
    let page = history::list(&f.target(), &request.target_project_id, None)?;
    assert_eq!(page.entries.len(), 1);
    assert_eq!(page.entries[0].preparation_id, receipt.preparation_id);
    assert_eq!(page.entries[0].kind, "project");
    assert_eq!(f.prepare(&request)?, receipt);
    let mut conflict = request.clone();
    conflict.object_id = "different-object".into();
    assert!(f
        .prepare(&conflict)
        .unwrap_err()
        .to_string()
        .contains("IMPORT_REQUEST_CONFLICT"));
    assert_eq!(
        get(&f.target(), &receipt.preparation_id)?,
        Some(receipt.clone())
    );
    fs::rename(offline, &f.source)?;
    let mut fresh = request;
    fresh.request_id = "deliberately-new-import".into();
    let independent = f.prepare(&fresh)?;
    assert_ne!(receipt.preparation_id, independent.preparation_id);
    assert_ne!(
        receipt.identity_map.objects,
        independent.identity_map.objects
    );
    assert_ne!(
        receipt.identity_map.components,
        independent.identity_map.components
    );
    assert_ne!(
        receipt.identity_map.versions,
        independent.identity_map.versions
    );
    assert_eq!(f.target().list::<Value>(PREPARATION_KIND)?.len(), 2);
    f.assert_no_imported_entities()
}

#[test]
fn legacy_live_file_receipts_are_not_reinterpreted_as_frozen_versions() -> Result<()> {
    let f = Fixture::new(vec![])?;
    let legacy = json!({"preparationId":"old","files":[],"objects":[],"readyToCommit":false});
    f.target().put(PREPARATION_KIND, "old", &legacy)?;
    assert!(get(&f.target(), "old")
        .unwrap_err()
        .to_string()
        .contains("IMPORT_PREPARATION_UNSUPPORTED"));
    assert_eq!(
        f.target().get::<Value>(PREPARATION_KIND, "old")?,
        Some(legacy)
    );
    Ok(())
}
