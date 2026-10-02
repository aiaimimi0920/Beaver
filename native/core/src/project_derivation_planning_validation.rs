//! Quiescent planning sessions, complete consumer heads and immutable request provenance.
use crate::{
    object_task_planning_types::{Record, Status},
    object_task_types::{valid_id, MAX_REVISION},
    project_derivation_plan_validation::Plans,
    project_derivation_planning_context::PlanningContext,
    project_derivation_planning_records as records,
    project_migration_ownership::Entity,
};
use anyhow::{ensure, Context, Result};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn validate(entities: &[Entity], plans: &Plans) -> Result<()> {
    let mut sessions = BTreeMap::new();
    for entity in entities.iter().filter(|e| e.kind == "object_task_planning") {
        let record = records::record(entity.value.as_ref().context("invalid planning JSON")?)?;
        ensure!(
            record.session.id == entity.id,
            "DERIVATION_PLANNING_IDENTITY"
        );
        validate_record(&record, plans)?;
        sessions.insert(entity.id.clone(), record);
    }
    let mut heads = BTreeMap::new();
    for entity in entities
        .iter()
        .filter(|e| e.kind == "object_task_planning_head")
    {
        let head = records::head(
            entity
                .value
                .as_ref()
                .context("invalid planning head JSON")?,
        )?;
        let record = sessions
            .get(&head.session_id)
            .context("DERIVATION_PLANNING_SESSION_MISSING")?;
        ensure!(
            valid_id(&entity.id)
                && head.project_id == plans.project
                && record.session.input.draft_id == entity.id,
            "DERIVATION_PLANNING_HEAD_IDENTITY"
        );
        heads.insert(entity.id.as_str(), head.session_id);
    }
    for record in sessions.values() {
        let head = heads
            .get(record.session.input.draft_id.as_str())
            .context("DERIVATION_PLANNING_HEAD_MISSING")?;
        ensure!(
            !matches!(
                record.session.status,
                Status::AwaitingInput | Status::Proposed
            ) || head == &record.session.id,
            "DERIVATION_PLANNING_ACTIVE_HEAD"
        );
    }
    crate::project_derivation_planning_receipts::validate(entities, plans, &sessions)?;
    crate::project_derivation_planning_provenance::validate(plans, &sessions)
}

fn validate_record(record: &Record, plans: &Plans) -> Result<()> {
    let session = &record.session;
    let input = &session.input;
    ensure!(
        record.project_id == plans.project
            && session.project_id == plans.project
            && input.project_id == plans.project
            && input.request_id == session.id
            && valid_id(&session.id)
            && valid_id(&input.draft_id)
            && valid_id(&session.round_id),
        "DERIVATION_PLANNING_IDENTITY"
    );
    ensure!(
        session.status != Status::Running,
        "DERIVATION_PLANNING_RUNNING_UNSUPPORTED"
    );
    ensure!(
        session.revision > 0
            && session.revision <= MAX_REVISION
            && (1..=12).contains(&session.round)
            && input.expected_draft_revision <= MAX_REVISION
            && input.expected_plan_revision <= plans.state.revision,
        "DERIVATION_PLANNING_REVISION"
    );
    ensure!(
        !input.goal.trim().is_empty()
            && input.goal.len() <= 20_000
            && input.acceptance.len() <= 10_000
            && crate::autonomy::valid(&json!(input.ask_ratio)),
        "DERIVATION_PLANNING_INPUT"
    );
    ensure!(
        session.thread_id.as_ref().is_none_or(|id| !id.is_empty())
            && session
                .turn_id
                .as_ref()
                .is_none_or(|id| !id.is_empty() && session.thread_id.is_some()),
        "DERIVATION_PLANNING_RPC_IDENTITY"
    );
    ensure!(
        session.questions.len() + session.decisions.len() <= 36,
        "DERIVATION_PLANNING_QUESTION_LIMIT"
    );
    if !session.questions.is_empty() {
        crate::clarifications::validate_questions(&json!({"questions":session.questions}))?;
    }
    let mut decisions = BTreeSet::new();
    for decision in &session.decisions {
        ensure!(
            valid_id(&decision.id) && decisions.insert(&decision.id),
            "DERIVATION_PLANNING_DECISION_IDENTITY"
        );
        let question = json!({"questions":[decision.question]});
        let question_id = decision.question["id"]
            .as_str()
            .context("invalid decision question")?;
        crate::clarifications::validate_answers(&question, json!({question_id:decision.answer}))?;
        ensure!(
            matches!(
                decision.source,
                crate::object_task_types::AssumptionSource::User
                    | crate::object_task_types::AssumptionSource::Automatic
            ),
            "DERIVATION_PLANNING_DECISION_SOURCE"
        );
    }
    ensure!(
        match session.status {
            Status::AwaitingInput => !session.questions.is_empty() && session.proposal.is_none(),
            Status::Proposed | Status::Adopted =>
                session.questions.is_empty() && session.proposal.is_some(),
            Status::Failed | Status::Interrupted =>
                session.questions.is_empty() && session.proposal.is_none(),
            Status::Cancelled => true,
            Status::Running => false,
        },
        "DERIVATION_PLANNING_STATUS"
    );
    plans.proposal(&record.baseline)?;
    if let Some(proposal) = &session.proposal {
        let merged = crate::object_task_planning_store::merged(record, proposal);
        plans.proposal(&merged)?;
        crate::object_task_types::validate_plan(&merged, false)?;
    }
    let context: PlanningContext = serde_json::from_value(record.context.clone())?;
    context.validate(plans, input.expected_plan_revision)?;
    ensure!(
        session.adopted_draft.is_some() == (session.status == Status::Adopted),
        "DERIVATION_PLANNING_ADOPTION"
    );
    if let Some(draft) = &session.adopted_draft {
        let current = plans
            .drafts
            .get(&input.draft_id)
            .context("DERIVATION_PLANNING_DRAFT_MISSING")?;
        ensure!(
            draft.project_id == plans.project
                && draft.id == input.draft_id
                && draft.revision == input.expected_draft_revision + 1
                && draft.revision <= current.revision
                && draft.plan_revision == input.expected_plan_revision
                && draft.committed_request_id.is_none()
                && draft.plan
                    == crate::object_task_planning_store::merged(
                        record,
                        session.proposal.as_ref().unwrap()
                    ),
            "DERIVATION_PLANNING_ADOPTION"
        );
        if draft.revision == current.revision {
            ensure!(
                draft == current,
                "DERIVATION_PLANNING_ADOPTED_DRAFT_MISMATCH"
            );
        }
    }
    Ok(())
}
