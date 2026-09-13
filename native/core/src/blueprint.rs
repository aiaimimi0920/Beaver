use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::collections::HashSet;

pub const PRIORITIES: [&str; 15] = [
    "story",
    "characters",
    "gameplay",
    "graphics",
    "ui",
    "physics",
    "ai",
    "sound",
    "modding",
    "network",
    "animation",
    "optimization",
    "artwork",
    "tutorial",
    "cutscenes",
];

pub const OVERVIEW: [&str; 7] = [
    "genres", "theme", "audience", "size", "style", "online", "name",
];

pub fn project_plan(project: &Value, catalog: &Value) -> Value {
    if project["blueprint"].is_object() {
        return project["blueprint"].clone();
    }
    let mut plan = catalog["default"].clone();
    let design = &project["design"];
    if let Some(genres) = design["genres"].as_array() {
        let translated: Vec<_> = genres
            .iter()
            .filter_map(Value::as_str)
            .map(|id| match id {
                "narrative" => "visual-novel",
                "roguelite" => "crook-like",
                "platformer" => "platforms",
                _ => id,
            })
            .filter(|id| {
                catalog["genres"]
                    .as_array()
                    .is_some_and(|items| items.iter().any(|item| item["id"] == *id))
            })
            .map(|id| Value::String(id.into()))
            .collect();
        if !translated.is_empty() {
            plan["genres"] = Value::Array(translated);
        }
    }
    if let Some(theme) = design["theme"].as_str() {
        let theme = match theme {
            "space" => "science-fiction",
            "modern" => "urban",
            "historical" => "history",
            "wasteland" => "postapocalyptic",
            "campus" => "校园青春",
            "cozy" => "治愈日常",
            "nature" => "自然生态",
            "mystery" => "侦探悬疑",
            _ => theme,
        };
        let preset = catalog["themes"]
            .as_array()
            .is_some_and(|items| items.iter().any(|item| item["id"] == theme));
        plan["theme"] =
            serde_json::json!({"mode":if preset {"preset"} else {"custom"},"value":theme});
    }
    if design["style"].is_string() {
        plan["style"] = design["style"].clone();
    }
    plan
}

pub fn save(
    store: &crate::store::Store,
    id: &str,
    input: Value,
    expected: u64,
    overview: bool,
    catalog: &Value,
) -> Result<Value> {
    let mut project: Value = store.get("project", id)?.context("项目不存在")?;
    if project["blueprintRevision"].as_u64().unwrap_or(0) != expected {
        bail!("项目设定已有更新，草稿已保留，请重新载入后再编辑");
    }
    let base = project_plan(&project, catalog);
    let mut name = project["name"].clone();
    let candidate = if overview {
        object(&input, &OVERVIEW)?;
        name = input["name"].clone();
        trim(&mut name)?;
        if text(&name, 1, 80)?.chars().any(|c| (c as u32) < 32) {
            bail!("游戏名称不能包含控制字符");
        }
        let mut plan = base.clone();
        for key in OVERVIEW.into_iter().filter(|key| *key != "name") {
            plan[key] = input[key].clone();
        }
        validate(plan, catalog)?
    } else {
        validate(input, catalog)?
    };
    let changed = OVERVIEW
        .into_iter()
        .filter(|key| *key != "name")
        .any(|key| base[key] != candidate[key]);
    if !overview && changed {
        bail!("基础设定已锁定，请在总览中开启编辑后保存");
    }
    if overview && !changed && name == project["name"] {
        return Ok(project);
    }
    project["name"] = name;
    project["blueprint"] = candidate;
    project["blueprintRevision"] =
        serde_json::json!(expected.checked_add(1).context("规划版本溢出")?);
    store.put("project", id, &project)?;
    Ok(project)
}

fn object(value: &Value, keys: &[&str]) -> Result<()> {
    let object = value.as_object().context("规划字段格式无效")?;
    if object.len() != keys.len() || !keys.iter().all(|key| object.contains_key(*key)) {
        bail!("规划包含未知字段或缺少必填字段");
    }
    Ok(())
}
fn text(value: &Value, min: usize, max: usize) -> Result<&str> {
    let text = value.as_str().context("规划文本格式无效")?;
    if !(min..=max).contains(&text.encode_utf16().count()) {
        bail!("规划文本长度无效");
    }
    Ok(text)
}
fn number(value: &Value, min: u64, max: u64) -> Result<u64> {
    value
        .as_u64()
        .filter(|n| (min..=max).contains(n))
        .context("规划数值超出范围或不是整数")
}
fn choice(value: &Value, choices: &[&str]) -> Result<()> {
    if !choices.contains(&value.as_str().unwrap_or("")) {
        bail!("规划选项无效");
    }
    Ok(())
}
fn catalog_choice(value: &Value, catalog: &Value) -> Result<()> {
    let id = value.as_str().context("目录选项无效")?;
    if !catalog
        .as_array()
        .context("目录缺失")?
        .iter()
        .any(|item| item["id"] == id)
    {
        bail!("请选择目录中的选项");
    }
    Ok(())
}
fn trim(value: &mut Value) -> Result<()> {
    let original = value.as_str().context("规划文本格式无效")?;
    *value = Value::String(
        original
            .trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
            .into(),
    );
    Ok(())
}

pub fn validate(mut value: Value, catalog: &Value) -> Result<Value> {
    object(
        &value,
        &[
            "version",
            "genres",
            "theme",
            "audience",
            "size",
            "style",
            "online",
            "priorities",
            "plannedFeatures",
            "campaigns",
        ],
    )?;
    if value["version"] != 1 {
        bail!("未知规划版本");
    }
    let genres = value["genres"].as_array().context("游戏类别格式无效")?;
    if !(1..=2).contains(&genres.len()) {
        bail!("请选择一项主类别及至多一项副类别");
    }
    let mut seen = HashSet::new();
    for genre in genres {
        catalog_choice(genre, &catalog["genres"])?;
        if !seen.insert(genre.as_str().unwrap()) {
            bail!("主副类别不能重复");
        }
    }
    object(&value["theme"], &["mode", "value"])?;
    match value["theme"]["mode"].as_str() {
        Some("preset") => catalog_choice(&value["theme"]["value"], &catalog["themes"])?,
        Some("custom") => {
            trim(&mut value["theme"]["value"])?;
            text(&value["theme"]["value"], 1, 120)?;
        }
        _ => bail!("主题模式无效"),
    }
    choice(&value["audience"], &["all", "young", "mature"])?;
    choice(&value["size"], &["indie", "normal", "big", "aaa"])?;
    choice(
        &value["style"],
        &[
            "pixel",
            "illustrated",
            "minimal-2d",
            "low-poly",
            "stylized-3d",
            "text",
        ],
    )?;
    object(
        &value["online"],
        &[
            "enabled",
            "topology",
            "playersPerSession",
            "peakCcu",
            "serverCount",
            "region",
        ],
    )?;
    let online = value["online"]["enabled"]
        .as_bool()
        .context("联机开关无效")?;
    choice(&value["online"]["topology"], &["dedicated", "host-relay"])?;
    let players = number(&value["online"]["playersPerSession"], 2, 10000)?;
    let peak = number(&value["online"]["peakCcu"], 2, 10000000)?;
    number(&value["online"]["serverCount"], 1, 100000)?;
    trim(&mut value["online"]["region"])?;
    text(&value["online"]["region"], 1, 120)?;
    if online && peak < players {
        bail!("峰值同时在线人数不能小于单局人数");
    }
    object(&value["priorities"], &PRIORITIES)?;
    for key in PRIORITIES {
        number(&value["priorities"][key], 0, 5)?;
    }
    let features = value["plannedFeatures"]
        .as_array()
        .context("功能规划格式无效")?;
    if features.len() > 100 {
        bail!("功能规划超过上限");
    }
    let mut seen = HashSet::new();
    for feature in features {
        let id = text(feature, 1, usize::MAX)?;
        if !id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            || !seen.insert(id)
        {
            bail!("功能规划标识无效或重复");
        }
    }
    object(&value["campaigns"], &["press", "demo", "advertising"])?;
    for kind in ["press", "demo", "advertising"] {
        let campaign = &value["campaigns"][kind];
        object(campaign, &["notes", "channels"])?;
        text(&campaign["notes"], 0, 3000)?;
        let channels = campaign["channels"]
            .as_array()
            .context("宣传渠道格式无效")?;
        if channels.len() > 8 {
            bail!("宣传渠道过多");
        }
        for channel in channels {
            text(channel, 1, 80)?;
        }
    }
    let genres = value["genres"].as_array().unwrap();
    if value["audience"] != "mature"
        && (genres.contains(&Value::String("eroge".into()))
            || (value["theme"]["mode"] == "preset" && value["theme"]["value"] == "erotic"))
    {
        bail!("成人向类别或主题需选择成年人目标评级");
    }
    if !online && genres.contains(&Value::String("mmorpg".into())) {
        bail!("大型多人在线角色扮演需开启在线多人规划");
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn catalog() -> Value {
        serde_json::from_str(include_str!("../../../dist-native/blueprint-catalog.json")).unwrap()
    }
    #[test]
    fn defaults_and_custom_theme_normalize_without_extra_fields() -> Result<()> {
        let catalog = catalog();
        let mut value = validate(catalog["default"].clone(), &catalog)?;
        value["theme"] = json!({"mode":"custom","value":"  原创主题  "});
        assert_eq!(
            validate(value.clone(), &catalog)?["theme"]["value"],
            "原创主题"
        );
        value["unexpected"] = json!(true);
        assert!(validate(value, &catalog).is_err());
        Ok(())
    }
    #[test]
    fn ratings_online_capacity_and_priority_constraints_hold() {
        let catalog = catalog();
        let default = catalog["default"].clone();
        let mut value = default.clone();
        value["genres"] = json!(["eroge"]);
        assert!(validate(value.clone(), &catalog).is_err());
        value["audience"] = json!("mature");
        assert!(validate(value, &catalog).is_ok());
        let mut value = default.clone();
        value["genres"] = json!(["mmorpg"]);
        assert!(validate(value.clone(), &catalog).is_err());
        value["online"]["enabled"] = json!(true);
        assert!(validate(value.clone(), &catalog).is_ok());
        value["online"]["peakCcu"] = json!(2);
        assert!(validate(value, &catalog).is_err());
        let mut value = default.clone();
        value["priorities"]["story"] = json!(6);
        assert!(validate(value, &catalog).is_err());
        let mut value = default;
        value["plannedFeatures"] = json!(["save", "save"]);
        assert!(validate(value, &catalog).is_err());
    }
    #[test]
    fn saves_preserve_locked_fields_revisions_and_unrelated_plans() -> Result<()> {
        let catalog = catalog();
        let temp = tempfile::tempdir()?;
        let store = crate::store::Store::open(temp.path())?;
        let project = json!({"id":"p","name":"原名称","design":{"genres":["narrative"],"theme":"campus","style":"pixel","scope":"prototype"},"future":42});
        store.put("project", "p", &project)?;
        let mut plan = project_plan(&project, &catalog);
        assert_eq!(plan["genres"], json!(["visual-novel"]));
        assert_eq!(plan["theme"], json!({"mode":"custom","value":"校园青春"}));
        plan["priorities"]["story"] = json!(5);
        let saved = save(&store, "p", plan.clone(), 0, false, &catalog)?;
        assert_eq!(saved["blueprintRevision"], 1);
        assert!(save(&store, "p", plan.clone(), 0, false, &catalog).is_err());
        plan["size"] = json!("big");
        assert!(save(&store, "p", plan.clone(), 1, false, &catalog).is_err());
        let mut overview = serde_json::Map::new();
        for key in OVERVIEW {
            overview.insert(
                key.into(),
                if key == "name" {
                    json!("  新名称  ")
                } else {
                    plan[key].clone()
                },
            );
        }
        let saved = save(
            &store,
            "p",
            Value::Object(overview.clone()),
            1,
            true,
            &catalog,
        )?;
        assert_eq!(saved["name"], "新名称");
        assert_eq!(saved["blueprint"]["priorities"]["story"], 5);
        assert_eq!(saved["future"], 42);
        assert_eq!(saved["blueprintRevision"], 2);
        assert_eq!(
            save(&store, "p", Value::Object(overview), 2, true, &catalog)?["blueprintRevision"],
            2
        );
        Ok(())
    }
}
