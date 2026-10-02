use super::*;

#[test]
fn project_derivation_planning_explicit_answer_uses_fresh_round_and_rejects_old_callbacks(
) -> Result<()> {
    let f = Fixture::new()?;
    let preparation = f.base.temp.path().join("answer-copy");
    copy::prepare(f.base.request(), &preparation)?;
    // Only assembly activation makes the copy an ordinary project.
    let target = f.base.temp.path().join("answer-target");
    assembly::create(&preparation, &target)?;
    assembly::activate(&preparation, &target)?;
    let runtime = ProjectStore::open(&target.join("project"), "derived")?.into_runtime();
    let waiting = current(&runtime, "waiting")?;
    let old = planning::RoundKey::from(&waiting);
    let input = planning::AnswerRequest {
        project_id: "derived".into(),
        session_id: waiting.id,
        request_id: "explicit-answer".into(),
        expected_revision: waiting.revision,
        answers: [("next".into(), "Answer original start-adopted".into())].into(),
    };
    let transition = planning::answer(&runtime, &input)?;
    assert!(transition.launch);
    assert_eq!(transition.session.round, 2);
    assert_ne!(transition.session.round_id, old.round_id);
    assert!(transition.session.thread_id.is_none());
    assert!(transition.session.turn_id.is_none());
    assert_eq!(
        transition.session.decisions[0].answer,
        "Answer original start-adopted"
    );
    assert!(!planning::answer(&runtime, &input)?.launch);
    assert!(round::propose(&runtime, &old, fixture::proposal("late-task")).is_err());
    round::finish(&runtime, &old, planning::Status::Failed, "late failure")?;
    assert_eq!(current(&runtime, "waiting")?, transition.session);
    round::propose(
        &runtime,
        &planning::RoundKey::from(&transition.session),
        fixture::proposal("answered-task"),
    )?;
    let proposed = current(&runtime, "waiting")?;
    planning::adopt(
        &runtime,
        &crate::object_task_planning_fixture::command(&proposed, "adopt-answer"),
    )?;
    fixture::commit(&runtime, "waiting", "commit-answer")?;
    assert!(object_tasks::queue(&runtime, "derived")?.is_empty());
    assert!(object_tasks::claim_next(&runtime, "derived", "answer-test")?.is_none());
    Ok(())
}

#[test]
fn project_derivation_planning_keeps_multiple_terminal_sessions_under_one_head() -> Result<()> {
    let f = Fixture::new()?;
    let runtime = ProjectStore::open(&f.base.source, "original")?.into_runtime();
    let previous = current(&runtime, "failed")?;
    let mut input = previous.input.clone();
    input.request_id = "latest-failed".into();
    let next = planning::start(&runtime, &input)?.session;
    round::finish(
        &runtime,
        &planning::RoundKey::from(&next),
        planning::Status::Interrupted,
        "newest",
    )?;
    drop(runtime);
    let preparation = f.base.temp.path().join("head-history");
    copy::prepare(f.base.request(), &preparation)?;
    copy::inspect(&preparation)?;
    Ok(())
}
