#![allow(dead_code)]
use crate::delivery_support::Fixture;
use anyhow::Result;
use beaver_core::task_callback;
use serde_json::{json, Value};

pub fn definition(title: &str) -> Value {
    json!({"title":title,"goal":"Write a design brief","acceptance":"Saved, inspectable design file"})
}

impl Fixture {
    pub fn work_request(&self, change: Value) -> Result<Value> {
        let state = self.state()?;
        Ok(
            json!({"operation":"work","requestId":uuid::Uuid::new_v4().to_string(),
            "expectedRevision":task_callback::inspect(&self.store.lock().unwrap(), "task", None)?["revision"],
            "assetRevision":state.revision,
            "stageId":state.stages[state.delivery.as_ref().unwrap().approved.len()].id,
            "change":change}),
        )
    }
    pub async fn work(&self, change: Value) -> Result<Value> {
        self.call(self.work_request(change)?).await
    }
    pub async fn create(&self, title: &str) -> Result<String> {
        let result = self
            .work(json!({"action":"create","definition":definition(title)}))
            .await?;
        Ok(result["subtaskId"].as_str().unwrap().into())
    }
    pub async fn begin(&self, id: &str, note: &str) -> Result<String> {
        let result = self.work(json!({"action":"begin","subtaskId":id,"inputs":{"goal":"Write docs/design.md"},"inputFiles":[],"recoveryNote":note})).await?;
        Ok(result["attemptId"].as_str().unwrap().into())
    }
    pub async fn finish(&self, id: &str, outcome: &str) -> Result<Value> {
        self.work(json!({"action":"finish","attemptId":id,"outcome":outcome,
            "summary":"Inspected saved output","outputs":{"path":"docs/design.md"},
            "tools":[{"name":"file-writer","source":"model report"}]}))
            .await
    }
    pub async fn submit(&self) -> Result<String> {
        let docs = self.workspace.join("docs");
        std::fs::create_dir_all(&docs)?;
        std::fs::write(docs.join("design.md"), "# Design\nSaved output\n")?;
        let result = self
            .call(self.request(&uuid::Uuid::new_v4().to_string(), &["docs/design.md"])?)
            .await?;
        Ok(result["candidateId"].as_str().unwrap().into())
    }
}
