use crate::{
    files::{safe_path, Change, Snapshot},
    store::Store,
    task_create,
};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{fs, path::Path};

fn prefix(text: &str, limit: usize) -> String {
    let mut count = 0;
    text.chars()
        .take_while(|c| {
            count += c.len_utf16();
            count <= limit
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files::Files;

    #[test]
    fn relations_freeze_current_project_and_preserve_parent_and_context() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("data");
        let project = temp.path().join("project");
        fs::create_dir(&project)?;
        fs::write(project.join("project.godot"), "[application]\n")?;
        fs::write(project.join("story.md"), "before")?;
        let mut store = Store::open(&root)?;
        let designs: Value =
            serde_json::from_str(include_str!("../../../dist-native/design-catalog.json"))?;
        let blueprints: Value =
            serde_json::from_str(include_str!("../../../dist-native/blueprint-catalog.json"))?;
        store.put("project","p",&json!({"id":"p","name":"游戏","path":project,"blueprint":blueprints["default"],"blueprintRevision":1}))?;
        let mut parent = task_create::create(
            &mut store,
            &root,
            json!({"projectId":"p","prompt":"原目标","title":"长".repeat(120),"direction":"story","references":[{"path":"story.md","note":"参考"}]}),
            &designs,
            &blueprints,
        )?;
        let id = parent["id"].as_str().unwrap().to_owned();
        assert!(create(
            &mut store,
            &root,
            "task.followup",
            json!({"id":id,"text":"继续"}),
            &designs,
            &blueprints
        )
        .is_err());
        parent["status"] = json!("awaitingInput");
        parent["report"] = json!("阶段汇报");
        parent["clarifications"] = json!([{"id":"question","answers":{"tone":"温暖"}}]);
        store.put("task", &id, &parent)?;
        fs::write(
            Path::new(parent["workspace"].as_str().unwrap()).join("unfinished.md"),
            "not merged",
        )?;
        let files = Files::new(root.clone());
        let before = files.capture(&project)?;
        fs::write(project.join("story.md"), "after and committed")?;
        let after = files.capture(&project)?;
        store.put("project","p",&json!({"id":"p","name":"新游戏","path":project,"blueprint":blueprints["default"],"blueprintRevision":2}))?;
        let child = create(
            &mut store,
            &root,
            "task.delegate",
            json!({"id":id,"text":"  独立子目标  "}),
            &designs,
            &blueprints,
        )?;
        assert_eq!(child["relation"], "child");
        assert_eq!(child["parentTaskId"], id);
        assert_eq!(child["references"], parent["references"]);
        assert_eq!(child["projectContext"]["revision"], 2);
        assert_eq!(child["title"], "子任务 · 独立子目标");
        assert_ne!(child["workspace"], parent["workspace"]);
        let workspace = Path::new(child["workspace"].as_str().unwrap());
        assert!(!workspace.join("unfinished.md").exists());
        assert_eq!(
            fs::read_to_string(workspace.join("story.md"))?,
            "after and committed"
        );
        let context: Value = serde_json::from_slice(&fs::read(
            workspace.join(".beaver-context/parent/task.json"),
        )?)?;
        assert_eq!(context["conversation"][0]["text"], "原目标");
        assert_eq!(store.get::<Value>("task", &id)?.unwrap(), parent);

        parent["status"] = json!("completed");
        parent["changes"] = serde_json::to_value(Files::changes(&before, &after))?;
        store.put("task", &id, &parent)?;
        let followup = create(
            &mut store,
            &root,
            "task.followup",
            json!({"id":id,"text":"继续原目标"}),
            &designs,
            &blueprints,
        )?;
        assert_eq!(followup["relation"], "followup");
        let context: Value = serde_json::from_slice(&fs::read(
            Path::new(followup["workspace"].as_str().unwrap())
                .join(".beaver-context/followup/task.json"),
        )?)?;
        assert_eq!(context["report"], parent["report"]);
        assert_eq!(context["clarifications"], parent["clarifications"]);
        let rollback = create(
            &mut store,
            &root,
            "task.dialogueRollback",
            json!({"id":id,"text":"只撤销故事"}),
            &designs,
            &blueprints,
        )?;
        assert_eq!(
            rollback["title"],
            format!("对话回退 · {}", "长".repeat(120))
        );
        let context =
            Path::new(rollback["workspace"].as_str().unwrap()).join(".beaver-context/rollback");
        assert_eq!(
            fs::read_to_string(context.join("before/story.md"))?,
            "before"
        );
        assert_eq!(
            fs::read_to_string(context.join("after/story.md"))?,
            "after and committed"
        );
        assert_eq!(
            fs::read_to_string(project.join("story.md"))?,
            "after and committed"
        );
        assert_eq!(store.get::<Value>("task", &id)?.unwrap(), parent);
        set_direction(&store, &json!({"id":id,"direction":"audio"}))?;
        assert_eq!(
            store.get::<Value>("task", &id)?.unwrap()["direction"],
            "audio"
        );
        assert!(set_direction(&store, &json!({"id":id,"direction":"unknown"})).is_err());
        assert!(create(
            &mut store,
            &root,
            "task.delegate",
            json!({"id":id,"text":"   "}),
            &designs,
            &blueprints
        )
        .is_err());
        let count = store.list::<Value>("task")?.len();
        let failed = task_create::create_with_context(
            &mut store,
            &root,
            json!({"projectId":"p","prompt":"bad context"}),
            &designs,
            &blueprints,
            |_, _, _| bail!("fixture context error"),
        );
        assert!(failed.is_err());
        assert_eq!(store.list::<Value>("task")?.len(), count);
        Ok(())
    }
}

pub fn set_direction(store: &Store, input: &Value) -> Result<Value> {
    let direction = input["direction"].as_str().context("任务方向无效")?;
    if ![
        "general",
        "story",
        "gameplay",
        "visual",
        "audio",
        "engineering",
        "review",
        "release",
    ]
    .contains(&direction)
    {
        bail!("任务方向无效");
    }
    let id = input["id"].as_str().context("任务标识无效")?;
    let mut task: Value = store.get("task", id)?.context("任务不存在")?;
    task["direction"] = json!(direction);
    task["updatedAt"] =
        json!(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
    store.put("task", id, &task)?;
    Ok(Value::Null)
}

pub fn create(
    store: &mut Store,
    root: &Path,
    method: &str,
    input: Value,
    designs: &Value,
    blueprints: &Value,
) -> Result<Value> {
    create_seeded(store, root, method, input, designs, blueprints, None)
}

pub(crate) fn create_seeded(
    store: &mut Store,
    root: &Path,
    method: &str,
    input: Value,
    designs: &Value,
    blueprints: &Value,
    asset_seed: Option<(&crate::asset_task::Feedback, Option<&str>)>,
) -> Result<Value> {
    let id = input["id"].as_str().context("任务标识无效")?;
    let text = input["text"].as_str().context("补充要求无效")?;
    let text = if method == "task.dialogueRollback" {
        text
    } else {
        text.trim()
    };
    let limit = if asset_seed.is_some() { 12000 } else { 10000 };
    if text.is_empty() || text.encode_utf16().count() > limit {
        bail!("补充要求长度无效");
    }
    let source: Value = store.get("task", id)?.context("任务不存在")?;
    let title = source["title"].as_str().context("原任务标题无效")?;
    let (new_title,mut prompt,folder,relation) = match method {
        "task.followup" => {
            if source["status"] != "completed" && source["status"] != "rolledBack" { bail!("当前任务尚未交付，请在原任务中补充"); }
            (format!("继续创作 · {}",prefix(title,80)), format!("基于项目当前版本继续任务「{title}」。原任务 ID：{id}。原目标、汇报与已确认回答保存在 .beaver-context/followup/task.json。阅读相关记录并检查现有文件，保留其他任务成果，不重复已完成的工作。\n用户新要求：{text}"),"followup",Some("followup"))
        }
        "task.delegate" => (format!("子任务 · {}",prefix(text,70)),format!("完成父任务「{title}」委派的独立子目标。父会话记录在 .beaver-context/parent/task.json。当前工作副本来自项目已合入版本，不包含父任务尚未合入的修改；缺少前置成果时询问用户，不假定其存在。不要替父任务宣布完成。\n子目标：{text}"),"parent",Some("child")),
        "task.dialogueRollback" => (format!("对话回退 · {title}"),format!("请处理任务「{title}」的选择性回退/冲突整合。\n用户要求：{text}\n原任务变更记录与前后文件保存在 .beaver-context/rollback。保留其他任务的成果，不能直接覆盖整个项目。"),"rollback",None),
        _ => bail!("未知任务关系"),
    };
    if asset_seed.is_some() {
        prompt = format!("检查资产任务「{title}」的当前项目成果和 .beaver-context/followup/task.json。通过 beaver_asset_task 的 poll 读取本后续任务仍有效的反馈和实际图像，再按协议修改、核验和交付。反馈可能已被用户撤回；不要从历史任务要求推断仍需执行的修改。保留其他任务的新成果。");
    }
    let mut request = json!({"projectId":source["projectId"],"title":prefix(&new_title,120),"prompt":prompt,"decompose":false});
    if asset_seed.is_some() {
        request["assetTask"] = json!(true);
        request["askRatio"] = source["askRatio"].clone();
    }
    if relation.is_some() {
        for key in [
            "references",
            "capability",
            "stopConditions",
            "maxMinutes",
            "direction",
        ] {
            if let Some(value) = source.get(key) {
                request[key] = value.clone();
            }
        }
    }
    let mut record = json!({});
    for key in [
        "id",
        "title",
        "prompt",
        "report",
        "clarifications",
        "references",
    ] {
        if let Some(value) = source.get(key) {
            record[key] = value.clone();
        }
    }
    if method == "task.delegate" {
        record["status"] = source["status"].clone();
        record["conversation"] = serde_json::to_value(store.events(id)?)?;
    }
    let changes: Vec<Change> = if relation.is_none() {
        serde_json::from_value(source["changes"].clone()).context("原任务变更记录无效")?
    } else {
        Vec::new()
    };
    task_create::create_with_context(
        store,
        root,
        request,
        designs,
        blueprints,
        |files, workspace, task| {
            task["title"] = json!(new_title);
            if let Some((seed, restore)) = asset_seed {
                task["assetFeedbackSeed"] = serde_json::to_value(seed)?;
                task["assetRestore"] = json!(restore);
            }
            let directory = safe_path(workspace, &format!(".beaver-context/{folder}"))?;
            fs::create_dir_all(&directory)?;
            if let Some(relation) = relation {
                task["parentTaskId"] = source["id"].clone();
                task["relation"] = json!(relation);
                fs::write(
                    directory.join("task.json"),
                    serde_json::to_vec_pretty(&record)?,
                )?;
            } else {
                fs::write(
                    directory.join("changes.json"),
                    serde_json::to_vec_pretty(&changes)?,
                )?;
                for side in ["before", "after"] {
                    let mut snapshot = Snapshot::new();
                    for change in &changes {
                        let value = if side == "before" {
                            &change.before
                        } else {
                            &change.after
                        };
                        if let Some(hash) = value {
                            snapshot.insert(change.path.clone(), hash.clone());
                        }
                    }
                    files.restore_copy(&snapshot, &directory.join(side))?;
                }
            }
            Ok(())
        },
    )
}
