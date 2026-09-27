//! Durable callbacks accept only the currently owned round, never a late model response.
use crate::{
    object_task_planning::{Decision, RoundKey, Status},
    object_task_planning_store as repository,
    object_task_types::{AssumptionSource, PlanAssumption, PlanProposal},
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Result};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub enum Questions {
    Automatic(BTreeMap<String, String>),
    Waiting,
}

pub fn context(runtime: &ProjectRuntime, key: &RoundKey) -> Result<Value> {
    repository::transact(runtime, &key.project_id, |db| {
        let record = repository::active(db, &key.project_id, &key.session_id, &key.round_id)?;
        Ok(json!({
            "goal":record.session.input.goal,"acceptance":record.session.input.acceptance,
            "askRatio":record.session.input.ask_ratio,"draft":record.baseline,
            "project":record.context,"decisions":record.session.decisions
        }))
    })
}

pub fn bind(
    runtime: &ProjectRuntime,
    key: &RoundKey,
    thread: &str,
    turn: Option<&str>,
) -> Result<()> {
    ensure!(
        !thread.is_empty() && turn.is_none_or(|id| !id.is_empty()),
        "INVALID_PLANNING_RPC_ID"
    );
    repository::transact(runtime, &key.project_id, |db| {
        let mut record = repository::active(db, &key.project_id, &key.session_id, &key.round_id)?;
        ensure!(
            record
                .session
                .thread_id
                .as_deref()
                .is_none_or(|id| id == thread),
            "OBJECT_PLANNING_THREAD_MISMATCH"
        );
        ensure!(
            record
                .session
                .turn_id
                .as_deref()
                .is_none_or(|id| Some(id) == turn),
            "OBJECT_PLANNING_TURN_MISMATCH"
        );
        record.session.thread_id = Some(thread.into());
        record.session.turn_id = turn.map(str::to_owned);
        repository::save(db, &mut record)
    })
}

pub fn questions(runtime: &ProjectRuntime, key: &RoundKey, input: &Value) -> Result<Questions> {
    let questions: Vec<Value> = crate::clarifications::validate_questions(input)?
        .into_iter()
        .map(serde_json::to_value)
        .collect::<std::result::Result<_, _>>()?;
    repository::transact(runtime, &key.project_id, |db| {
        let mut record = repository::active(db, &key.project_id, &key.session_id, &key.round_id)?;
        ensure!(
            record.session.decisions.len() + questions.len() <= 36,
            "OBJECT_PLANNING_QUESTION_LIMIT"
        );
        let mut answers = BTreeMap::new();
        for question in questions {
            if crate::autonomy::automatic(record.session.input.ask_ratio, &question)? {
                let answer = crate::autonomy::recommendation(&question)?.to_owned();
                answers.insert(question["id"].as_str().unwrap().to_owned(), answer.clone());
                record.session.decisions.push(Decision {
                    id: uuid::Uuid::new_v4().to_string(),
                    question,
                    answer,
                    source: AssumptionSource::Automatic,
                });
            } else {
                record.session.questions.push(question);
            }
        }
        let waiting = !record.session.questions.is_empty();
        if waiting {
            record.session.status = Status::AwaitingInput;
        }
        repository::save(db, &mut record)?;
        Ok(if waiting {
            Questions::Waiting
        } else {
            Questions::Automatic(answers)
        })
    })
}

pub fn propose(runtime: &ProjectRuntime, key: &RoundKey, mut proposal: PlanProposal) -> Result<()> {
    repository::transact(runtime, &key.project_id, |db| {
        let mut record = repository::active(db, &key.project_id, &key.session_id, &key.round_id)?;
        // The model may reference existing IDs, but may only define new entities.
        for object in &proposal.objects {
            ensure!(
                crate::object_catalog::read(db, &object.id)?.is_none(),
                "OBJECT_PLANNING_EXISTING_OBJECT"
            );
        }
        for task in &proposal.tasks {
            ensure!(
                crate::object_task_storage::read::<Value>(db, "object_task", &task.id)?.is_none(),
                "OBJECT_PLANNING_EXISTING_TASK"
            );
        }
        let state = crate::object_task_storage::plan_state(db, &key.project_id)?;
        for assumption in &mut proposal.assumptions {
            ensure!(
                !state
                    .assumptions
                    .iter()
                    .any(|item| item.id == assumption.id),
                "OBJECT_PLANNING_EXISTING_ASSUMPTION"
            );
            assumption.source = AssumptionSource::Codex;
            assumption.source_detail = Some(format!("planning:{}", key.session_id));
        }
        proposal
            .assumptions
            .extend(record.session.decisions.iter().map(|decision| {
                PlanAssumption {
                    id: format!("decision-{}", decision.id),
                    statement: format!(
                        "{}\n{}",
                        excerpt(
                            decision.question["question"].as_str().unwrap_or("Decision"),
                            900
                        ),
                        excerpt(&decision.answer, 1000)
                    ),
                    basis: excerpt(
                        decision.question["reason"]
                            .as_str()
                            .unwrap_or("Explicit planning answer"),
                        4000,
                    ),
                    source: decision.source.clone(),
                    source_detail: Some(format!(
                        "planning:{}/decision:{}",
                        key.session_id, decision.id
                    )),
                }
            }));
        crate::object_task_types::validate_plan(&proposal, false)?;
        let plan = repository::merged(&record, &proposal);
        crate::object_task_types::validate_plan(&plan, false)?;
        let conflict = repository::scope(db, &record.session.input).err();
        if conflict.is_none() {
            crate::object_task_validation::validate_plan(db, &key.project_id, &plan, false)?;
        }
        record.session.conflict = conflict.map(|error| error.to_string());
        record.session.proposal = Some(proposal);
        record.session.status = Status::Proposed;
        repository::save(db, &mut record)
    })
}

pub fn finish(runtime: &ProjectRuntime, key: &RoundKey, status: Status, error: &str) -> Result<()> {
    ensure!(
        matches!(status, Status::Failed | Status::Interrupted),
        "INVALID_PLANNING_TERMINAL_STATUS"
    );
    repository::transact(runtime, &key.project_id, |db| {
        let mut record = repository::read(db, &key.project_id, &key.session_id)?;
        if record.session.status != Status::Running || record.session.round_id != key.round_id {
            return Ok(());
        }
        record.session.status = status;
        record.session.error = Some(excerpt(error, 4000));
        repository::save(db, &mut record)
    })
}

fn excerpt(value: &str, max: usize) -> String {
    if value.len() <= max {
        return value.into();
    }
    let mut end = max - 3;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &value[..end])
}
