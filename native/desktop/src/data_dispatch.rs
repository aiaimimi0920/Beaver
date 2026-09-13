use crate::{anyhow_result, Backend};
use beaver_core::{documents, files::Files};
use serde_json::{json, Value};
use std::sync::atomic::Ordering;

pub(crate) fn call(
    backend: &Backend,
    method: String,
    input: Option<Value>,
) -> Result<Value, String> {
    let mut store = backend.store.lock().map_err(|_| "数据库锁不可用")?;
    if backend.closing.load(Ordering::SeqCst) {
        return Err("应用正在退出".into());
    }
    let result = (|| -> anyhow_result::Result<Value> {
        match method.as_str() {
            "logs.query" => Ok(beaver_core::call_log::query(
                &store,
                &input.unwrap_or_else(|| json!({})),
            )?),
            "task.approval" => {
                let input = input.as_ref().ok_or("缺少审批参数")?;
                Ok(beaver_core::task_plan::approval(
                    &store,
                    input["id"].as_str().ok_or("缺少任务标识")?,
                    input["autoAccept"].as_bool().ok_or("审批策略无效")?,
                )?)
            }
            "task.autonomy" => {
                let input = input.as_ref().ok_or("缺少任务设置")?;
                Ok(beaver_core::autonomy::set(
                    &store,
                    input["id"].as_str().ok_or("缺少任务标识")?,
                    input.get("askRatio").cloned().ok_or("缺少询问档位")?,
                )?)
            }
            "feature.add" => Ok(beaver_core::feature_tasks::create(
                &mut store,
                &backend.root,
                input.ok_or("缺少功能块参数")?,
                &serde_json::from_str(include_str!("../../../dist-native/feature-sources.json"))?,
                &serde_json::from_str(include_str!("../../../dist-native/design-catalog.json"))?,
                &serde_json::from_str(include_str!("../../../dist-native/blueprint-catalog.json"))?,
            )?),
            "task.direction" => Ok(beaver_core::task_relations::set_direction(
                &store,
                &input.ok_or("缺少任务参数")?,
            )?),
            "task.followup" | "task.delegate" | "task.dialogueRollback" => {
                Ok(beaver_core::task_relations::create(
                    &mut store,
                    &backend.root,
                    &method,
                    input.ok_or("缺少任务参数")?,
                    &serde_json::from_str(include_str!(
                        "../../../dist-native/design-catalog.json"
                    ))?,
                    &serde_json::from_str(include_str!(
                        "../../../dist-native/blueprint-catalog.json"
                    ))?,
                )?)
            }
            "project.create" => Ok(beaver_core::projects::create_project(
                &store,
                &backend.root,
                input.ok_or("缺少项目参数")?,
                &serde_json::from_str(include_str!("../../../dist-native/design-catalog.json"))?,
                &serde_json::from_str(include_str!("../../../dist-native/blueprint-catalog.json"))?,
                &serde_json::from_str(include_str!("../../../dist-native/templates.json"))?,
            )?),
            "task.create" => Ok(beaver_core::task_create::create(
                &mut store,
                &backend.root,
                input.ok_or("缺少任务参数")?,
                &serde_json::from_str(include_str!("../../../dist-native/design-catalog.json"))?,
                &serde_json::from_str(include_str!("../../../dist-native/blueprint-catalog.json"))?,
            )?),
            "state" => {
                let mut tasks = store.list::<Value>("task")?;
                let projects = store.list::<Value>("project")?;
                for task in &mut tasks {
                    task["effectiveAskRatio"] =
                        json!(beaver_core::autonomy::effective(&store, task)?);
                    task["delivery"] = projects
                        .iter()
                        .find(|p| p["id"] == task["projectId"])
                        .map(|p| p["delivery"].clone())
                        .unwrap_or(Value::Null);
                }
                let settings = beaver_core::preferences::read(
                    &store,
                    serde_json::from_str(include_str!(
                        "../../../dist-native/default-settings.json"
                    ))?,
                )?;
                Ok(
                    json!({"projects":projects, "tasks":tasks, "settings":settings,
                    "features":serde_json::from_str::<Value>(include_str!("../../../dist-native/features.json"))?}),
                )
            }
            "project.blueprint.save" | "project.overview.save" => {
                let input = input.as_ref().ok_or("缺少项目规划参数")?;
                let id = input["id"].as_str().ok_or("缺少项目标识")?;
                let revision = input["expectedRevision"].as_u64().ok_or("规划版本无效")?;
                let overview = method == "project.overview.save";
                if overview && input["allowRiskyChanges"] != true {
                    return Err("修改基础设定需要明确确认风险".into());
                }
                Ok(beaver_core::blueprint::save(
                    &store,
                    id,
                    input[if overview { "overview" } else { "blueprint" }].clone(),
                    revision,
                    overview,
                    &serde_json::from_str(include_str!(
                        "../../../dist-native/blueprint-catalog.json"
                    ))?,
                )?)
            }
            "settings.save" => {
                let _setup = backend
                    .setup_gate
                    .try_lock()
                    .map_err(|_| "工具安装期间不能修改设置")?;
                let input = input.as_ref().ok_or("缺少设置参数")?;
                Ok(beaver_core::preferences::save(
                    &mut store,
                    &beaver_core::preferences::SystemVault,
                    input["settings"].clone(),
                    input["keys"].clone(),
                )?)
            }
            "settings.importLocalCodex" => {
                let _setup = backend
                    .setup_gate
                    .try_lock()
                    .map_err(|_| "工具安装期间不能导入设置")?;
                Ok(beaver_core::local_codex::import_current(
                    &mut store,
                    input.as_ref().ok_or("缺少设置参数")?,
                )?)
            }
            "tools.setupStatus" => Ok(store
                .get::<Value>("toolSetup", "main")?
                .unwrap_or_else(beaver_core::tool_setup::idle)),
            "settings.clearKey" => {
                let slot = input
                    .as_ref()
                    .and_then(|v| v["slot"].as_str())
                    .ok_or("缺少凭据类型")?;
                beaver_core::preferences::clear_key(&store, slot)?;
                Ok(Value::Null)
            }
            "task.retryMerge" => {
                let id = input
                    .as_ref()
                    .and_then(|v| v["id"].as_str())
                    .ok_or("缺少任务标识")?;
                Ok(beaver_core::task_finish::retry_merge(
                    &mut store,
                    &Files::new(backend.root.clone()),
                    id,
                    &backend.closing,
                )?)
            }
            "task.rollback" | "task.accept" => {
                let input = input.as_ref().ok_or("缺少任务参数")?;
                let id = input["id"].as_str().ok_or("缺少任务标识")?;
                if method == "task.accept" {
                    beaver_core::task_actions::accept(&mut store, id)?;
                } else {
                    let keep: Vec<String> = serde_json::from_value(input["keep"].clone())
                        .map_err(|_| "保留文件列表无效")?;
                    beaver_core::task_actions::rollback(
                        &mut store,
                        &Files::new(backend.root.clone()),
                        id,
                        keep,
                    )?;
                }
                Ok(Value::Null)
            }
            "task.resources" | "task.resourceText" | "task.resourceBytes" => {
                let input = input.as_ref().ok_or("缺少任务参数")?;
                let id = input["id"].as_str().ok_or("缺少任务标识")?;
                let task: Value = store.get("task", id)?.ok_or("任务不存在")?;
                if method == "task.resources" {
                    Ok(beaver_core::task_resources::resources(&task)?)
                } else {
                    let path = input["path"]
                        .as_str()
                        .filter(|s| !s.is_empty() && s.encode_utf16().count() <= 2000)
                        .ok_or("资源路径无效")?;
                    if method == "task.resourceBytes" {
                        Ok(beaver_core::task_resources::raw(&task, path)?)
                    } else {
                        Ok(json!(beaver_core::task_resources::text(&task, path)?))
                    }
                }
            }
            "project.reveal" | "task.reveal" | "asset.reveal" => {
                let target = beaver_core::reveal::resolve(
                    &store,
                    &method,
                    input.as_ref().ok_or("缺少显示参数")?,
                )?;
                beaver_core::reveal::open(&target)?;
                Ok(if method == "asset.reveal" {
                    Value::Null
                } else {
                    json!("")
                })
            }
            "task.events" => {
                let id = input
                    .as_ref()
                    .and_then(|v| v.get("id"))
                    .and_then(Value::as_str)
                    .ok_or("缺少任务标识")?;
                Ok(serde_json::to_value(store.events(id)?)?)
            }
            "assets" | "asset.text" | "document.read" | "document.save" => {
                let input = input.as_ref().ok_or("缺少操作参数")?;
                let id = input["id"].as_str().ok_or("缺少项目标识")?;
                let root = documents::project_path(&store, id)?;
                if method == "assets" {
                    return Ok(json!(documents::assets(&root)?));
                }
                let relative = input["path"]
                    .as_str()
                    .filter(|s| s.len() <= 2000)
                    .ok_or("资料路径无效")?;
                match method.as_str() {
                    "asset.text" => Ok(json!(documents::text(&root, relative)?)),
                    "document.read" => Ok(documents::read_document(&root, relative)?),
                    _ => {
                        let text = input["text"].as_str().ok_or("缺少资料内容")?;
                        let revision = match input.get("revision") {
                            Some(Value::Null) => None,
                            Some(Value::String(value))
                                if value.len() == 64
                                    && value.bytes().all(|b| {
                                        b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
                                    }) =>
                            {
                                Some(value.as_str())
                            }
                            _ => return Err("资料版本无效".into()),
                        };
                        documents::save_document(
                            &mut store,
                            &backend.root,
                            id,
                            relative,
                            text,
                            revision,
                        )?;
                        Ok(Value::Null)
                    }
                }
            }
            "project.import" => {
                let directory = input
                    .as_ref()
                    .and_then(|v| v["path"].as_str())
                    .filter(|s| !s.is_empty() && s.len() <= 2000)
                    .ok_or("项目路径无效")?;
                Ok(beaver_core::projects::import_project(
                    &store,
                    &backend.root,
                    std::path::Path::new(directory),
                    &serde_json::from_str(include_str!(
                        "../../../dist-native/design-catalog.json"
                    ))?,
                )?)
            }
            _ => Err(format!("原生后端尚未迁移此操作：{method}").into()),
        }
    })();
    result.map_err(|error| error.to_string())
}
