//! Keep source reads outside the target database lock for all import callers.
use crate::business_routing::project_runtime_handles;
use beaver_core::object_file_import_preparation as file_preparation;
use beaver_core::object_import_preparation::{self as preparation, Baseline, Request};
use beaver_core::{project_storage_router::ProjectStorageRouter, store::Store};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const MAX_FILE_PATHS: usize = 100;
const MAX_PATH_BYTES: usize = 2_000;

pub(super) fn dispatch(
    router: &ProjectStorageRouter,
    host: &Arc<Mutex<Store>>,
    root: &Path,
    method: &str,
    input: Option<&Value>,
) -> Option<Result<Value, String>> {
    if !matches!(
        method,
        "object.inspectExternal"
            | "object.inspectFiles"
            | "object.prepareImport"
            | "object.getImportPreparation"
            | "object.importPreparations"
            | "object.prepareFileImport"
            | "object.getFileImportPreparation"
            | "object.commitFileImport"
            | "object.fileImportOperation"
            | "object.abortFileImport"
            | "object.commitImport"
            | "object.importOperation"
            | "object.abortImport"
    ) {
        return None;
    }
    Some(
        input
            .ok_or_else(|| "缺少对象导入参数".into())
            .and_then(|input| call(router, host, root, method, input)),
    )
}

fn call(
    router: &ProjectStorageRouter,
    host: &Arc<Mutex<Store>>,
    root: &Path,
    method: &str,
    input: &Value,
) -> Result<Value, String> {
    if matches!(
        method,
        "object.commitFileImport"
            | "object.fileImportOperation"
            | "object.abortFileImport"
            | "object.commitImport"
            | "object.importOperation"
            | "object.abortImport"
    ) {
        let project = input["projectId"]
            .as_str()
            .filter(|id| !id.is_empty())
            .ok_or("项目 ID 无效")?;
        let id = input["preparationId"]
            .as_str()
            .filter(|id| !id.is_empty())
            .ok_or("准备 ID 无效")?;
        let runtime = router
            .runtime_for_project(project)
            .map_err(|error| error.to_string())?;
        use beaver_core::object_file_import as commit;
        let result = match method {
            "object.commitImport" => commit::execute_project(&runtime, router, id).map(Some),
            "object.commitFileImport" => commit::execute(&runtime, id).map(Some),
            "object.abortFileImport" | "object.abortImport" => {
                commit::abort(&runtime, id).map(Some)
            }
            _ => commit::get(&runtime, id),
        };
        return result
            .and_then(|value| serde_json::to_value(value).map_err(Into::into))
            .map_err(|error| error.to_string());
    }
    if method == "object.inspectFiles" {
        let paths = file_paths(input)?;
        return beaver_core::object_import_file_source::inspect(&paths)
            .and_then(|value| serde_json::to_value(value).map_err(Into::into))
            .map_err(|error| error.to_string());
    }
    if method == "object.inspectExternal" {
        let project_id = input
            .get("projectId")
            .map(|value| {
                value
                    .as_str()
                    .filter(|id| !id.is_empty())
                    .ok_or("项目 ID 无效")
            })
            .transpose()?;
        let path = input["path"]
            .as_str()
            .filter(|path| !path.is_empty() && path.len() <= 2000)
            .ok_or("项目路径无效")?;
        let path = Path::new(path);
        if !path.is_absolute() {
            return Err("项目路径必须是绝对路径".into());
        }
        return router
            .inspect_object_source(path, project_id, input.get("query").and_then(Value::as_str))
            .and_then(|value| serde_json::to_value(value).map_err(Into::into))
            .map_err(|error| error.to_string());
    }
    if method == "object.prepareFileImport" || method == "object.getFileImportPreparation" {
        let target_id = input["targetProjectId"]
            .as_str()
            .filter(|id| !id.is_empty())
            .ok_or("项目 ID 无效")?;
        let handles = project_runtime_handles(router, host.clone(), root, target_id)?;
        if method == "object.getFileImportPreparation" {
            let id = input["preparationId"]
                .as_str()
                .filter(|id| !id.is_empty())
                .ok_or("准备 ID 无效")?;
            let store = handles.store.lock().map_err(|_| "数据库锁不可用")?;
            let value = file_preparation::get(&store, id)
                .map_err(|error| error.to_string())?
                .filter(|item| item.target_project_id == target_id);
            return serde_json::to_value(value).map_err(|error| error.to_string());
        }
        let request: file_preparation::Request = serde_json::from_value(input.clone())
            .map_err(|error| format!("invalid file import request: {error}"))?;
        return file_preparation::prepare(&handles.store, &request)
            .and_then(|value| serde_json::to_value(value).map_err(Into::into))
            .map_err(|error| error.to_string());
    }
    let target_field = if method == "object.prepareImport" {
        "targetProjectId"
    } else {
        "projectId"
    };
    let target_id = input[target_field]
        .as_str()
        .filter(|id| !id.is_empty())
        .ok_or("项目 ID 无效")?;
    let handles = project_runtime_handles(router, host.clone(), root, target_id)?;
    if method == "object.importPreparations" {
        let store = handles.store.lock().map_err(|_| "数据库锁不可用")?;
        return preparation::history::list(
            &store,
            target_id,
            input.get("after").and_then(Value::as_str),
        )
        .and_then(|value| serde_json::to_value(value).map_err(Into::into))
        .map_err(|error| error.to_string());
    }
    if method == "object.getImportPreparation" {
        let id = input["preparationId"]
            .as_str()
            .filter(|id| !id.is_empty())
            .ok_or("准备 ID 无效")?;
        let store = handles.store.lock().map_err(|_| "数据库锁不可用")?;
        let value = preparation::get(&store, id)
            .map_err(|error| error.to_string())?
            .filter(|item| item.target_project_id == target_id);
        return serde_json::to_value(value).map_err(|error| error.to_string());
    }
    let request = request(input, target_id)?;
    preparation::prepare(&handles.store, router, &request)
        .and_then(|value| serde_json::to_value(value).map_err(Into::into))
        .map_err(|error| error.to_string())
}

fn file_paths(input: &Value) -> Result<Vec<PathBuf>, String> {
    let paths = input
        .get("paths")
        .and_then(Value::as_array)
        .ok_or("paths must be an array")?;
    if paths.is_empty() || paths.len() > MAX_FILE_PATHS {
        return Err(format!(
            "paths must contain 1..{MAX_FILE_PATHS} absolute paths"
        ));
    }
    paths
        .iter()
        .map(|value| {
            let text = value
                .as_str()
                .filter(|text| !text.is_empty() && text.len() <= MAX_PATH_BYTES)
                .ok_or("paths must contain non-empty paths up to 2000 bytes".to_owned())?;
            let path = PathBuf::from(text);
            if !path.is_absolute() {
                return Err("paths must contain absolute paths".into());
            }
            Ok(path)
        })
        .collect()
}

fn request(input: &Value, target_id: &str) -> Result<Request, String> {
    let source = input["source"]
        .as_object()
        .ok_or("source must be an object")?;
    if source.len() != 2 || !source.contains_key("path") || !source.contains_key("projectId") {
        return Err("source requires only path and projectId".into());
    }
    let text = |value: &Value, field: &str| -> Result<String, String> {
        value
            .as_str()
            .filter(|text| !text.is_empty())
            .map(str::to_owned)
            .ok_or_else(|| format!("invalid {field}"))
    };
    let source_path = PathBuf::from(text(&input["source"]["path"], "source.path")?);
    if !source_path.is_absolute() || source_path.as_os_str().len() > 2000 {
        return Err("source.path must be an absolute project path".into());
    }
    let baseline: Baseline = serde_json::from_value(input["baseline"].clone())
        .map_err(|error| format!("invalid baseline: {error}"))?;
    Ok(Request {
        request_id: text(&input["requestId"], "requestId")?,
        target_project_id: target_id.into(),
        source_path,
        source_project_id: text(&input["source"]["projectId"], "source.projectId")?,
        object_id: text(&input["objectId"], "objectId")?,
        baseline,
        source_digest: text(&input["sourceDigest"], "sourceDigest")?,
    })
}

#[cfg(test)]
#[path = "object_import_dispatch_tests.rs"]
mod dispatch_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn input() -> Value {
        json!({
            "requestId":"request-1", "targetProjectId":"target-1",
            "source":{"path":std::env::temp_dir(),"projectId":"source-1"},
            "objectId":"object-1", "baseline":{"kind":"pinnedVersion","versionId":"v1"},
            "sourceDigest":"a".repeat(64)
        })
    }

    #[test]
    fn transports_request_identity_and_pinned_selection() {
        let parsed = request(&input(), "bound-target").unwrap();
        assert_eq!(parsed.target_project_id, "bound-target");
        assert_eq!(parsed.request_id, "request-1");
        assert_eq!(parsed.source_digest, "a".repeat(64));
        assert_eq!(parsed.baseline, Baseline::PinnedVersion("v1".into()));
    }

    #[test]
    fn incomplete_source_and_unpinned_input_fail_without_panicking() {
        for source in [
            Value::Null,
            json!({}),
            json!({"path":"missing"}),
            json!({"projectId":"source"}),
            json!({"path":"relative","projectId":"source"}),
            json!({"path":std::env::temp_dir(),"projectId":"source","extra":true}),
        ] {
            let mut invalid = input();
            invalid["source"] = source;
            assert!(request(&invalid, "target").is_err());
        }
        for field in ["requestId", "sourceDigest", "baseline", "objectId"] {
            let mut invalid = input();
            invalid.as_object_mut().unwrap().remove(field);
            assert!(request(&invalid, "target").is_err(), "missing {field}");
        }
        let mut invalid = input();
        invalid["baseline"] = json!({"kind":"empty"});
        assert!(request(&invalid, "target").is_err());
    }

    #[test]
    fn file_paths_require_bounded_absolute_strings() {
        let path = std::env::temp_dir().to_string_lossy().into_owned();
        let valid = json!({"paths":[path.clone(), path]});
        assert_eq!(file_paths(&valid).unwrap().len(), 2);
        for invalid in [
            json!({}),
            json!({"paths": []}),
            json!({"paths": [1]}),
            json!({"paths": [""]}),
            json!({"paths": ["relative"]}),
            json!({"paths": ["x".repeat(2_001)]}),
        ] {
            assert!(file_paths(&invalid).is_err());
        }
        let too_many = json!({"paths": vec![path; MAX_FILE_PATHS + 1]});
        assert!(file_paths(&too_many).is_err());
    }

    #[test]
    fn file_preparation_request_requires_snapshot_and_groups_contract() {
        let valid = serde_json::json!({
            "requestId":"request-1",
            "targetProjectId":"target-1",
            "snapshot": {
                "source": {"kind":"files", "paths":[std::env::temp_dir()]},
                "files": [],
                "digest":"a".repeat(64)
            },
            "groups": []
        });
        assert!(serde_json::from_value::<file_preparation::Request>(valid.clone()).is_ok());

        for invalid in [
            serde_json::json!({"requestId":"request-1"}),
            serde_json::json!({"requestId":"request-1","targetProjectId":"target-1","snapshot":{},"groups":[]}),
            {
                let mut value = valid.clone();
                value["extra"] = serde_json::json!(true);
                value
            },
        ] {
            assert!(serde_json::from_value::<file_preparation::Request>(invalid).is_err());
        }
    }
}
