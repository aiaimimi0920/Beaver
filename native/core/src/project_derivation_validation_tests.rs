use super::*;
use rusqlite::params;
use serde_json::{json, Value};

fn put(db: &Connection, kind: &str, id: &str, value: Value) -> Result<()> {
    db.execute(
        "INSERT OR REPLACE INTO entities VALUES(?,?,?)",
        params![kind, id, value.to_string()],
    )?;
    Ok(())
}

fn fixture() -> Result<Connection> {
    let db = Connection::open_in_memory()?;
    crate::store_schema::initialize(&db)?;
    put(&db, "project", "original", json!({"id":"original"}))?;
    put(
        &db,
        "task",
        "task",
        json!({"id":"task","projectId":"original"}),
    )?;
    Ok(db)
}

#[test]
fn rejects_unknown_foreign_host_and_unowned_receipts() -> Result<()> {
    for (kind, id, value, reason) in [
        (
            "unknown",
            "x",
            json!({"projectId":"original"}),
            "UNKNOWN_ENTITY_KIND",
        ),
        (
            "task",
            "foreign",
            json!({"id":"foreign","projectId":"other"}),
            "PROJECT_NOT_REGISTERED",
        ),
        ("settings", "main", json!({}), "PROJECT_OWNER_MISMATCH"),
        (
            "validationRequest",
            "old",
            json!({"hash":"hash","result":{}}),
            "VALIDATION_REQUEST_OWNER_UNKNOWN",
        ),
    ] {
        let db = fixture()?;
        put(&db, kind, id, value)?;
        assert!(validate(&db, "original")
            .err()
            .expect("invalid input must fail")
            .to_string()
            .contains(reason));
    }
    Ok(())
}

#[test]
fn checks_references_but_preserves_narrative() -> Result<()> {
    let db = fixture()?;
    let value = json!({"id":"task","projectId":"original","dependsOn":["missing"],"prompt":"missing other project"});
    put(&db, "task", "task", value.clone())?;
    assert!(validate(&db, "original")
        .err()
        .expect("invalid input must fail")
        .to_string()
        .contains("REFERENCE_NOT_FOUND"));
    put(
        &db,
        "task",
        "missing",
        json!({"id":"missing","projectId":"original"}),
    )?;
    validate(&db, "original")?;
    let raw: String = db.query_row(
        "SELECT value FROM entities WHERE kind='task' AND id='task'",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(serde_json::from_str::<Value>(&raw)?, value);
    Ok(())
}

#[test]
fn checks_event_ownership_and_call_column_consistency() -> Result<()> {
    let db = fixture()?;
    db.execute(
        "INSERT INTO events VALUES(1,'missing','now','log','task other')",
        [],
    )?;
    assert!(validate(&db, "original")
        .err()
        .expect("invalid input must fail")
        .to_string()
        .contains("TASK_NOT_FOUND"));
    db.execute("UPDATE events SET task='task'", [])?;
    let call = json!({"id":"call","taskId":"task","projectId":"original","output":"foreign IDs are narrative"});
    db.execute(
        "INSERT INTO calls(id,task,project,method,value) VALUES('call','task','original','test',?)",
        [call.to_string()],
    )?;
    validate(&db, "original")?;
    db.execute("UPDATE calls SET task=NULL", [])?;
    assert!(validate(&db, "original")
        .err()
        .expect("invalid input must fail")
        .to_string()
        .contains("CALL_COLUMN_MISMATCH"));
    Ok(())
}
