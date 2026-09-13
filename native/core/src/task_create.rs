use crate::{
    blueprint,
    files::{safe_path, Files},
    journal::Journal,
    store::Store,
};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{fs, io::Read, path::Path};

#[derive(Deserialize, Serialize)]
struct Region {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}
#[derive(Deserialize, Serialize)]
struct Reference {
    path: String,
    note: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    region: Option<Region>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Input {
    ask_ratio: Option<u8>,
    project_id: String,
    prompt: String,
    title: Option<String>,
    #[serde(default)]
    stop_conditions: String,
    #[serde(default)]
    references: Vec<Reference>,
    #[serde(default)]
    max_minutes: u16,
    capability: Option<String>,
    direction: Option<String>,
    decompose: Option<bool>,
    auto_accept: Option<bool>,
}
fn length(text: &str, min: usize, max: usize) -> bool {
    (min..=max).contains(&text.encode_utf16().count())
}

/// Caller owns the project snapshot lock. This persists a queue item, not execution.
pub fn create(
    store: &mut Store,
    root: &Path,
    input: Value,
    design_catalog: &Value,
    blueprint_catalog: &Value,
) -> Result<Value> {
    create_with_context(
        store,
        root,
        input,
        design_catalog,
        blueprint_catalog,
        |_, _, _| Ok(()),
    )
}

pub(crate) fn create_with_context(
    store: &mut Store,
    root: &Path,
    input: Value,
    design_catalog: &Value,
    blueprint_catalog: &Value,
    prepare: impl FnOnce(&Files, &Path, &mut Value) -> Result<()>,
) -> Result<Value> {
    let input: Input = serde_json::from_value(input).context("任务输入格式无效")?;
    if input
        .ask_ratio
        .is_some_and(|r| ![0, 10, 30, 70, 100].contains(&r))
    {
        anyhow::bail!("询问档位无效");
    }
    if !length(&input.prompt, 1, 50000)
        || input.title.as_ref().is_some_and(|s| !length(s, 0, 120))
        || !length(&input.stop_conditions, 0, 5000)
        || input.references.len() > 50
        || input.max_minutes > 1440
    {
        bail!("任务输入超出允许范围");
    }
    let capability = input.capability.as_deref().unwrap_or("code");
    if !["code", "review"].contains(&capability) {
        bail!("任务能力无效");
    }
    if input.direction.as_ref().is_some_and(|s| {
        ![
            "general",
            "story",
            "gameplay",
            "visual",
            "audio",
            "engineering",
            "review",
            "release",
        ]
        .contains(&s.as_str())
    }) {
        bail!("任务方向无效");
    }
    for reference in &input.references {
        if !length(&reference.path, 1, 2000) || !length(&reference.note, 0, 5000) {
            bail!("参考素材参数无效");
        }
        if let Some(r) = &reference.region {
            if [r.x, r.y, r.w, r.h]
                .iter()
                .any(|n| !n.is_finite() || !(0.0..=1.0).contains(n))
            {
                bail!("参考区域必须是归一化坐标");
            }
        }
    }
    let project: Value = store
        .get("project", &input.project_id)?
        .context("项目不存在")?;
    let project_path = Path::new(project["path"].as_str().context("项目路径无效")?);
    let files = Files::new(root.into());
    if Journal::new(store, &files).blocked(&input.project_id)? {
        bail!("项目存在未完成的文件恢复，禁止创建任务");
    }
    let context = if project["blueprint"].is_object() {
        Some(
            json!({"name":project["name"],"revision":project["blueprintRevision"].as_u64().unwrap_or(0),"blueprint":blueprint::validate(project["blueprint"].clone(),blueprint_catalog)?}),
        )
    } else {
        None
    };
    for reference in &input.references {
        safe_path(project_path, &reference.path)?;
    }
    let baseline = files.capture(project_path)?;
    let id = uuid::Uuid::new_v4().to_string();
    let workspaces = safe_path(root, "workspaces")?;
    fs::create_dir_all(&workspaces)?;
    let workspace = safe_path(&workspaces, &id)?;
    files.restore_copy(&baseline, &workspace)?;
    let metadata = safe_path(&workspace, "beaver.project.json")?;
    let design = match fs::File::open(metadata) {
        Ok(handle) => {
            let mut bytes = Vec::new();
            handle.take(512001).read_to_end(&mut bytes)?;
            if bytes.len() > 512000 {
                bail!("项目元数据过大");
            }
            let metadata: Value = serde_json::from_slice(&bytes)?;
            if metadata["schemaVersion"] != 1 {
                bail!("未知项目元数据版本");
            }
            crate::projects::validate_design(&metadata["design"], design_catalog)?;
            Some(metadata["design"].clone())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let title = input.title.filter(|s| !s.is_empty()).unwrap_or_else(|| {
        let mut count = 0;
        input
            .prompt
            .chars()
            .take_while(|c| {
                count += c.len_utf16();
                count <= 60
            })
            .collect()
    });
    let mut task = json!({"id":id,"projectId":input.project_id,"title":title,"prompt":input.prompt,"stopConditions":input.stop_conditions,"references":input.references,"maxMinutes":input.max_minutes,"capability":capability,"status":"queued","createdAt":now,"updatedAt":now,"workspace":workspace,"baseline":baseline,"changes":[],"conflicts":[]});
    if let Some(ref direction) = input.direction {
        task["direction"] = json!(direction);
    }
    task["askRatio"] = json!(input.ask_ratio);
    task["decompose"] = json!(
        capability == "code"
            && input
                .decompose
                .unwrap_or(input.direction.as_deref().is_none_or(|s| s == "general"))
    );
    task["autoAccept"] = json!(input.auto_accept.unwrap_or(true));
    if let Some(design) = design {
        task["design"] = design;
    }
    if let Some(context) = context {
        let directory = safe_path(&workspace, ".beaver-context/project")?;
        fs::create_dir_all(&directory)?;
        fs::write(
            directory.join("blueprint.json"),
            serde_json::to_vec_pretty(&context)?,
        )?;
        task["projectContext"] = context;
        fs::write(
            directory.join("brief.md"),
            crate::task_brief::format(&task, blueprint_catalog),
        )?;
    }
    prepare(&files, &workspace, &mut task)?;
    // Snapshot/context must be complete before a scheduler can see the task.
    store.transaction(|db| {
        db.execute(
            "INSERT INTO entities(kind,id,value) VALUES('task',?,?)",
            rusqlite::params![id, task.to_string()],
        )?;
        let mut count = 0;
        let event: String = input
            .prompt
            .chars()
            .take_while(|c| {
                count += c.len_utf16();
                count <= 32000
            })
            .collect();
        db.execute(
            "INSERT INTO events(task,time,kind,text) VALUES(?,?,'user',?)",
            rusqlite::params![id, now, event],
        )?;
        Ok(())
    })?;
    Ok(task)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn task_freezes_actual_snapshot_and_planning_before_queueing() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("data");
        let mut store = Store::open(&root)?;
        let project = temp.path().join("project");
        fs::create_dir(&project)?;
        fs::write(project.join("project.godot"), "[application]\n")?;
        let designs: Value =
            serde_json::from_str(include_str!("../../../dist-native/design-catalog.json"))?;
        let catalog: Value =
            serde_json::from_str(include_str!("../../../dist-native/blueprint-catalog.json"))?;
        store.put("project","p",&json!({"id":"p","name":"原游戏","path":project,"blueprint":catalog["default"],"blueprintRevision":3,"design":{"outdated":true}}))?;
        let task = create(
            &mut store,
            &root,
            json!({"projectId":"p","prompt":"制作游戏","direction":"story"}),
            &designs,
            &catalog,
        )?;
        assert_eq!(task["status"], "queued");
        assert!(task.get("design").is_none());
        assert_eq!(task["projectContext"]["revision"], 3);
        assert_eq!(task["projectContext"]["name"], "原游戏");
        let workspace = Path::new(task["workspace"].as_str().unwrap());
        let context_file: Value = serde_json::from_slice(&fs::read(
            workspace.join(".beaver-context/project/blueprint.json"),
        )?)?;
        assert_eq!(context_file, task["projectContext"]);
        let brief = fs::read_to_string(workspace.join(".beaver-context/project/brief.md"))?;
        assert_eq!(brief, crate::task_brief::format(&task, &catalog));
        assert!(brief.contains("规划版本 3"));
        assert!(brief.contains("不是完成度"));
        assert!(brief.contains("beaver_ask_user"));
        fs::write(project.join("project.godot"), "human changed")?;
        assert_eq!(
            fs::read_to_string(
                Path::new(task["workspace"].as_str().unwrap()).join("project.godot")
            )?,
            "[application]\n"
        );
        assert_eq!(store.events(task["id"].as_str().unwrap())?.len(), 1);
        assert!(create(
            &mut store,
            &root,
            json!({"projectId":"p","prompt":"x","references":[{"path":"../escape","note":""}]}),
            &designs,
            &catalog
        )
        .is_err());
        assert_eq!(store.list::<Value>("task")?.len(), 1);
        Ok(())
    }
}
