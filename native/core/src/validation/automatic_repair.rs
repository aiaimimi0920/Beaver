use super::{feedback, model::Run, repository};
use crate::store::Store;
use anyhow::Result;
use serde_json::{json, Value};
use std::{collections::BTreeSet, path::Path};

pub fn refresh(store: &mut Store, data: &Path) -> Result<bool> {
    let mut related = BTreeSet::new();
    for record in store.list::<Value>("validationFeedback")? {
        for key in ["runId", "rerunId"] {
            if let Some(id) = record[key].as_str() {
                related.insert(id.to_owned());
            }
        }
    }
    let mut changed = false;
    for run in store.list::<Run>("validationRun")? {
        if !run.managed
            || run.kind != "code"
            || run.status != "failed"
            || run.release_id.is_some()
            || run.engine_version.is_empty()
            || related.contains(&run.id)
            || store
                .get::<Value>("validationRepairDecision", &run.id)?
                .is_some()
        {
            continue;
        }
        let request = json!({"projectId":run.project_id,"runId":run.id,
            "snapshotId":run.snapshot_id,"requestId":format!("auto-repair-{}", run.id),
            "mode":"new","text":"代码验收失败。依据原始 GUT 日志和失败用例定位并修复，保留必需测试范围和有效断言。环境或需求不明确时停止并提问。"});
        let result = feedback::create(
            store,
            data,
            &request,
            &serde_json::from_str(include_str!("../../../../dist-native/design-catalog.json"))?,
            &serde_json::from_str(include_str!(
                "../../../../dist-native/blueprint-catalog.json"
            ))?,
            "system",
        );
        let decision = match result {
            Ok(result) => json!({"status":"taskQueued","result":result}),
            Err(error) => json!({"status":"needsInput","reason":error.to_string()}),
        };
        store.put(
            "validationRepairDecision",
            &run.id,
            &json!({"id":run.id,"projectId":run.project_id,
            "decision":decision,"createdAt":repository::now()}),
        )?;
        changed = true;
    }
    Ok(changed)
}
