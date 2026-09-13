use crate::{
    files::{safe_path, Files, Snapshot},
    store::Store,
    task_create,
};
use anyhow::{bail, Context, Result};
use base64::Engine;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{fs, path::Path};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    id: String,
    name: String,
    version: String,
    description: String,
    category: Option<String>,
    kind: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_actions;

    #[test]
    fn embedded_packages_and_updates_preserve_customization_and_adoption_boundary() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("data");
        let mut store = Store::open(&root)?;
        let project = temp.path().join("project");
        fs::create_dir(&project)?;
        fs::write(project.join("project.godot"), "[application]\n")?;
        fs::write(project.join("custom.gd"), "human customization")?;
        store.put(
            "project",
            "p",
            &json!({"id":"p","name":"Game","path":project}),
        )?;
        let designs: Value =
            serde_json::from_str(include_str!("../../../dist-native/design-catalog.json"))?;
        let blueprints: Value =
            serde_json::from_str(include_str!("../../../dist-native/blueprint-catalog.json"))?;
        let mut sources: Value =
            serde_json::from_str(include_str!("../../../dist-native/feature-sources.json"))?;
        let mut dialogue = Value::Null;
        for (id, package) in sources.as_object().unwrap() {
            let task = create(
                &mut store,
                &root,
                json!({"projectId":"p","featureId":id}),
                &sources,
                &designs,
                &blueprints,
            )?;
            let workspace = Path::new(task["workspace"].as_str().unwrap());
            for (relative, bytes) in package.as_object().unwrap() {
                assert_eq!(
                    fs::read(workspace.join(".beaver-context/feature/new").join(relative))?,
                    base64::engine::general_purpose::STANDARD.decode(bytes.as_str().unwrap())?
                );
            }
            assert!(store.get::<Value>("feature", &format!("p:{id}"))?.is_none());
            if id == "dialogue" {
                dialogue = task;
            }
        }
        assert!(!dialogue.is_null());
        dialogue["status"] = json!("completed");
        let first = dialogue["id"].as_str().unwrap().to_owned();
        store.put("task", &first, &dialogue)?;
        task_actions::accept(&mut store, &first)?;
        let old = store.get::<Value>("feature", "p:dialogue")?.unwrap();
        assert_eq!(old["version"], "1.0.0");
        let manifest_bytes = base64::engine::general_purpose::STANDARD
            .decode(sources["dialogue"]["feature.json"].as_str().unwrap())?;
        let mut manifest: Value = serde_json::from_slice(&manifest_bytes)?;
        manifest["version"] = json!("1.1.0");
        sources["dialogue"]["feature.json"] =
            json!(base64::engine::general_purpose::STANDARD.encode(serde_json::to_vec(&manifest)?));
        sources["dialogue"]["new-upstream.gd"] =
            json!(base64::engine::general_purpose::STANDARD.encode("new code"));
        let mut upgrade = create(
            &mut store,
            &root,
            json!({"projectId":"p","featureId":"dialogue"}),
            &sources,
            &designs,
            &blueprints,
        )?;
        let mut stale = create(
            &mut store,
            &root,
            json!({"projectId":"p","featureId":"dialogue"}),
            &sources,
            &designs,
            &blueprints,
        )?;
        assert_eq!(upgrade["feature"]["previous"], old);
        let context =
            Path::new(upgrade["workspace"].as_str().unwrap()).join(".beaver-context/feature");
        assert_eq!(
            serde_json::from_slice::<Value>(&fs::read(context.join("old/feature.json"))?)?
                ["version"],
            "1.0.0"
        );
        let changes: Value =
            serde_json::from_slice(&fs::read(context.join("upstream-changes.json"))?)?;
        assert_eq!(changes["from"], "1.0.0");
        assert_eq!(changes["to"], "1.1.0");
        assert!(changes["changes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|change| change["path"] == "new-upstream.gd"));
        assert_eq!(store.get::<Value>("feature", "p:dialogue")?.unwrap(), old);
        upgrade["status"] = json!("completed");
        let id = upgrade["id"].as_str().unwrap().to_owned();
        store.put("task", &id, &upgrade)?;
        task_actions::accept(&mut store, &id)?;
        stale["status"] = json!("completed");
        let stale_id = stale["id"].as_str().unwrap().to_owned();
        store.put("task", &stale_id, &stale)?;
        assert!(task_actions::accept(&mut store, &stale_id).is_err());
        assert_eq!(
            store.get::<Value>("feature", "p:dialogue")?.unwrap()["taskId"],
            id
        );
        assert_eq!(
            fs::read_to_string(project.join("custom.gd"))?,
            "human customization"
        );
        assert!(!project.join("new-upstream.gd").exists());
        let count = store.list::<Value>("task")?.len();
        sources["dialogue"]["../escape.gd"] = json!("YQ==");
        assert!(create(
            &mut store,
            &root,
            json!({"projectId":"p","featureId":"dialogue"}),
            &sources,
            &designs,
            &blueprints
        )
        .is_err());
        assert!(create(
            &mut store,
            &root,
            json!({"projectId":"p","featureId":"../escape"}),
            &sources,
            &designs,
            &blueprints
        )
        .is_err());
        assert!(create(
            &mut store,
            &root,
            json!({"projectId":"p","featureId":"engine-planning-only"}),
            &sources,
            &designs,
            &blueprints
        )
        .is_err());
        assert_eq!(store.list::<Value>("task")?.len(), count);
        Ok(())
    }
}

pub fn create(
    store: &mut Store,
    root: &Path,
    input: Value,
    sources: &Value,
    designs: &Value,
    blueprints: &Value,
) -> Result<Value> {
    let id = input["featureId"].as_str().context("功能块标识无效")?;
    let project = input["projectId"].as_str().context("项目标识无效")?;
    if id.is_empty()
        || !id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        bail!("非法功能块标识");
    }
    let source = sources
        .get(id)
        .and_then(Value::as_object)
        .context("该功能块不存在或仅支持规划，尚未提供可接入源码")?;
    let staging = tempfile::Builder::new()
        .prefix("feature-source-")
        .tempdir_in(root)?;
    for (relative, value) in source {
        let target = safe_path(staging.path(), relative)?;
        fs::create_dir_all(target.parent().context("功能块目录无效")?)?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(value.as_str().context("功能块资源格式无效")?)?;
        fs::write(target, bytes)?;
    }
    let manifest: Manifest =
        serde_json::from_slice(&fs::read(staging.path().join("feature.json"))?)
            .context("功能块清单无效")?;
    if manifest.id != id
        || manifest.name.is_empty()
        || manifest.description.is_empty()
        || !regex::Regex::new(r"^[0-9]+\.[0-9]+\.[0-9]+$")?.is_match(&manifest.version)
        || manifest.category.as_ref().is_some_and(String::is_empty)
        || manifest
            .kind
            .as_deref()
            .is_some_and(|kind| !["module", "reference"].contains(&kind))
    {
        bail!("功能块清单与标识不匹配或字段无效");
    }
    let files = Files::new(root.into());
    let snapshot = files.capture(staging.path())?;
    let old: Option<Value> = store.get("feature", &format!("{project}:{id}"))?;
    let previous: Snapshot = match &old {
        Some(old) => {
            serde_json::from_value(old["snapshot"].clone()).context("已采用功能块快照无效")?
        }
        None => Snapshot::new(),
    };
    let version = old
        .as_ref()
        .and_then(|v| v["version"].as_str())
        .unwrap_or("无");
    let action = if old.is_some() { "更新" } else { "接入" };
    let title = format!(
        "{}功能块 · {}",
        if old.is_some() { "更新" } else { "添加" },
        manifest.name
    );
    let prompt = format!("为项目{action}功能块 {}。已采用版本：{version}；新版本：{}。读取 .beaver-context/feature 中的上游新旧源文件，理解差异后适配到游戏，保留项目定制，验证运行效果。汇报实际集成结果。不要直接覆盖现有定制代码。",manifest.name,manifest.version);
    task_create::create_with_context(
        store,
        root,
        json!({"projectId":project,"prompt":prompt,"decompose":false}),
        designs,
        blueprints,
        |files, workspace, task| {
            let directory = safe_path(workspace, ".beaver-context/feature")?;
            files.restore_copy(&snapshot, &directory.join("new"))?;
            if old.is_some() {
                files.restore_copy(&previous, &directory.join("old"))?;
            }
            fs::write(
                directory.join("upstream-changes.json"),
                serde_json::to_vec_pretty(
                    &json!({"from":old.as_ref().map(|v| &v["version"]),"to":manifest.version,"changes":Files::changes(&previous,&snapshot)}),
                )?,
            )?;
            task["title"] = json!(title);
            task["feature"] = json!({"id":id,"version":manifest.version,"snapshot":snapshot});
            if let Some(old) = old {
                task["feature"]["previous"] = old;
            }
            Ok(())
        },
    )
}
