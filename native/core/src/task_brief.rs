use serde_json::Value;

pub fn prompt(task: &Value, catalog: &Value) -> String {
    let stop = text(&task["stopConditions"]);
    [
        text(&task["prompt"]).to_owned(),
        "资料使用可人工阅读和编辑的 Markdown，按需归档到 docs/world（世界观）、docs/characters（人物）、docs/mechanics（玩法）、docs/art（美术规范）、docs/audio（声音规范）、docs/production（制作计划）、docs/decisions（用户确认）。沿用已有项目的有效分类，不为凑目录生成空文档。由你自行检索并选择上下文。".to_owned(),
        "先阅读项目资料。缺少必须由用户决定的创作意图或知识时，调用 beaver_ask_user 提问；任务会等待回答，不要自行假设用户决定。收到回答后，将确认的设定归档到项目 docs/decisions 下的 Markdown 文档（仅审查任务不得写入）。".to_owned(),
        format(task, catalog),
        if task["validationVersion"] == 1 && task["capability"] != "review" {
            format!("{}\n当前任务 ID：{}", include_str!("../../../resources/instructions/game-validation.md"), text(&task["id"]))
        } else { String::new() },
        task.get("validationRepair").map(|context| format!("本次代码验收失败，请定位并修复后保留回归检查。读取 .beaver-context/validation/repairs/{} 中的 repair.json、run.json、run.log 和 source，查看失败集成候选；原始用户反馈仍保存在 .beaver-context/validation/feedback.json。历史源码仅作对照，不覆盖当前项目。以下是真实 GUT 结果；不要删除断言、跳过失败用例或缩小必需套件来消除红色：\n{context}", text(&context["runId"]))).unwrap_or_default(),
        task.get("restartContext").map(|context| format!("这是同一任务的新 AI 会话。工作副本包含此前未完成的修改，原基线与回退边界仍有效。先核实下列线索和已有文件、docs/decisions 及运行结果，从未完成处继续；不要重新初始化项目或重放已成功操作。旧汇报仅是线索，不是验收证据：\n{context}")).unwrap_or_default(),
        "Beaver 标准工作流由 beaver_workflow_list / beaver_workflow_run 提供。你负责选择、创作和修复决策，AI 可参与任何环节。先发现已启用的工作流；涉及 NPR 人物时读取 beaver.runtime.json 及 inspect 返回的制作规范，使用 Blender MCP 建模，并调用 validate / preview 检查真实产物。涉及人物外观时，先读取 blender-production 的参考对照和分阶段视觉检查要求；计划与子任务验收应包含实际近景和造型质量，不只列文件、贴图数量或技术检查。不要把规划功能意向当成已安装插件。".to_owned(),
        if task["decompose"] == true { include_str!("../../../dist-native/planning-instruction.txt").to_owned() } else { "本任务是独立执行单元，完成自己的目标及验收验证，不再拆分。".to_owned() },
        "所有源代码、场景、配置和 Markdown 必须写为 UTF-8 无 BOM。PowerShell 写入使用 [IO.File]::WriteAllText(path, text, [Text.UTF8Encoding]::new($false))，不要使用默认 Out-File 重定向写源文件。验证前检查修改文件的编码；发现 BOM 或解析错误时先修复再继续。明确失败的工具操作先检查当前文件状态，修正原因后最多重试两次，禁止盲目重放已成功的写入。持续报告当前阶段、失败原因和恢复结果。任务结束仅表示修改已完成；导出交付须以 Beaver 标准导出与包验证结果为准，直接 Godot 导出成功不能证明软件导出链路通过。".to_owned(),
        format!("停止条件：{}", if stop.is_empty() { "完成目标后停止。遇到无法解决的授权或工具缺失时如实报告，不伪造成功。" } else { stop }),
        "参考素材（相对当前项目；区域为归一化坐标）：".to_owned(),
        task.get("references").filter(|v|v.is_array()).map(Value::to_string).unwrap_or_else(||"[]".to_owned()),
        if task["capability"] == "review" { "本任务只做审查，不修改项目。" } else { "在当前项目副本完成修改，不访问或修改原始项目。最后验证结果并用中文汇报。" }.to_owned(),
    ].join("\n\n")
}

fn text(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}
fn choice<'a>(catalog: &'a Value, group: &str, id: &Value) -> Option<&'a Value> {
    catalog[group]
        .as_array()?
        .iter()
        .find(|item| item["id"] == *id)
}
fn name(catalog: &Value, group: &str, id: &Value) -> String {
    choice(catalog, group, id)
        .and_then(|item| item["name"].as_str())
        .unwrap_or(text(id))
        .to_owned()
}
fn joined(value: &Value, separator: &str) -> String {
    value
        .as_array()
        .map(|items| items.iter().map(text).collect::<Vec<_>>().join(separator))
        .unwrap_or_default()
}

/// The task's frozen context is authoritative, never the project's current plan.
pub fn format(task: &Value, catalog: &Value) -> String {
    let context = &task["projectContext"];
    if !context.is_object() {
        let design = &task["design"];
        if !design.is_object() {
            return String::new();
        }
        let legacy = &catalog["legacy"];
        let genres = design["genres"]
            .as_array()
            .map(|ids| {
                ids.iter()
                    .map(|id| name(legacy, "genres", id))
                    .collect::<Vec<_>>()
                    .join(" + ")
            })
            .unwrap_or_default();
        return [
            "项目创作方向（不是已实现功能；本次明确要求优先，不自行接入未要求的功能块）：".to_owned(),
            format!("类型：{genres}"),
            format!("题材：{}", name(legacy,"themes",&design["theme"])),
            format!("表现：{}", name(legacy,"styles",&design["style"])),
            format!("规模：{}", choice(legacy,"scopes",&design["scope"]).map(|v|text(&v["goal"])).unwrap_or("")),
            "剧情、角色、图像、音乐与代码均采用原创或有权使用的内容。不要把类型选择误当成完整游戏模板已经具备相关玩法。".to_owned(),
        ].join("\n");
    }
    let b = &context["blueprint"];
    let audience = choice(catalog, "audiences", &b["audience"]);
    let size = choice(catalog, "sizes", &b["size"]);
    let meaning = |item: Option<&Value>| {
        item.map(|v| text(&v["meaning"]).to_owned())
            .unwrap_or_default()
    };
    let theme = if b["theme"]["mode"] == "custom" {
        text(&b["theme"]["value"]).to_owned()
    } else {
        name(catalog, "themes", &b["theme"]["value"])
    };
    let secondary = if b["genres"][1].is_null() {
        "无".to_owned()
    } else {
        name(catalog, "briefGenres", &b["genres"][1])
    };
    let online = &b["online"];
    let mut lines = vec![
        format!(
            "项目创作约定：{}（任务创建时冻结，规划版本 {}）",
            text(&context["name"]),
            context["revision"]
        ),
        format!("主类别：{}", name(catalog, "briefGenres", &b["genres"][0])),
        format!("副类别：{secondary}"),
        format!("主题：{theme}"),
        format!(
            "目标评级：{}。{} 这是创作尺度，不是已取得官方评级。",
            name(catalog, "audiences", &b["audience"]),
            meaning(audience)
        ),
        format!(
            "游戏大小：{}。{}",
            name(catalog, "sizes", &b["size"]),
            meaning(size)
        ),
        format!("表现风格：{}", name(catalog, "styles", &b["style"])),
        if online["enabled"] == true {
            format!("在线多人：开启；拓扑：{}；每房间 {} 人；目标峰值 {} 人；初期实例 {}；区域：{}。这些数字是设计目标，不是已部署或已压测结果。",if online["topology"] == "dedicated" {"专用服务器"} else {"房主与中继"},online["playersPerSession"],online["peakCcu"],online["serverCount"],text(&online["region"]))
        } else {
            "在线多人：关闭。不自行添加在线服务器、账号或联机依赖。".to_owned()
        },
        "阶段投入优先级（0-5，不是完成度，不自动把所有项目变成本次任务）：".to_owned(),
    ];
    if let Some(phases) = catalog["phases"].as_array() {
        for phase in phases {
            let metrics = phase["metrics"]
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .map(|metric| {
                            format!(
                                "{} {}",
                                text(&metric["name"]),
                                b["priorities"][text(&metric["id"])]
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("、")
                })
                .unwrap_or_default();
            lines.push(format!("{}：{metrics}", text(&phase["name"])));
        }
    }
    let features = joined(&b["plannedFeatures"], "、");
    lines.push(format!(
        "规划功能包 ID：{}。勾选仅表示意向，不代表已安装、已实现或授权一次性接入全部功能。",
        if features.is_empty() {
            "无"
        } else {
            &features
        }
    ));
    if let Some(choices) = catalog["campaigns"].as_array() {
        for item in choices {
            let campaign = &b["campaigns"][text(&item["id"])];
            let channels = joined(&campaign["channels"], "、");
            let notes = text(&campaign["notes"]);
            if !channels.is_empty() || !notes.is_empty() {
                lines.push(format!("{}草案：{channels}；{notes}", text(&item["name"])));
            }
        }
    }
    lines.extend([
        "发行与宣传信息仅供准备内容，不自动发布、联系媒体或付费投放。".to_owned(),
        "围绕本次明确目标自行寻找上下文。若目标与已锁定的类别、评级、规模或联机设定矛盾，调用 beaver_ask_user 询问，不静默改变基本设定。".to_owned(),
        "保持原创或使用有权使用的内容；按实际运行结果报告，不把规划数值或功能意向当作已完成成果。".to_owned(),
    ]);
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn prompt_preserves_user_intent_references_stop_and_review_boundary() {
        let task = json!({"prompt":"只审查对白","capability":"review","stopConditions":"缺少角色设定时停下","references":[{"path":"docs/characters/主角.md","note":"保持性格","region":{"x":0.1,"y":0.2,"w":0.3,"h":0.4}}]});
        let actual = prompt(&task, &Value::Null);
        assert!(actual.starts_with("只审查对白\n\n"));
        assert!(actual.contains("停止条件：缺少角色设定时停下"));
        assert!(actual.contains(&task["references"].to_string()));
        assert!(actual.contains("beaver_ask_user"));
        assert!(actual.contains("docs/decisions"));
        assert!(actual.ends_with("本任务只做审查，不修改项目。"));
        assert!(!actual.contains("最后验证结果并用中文汇报。"));
        let code = prompt(
            &json!({"prompt":"制作对白","capability":"code","references":[]}),
            &Value::Null,
        );
        assert!(code.contains("完成目标后停止。"));
        assert!(code
            .ends_with("在当前项目副本完成修改，不访问或修改原始项目。最后验证结果并用中文汇报。"));
    }
}
