use super::repository;
use crate::store::Store;
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

pub type Record = (&'static str, String, Value);

pub fn commit(store: &mut Store, records: Vec<Record>) -> Result<()> {
    store.transaction(|db| {
        for (kind, id, value) in records {
            db.execute(
                "INSERT INTO entities(kind,id,value) VALUES(?,?,?) ON CONFLICT(kind,id) DO UPDATE SET value=excluded.value",
                rusqlite::params![kind, id, value.to_string()],
            )?;
        }
        Ok(())
    })
}

pub struct Request {
    key: String,
    hash: String,
    project_id: String,
}

impl Request {
    pub fn new(method: &str, input: &Value) -> Result<Self> {
        let id = input["requestId"]
            .as_str()
            .context("A requestId is required for mutations")?;
        if id.is_empty() || id.len() > 100 {
            bail!("Invalid requestId");
        }
        let project_id = input["projectId"]
            .as_str()
            .filter(|id| !id.is_empty())
            .context("A projectId is required for mutations")?;
        Ok(Self {
            key: repository::digest(&json!([input["projectId"], id]))?,
            hash: repository::digest(&json!([method, input]))?,
            project_id: project_id.into(),
        })
    }

    pub fn replay(&self, store: &Store) -> Result<Option<Value>> {
        let record = store.get::<Value>("validationRequest", &self.key)?;
        if let Some(record) = record {
            if record
                .get("projectId")
                .is_some_and(|id| id != &self.project_id)
            {
                bail!("Validation request belongs to another project");
            }
            if record["hash"] != self.hash {
                bail!("requestId was already used with different parameters");
            }
            return Ok(Some(record["result"].clone()));
        }
        Ok(None)
    }

    pub fn record(&self, result: &Value) -> Record {
        (
            "validationRequest",
            self.key.clone(),
            json!({"projectId":self.project_id,"hash":self.hash,"result":result}),
        )
    }

    pub fn finish(
        &self,
        store: &mut Store,
        result: Value,
        mut records: Vec<Record>,
    ) -> Result<Value> {
        records.push(self.record(&result));
        commit(store, records)?;
        Ok(result)
    }
}

#[cfg(test)]
#[path = "requests_tests.rs"]
mod tests;
