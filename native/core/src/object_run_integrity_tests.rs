use super::run_fixture::{accept, capture, claim, fixture};
use crate::object_run_preparation::{self, PreparationState};
use anyhow::Result;
use rusqlite::params;

#[test]
fn multiple_legacy_acceptances_require_proven_order() -> Result<()> {
    let fixture = fixture()?;
    for label in ["first", "second"] {
        let version = capture(&fixture, "hero", label, label, vec![])?;
        accept(&fixture, "hero", &version)?;
    }
    fixture.runtime.store().lock().unwrap().connection.execute(
        "DELETE FROM entities WHERE kind='object_command_receipt'",
        [],
    )?;
    let claimed = claim(&fixture, None)?;
    assert_eq!(claimed.record().state, PreparationState::Failed);
    assert!(claimed
        .record()
        .error
        .as_ref()
        .unwrap()
        .contains("OBJECT_ACCEPTANCE_ORDER_AMBIGUOUS"));
    assert!(!fixture
        .runtime
        .files()
        .workspace(&claimed.record().id)?
        .exists());
    Ok(())
}

#[test]
fn damaged_acceptance_evidence_never_selects_a_baseline() -> Result<()> {
    for damage in ["key", "digest", "project", "version"] {
        let fixture = fixture()?;
        let version = capture(&fixture, "hero", "first", "first", vec![])?;
        accept(&fixture, "hero", &version)?;
        let handle = fixture.runtime.store();
        {
            let store = handle.lock().unwrap();
            let request_id = format!("accept-{version}");
            let (key, mut receipt): (String, serde_json::Value) = {
                let (key, json): (String, String) = store.connection.query_row(
                    "SELECT id,value FROM entities WHERE kind='object_command_receipt'
                     AND json_extract(value,'$.result.requestId')=?",
                    [&request_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )?;
                (key, serde_json::from_str(&json)?)
            };
            match damage {
                "key" => {
                    store.connection.execute(
                        "UPDATE entities SET id='wrong-key' WHERE kind='object_command_receipt' AND id=?",
                        [&key],
                    )?;
                }
                _ => {
                    match damage {
                        "digest" => receipt["inputDigest"] = "wrong-digest".into(),
                        "project" => receipt["result"]["projectId"] = "other-project".into(),
                        _ => {
                            receipt["result"]["object"]["versions"][0]["manifest"]["name"] =
                                "changed".into()
                        }
                    }
                    store.put("object_command_receipt", &key, &receipt)?;
                }
            }
        }
        let claimed = claim(&fixture, None)?;
        assert_eq!(claimed.record().state, PreparationState::Failed, "{damage}");
        assert!(claimed.record().baseline.is_none(), "{damage}");
        assert!(
            claimed.record().error.as_ref().unwrap().contains(
                if matches!(damage, "key" | "project") {
                    "OBJECT_RECEIPT_IDENTITY_MISMATCH"
                } else {
                    "OBJECT_ACCEPTANCE_HISTORY_INVALID"
                }
            ),
            "{damage}"
        );
        assert!(!fixture
            .runtime
            .files()
            .workspace(&claimed.record().id)?
            .exists());
    }
    Ok(())
}

#[test]
fn torn_version_entity_fails_before_workspace_creation() -> Result<()> {
    let fixture = fixture()?;
    let version = capture(&fixture, "hero", "first", "first", vec![])?;
    accept(&fixture, "hero", &version)?;
    fixture.runtime.store().lock().unwrap().connection.execute(
        "UPDATE entities SET value=json_set(value,'$[1].manifest.name',?)
         WHERE kind='object_version' AND id=?",
        params!["changed", version],
    )?;
    let claimed = claim(&fixture, None)?;
    let prepared = object_run_preparation::prepare(&fixture.runtime, claimed)?;
    assert_eq!(prepared.state, PreparationState::Failed);
    assert!(prepared.error.unwrap().contains("OBJECT_VERSION_MISMATCH"));
    assert!(!fixture.runtime.files().workspace(&prepared.id)?.exists());
    Ok(())
}
