use super::*;
use crate::{
    object_catalog_test_fixture::Fixture,
    object_task_planning_fixture::{command, input, proposal, question, session},
    object_task_planning_round as round,
    object_tasks::{self, AssumptionSource, CommitRequest, PlanProposal, SaveDraftRequest},
};
use anyhow::Result;
use serde_json::json;

#[test]
fn explicit_adoption_appends_then_ordinary_commit_creates_one_medium_run() -> Result<()> {
    let fixture = Fixture::new()?;
    let mut input = input();
    let baseline: PlanProposal = serde_json::from_value(json!({"tasks":[{
        "id":"manual","granularity":"coarse","title":"Manual","prompt":"Keep this","acceptance":""
    }]}))?;
    object_tasks::save_draft(
        &fixture.runtime,
        &SaveDraftRequest {
            project_id: input.project_id.clone(),
            draft_id: input.draft_id.clone(),
            expected_revision: 0,
            expected_plan_revision: 0,
            plan: baseline.clone(),
        },
    )?;
    input.expected_draft_revision = 1;
    let started = start(&fixture.runtime, &input)?;
    assert!(started.launch);
    assert!(!start(&fixture.runtime, &input)?.launch);
    round::propose(
        &fixture.runtime,
        &RoundKey::from(&started.session),
        proposal(),
    )?;
    assert_eq!(fixture.count("object_task")?, 0);
    assert_eq!(
        object_tasks::get_draft(&fixture.runtime, "project-1", "main")?
            .unwrap()
            .plan,
        baseline
    );
    let request = command(&session(&fixture.runtime)?, "adopt");
    let adopted = adopt(&fixture.runtime, &request)?;
    assert_eq!(adopt(&fixture.runtime, &request)?, adopted);
    assert_eq!(fixture.count("object")?, 0);
    let draft = adopted.adopted_draft.unwrap();
    assert_eq!(draft.plan.tasks[0], baseline.tasks[0]);
    assert_eq!(draft.plan.assumptions[0].source, AssumptionSource::Codex);
    assert_eq!(
        draft.plan.assumptions[0].source_detail.as_deref(),
        Some("planning:planning:1")
    );
    let receipt = object_tasks::commit(
        &fixture.runtime,
        &CommitRequest {
            project_id: "project-1".into(),
            request_id: "commit".into(),
            draft_id: "main".into(),
            expected_draft_revision: draft.revision,
            expected_plan_revision: 0,
        },
    )?;
    assert_eq!(receipt.task_ids.len(), 2);
    assert_eq!(receipt.runs.len(), 1);
    assert_eq!(fixture.count("task")?, 0);
    assert!(session(&fixture.runtime)?.conflict.is_none());
    Ok(())
}

#[test]
fn mixed_questions_keep_sources_and_answer_retry_never_launches_twice() -> Result<()> {
    let fixture = Fixture::new()?;
    let mut input = input();
    input.ask_ratio = 30;
    let started = start(&fixture.runtime, &input)?;
    let old = RoundKey::from(&started.session);
    assert!(matches!(
        round::questions(
            &fixture.runtime,
            &old,
            &json!({"questions":[question("low", 10), question("high", 90)]})
        )?,
        round::Questions::Waiting
    ));
    let waiting = session(&fixture.runtime)?;
    assert_eq!(waiting.questions.len(), 1);
    assert_eq!(waiting.decisions[0].source, AssumptionSource::Automatic);
    let mut request = AnswerRequest {
        project_id: "project-1".into(),
        session_id: waiting.id,
        request_id: "answer".into(),
        expected_revision: waiting.revision,
        answers: Default::default(),
    };
    assert!(answer(&fixture.runtime, &request).is_err());
    request
        .answers
        .insert("high".into(), " Custom user answer ".into());
    let answered = answer(&fixture.runtime, &request)?;
    assert!(answered.launch);
    assert!(!answer(&fixture.runtime, &request)?.launch);
    assert_eq!(answered.session.decisions[1].answer, "Custom user answer");
    assert_ne!(answered.session.round_id, old.round_id);
    assert!(round::propose(&fixture.runtime, &old, proposal()).is_err());
    round::finish(&fixture.runtime, &old, Status::Failed, "late error")?;
    round::propose(
        &fixture.runtime,
        &RoundKey::from(&answered.session),
        proposal(),
    )?;
    let assumptions = session(&fixture.runtime)?.proposal.unwrap().assumptions;
    assert_eq!(
        assumptions
            .iter()
            .map(|a| a.source.clone())
            .collect::<Vec<_>>(),
        vec![
            AssumptionSource::Codex,
            AssumptionSource::Automatic,
            AssumptionSource::User
        ]
    );
    Ok(())
}

#[test]
fn concurrent_draft_edit_keeps_proposal_reviewable_but_rejects_adoption() -> Result<()> {
    let fixture = Fixture::new()?;
    let started = start(&fixture.runtime, &input())?;
    object_tasks::save_draft(
        &fixture.runtime,
        &SaveDraftRequest {
            project_id: "project-1".into(),
            draft_id: "main".into(),
            expected_revision: 0,
            expected_plan_revision: 0,
            plan: PlanProposal::default(),
        },
    )?;
    round::propose(
        &fixture.runtime,
        &RoundKey::from(&started.session),
        proposal(),
    )?;
    let proposed = session(&fixture.runtime)?;
    assert_eq!(proposed.status, Status::Proposed);
    assert!(proposed
        .conflict
        .unwrap()
        .contains("DRAFT_REVISION_CONFLICT"));
    assert!(adopt(
        &fixture.runtime,
        &command(&session(&fixture.runtime)?, "adopt")
    )
    .is_err());
    assert_eq!(fixture.count("object_task")?, 0);
    assert_eq!(
        object_tasks::get_draft(&fixture.runtime, "project-1", "main")?
            .unwrap()
            .revision,
        1
    );
    Ok(())
}

#[test]
fn cancellation_recovery_and_project_identity_reject_late_callbacks() -> Result<()> {
    let fixture = Fixture::new()?;
    let mut input = input();
    let started = start(&fixture.runtime, &input)?;
    let old = RoundKey::from(&started.session);
    recover(&mut fixture.runtime.store().lock().unwrap())?;
    assert_eq!(session(&fixture.runtime)?.status, Status::Interrupted);
    assert!(!start(&fixture.runtime, &input)?.launch);
    assert!(round::propose(&fixture.runtime, &old, proposal()).is_err());
    input.request_id = "new-request".into();
    let next = start(&fixture.runtime, &input)?;
    cancel(&fixture.runtime, &command(&next.session, "cancel"))?;
    assert!(round::questions(
        &fixture.runtime,
        &RoundKey::from(&next.session),
        &json!({"questions":[question("q", 100)]})
    )
    .is_err());
    input.project_id = "other-project".into();
    assert!(start(&fixture.runtime, &input).is_err());
    assert!(get(&fixture.runtime, "other-project", "main").is_err());
    Ok(())
}

#[test]
fn proposal_validation_and_receipt_failure_leave_draft_and_session_unchanged() -> Result<()> {
    let fixture = Fixture::new()?;
    let started = start(&fixture.runtime, &input())?;
    let key = RoundKey::from(&started.session);
    let mut invalid = proposal();
    invalid.tasks[0].depends_on.push("make-hero".into());
    assert!(round::propose(&fixture.runtime, &key, invalid).is_err());
    assert_eq!(session(&fixture.runtime)?.status, Status::Running);
    round::propose(&fixture.runtime, &key, proposal())?;
    let proposed = session(&fixture.runtime)?;
    fixture.runtime.store().lock().unwrap().transaction(|db| {
        db.execute_batch("CREATE TRIGGER reject_planning_receipt BEFORE INSERT ON entities WHEN NEW.kind = 'object_task_planning_receipt' BEGIN SELECT RAISE(ABORT, 'receipt rejected'); END;")?;
        Ok(())
    })?;
    assert!(adopt(&fixture.runtime, &command(&proposed, "adopt")).is_err());
    assert_eq!(session(&fixture.runtime)?, proposed);
    assert!(object_tasks::get_draft(&fixture.runtime, "project-1", "main")?.is_none());
    Ok(())
}
