use crate::store::Store;
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

pub fn valid(value: &Value) -> bool {
    value
        .as_u64()
        .is_some_and(|n| [0, 10, 30, 70, 100].contains(&n))
}
pub fn effective(store: &Store, task: &Value) -> Result<u64> {
    let settings: Value = store.get("settings", "main")?.unwrap_or(json!({}));
    let value = if task["askRatio"].is_null() {
        &settings["askRatio"]
    } else {
        &task["askRatio"]
    };
    if value.is_null() {
        return Ok(100);
    }
    anyhow::ensure!(valid(value), "询问档位无效");
    Ok(value.as_u64().unwrap())
}
pub fn recommendation(question: &Value) -> Result<&str> {
    let label = question["recommended"]
        .as_str()
        .context("请通过 beaver_ask_user 提供 recommended、importance 和 reason")?;
    anyhow::ensure!(
        question["options"]
            .as_array()
            .is_some_and(|opts| opts.iter().any(|o| o["label"] == label)),
        "推荐答案必须对应一个选项"
    );
    anyhow::ensure!(
        question["reason"]
            .as_str()
            .is_some_and(|r| !r.trim().is_empty()),
        "推荐答案需要理由"
    );
    Ok(label)
}
pub fn automatic(ratio: u64, question: &Value) -> Result<bool> {
    if ratio == 100 {
        return Ok(false);
    }
    recommendation(question)?;
    let importance = question["importance"]
        .as_u64()
        .filter(|n| (1..=100).contains(n))
        .context("请提供 1-100 的决策重要性")?;
    Ok(ratio == 0 || importance <= 100 - ratio)
}
pub fn set(store: &Store, id: &str, value: Value) -> Result<Value> {
    if !value.is_null() && !valid(&value) {
        bail!("询问档位无效");
    }
    let mut task: Value = store.get("task", id)?.context("任务不存在")?;
    task["askRatio"] = value;
    task["updatedAt"] = json!(chrono::Utc::now().to_rfc3339());
    store.put("task", id, &task)?;
    task["effectiveAskRatio"] = json!(effective(store, &task)?);
    Ok(task)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn thresholds_inheritance_and_overrides() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let store = Store::open(temp.path())?;
        let q = |n| json!({"importance":n,"recommended":"a","reason":"fits","options":[{"label":"a"},{"label":"b"}]});
        assert_eq!(effective(&store, &json!({}))?, 100);
        store.put("settings", "main", &json!({"askRatio":30}))?;
        assert_eq!(effective(&store, &json!({"askRatio":null}))?, 30);
        assert_eq!(effective(&store, &json!({"askRatio":0}))?, 0);
        for (ratio, low, high) in [(10, 90, 91), (30, 70, 71), (70, 30, 31)] {
            assert!(automatic(ratio, &q(low))?);
            assert!(!automatic(ratio, &q(high))?);
        }
        assert!(automatic(0, &q(100))?);
        assert!(!automatic(100, &q(1))?);
        assert!(automatic(0, &json!({"importance":10})).is_err());
        Ok(())
    }
}
