//! Reject ambiguous identity inputs before preparing an independent project copy.
use crate::{
    project_migration_ownership::{Entity, Owner, Ownership},
    project_migration_references,
};
use anyhow::{bail, ensure, Context, Result};
use rusqlite::Connection;

fn owned(owner: Owner, project: &str, kind: &str, id: &str) -> Result<()> {
    match owner {
        Ok(Some(actual)) if actual == project => Ok(()),
        Ok(_) => bail!("derivation {kind}/{id}: PROJECT_OWNER_MISMATCH"),
        Err(reason) => bail!("derivation {kind}/{id}: {reason}"),
    }
}

pub(crate) fn validate(connection: &Connection, project: &str) -> Result<Vec<Entity>> {
    let mut statement =
        connection.prepare("SELECT kind,id,value FROM entities ORDER BY kind,id")?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    let mut entities = Vec::new();
    for row in rows {
        let (kind, id, raw) = row?;
        let value = serde_json::from_str(&raw)
            .with_context(|| format!("derivation {kind}/{id}: INVALID_JSON"))?;
        entities.push(Entity {
            kind,
            id,
            value: Some(value),
        });
    }
    ensure!(
        entities
            .iter()
            .any(|entity| entity.kind == "project" && entity.id == project),
        "derivation: missing project entity"
    );
    let ownership = Ownership::new([project.to_owned()].into(), &entities);
    for entity in &entities {
        owned(ownership.entity(entity), project, &entity.kind, &entity.id)?;
    }
    let references = project_migration_references::inspect(&entities, &ownership);
    if let Some(issue) = references.issues.first() {
        bail!(
            "derivation {}/{} {}: {}",
            issue.source_kind,
            issue.source_id,
            issue.field,
            issue.reason
        );
    }
    let mut events = connection.prepare("SELECT DISTINCT task FROM events ORDER BY task")?;
    for task in events.query_map([], |row| row.get::<_, String>(0))? {
        let task = task?;
        owned(ownership.task(&task), project, "events", &task)?;
    }
    let mut calls = connection.prepare("SELECT id,task,project,value FROM calls ORDER BY seq")?;
    let rows = calls.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;
    for row in rows {
        let (id, task, owner, raw) = row?;
        owned(
            ownership.call(&id, task.as_deref(), owner.as_deref(), &raw),
            project,
            "calls",
            &id,
        )?;
    }
    Ok(entities)
}

#[cfg(test)]
#[path = "project_derivation_validation_tests.rs"]
mod tests;
