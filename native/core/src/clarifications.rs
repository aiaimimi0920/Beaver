use crate::store::Store;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};

#[derive(Deserialize, Serialize)]
pub struct Choice {
    label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Question {
    #[serde(skip_serializing_if = "Option::is_none")]
    importance: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    recommended: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    id: String,
    question: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_secret: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<Vec<Choice>>,
}

fn length(text: &str, min: usize, max: usize) -> bool {
    (min..=max).contains(&text.encode_utf16().count())
}

pub fn validate_questions(value: &Value) -> Result<Vec<Question>> {
    let items = value["questions"]
        .as_array()
        .context("请提出 1-3 个非敏感问题")?;
    if !(1..=3).contains(&items.len()) {
        bail!("请提出 1-3 个非敏感问题");
    }
    let mut ids = HashSet::new();
    let mut result = Vec::new();
    for item in items {
        if item.get("isSecret").is_some_and(|v| v != false) {
            bail!("禁止通过创作问题索取凭据");
        }
        let mut question: Question =
            serde_json::from_value(item.clone()).map_err(|_| anyhow::anyhow!("问题格式无效"))?;
        question.question = question
            .question
            .trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
            .into();
        if !length(&question.id, 1, 80)
            || !length(&question.question, 1, 3000)
            || !ids.insert(question.id.clone())
        {
            bail!("问题内容为空、过长或标识重复");
        }
        if question.importance.is_some_and(|n| !(1..=100).contains(&n))
            || question
                .recommended
                .as_ref()
                .is_some_and(|s| !length(s, 1, 200))
            || question
                .reason
                .as_ref()
                .is_some_and(|s| !length(s, 1, 1000))
        {
            bail!("决策元数据无效");
        }
        if let Some(options) = &question.options {
            if options.len() > 8 {
                bail!("问题选项过多");
            }
            for option in options {
                if !length(&option.label, 1, 200)
                    || option
                        .description
                        .as_ref()
                        .is_some_and(|s| !length(s, 0, 1000))
                {
                    bail!("问题选项无效");
                }
            }
        }
        result.push(question);
    }
    Ok(result)
}

pub fn validate_answers(item: &Value, input: Value) -> Result<BTreeMap<String, String>> {
    if item.get("answers").is_some_and(|v| !v.is_null()) {
        bail!("问题已经回答");
    }
    let questions = validate_questions(item)?;
    let mut answers: BTreeMap<String, String> =
        serde_json::from_value(input).map_err(|_| anyhow::anyhow!("回答格式无效"))?;
    if answers.len() != questions.len() || questions.iter().any(|q| !answers.contains_key(&q.id)) {
        bail!("请回答全部问题");
    }
    for answer in answers.values_mut() {
        *answer = answer
            .trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
            .into();
        if !length(answer, 1, 10000) {
            bail!("回答为空或过长");
        }
    }
    Ok(answers)
}

fn persist(store: &mut Store, task: &Value, kind: &str, text: &str, now: &str) -> Result<()> {
    let id = task["id"].as_str().context("任务标识无效")?;
    let mut units = 0;
    let text: String = text
        .chars()
        .take_while(|c| {
            units += c.len_utf16();
            units <= 32000
        })
        .collect();
    store.transaction(|db| {
        let updated = db.execute(
            "UPDATE entities SET value=? WHERE kind='task' AND id=?",
            rusqlite::params![task.to_string(), id],
        )?;
        if updated != 1 {
            bail!("任务已被移除，未保存回答");
        }
        db.execute(
            "INSERT INTO events(task,time,kind,text) VALUES(?,?,?,?)",
            rusqlite::params![id, now, kind, text],
        )?;
        Ok(())
    })
}

/// Persist the question before the executor closes the process. No invented reply.
pub fn park(store: &mut Store, id: &str, method: &str, params: &Value) -> Result<Value> {
    let mut task: Value = store.get("task", id)?.context("任务不存在")?;
    if task["id"] != id {
        bail!("任务标识不匹配");
    }
    if task["status"] != "running"
        || !task["threadId"].is_string()
        || params["threadId"] != task["threadId"]
        || (task["turnId"].is_string() && params["turnId"] != task["turnId"])
    {
        bail!("过期或不匹配的任务问题");
    }
    let body = match method {
        "item/tool/requestUserInput" => params,
        "item/tool/call" if params["tool"] == "beaver_ask_user" => &params["arguments"],
        _ => bail!("不支持此交互请求"),
    };
    let questions = validate_questions(body)?;
    let ratio = crate::autonomy::effective(store, &task)?;
    let mut manual = Vec::new();
    let mut automatic_questions = Vec::new();
    let mut auto_answers = serde_json::Map::new();
    for question in &questions {
        let value = serde_json::to_value(question)?;
        if crate::autonomy::automatic(ratio, &value)? {
            auto_answers.insert(
                question.id.clone(),
                json!(crate::autonomy::recommendation(&value)?),
            );
            automatic_questions.push(value);
        } else {
            manual.push(value);
        }
    }
    let text = questions
        .iter()
        .map(|q| q.question.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let all_automatic = manual.is_empty();
    let mut item = json!({"id":uuid::Uuid::new_v4().to_string(),"createdAt":now,"questions":manual,"automaticQuestions":automatic_questions,"autoAnswers":auto_answers,"askRatio":ratio});
    if all_automatic {
        item["answers"] = json!(auto_answers);
        item["answeredAt"] = json!(now);
    }
    if !auto_answers.is_empty() {
        task["prompt"] = json!(format!(
            "{}\n\n按自动决策设置采用（用户后续明确要求优先）：\n{}",
            task["prompt"].as_str().context("任务目标无效")?,
            serde_json::to_string(&item["automaticQuestions"])?
        ));
    }
    if task["clarifications"].is_null() {
        task["clarifications"] = json!([]);
    }
    task["clarifications"]
        .as_array_mut()
        .context("任务问题记录格式无效")?
        .push(item);
    task["status"] = json!(if all_automatic {
        "running"
    } else {
        "awaitingInput"
    });
    task["updatedAt"] = json!(now);
    persist(
        store,
        &task,
        if all_automatic {
            "decision"
        } else {
            "question"
        },
        &format!("{text}\n自动采用：{}", json!(auto_answers)),
        &now,
    )?;
    Ok(task)
}

/// Caller must stop and await the old executor, then hold its task/project lock.
/// Queueing is durable; the scheduler, not this function, starts a new process.
pub fn answer(store: &mut Store, id: &str, question_id: &str, input: Value) -> Result<Value> {
    answer_with_auto(store, id, question_id, input, &[])
}

pub fn validate_automatic(
    item: &Value,
    answers: &BTreeMap<String, String>,
    automatic: &[String],
    ratio: u64,
) -> Result<()> {
    let mut seen = HashSet::new();
    let questions = validate_questions(item)?;
    for id in automatic {
        anyhow::ensure!(seen.insert(id), "自动回答标识重复");
        let question = questions
            .iter()
            .find(|q| &q.id == id)
            .context("自动回答标识无效")?;
        let value = serde_json::to_value(question)?;
        anyhow::ensure!(
            crate::autonomy::automatic(ratio, &value)?
                && answers.get(id).map(String::as_str)
                    == Some(crate::autonomy::recommendation(&value)?),
            "自动回答与当前策略或推荐不匹配"
        );
    }
    Ok(())
}

pub fn answer_with_auto(
    store: &mut Store,
    id: &str,
    question_id: &str,
    input: Value,
    automatic: &[String],
) -> Result<Value> {
    let mut task: Value = store.get("task", id)?.context("任务不存在")?;
    let ratio = crate::autonomy::effective(store, &task)?;
    if task["id"] != id {
        bail!("任务标识不匹配");
    }
    if task["status"] != "awaitingInput" {
        bail!("任务当前没有等待回答");
    }
    let project_id = task["projectId"].as_str().context("项目标识无效")?;
    if store.get::<Value>("project", project_id)?.is_none() {
        bail!("项目不存在");
    }
    let item = task["clarifications"]
        .as_array_mut()
        .and_then(|items| items.iter_mut().find(|item| item["id"] == question_id))
        .context("问题不存在或已过期")?;
    let answers = validate_answers(item, input)?;
    validate_automatic(item, &answers, automatic, ratio)?;
    let questions = validate_questions(item)?;
    if item["autoAnswers"].is_null() {
        item["autoAnswers"] = json!({});
    }
    if item["automaticQuestions"].is_null() {
        item["automaticQuestions"] = json!([]);
    }
    for id in automatic {
        item["autoAnswers"][id] = json!(answers[id]);
        item["automaticQuestions"]
            .as_array_mut()
            .context("自动决策记录无效")?
            .push(serde_json::to_value(
                questions.iter().find(|q| &q.id == id).unwrap(),
            )?);
    }
    item["answerSources"] = json!(answers
        .keys()
        .map(|id| (
            id.clone(),
            if automatic.contains(id) {
                "automatic"
            } else {
                "user"
            }
        ))
        .collect::<BTreeMap<_, _>>());
    if !automatic.is_empty() {
        item["appliedAskRatio"] = json!(ratio);
    }
    let text = validate_questions(item)?
        .iter()
        .map(|q| {
            format!(
                "{}\n{}：{}",
                q.question,
                if automatic.contains(&q.id) {
                    "按当前设置自动采用"
                } else {
                    "用户回答"
                },
                answers[&q.id]
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    item["answers"] = json!(answers);
    item["answeredAt"] = json!(now);
    task["prompt"] = json!(format!(
        "{}\n\n用户已确认的补充（先检查工作副本，继续未完成目标）：\n{text}",
        task["prompt"].as_str().context("任务目标无效")?
    ));
    task["status"] = json!("queued");
    task["updatedAt"] = json!(now);
    task.as_object_mut()
        .context("任务格式无效")?
        .remove("error");
    persist(
        store,
        &task,
        if automatic.is_empty() {
            "user"
        } else {
            "decision"
        },
        &text,
        &now,
    )?;
    Ok(task)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn questions_and_answers_enforce_boundaries() -> Result<()> {
        for value in [
            json!({"questions":[]}),
            json!({"questions":[{"id":"x","question":"password","isSecret":true}]}),
            json!({"questions":[{"id":"x","question":"a"},{"id":"x","question":"b"}]}),
        ] {
            assert!(validate_questions(&value).is_err());
        }
        let item = json!({"questions":[{"id":"tone","question":" 氛围？ ","options":null}]});
        assert_eq!(validate_questions(&item)?[0].question, "氛围？");
        assert_eq!(
            validate_answers(&item, json!({"tone":"  温暖  "}))?["tone"],
            "温暖"
        );
        for input in [
            json!({}),
            json!({"tone":""}),
            json!({"other":"x"}),
            json!({"tone":"x","extra":"x"}),
        ] {
            assert!(validate_answers(&item, input).is_err());
        }
        let mut answered = item;
        answered["answers"] = json!({"tone":"x"});
        assert!(validate_answers(&answered, json!({"tone":"y"})).is_err());
        Ok(())
    }
    #[test]
    fn parked_question_survives_restart_and_answer_is_not_replayed() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let mut store = Store::open(temp.path())?;
        store.put("project", "p", &json!({"id":"p"}))?;
        store.put("task", "t", &json!({"id":"t","projectId":"p","status":"running","threadId":"thread","turnId":"turn","prompt":"制作游戏","futureField":42}))?;
        let request = json!({"threadId":"thread","turnId":"turn","tool":"beaver_ask_user","arguments":{"questions":[{"id":"tone","question":"氛围？"}]}});
        let mut stale = request.clone();
        stale["turnId"] = json!("old");
        assert!(park(&mut store, "t", "item/tool/call", &stale).is_err());
        assert!(store.events("t")?.is_empty());
        let parked = park(&mut store, "t", "item/tool/call", &request)?;
        let question_id = parked["clarifications"][0]["id"]
            .as_str()
            .unwrap()
            .to_string();
        drop(store);
        let mut store = Store::open(temp.path())?;
        store.recover_tasks()?;
        assert_eq!(
            store.get::<Value>("task", "t")?.unwrap()["status"],
            "awaitingInput"
        );
        assert!(answer(&mut store, "t", &question_id, json!({})).is_err());
        assert_eq!(store.events("t")?.len(), 1);
        let resumed = answer(&mut store, "t", &question_id, json!({"tone":"温暖"}))?;
        assert_eq!(resumed["status"], "queued");
        assert_eq!(resumed["futureField"], 42);
        assert!(resumed["prompt"]
            .as_str()
            .unwrap()
            .contains("用户回答：温暖"));
        assert_eq!(store.events("t")?.len(), 2);
        assert!(answer(&mut store, "t", &question_id, json!({"tone":"改变"})).is_err());
        assert_eq!(store.events("t")?.len(), 2);
        Ok(())
    }

    #[test]
    fn mixed_policy_preserves_manual_choices_and_records_automatic_sources() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let mut store = Store::open(temp.path())?;
        store.put("project", "p", &json!({"id":"p"}))?;
        store.put("settings", "main", &json!({"askRatio":30}))?;
        store.put("task", "t", &json!({"id":"t","projectId":"p","status":"running","threadId":"thread","turnId":"turn","prompt":"Test"}))?;
        let question = |id: &str, importance: u64| json!({"id":id,"question":"Choose?","options":[{"label":"A","description":"First"},{"label":"B","description":"Second"}],"recommended":"A","reason":"Fits scope","importance":importance});
        let request = json!({"threadId":"thread","turnId":"turn","tool":"beaver_ask_user","arguments":{"questions":[question("minor",30),question("major",95)]}});
        let parked = park(&mut store, "t", "item/tool/call", &request)?;
        assert_eq!(parked["status"], "awaitingInput");
        assert_eq!(
            parked["clarifications"][0]["questions"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(parked["clarifications"][0]["autoAnswers"]["minor"], "A");
        let id = parked["clarifications"][0]["id"]
            .as_str()
            .unwrap()
            .to_owned();
        assert!(answer_with_auto(
            &mut store,
            "t",
            &id,
            json!({"major":"A"}),
            &["major".into()]
        )
        .is_err());
        assert_eq!(store.events("t")?.len(), 1);
        crate::autonomy::set(&mut store, "t", json!(0))?;
        assert_eq!(
            store.get::<Value>("task", "t")?.unwrap()["status"],
            "awaitingInput"
        );
        assert!(answer_with_auto(
            &mut store,
            "t",
            &id,
            json!({"major":"Custom"}),
            &["major".into()]
        )
        .is_err());
        let answered = answer_with_auto(&mut store, "t", &id, json!({"major":"Custom"}), &[])?;
        assert_eq!(answered["clarifications"][0]["answers"]["major"], "Custom");
        assert_eq!(
            answered["clarifications"][0]["answerSources"]["major"],
            "user"
        );
        assert_eq!(answered["clarifications"][0]["autoAnswers"]["minor"], "A");
        drop(store);
        let store = Store::open(temp.path())?;
        assert_eq!(
            store.get::<Value>("task", "t")?.unwrap()["clarifications"],
            answered["clarifications"]
        );
        Ok(())
    }

    #[test]
    fn fully_automatic_questions_continue_running() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let mut store = Store::open(temp.path())?;
        store.put("task", "t", &json!({"id":"t","status":"running","threadId":"thread","turnId":"turn","prompt":"Test","askRatio":0}))?;
        let request = json!({"threadId":"thread","turnId":"turn","tool":"beaver_ask_user","arguments":{"questions":[{"id":"q","question":"Choose?","options":[{"label":"A"},{"label":"B"}],"recommended":"B","reason":"Best fit","importance":100}]}});
        let task = park(&mut store, "t", "item/tool/call", &request)?;
        assert_eq!(task["status"], "running");
        assert_eq!(task["clarifications"][0]["answers"]["q"], "B");
        assert_eq!(task["clarifications"][0]["questions"], json!([]));
        assert_eq!(store.events("t")?[0].kind, "decision");
        Ok(())
    }
}
