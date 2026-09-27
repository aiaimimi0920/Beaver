//! Read-only Codex planning. Only explicit adoption writes the ordinary task draft.
use crate::{
    object_task_planning_store as repository,
    object_task_planning_types::{Head, Record},
    object_task_storage as storage,
    object_task_types::{valid_id, Draft, MAX_REVISION},
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Result};
use serde_json::json;

pub use crate::object_task_planning_types::{
    AnswerRequest, Decision, RoundKey, Session, SessionRequest, StartRequest, Status, Transition,
};

pub fn get(runtime: &ProjectRuntime, project_id: &str, draft_id: &str) -> Result<Option<Session>> {
    repository::transact(runtime, project_id, |db| {
        Ok(
            repository::current(db, project_id, draft_id)?.map(|mut record| {
                if matches!(
                    record.session.status,
                    Status::Running | Status::AwaitingInput | Status::Proposed
                ) {
                    record.session.conflict = repository::scope(db, &record.session.input)
                        .err()
                        .map(|error| error.to_string());
                }
                record.session
            }),
        )
    })
}

pub fn start(runtime: &ProjectRuntime, input: &StartRequest) -> Result<Transition> {
    ensure!(
        valid_id(&input.draft_id) && valid_id(&input.request_id),
        "INVALID_PLANNING_ID"
    );
    ensure!(
        !input.goal.trim().is_empty()
            && input.goal.len() <= 20_000
            && input.acceptance.len() <= 10_000,
        "INVALID_PLANNING_GOAL"
    );
    ensure!(
        crate::autonomy::valid(&json!(input.ask_ratio)),
        "INVALID_PLANNING_ASK_RATIO"
    );
    repository::transact(runtime, &input.project_id, |db| {
        let value = serde_json::to_value(input)?;
        if let Some(session) =
            repository::repeat(db, &input.project_id, &input.request_id, "start", &value)?
        {
            return Ok(Transition {
                session,
                launch: false,
            });
        }
        if let Some(previous) = repository::current(db, &input.project_id, &input.draft_id)? {
            ensure!(
                !matches!(
                    previous.session.status,
                    Status::Running | Status::AwaitingInput | Status::Proposed
                ),
                "OBJECT_PLANNING_ACTIVE: cancel or adopt the previous proposal first"
            );
        }
        let baseline = repository::scope(db, input)?;
        crate::object_task_validation::validate_plan(db, &input.project_id, &baseline, true)?;
        let mut record = Record {
            project_id: input.project_id.clone(),
            baseline,
            context: repository::context(db, &input.project_id)?,
            session: Session {
                id: input.request_id.clone(),
                project_id: input.project_id.clone(),
                revision: 0,
                status: Status::Running,
                input: input.clone(),
                round_id: uuid::Uuid::new_v4().to_string(),
                round: 1,
                thread_id: None,
                turn_id: None,
                questions: vec![],
                decisions: vec![],
                proposal: None,
                conflict: None,
                error: None,
                adopted_draft: None,
            },
        };
        repository::save(db, &mut record)?;
        storage::replace(
            db,
            repository::HEAD_KIND,
            &input.draft_id,
            &Head {
                project_id: input.project_id.clone(),
                session_id: record.session.id.clone(),
            },
        )?;
        repository::receipt(db, &record.session, &input.request_id, "start", value)?;
        Ok(Transition {
            session: record.session,
            launch: true,
        })
    })
}

pub fn answer(runtime: &ProjectRuntime, input: &AnswerRequest) -> Result<Transition> {
    repository::transact(runtime, &input.project_id, |db| {
        let value = serde_json::to_value(input)?;
        if let Some(session) =
            repository::repeat(db, &input.project_id, &input.request_id, "answer", &value)?
        {
            return Ok(Transition {
                session,
                launch: false,
            });
        }
        let mut record = repository::read(db, &input.project_id, &input.session_id)?;
        ensure!(
            record.session.revision == input.expected_revision,
            "OBJECT_PLANNING_REVISION_CONFLICT"
        );
        ensure!(
            record.session.status == Status::AwaitingInput,
            "OBJECT_PLANNING_NOT_WAITING"
        );
        ensure!(record.session.round < 12, "OBJECT_PLANNING_ROUND_LIMIT");
        repository::scope(db, &record.session.input)?;
        let answers = crate::clarifications::validate_answers(
            &json!({"questions":record.session.questions}),
            serde_json::to_value(&input.answers)?,
        )?;
        for question in record.session.questions.drain(..) {
            let answer = answers[question["id"].as_str().expect("validated question")].clone();
            record.session.decisions.push(Decision {
                id: uuid::Uuid::new_v4().to_string(),
                question,
                answer,
                source: crate::object_task_types::AssumptionSource::User,
            });
        }
        record.session.status = Status::Running;
        record.session.round += 1;
        record.session.round_id = uuid::Uuid::new_v4().to_string();
        record.session.thread_id = None;
        record.session.turn_id = None;
        record.session.error = None;
        repository::save(db, &mut record)?;
        repository::receipt(db, &record.session, &input.request_id, "answer", value)?;
        Ok(Transition {
            session: record.session,
            launch: true,
        })
    })
}

pub fn cancel(runtime: &ProjectRuntime, input: &SessionRequest) -> Result<Session> {
    repository::transact(runtime, &input.project_id, |db| {
        let value = serde_json::to_value(input)?;
        if let Some(session) =
            repository::repeat(db, &input.project_id, &input.request_id, "cancel", &value)?
        {
            return Ok(session);
        }
        let mut record = repository::read(db, &input.project_id, &input.session_id)?;
        ensure!(
            record.session.revision == input.expected_revision,
            "OBJECT_PLANNING_REVISION_CONFLICT"
        );
        ensure!(
            record.session.status != Status::Adopted,
            "OBJECT_PLANNING_ALREADY_ADOPTED"
        );
        record.session.status = Status::Cancelled;
        repository::save(db, &mut record)?;
        repository::receipt(db, &record.session, &input.request_id, "cancel", value)?;
        Ok(record.session)
    })
}

pub fn adopt(runtime: &ProjectRuntime, input: &SessionRequest) -> Result<Session> {
    repository::transact(runtime, &input.project_id, |db| {
        let value = serde_json::to_value(input)?;
        if let Some(session) =
            repository::repeat(db, &input.project_id, &input.request_id, "adopt", &value)?
        {
            return Ok(session);
        }
        let mut record = repository::read(db, &input.project_id, &input.session_id)?;
        ensure!(
            record.session.revision == input.expected_revision,
            "OBJECT_PLANNING_REVISION_CONFLICT"
        );
        ensure!(
            record.session.status == Status::Proposed,
            "OBJECT_PLANNING_NO_PROPOSAL"
        );
        repository::scope(db, &record.session.input)?;
        let plan = repository::merged(
            &record,
            record.session.proposal.as_ref().expect("proposed plan"),
        );
        crate::object_task_validation::validate_plan(db, &input.project_id, &plan, false)?;
        ensure!(
            record.session.input.expected_draft_revision < MAX_REVISION,
            "OBJECT_TASK_REVISION_EXHAUSTED"
        );
        let draft = Draft {
            id: record.session.input.draft_id.clone(),
            project_id: input.project_id.clone(),
            revision: record.session.input.expected_draft_revision + 1,
            plan_revision: record.session.input.expected_plan_revision,
            plan,
            committed_request_id: None,
        };
        storage::replace(db, storage::DRAFT_KIND, &draft.id, &draft)?;
        record.session.status = Status::Adopted;
        record.session.conflict = None;
        record.session.adopted_draft = Some(draft);
        repository::save(db, &mut record)?;
        repository::receipt(db, &record.session, &input.request_id, "adopt", value)?;
        Ok(record.session)
    })
}

/// Recovery only marks interrupted work; it never launches Codex or spends again.
pub fn recover(store: &mut crate::store::Store) -> Result<()> {
    store.transaction(|db| {
        for mut record in storage::read_all::<Record>(db, repository::KIND)? {
            if record.session.status == Status::Running {
                record.session.status = Status::Interrupted;
                record.session.error = Some(
                    "Planning interrupted by host shutdown; start a new request to retry.".into(),
                );
                repository::save(db, &mut record)?;
            }
        }
        Ok(())
    })
}

#[cfg(test)]
#[path = "object_task_planning_tests.rs"]
mod tests;
