//! Target-owned preparation discovery. Reading history never opens a source.
use crate::{object_file_import_preparation, object_import_receipt, store::Store};
use anyhow::{ensure, Result};
use rusqlite::params;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub kind: String,
    pub preparation_id: String,
    pub request_id: String,
    pub cursor: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub target_project_id: String,
    pub entries: Vec<Entry>,
    pub next: Option<String>,
}

pub fn list(store: &Store, project_id: &str, after: Option<&str>) -> Result<Page> {
    ensure!(
        crate::object_version_manifest::valid_id(project_id),
        "IMPORT_INVALID_ID"
    );
    ensure!(
        store
            .get::<Value>("project", project_id)?
            .is_some_and(|p| p["id"] == project_id),
        "UNKNOWN_IMPORT_TARGET_PROJECT"
    );
    ensure!(
        after.is_none_or(|cursor| !cursor.is_empty() && cursor.len() <= 256),
        "IMPORT_INVALID_CURSOR"
    );
    let mut statement = store.connection.prepare(
        "SELECT kind,id,json_extract(value,'$.requestId'),kind || ':' || id AS cursor
         FROM entities WHERE kind IN (?1,?2)
         AND json_extract(value,'$.targetProjectId')=?3
         AND (?4 IS NULL OR kind || ':' || id > ?4)
         ORDER BY kind,id LIMIT 51",
    )?;
    let mut entries = statement
        .query_map(
            params![
                object_import_receipt::PREPARATION_KIND,
                object_file_import_preparation::PREPARATION_KIND,
                project_id,
                after
            ],
            |row| {
                let kind: String = row.get(0)?;
                Ok(Entry {
                    kind: if kind == object_import_receipt::PREPARATION_KIND {
                        "project"
                    } else {
                        "files"
                    }
                    .into(),
                    preparation_id: row.get(1)?,
                    request_id: row.get(2)?,
                    cursor: row.get(3)?,
                })
            },
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let next = if entries.len() > 50 {
        entries.truncate(50);
        entries.last().map(|entry| entry.cursor.clone())
    } else {
        None
    };
    Ok(Page {
        target_project_id: project_id.into(),
        entries,
        next,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn history_reopens_offline_and_pages_both_sources_without_cross_target_rows() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let store = Store::open(temp.path())?;
        store.put("project", "target", &json!({"id":"target"}))?;
        for index in 0..53 {
            let kind = if index % 2 == 0 {
                object_import_receipt::PREPARATION_KIND
            } else {
                object_file_import_preparation::PREPARATION_KIND
            };
            store.put(
                kind,
                &format!("receipt-{index:03}"),
                &json!({
                    "targetProjectId":"target", "requestId":format!("request-{index}"),
                    "sourcePath":"Z:/offline/source"
                }),
            )?;
        }
        store.put(
            object_import_receipt::PREPARATION_KIND,
            "foreign",
            &json!({
                "targetProjectId":"other", "requestId":"foreign"
            }),
        )?;
        drop(store);
        let store = Store::open(temp.path())?;
        let first = list(&store, "target", None)?;
        assert_eq!(first.entries.len(), 50);
        let second = list(&store, "target", first.next.as_deref())?;
        assert_eq!(second.entries.len(), 3);
        assert!(second.next.is_none());
        let cursors: std::collections::BTreeSet<_> = first
            .entries
            .iter()
            .chain(&second.entries)
            .map(|entry| &entry.cursor)
            .collect();
        assert_eq!(cursors.len(), 53);
        assert!(first.entries.iter().any(|entry| entry.kind == "files"));
        assert!(first.entries.iter().any(|entry| entry.kind == "project"));
        assert!(list(&store, "unknown", None).is_err());
        assert!(store.list::<Value>("object")?.is_empty());
        Ok(())
    }
}
