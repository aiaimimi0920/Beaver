use serde_json::{json, Value};
use std::path::Path;

fn same_path(previous: &Value, path: &str) -> bool {
    previous["path"].as_str().is_some_and(|old| {
        old == path
            || std::fs::canonicalize(old)
                .ok()
                .zip(std::fs::canonicalize(Path::new(path)).ok())
                .is_some_and(|(a, b)| a == b)
    })
}

/// A package manifest reports provenance; only Beaver's existing receipt attests it.
pub fn verification(previous: &Value, path: &str, result: &mut Result<Value, String>) -> Value {
    let known = same_path(previous, path) && !previous["validation"].is_null();
    if known
        && result
            .as_ref()
            .is_ok_and(|v| v["reportedValidation"] != previous["validation"])
    {
        *result = Err("导出清单中的发布诊断记录与 Beaver 保存的记录不一致".into());
    }
    let mut record = crate::game_export::delivery_record(result);
    record["path"] = json!(path);
    if known {
        record["validation"] = previous["validation"].clone();
    } else if let Ok(value) = result {
        record["reportedValidation"] = value["reportedValidation"].clone();
    }
    record
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rechecking_keeps_provenance_even_when_bundle_is_broken() {
        let provenance = json!({"purpose":"formal","releaseCheckId":"original"});
        let previous = json!({"path":"bundle","validation":provenance});
        let mut success = Ok(json!({"path":"bundle","reportedValidation":provenance}));
        let record = verification(&previous, "bundle", &mut success);
        assert_eq!(record["status"], "verified");
        assert_eq!(record["validation"], provenance);
        let mut failure = Err("Missing Game.pck".into());
        let record = verification(&previous, "bundle", &mut failure);
        assert_eq!(record["status"], "failed");
        assert_eq!(record["validation"], provenance);
        let mut changed =
            Ok(json!({"reportedValidation":{"purpose":"formal","releaseCheckId":"forged"}}));
        assert_eq!(
            verification(&previous, "bundle", &mut changed)["status"],
            "failed"
        );
        assert!(changed.is_err());
        let mut foreign = Ok(json!({"reportedValidation":provenance}));
        let record = verification(&previous, "different-bundle", &mut foreign);
        assert!(record["validation"].is_null());
        assert_eq!(record["reportedValidation"], provenance);
    }
}
