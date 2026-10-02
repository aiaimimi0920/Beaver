use super::*;
use crate::object_task_planning_fixture::{command, question};

fn repeated_answers(f: &Fixture) -> Result<()> {
    let runtime = ProjectStore::open(&f.base.source, "original")?.into_runtime();
    for index in 0..2 {
        let session = current(&runtime, "waiting")?;
        let answered = planning::answer(
            &runtime,
            &planning::AnswerRequest {
                project_id: "original".into(),
                session_id: session.id,
                request_id: format!("repeated-answer-{index}"),
                expected_revision: session.revision,
                answers: [("next".into(), "same answer".into())].into(),
            },
        )?
        .session;
        if index == 0 {
            round::questions(
                &runtime,
                &planning::RoundKey::from(&answered),
                &json!({"questions":[question("next",90)]}),
            )?;
        } else {
            round::finish(
                &runtime,
                &planning::RoundKey::from(&answered),
                planning::Status::Failed,
                "controlled",
            )?;
        }
    }
    Ok(())
}

#[test]
fn project_derivation_planning_repeated_question_ids_across_rounds_are_valid() -> Result<()> {
    let f = Fixture::new()?;
    repeated_answers(&f)?;
    let preparation = f.base.temp.path().join("repeated-questions");
    copy::prepare(f.base.request(), &preparation)?;
    copy::inspect(&preparation)?;
    Ok(())
}

#[test]
fn project_derivation_planning_answer_receipts_cannot_borrow_automatic_or_consumed_decisions(
) -> Result<()> {
    for case in [
        "automatic",
        "duplicate-revision",
        "duplicate-consumption",
        "missing",
    ] {
        let f = Fixture::new()?;
        let storage = ProjectStore::open(&f.base.source, "original")?;
        let mut receipt: Value = storage
            .store()
            .get("object_task_planning_receipt", "answer-adopted")?
            .unwrap();
        let expected = match case {
            "automatic" => {
                receipt["request"]["answers"] = json!({"auto":"A"});
                storage
                    .store()
                    .put("object_task_planning_receipt", "answer-adopted", &receipt)?;
                "ANSWER_RECEIPT"
            }
            "missing" => {
                storage
                    .store()
                    .remove("object_task_planning_receipt", "answer-adopted")?;
                "ANSWER_ROUNDS"
            }
            _ => {
                receipt["request"]["requestId"] = json!("borrowed-answer");
                if case == "duplicate-consumption" {
                    receipt["request"]["expectedRevision"] = json!(5);
                    let mut record: Value = storage
                        .store()
                        .get("object_task_planning", "start-adopted")?
                        .unwrap();
                    record["session"]["round"] = json!(3);
                    storage
                        .store()
                        .put("object_task_planning", "start-adopted", &record)?;
                }
                storage
                    .store()
                    .put("object_task_planning_receipt", "borrowed-answer", &receipt)?;
                if case == "duplicate-revision" {
                    "RECEIPT_IDENTITY"
                } else {
                    "ANSWER_RECEIPT"
                }
            }
        };
        drop(storage);
        rejection::rejected(&f, case, expected)?;
    }
    Ok(())
}

#[test]
fn project_derivation_planning_requires_all_manual_decisions_in_each_answer_history() -> Result<()>
{
    let f = Fixture::new()?;
    let runtime = ProjectStore::open(&f.base.source, "original")?.into_runtime();
    let session = fixture::start(&runtime, "two-questions")?;
    round::questions(
        &runtime,
        &planning::RoundKey::from(&session),
        &json!({"questions":[question("one",90),question("two",90)]}),
    )?;
    let waiting = current(&runtime, "two-questions")?;
    let answered = planning::answer(
        &runtime,
        &planning::AnswerRequest {
            project_id: "original".into(),
            session_id: waiting.id,
            request_id: "answer-two".into(),
            expected_revision: waiting.revision,
            answers: [("one".into(), "A".into()), ("two".into(), "B".into())].into(),
        },
    )?
    .session;
    planning::cancel(&runtime, &command(&answered, "cancel-two"))?;
    drop(runtime);
    let storage = ProjectStore::open(&f.base.source, "original")?;
    let mut receipt: Value = storage
        .store()
        .get("object_task_planning_receipt", "answer-two")?
        .unwrap();
    receipt["request"]["answers"]
        .as_object_mut()
        .unwrap()
        .remove("two");
    storage
        .store()
        .put("object_task_planning_receipt", "answer-two", &receipt)?;
    drop(storage);
    rejection::rejected(&f, "missing manual answer", "ANSWER_DECISIONS")
}

#[test]
fn project_derivation_planning_context_rejects_missing_future_and_mistimed_tasks() -> Result<()> {
    for case in [
        "missing",
        "future",
        "cancelled-too-early",
        "missing-assumption",
        "missing-run",
    ] {
        let f = Fixture::new()?;
        let storage = ProjectStore::open(&f.base.source, "original")?;
        let mut record: Value = storage
            .store()
            .get("object_task_planning", "start-adopted")?
            .unwrap();
        let expected = match case {
            "missing" => {
                record["context"]["tasks"].as_array_mut().unwrap().pop();
                "CONTEXT_TASK_SET"
            }
            "future" => {
                let task: Value = storage
                    .store()
                    .list::<Value>("object_task")?
                    .into_iter()
                    .find(|t| t["id"] == "ai-task")
                    .unwrap();
                record["context"]["tasks"]
                    .as_array_mut()
                    .unwrap()
                    .push(task);
                "CONTEXT_TASK"
            }
            "cancelled-too-early" => {
                record["context"]["tasks"][1]["status"] = json!("cancelled");
                "CONTEXT_TASK"
            }
            "missing-assumption" => {
                record["context"]["assumptions"] = json!([]);
                "CONTEXT_ASSUMPTION_SET"
            }
            _ => {
                record["context"]["runs"] = json!([]);
                "CONTEXT_RUN_SET"
            }
        };
        storage
            .store()
            .put("object_task_planning", "start-adopted", &record)?;
        drop(storage);
        rejection::rejected(&f, case, expected)?;
    }
    Ok(())
}

#[test]
fn project_derivation_planning_keeps_opaque_question_extensions_without_retyping() -> Result<()> {
    let f = Fixture::new()?;
    let storage = ProjectStore::open(&f.base.source, "original")?;
    let mut record: Value = storage
        .store()
        .get("object_task_planning", "start-waiting")?
        .unwrap();
    record["session"]["questions"][0]["futureMetadata"] =
        json!({"narrative":"planning:start-adopted","nested":[1,2,3]});
    record["session"]["questions"][0]["options"][0]["futureChoice"] = json!("unchanged");
    let questions = record["session"]["questions"].clone();
    storage
        .store()
        .put("object_task_planning", "start-waiting", &record)?;
    drop(storage);
    let preparation = f.base.temp.path().join("opaque-question");
    copy::prepare(f.base.request(), &preparation)?;
    let target = f.base.temp.path().join("opaque-target");
    assembly::create(&preparation, &target)?;
    assembly::activate(&preparation, &target)?;
    let runtime = ProjectStore::open(&target.join("project"), "derived")?.into_runtime();
    assert_eq!(
        serde_json::to_value(current(&runtime, "waiting")?.questions)?,
        questions
    );
    Ok(())
}

#[test]
fn project_derivation_planning_non_adopted_session_allows_exhausted_draft_revision() -> Result<()> {
    let f = Fixture::new()?;
    let runtime = ProjectStore::open(&f.base.source, "original")?.into_runtime();
    fixture::save(&runtime, "exhausted", json!({}))?;
    let handle = runtime.store();
    let mut draft = object_tasks::get_draft(&runtime, "original", "exhausted")?.unwrap();
    draft.revision = crate::object_task_types::MAX_REVISION;
    handle
        .lock()
        .unwrap()
        .put("object_task_draft", "exhausted", &draft)?;
    let session = fixture::start(&runtime, "exhausted")?;
    round::propose(
        &runtime,
        &planning::RoundKey::from(&session),
        fixture::proposal("exhausted-task"),
    )?;
    let proposed = current(&runtime, "exhausted")?;
    assert!(
        planning::adopt(&runtime, &command(&proposed, "exhausted-adopt"))
            .unwrap_err()
            .to_string()
            .contains("REVISION_EXHAUSTED")
    );
    drop(handle);
    drop(runtime);
    copy::prepare(f.base.request(), &f.base.temp.path().join("exhausted-copy"))?;
    Ok(())
}
