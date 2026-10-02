//! Validate full stored requests and consume each manual decision exactly once in round order.
use crate::{
    object_task_planning_types::{AnswerRequest, Record, SessionRequest, StartRequest, Status},
    object_task_types::{valid_id, AssumptionSource},
    project_derivation_plan_validation::Plans,
    project_derivation_planning_records,
    project_migration_ownership::Entity,
};
use anyhow::{bail, ensure, Context, Result};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn validate(
    entities: &[Entity],
    plans: &Plans,
    sessions: &BTreeMap<String, Record>,
) -> Result<()> {
    let mut starts = BTreeSet::new();
    let mut adopts = BTreeSet::new();
    let mut cancels = BTreeSet::new();
    let mut revisions = BTreeSet::new();
    let mut answers: BTreeMap<String, Vec<AnswerRequest>> = BTreeMap::new();
    for entity in entities
        .iter()
        .filter(|e| e.kind == "object_task_planning_receipt")
    {
        let receipt = project_derivation_planning_records::receipt(
            entity
                .value
                .as_ref()
                .context("invalid planning receipt JSON")?,
        )?;
        let session = &sessions
            .get(&receipt.session_id)
            .context("DERIVATION_PLANNING_SESSION_MISSING")?
            .session;
        ensure!(
            receipt.project_id == plans.project && valid_id(&entity.id),
            "DERIVATION_PLANNING_RECEIPT_IDENTITY"
        );
        let (project, request, owner, revision) = match receipt.operation.as_str() {
            "start" => {
                let input: StartRequest = serde_json::from_value(receipt.request)?;
                ensure!(
                    input == session.input && starts.insert(session.id.clone()),
                    "DERIVATION_PLANNING_START_RECEIPT"
                );
                (
                    input.project_id,
                    input.request_id.clone(),
                    input.request_id,
                    0,
                )
            }
            "answer" => {
                let input: AnswerRequest = serde_json::from_value(receipt.request)?;
                answers
                    .entry(session.id.clone())
                    .or_default()
                    .push(input.clone());
                (
                    input.project_id,
                    input.request_id,
                    input.session_id,
                    input.expected_revision,
                )
            }
            "adopt" | "cancel" => {
                let input: SessionRequest = serde_json::from_value(receipt.request)?;
                ensure!(
                    if receipt.operation == "adopt" {
                        session.status == Status::Adopted && adopts.insert(session.id.clone())
                    } else {
                        cancels.insert(session.id.clone());
                        session.status == Status::Cancelled
                    },
                    "DERIVATION_PLANNING_TERMINAL_RECEIPT"
                );
                (
                    input.project_id,
                    input.request_id,
                    input.session_id,
                    input.expected_revision,
                )
            }
            _ => bail!("DERIVATION_PLANNING_UNKNOWN_OPERATION"),
        };
        ensure!(
            project == plans.project
                && request == entity.id
                && owner == session.id
                && revision < session.revision
                && (receipt.operation == "start" || revision > 0)
                && revisions.insert((owner, revision)),
            "DERIVATION_PLANNING_RECEIPT_IDENTITY"
        );
    }
    for session in sessions.values().map(|r| &r.session) {
        ensure!(
            starts.contains(&session.id),
            "DERIVATION_PLANNING_START_MISSING"
        );
        ensure!(
            session.status != Status::Adopted || adopts.contains(&session.id),
            "DERIVATION_PLANNING_ADOPT_MISSING"
        );
        ensure!(
            session.status != Status::Cancelled || cancels.contains(&session.id),
            "DERIVATION_PLANNING_CANCEL_MISSING"
        );
        let mut receipts = answers.remove(&session.id).unwrap_or_default();
        receipts.sort_by_key(|r| r.expected_revision);
        ensure!(
            session.round as usize == receipts.len() + 1,
            "DERIVATION_PLANNING_ANSWER_ROUNDS"
        );
        let manual: Vec<_> = session
            .decisions
            .iter()
            .filter(|d| d.source == AssumptionSource::User)
            .collect();
        let mut consumed = 0;
        let mut previous = 0;
        for receipt in receipts {
            ensure!(
                (1..=3).contains(&receipt.answers.len())
                    && receipt.expected_revision >= previous + 2,
                "DERIVATION_PLANNING_ANSWER_RECEIPT"
            );
            let group = manual
                .get(consumed..consumed + receipt.answers.len())
                .context("DERIVATION_PLANNING_ANSWER_RECEIPT")?;
            let questions: Vec<_> = group.iter().map(|d| d.question.clone()).collect();
            let normalized = crate::clarifications::validate_answers(
                &json!({"questions":questions}),
                serde_json::to_value(&receipt.answers)?,
            )
            .context("DERIVATION_PLANNING_ANSWER_RECEIPT")?;
            ensure!(
                group.iter().all(|d| d.question["id"]
                    .as_str()
                    .is_some_and(|id| normalized.get(id) == Some(&d.answer))),
                "DERIVATION_PLANNING_ANSWER_RECEIPT"
            );
            consumed += group.len();
            previous = receipt.expected_revision;
        }
        ensure!(
            consumed == manual.len(),
            "DERIVATION_PLANNING_ANSWER_DECISIONS"
        );
    }
    Ok(())
}
