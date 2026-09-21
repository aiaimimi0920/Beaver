use crate::{
    files::safe_path, project_storage::ProjectStore, project_storage_layout as layout, store::Store,
};
use anyhow::{bail, ensure, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::BTreeSet,
    fs,
    io::{Read, Write},
    path::Path,
};

#[derive(Deserialize)]
struct CreateInput {
    parent: String,
    name: String,
    template: String,
    design: Option<Value>,
    blueprint: Option<Value>,
}

pub fn create_project(
    store: &Store,
    data: &Path,
    input: Value,
    designs: &Value,
    plans: &Value,
    templates: &Value,
) -> Result<Value> {
    use base64::Engine;
    let input: CreateInput = serde_json::from_value(input).context("项目创建参数无效")?;
    let name = &input.name;
    let device = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    if !(1..=80).contains(&name.encode_utf16().count())
        || name
            .chars()
            .any(|c| c <= '\u{1f}' || "<>:\"/\\|?*".contains(c))
        || [".", ".."].contains(&name.as_str())
        || name.ends_with(['.', ' '])
        || ["CON", "PRN", "AUX", "NUL"].contains(&device.as_str())
        || (device.len() == 4
            && (device.starts_with("COM") || device.starts_with("LPT"))
            && matches!(device.as_bytes()[3], b'1'..=b'9'))
    {
        bail!("项目名称包含非法文件名字符");
    }
    if !["blank", "nightbar"].contains(&input.template.as_str()) {
        bail!("模板不存在");
    }
    if let Some(design) = &input.design {
        validate_design(design, designs)?;
    }
    let blueprint = input
        .blueprint
        .map(|value| crate::blueprint::validate(value, plans))
        .transpose()?;
    let parent = fs::canonicalize(&input.parent)?;
    let destination = parent.join(name);
    if destination.starts_with(fs::canonicalize(data)?) {
        bail!("不能在应用内部数据目录创建项目");
    }
    let entries = templates[&input.template]
        .as_object()
        .context("模板内容不存在")?;
    if !entries.contains_key("project.godot") {
        bail!("模板缺少 Godot 项目入口");
    }
    let mut files = Vec::new();
    for (relative, encoded) in entries {
        // Validate every archive path before reserving the destination directory.
        if relative.is_empty()
            || relative.contains(['\\', ':', '\0'])
            || relative
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
        {
            bail!("模板路径无效");
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded.as_str().context("模板内容无效")?)
            .context("模板内容损坏")?;
        files.push((relative.clone(), bytes));
    }
    if let Some(design) = &input.design {
        if entries.contains_key("beaver.project.json") {
            bail!("模板元数据与项目设计冲突");
        }
        let mut bytes = serde_json::to_vec_pretty(&json!({"schemaVersion":1,"design":design}))?;
        bytes.push(b'\n');
        files.push(("beaver.project.json".into(), bytes));
    }
    // Never reuse or overwrite an existing project. Preserve partial output on I/O failure.
    fs::create_dir(&destination).context("无法创建项目目录；请确认名称未被占用")?;
    for (relative, bytes) in files {
        let path = safe_path(&destination, &relative)?;
        fs::create_dir_all(path.parent().context("模板路径无效")?)?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(safe_path(&destination, &relative)?)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
    }
    let project_id = uuid::Uuid::new_v4().to_string();
    let mut project = json!({"id":project_id,"name":name,"path":fs::canonicalize(&destination)?,"createdAt":chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis,true)});
    if let Some(design) = input.design {
        project["design"] = design;
    }
    if let Some(blueprint) = blueprint {
        project["blueprint"] = blueprint;
        project["blueprintRevision"] = json!(1);
    }
    let project_id = project["id"].as_str().context("项目标识无效")?;
    let project_store =
        ProjectStore::initialize(&destination, project_id).context("无法初始化项目本地存储")?;
    project_store.store().put("project", project_id, &project)?;
    store.put("project", project_id, &project)?;
    Ok(project)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GameBrief {
    genres: Vec<String>,
    theme: String,
    style: String,
    scope: String,
}
pub(crate) fn validate_design(design: &Value, catalog: &Value) -> Result<()> {
    let brief: GameBrief = serde_json::from_value(design.clone())?;
    let valid = |key: &str, id: &str| {
        catalog[key]
            .as_array()
            .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(id)))
    };
    if !(1..=2).contains(&brief.genres.len())
        || brief.genres.iter().collect::<BTreeSet<_>>().len() != brief.genres.len()
        || !brief.genres.iter().all(|id| valid("genres", id))
        || !valid("themes", &brief.theme)
        || !valid("styles", &brief.style)
        || !valid("scopes", &brief.scope)
    {
        bail!("项目创作方向包含无效目录选项");
    }
    Ok(())
}
pub fn import_project(
    store: &Store,
    data: &Path,
    directory: &Path,
    catalog: &Value,
) -> Result<Value> {
    let root = fs::canonicalize(directory)?;
    if root.starts_with(fs::canonicalize(data)?) {
        bail!("不能把应用内部数据注册为游戏项目");
    }
    if !fs::metadata(safe_path(&root, "project.godot")?)?.is_file() {
        bail!("不是有效 Godot 项目");
    }
    let metadata = safe_path(&root, "beaver.project.json")?;
    let design = match fs::File::open(metadata) {
        Ok(handle) => {
            let mut bytes = Vec::new();
            handle.take(512001).read_to_end(&mut bytes)?;
            if bytes.len() > 512000 {
                bail!("项目元数据过大");
            }
            let value: Value = serde_json::from_slice(&bytes)?;
            if value["schemaVersion"] != 1 {
                bail!("未知项目元数据版本");
            }
            validate_design(&value["design"], catalog)?;
            Some(value["design"].clone())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    let existing = store.list::<Value>("project")?.into_iter().find(|project| {
        project["path"]
            .as_str()
            .and_then(|path| fs::canonicalize(path).ok())
            .as_ref()
            == Some(&root)
    });
    let local_project = match fs::symlink_metadata(root.join(layout::CONTROL_DIR)) {
        Ok(metadata) => {
            ensure!(metadata.is_dir(), "项目 .beaver 必须是实际目录");
            let local_id = layout::read_manifest(
                &root,
                existing.as_ref().and_then(|project| project["id"].as_str()),
            )?
            .project_id;
            if existing.is_some() {
                None
            } else {
                ensure!(
                    store.get::<Value>("project", &local_id)?.is_none(),
                    "同一项目 ID 已登记在其他位置；请明确重新关联或派生独立项目身份：{local_id}"
                );
                Some(ProjectStore::read_project(&root, &local_id)?)
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    let mut project = match existing.or(local_project) {
        Some(project) => project,
        None => {
            let name = root
                .file_name()
                .and_then(|value| value.to_str())
                .context("项目名称无效")?;
            json!({
                "id": uuid::Uuid::new_v4().to_string(),
                "name": name,
                "path": root.clone(),
                "createdAt": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
            })
        }
    };
    project["path"] = json!(root);
    if let Some(design) = design {
        project["design"] = design;
    } else {
        project
            .as_object_mut()
            .context("项目记录无效")?
            .remove("design");
    }
    store.put(
        "project",
        project["id"].as_str().context("项目标识无效")?,
        &project,
    )?;
    Ok(project)
}

#[cfg(test)]
#[path = "project_import_identity_tests.rs"]
mod identity_tests;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn create_embedded_templates_preserves_bytes_and_initial_plan() -> Result<()> {
        use base64::Engine;
        let temp = tempfile::tempdir()?;
        let data = temp.path().join("data");
        let store = Store::open(&data)?;
        let designs: Value =
            serde_json::from_str(include_str!("../../../dist-native/design-catalog.json"))?;
        let plans: Value =
            serde_json::from_str(include_str!("../../../dist-native/blueprint-catalog.json"))?;
        let templates: Value =
            serde_json::from_str(include_str!("../../../dist-native/templates.json"))?;
        for template in ["blank", "nightbar"] {
            let name = format!("原创-{template}");
            let input = json!({"parent":temp.path(),"name":name,"template":template,"blueprint":plans["default"]});
            let project =
                create_project(&store, &data, input.clone(), &designs, &plans, &templates)?;
            assert_eq!(project["name"], name);
            assert_eq!(project["blueprintRevision"], 1);
            assert_eq!(project["blueprint"], plans["default"]);
            let path = Path::new(project["path"].as_str().unwrap());
            for (relative, encoded) in templates[template].as_object().unwrap() {
                assert_eq!(
                    fs::read(path.join(relative))?,
                    base64::engine::general_purpose::STANDARD.decode(encoded.as_str().unwrap())?
                );
            }
            assert!(create_project(&store, &data, input, &designs, &plans, &templates).is_err());
            assert_eq!(
                store
                    .get::<Value>("project", project["id"].as_str().unwrap())?
                    .unwrap(),
                project
            );
            let project_store = ProjectStore::open(path, project["id"].as_str().unwrap())?;
            assert_eq!(
                project_store
                    .store()
                    .get::<Value>("project", project["id"].as_str().unwrap())?
                    .unwrap(),
                project
            );
        }
        assert_eq!(store.list::<Value>("project")?.len(), 2);
        for name in ["../escape", "CON.txt", "trailing.", "trailing ", ""] {
            assert!(create_project(
                &store,
                &data,
                json!({"parent":temp.path(),"name":name,"template":"blank"}),
                &designs,
                &plans,
                &templates
            )
            .is_err());
        }
        assert!(create_project(
            &store,
            &data,
            json!({"parent":data,"name":"internal","template":"blank"}),
            &designs,
            &plans,
            &templates
        )
        .is_err());
        assert!(create_project(
            &store,
            &data,
            json!({"parent":temp.path(),"name":"invalid-plan","template":"blank","blueprint":{}}),
            &designs,
            &plans,
            &templates
        )
        .is_err());
        assert!(!temp.path().join("invalid-plan").exists());
        let malicious = json!({"blank":{"project.godot":"eA==","../escape":"eA=="}});
        assert!(create_project(
            &store,
            &data,
            json!({"parent":temp.path(),"name":"invalid-template","template":"blank"}),
            &designs,
            &plans,
            &malicious
        )
        .is_err());
        assert!(!temp.path().join("invalid-template").exists());
        Ok(())
    }
    #[test]
    fn reimport_preserves_saved_blueprint_and_validates_design() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let data = temp.path().join("data");
        let root = temp.path().join("game");
        fs::create_dir(&root)?;
        fs::write(root.join("project.godot"), "[application]")?;
        let store = Store::open(&data)?;
        let catalog = json!({"genres":["narrative"],"themes":["cyberpunk"],"styles":["pixel"],"scopes":["prototype"]});
        let mut first = import_project(&store, &data, &root, &catalog)?;
        first["blueprintRevision"] = json!(7);
        first["blueprint"] = json!({"custom":"保留"});
        store.put("project", first["id"].as_str().unwrap(), &first)?;
        let second = import_project(&store, &data, &root, &catalog)?;
        assert_eq!(first, second);
        assert_eq!(store.list::<Value>("project")?.len(), 1);
        fs::write(
            root.join("beaver.project.json"),
            serde_json::to_vec(
                &json!({"schemaVersion":1,"design":{"genres":["narrative","narrative"],"theme":"cyberpunk","style":"pixel","scope":"prototype"}}),
            )?,
        )?;
        assert!(import_project(&store, &data, &root, &catalog).is_err());
        assert!(import_project(&store, &data, &data, &catalog).is_err());
        Ok(())
    }

    #[test]
    fn import_existing_beaver_project_preserves_local_identity() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let data = temp.path().join("data");
        let root = temp.path().join("game");
        fs::create_dir(&root)?;
        fs::write(root.join("project.godot"), "config_version=5\n")?;
        let local_id = "local-project";
        let local = ProjectStore::initialize(&root, local_id)?;
        let project = json!({
            "id": local_id,
            "name": "Portable",
            "path": root,
            "createdAt": "2026-09-19T00:00:00.000Z"
        });
        local.store().put("project", local_id, &project)?;
        drop(local);

        let host = std::sync::Arc::new(std::sync::Mutex::new(Store::open(&data)?));
        let catalog = json!({"genres":["narrative"],"themes":["cyberpunk"],"styles":["pixel"],"scopes":["prototype"]});
        let imported = {
            let host_store = host.lock().map_err(|_| anyhow::anyhow!("host lock"))?;
            import_project(&host_store, &data, &root, &catalog)?
        };

        assert_eq!(imported["id"], local_id);
        let host_store = host.lock().map_err(|_| anyhow::anyhow!("host lock"))?;
        assert_eq!(host_store.list::<Value>("project")?.len(), 1);
        let host_project = host_store.get::<Value>("project", local_id)?.unwrap();
        assert_eq!(
            host_project["path"],
            fs::canonicalize(&root)?.to_string_lossy().as_ref()
        );
        drop(host_store);
        let router = crate::project_storage_router::ProjectStorageRouter::new(host.clone());
        router.open_registered(local_id)?;
        let reimported = {
            let host_store = host.lock().map_err(|_| anyhow::anyhow!("host lock"))?;
            import_project(&host_store, &data, &root, &catalog)?
        };
        assert_eq!(reimported["id"], local_id);
        Ok(())
    }
}
