//! Read-only readiness probe; never opens a Store or initializes an external project.
use crate::store::Store;
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::{fs, path::Path};

use crate::project_storage_layout::{CONTROL_DIR, DATABASE, MANIFEST};

fn probe(path: &Path, project_id: &str) -> Result<(&'static str, &'static str)> {
    if !path.is_absolute() || !path.is_dir() {
        return Ok(("offline", "项目目录不可用，请重新关联项目位置。"));
    }
    let path = crate::project_storage_layout::root(path)?;
    let directory = path.join(CONTROL_DIR);
    match fs::symlink_metadata(&directory) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(("legacy", "项目尚未启用 .beaver 存储，需要完成副本迁移。"))
        }
        Err(e) => return Err(e.into()),
        Ok(_) => {}
    }
    crate::project_storage_layout::read_manifest(&path, Some(project_id))?;
    Ok((
        "detected",
        "已识别项目清单；项目数据库路由尚未启用，未读取对象记录。",
    ))
}

/// Readiness as seen from the host store: project storage is not routed here.
pub fn status(store: &Store, project_id: &str) -> Result<Value> {
    status_with_routing(store, project_id, false)
}

/// Inspect the host registration without opening or writing the project database.
pub fn registration_status(store: &Store, project_id: &str) -> Result<Value> {
    let project: Value = store.get("project", project_id)?.context("项目不存在")?;
    let path = project["path"].as_str().context("项目位置无效")?;
    let (state, message) = match probe(Path::new(path), project_id) {
        Ok(("detected", _)) => (
            "detected",
            "项目目录和存储清单可读取；尚未检查数据库及恢复状态。".to_owned(),
        ),
        Ok((state, message)) => (state, message.to_owned()),
        Err(error) => ("invalid", format!("无法读取项目存储：{error:#}")),
    };
    let normalized = normalize_path(path);
    let conflict_project_id = store
        .list_with_ids::<Value>("project")?
        .into_iter()
        .filter_map(|(id, value)| {
            (id != project_id)
                .then(|| {
                    value["path"]
                        .as_str()
                        .map(|other| (id, normalize_path(other)))
                })
                .flatten()
        })
        .find_map(|(id, other)| (other == normalized).then_some(id));
    let action = if conflict_project_id.is_some() {
        "resolve_registration_conflict"
    } else {
        match state {
            "offline" => "reassociate",
            "legacy" => "migrate",
            "invalid" => "repair_storage",
            _ => "none",
        }
    };
    Ok(json!({
        "projectId": project_id,
        "path": path,
        "registeredPath": path,
        "state": state,
        "message": message,
        "conflictProjectId": conflict_project_id,
        "action": action,
    }))
}

fn normalize_path(path: &str) -> String {
    let mut value = path.replace('/', "\\");
    while value.ends_with('\\') && value.len() > 3 {
        value.pop();
    }
    value.to_ascii_lowercase()
}

/// Readiness for a store chosen by the caller. `routed` is true only when `store` is the
/// project's own runtime store obtained through `ProjectStorageRouter`; the probe itself
/// cannot tell the two apart and never opens a runtime to find out.
pub fn status_with_routing(store: &Store, project_id: &str, routed: bool) -> Result<Value> {
    let project: Value = store.get("project", project_id)?.context("项目不存在")?;
    let path = project["path"].as_str().context("项目位置无效")?;
    let (state, message) = match probe(Path::new(path), project_id) {
        Ok((state, message)) => (state, message.to_owned()),
        Err(error) => ("invalid", format!("无法读取项目存储：{error:#}")),
    };
    let routed = routed && state == "detected";
    let message = if routed {
        "已通过项目路由打开项目数据库；对象记录查询尚未接通。".to_owned()
    } else {
        message
    };
    let mut blockers = Vec::new();
    if !routed {
        blockers
            .push(json!({"code":"PROJECT_STORAGE_NOT_ROUTED", "message":"项目本地存储尚未接通。"}));
    }
    blockers.push(
        json!({"code":"OBJECT_QUERIES_UNAVAILABLE", "message":"对象与制造记录查询尚未接通。"}),
    );
    blockers.push(json!({"code":"OBJECT_FRAMEWORK_DISABLED", "message":"对象队列和阶段门槛尚未接通，执行入口保持关闭。"}));
    Ok(json!({
        "schemaVersion": crate::object_framework::VERSION,
        "projectId": project_id,
        "storage": {"state": state, "message": message, "routed": routed,
            "manifestPath": format!("{CONTROL_DIR}/{MANIFEST}"),
            "databasePath": format!("{CONTROL_DIR}/{DATABASE}")},
        "capabilities": {"objectsRead": false, "manufactureRead": false, "execution": false},
        "blockers": blockers
    }))
}
