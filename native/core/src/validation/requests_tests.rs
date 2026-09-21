use super::*;
use crate::validation::{settings, test_support::Fixture};

fn input(project: &str) -> Value {
    json!({"projectId":project,"requestId":"same-request",
        "expectedRevision":0,"settings":{"visualRequired":false}})
}

#[test]
fn receipts_remain_project_scoped_and_conflicting_retries_do_not_write() -> Result<()> {
    let mut fixture = Fixture::new()?;
    fixture
        .store
        .put("project", "other", &json!({"id":"other"}))?;
    for project in ["p", "other"] {
        let input = input(project);
        let result = settings::save(&mut fixture.store, &input)?;
        assert_eq!(result["revision"], 1);
        assert_eq!(settings::save(&mut fixture.store, &input)?, result);
        let key = repository::digest(&json!([project, "same-request"]))?;
        let record: Value = fixture.store.get("validationRequest", &key)?.unwrap();
        assert_eq!(record["projectId"], project);
        let mut conflict = input.clone();
        conflict["settings"]["visualRequired"] = json!(true);
        assert!(settings::save(&mut fixture.store, &conflict).is_err());
        assert_eq!(settings::read(&fixture.store, project)?.revision, 1);
        assert_eq!(
            fixture.store.get::<Value>("validationRequest", &key)?,
            Some(record)
        );
    }
    Ok(())
}

#[test]
fn legacy_receipt_replays_without_silent_backfill_and_explicit_wrong_owner_rejects() -> Result<()> {
    let mut fixture = Fixture::new()?;
    let input = input("p");
    let result = settings::save(&mut fixture.store, &input)?;
    let key = repository::digest(&json!(["p", "same-request"]))?;
    let mut record: Value = fixture.store.get("validationRequest", &key)?.unwrap();
    record.as_object_mut().unwrap().remove("projectId");
    fixture.store.put("validationRequest", &key, &record)?;
    assert_eq!(settings::save(&mut fixture.store, &input)?, result);
    assert_eq!(
        fixture.store.get::<Value>("validationRequest", &key)?,
        Some(record.clone())
    );
    for owner in [json!("other"), Value::Null, json!(12)] {
        record["projectId"] = owner;
        fixture.store.put("validationRequest", &key, &record)?;
        let error = settings::save(&mut fixture.store, &input).unwrap_err();
        assert!(error.to_string().contains("another project"));
        assert_eq!(settings::read(&fixture.store, "p")?.revision, 1);
    }
    Ok(())
}

#[test]
fn mutations_require_an_explicit_project_identity() {
    for input in [
        json!({"requestId":"r"}),
        json!({"requestId":"r","projectId":null}),
        json!({"requestId":"r","projectId":""}),
        json!({"requestId":"r","projectId":7}),
    ] {
        assert!(Request::new("validation.settings.save", &input).is_err());
    }
}
