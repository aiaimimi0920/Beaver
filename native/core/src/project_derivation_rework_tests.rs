use super::*;
use crate::project_runtime::ProjectRuntime;
use sha2::Digest;

#[path = "project_derivation_rework_fixture.rs"]
mod fixture;
#[path = "project_derivation_rework_rejection_tests.rs"]
mod rejection;
use fixture::ReworkFixture;

fn replay(runtime: &ProjectRuntime) -> Result<()> {
    let (verifications, resumes): (Vec<Value>, Vec<Value>) = {
        let store = runtime.store();
        let store = store.lock().unwrap();
        (
            store.list("object_recovery_verification")?,
            store.list("object_recovery_resume")?,
        )
    };
    for saved in verifications {
        let op: recovery::Operation = serde_json::from_value(saved["operation"].clone())?;
        assert_eq!(recovery::verify(runtime, &op.request, false)?, op);
    }
    for saved in resumes {
        let op: resume::Operation = serde_json::from_value(saved["operation"].clone())?;
        let (receipt, lease) = resume::execute(runtime, &op.request, &AtomicBool::new(false))?;
        assert_eq!(receipt, op);
        assert!(lease.is_none());
    }
    Ok(())
}

#[test]
fn project_derivation_rework_history_roundtrips_and_only_fresh_action_starts_once() -> Result<()> {
    for (staged, count, terminal, schema) in [
        (false, 1, State::Failed, 1),
        (true, 2, State::Interrupted, 2),
        (false, 2, State::AwaitingGate, 2),
        (true, 2, State::AwaitingGate, 1),
    ] {
        let f = ReworkFixture::new(staged, count, terminal.clone(), schema)?;
        let base = &f.candidate.execution.base;
        let mut source = base.source.clone();
        let mut project = "original".to_string();
        let mut task = "build".to_string();
        let mut run = f.candidate.terminal.preparation.run.id.clone();
        let mut attempts = f.attempts.clone();
        let mut reviews = f.reviews.clone();
        for round in 0..2 {
            let source_before = data_backup::inventory(&source)?;
            let preparation = base.temp.path().join(format!("rework-prepared-{round}"));
            let prepared = copy::prepare(
                copy::Request {
                    request_id: format!("rework-copy-{round}"),
                    source: source.clone(),
                    source_project_id: project.clone(),
                    target_project_id: format!("copy-{round}"),
                },
                &preparation,
            )?;
            let map = |kind: &str, id: &str| -> Result<String> {
                Ok(Rewrite(&prepared.identities).key(kind, id)?.id)
            };
            let original = ProjectStore::open(&source, &project)?;
            let receipts: Vec<Value> = original.store().list("object_recovery_resume")?;
            drop(original);
            let prepared_before = data_backup::inventory(&preparation)?;
            let destination = base.temp.path().join(format!("rework-target-{round}"));
            assembly::create(&preparation, &destination)?;
            assembly::activate(&preparation, &destination)?;
            let next_project = prepared.request.target_project_id.clone();
            let runtime =
                ProjectStore::open(&destination.join("project"), &next_project)?.into_runtime();
            let mapped_run = map("object_run", &run)?;
            let mapped_task = map("object_task", &task)?;
            let history = object_attempt::list(&runtime, &mapped_run)?;
            assert_eq!(history.len(), attempts.len());
            for attempt in &attempts {
                let id = map("object_attempt", &attempt.id)?;
                let copied = history.iter().find(|a| a.id == id).unwrap();
                assert_eq!(copied.input, attempt.input);
                assert_eq!(copied.output, attempt.output);
                assert_eq!(copied.state, attempt.state);
                for report in checks::list(&runtime, &next_project, &id)? {
                    assert_eq!(checks::run(&runtime, &report.request)?, report);
                }
            }
            // Independently reconstruct only the generated suffix using mapped typed IDs.
            for receipt in &receipts {
                let Some(approval) = receipt["operation"]["request"].get("rework") else {
                    continue;
                };
                let key = format!(
                    "{:x}",
                    sha2::Sha256::digest(serde_json::to_vec(&(
                        &project,
                        receipt["operation"]["request"]["requestId"]
                            .as_str()
                            .unwrap()
                    ))?)
                );
                let copied: Value = runtime
                    .store()
                    .lock()
                    .unwrap()
                    .get(
                        "object_recovery_resume",
                        &map("object_recovery_resume", &key)?,
                    )?
                    .unwrap();
                let mapped = &copied["operation"]["request"]["rework"];
                assert_eq!(mapped["feedback"], approval["feedback"]);
                assert_eq!(
                    mapped["reviewRequestId"],
                    map(KIND, approval["reviewRequestId"].as_str().unwrap())?
                );
                let prefix = copied["verification"]["records"]["fine"]["prompt"]
                    .as_str()
                    .unwrap();
                let expected = format!(
                    "{prefix}\n\nOwner rework feedback (candidate review {}, attempt {}):\n{}",
                    mapped["reviewRequestId"].as_str().unwrap(),
                    mapped["attemptId"].as_str().unwrap(),
                    approval["feedback"].as_str().unwrap()
                );
                assert_eq!(copied["next"]["prompt"], expected);
                if copied["operation"]["result"]["status"] == "started" {
                    let started = history
                        .iter()
                        .find(|a| a.id == copied["operation"]["result"]["attemptId"])
                        .unwrap();
                    assert_eq!(started.fine.prompt, expected);
                }
            }
            let mut mapped_reviews = Vec::new();
            for review in &reviews {
                let id = map("object_attempt", &review.request.target.attempt_id)?;
                let saved = candidate::list(&runtime, &next_project, &id)?
                    .into_iter()
                    .find(|r| {
                        r.request.request_id == map(KIND, &review.request.request_id).unwrap()
                    })
                    .unwrap();
                assert_eq!(candidate::prepare(&runtime, &saved.request)?, saved);
                assert_eq!(saved.time, review.time);
                assert_eq!(saved.rules, review.rules);
                assert_eq!(saved.schema_version, review.schema_version);
                assert_eq!(saved.output_digest, review.output_digest);
                mapped_reviews.push(saved);
            }
            let before = object_tasks::snapshot(&runtime, &next_project)?;
            replay(&runtime)?;
            assert_eq!(object_tasks::snapshot(&runtime, &next_project)?, before);
            assert!(
                recovery::get(&runtime, &next_project, &mapped_task)?
                    .unwrap()
                    .paused
            );
            drop(runtime);
            let runtime =
                ProjectStore::open(&destination.join("project"), &next_project)?.into_runtime();
            replay(&runtime)?;
            assert!(crate::object_run_preparation::claim_next(
                &runtime,
                &next_project,
                "never-auto-start"
            )?
            .is_none());
            if round == 1 {
                pause(&runtime, &mapped_task, "fresh-unpause", false)?;
                assert!(crate::object_run_preparation::claim_next(
                    &runtime,
                    &next_project,
                    "still-no-start"
                )?
                .is_none());
                let historical_verification = recovery::get(&runtime, &next_project, &mapped_task)?
                    .unwrap()
                    .operation
                    .unwrap()
                    .request
                    .request_id;
                let view = fixture::verify(&runtime, &mapped_task, "fresh-verify")?;
                let Some(resume::Outcome::Started { attempt_id, .. }) =
                    &view.resume.as_ref().unwrap().result
                else {
                    panic!("missing terminal rework");
                };
                let terminal_output = &history.iter().find(|a| a.id == *attempt_id).unwrap().output;
                let mut request: resume::Request = serde_json::from_value(json!({
                    "projectId":next_project, "requestId":"explicit-new-action", "target":view.target,
                    "verificationRequestId":"fresh-verify"
                }))?;
                let mut stale = request.clone();
                stale.verification_request_id = historical_verification;
                assert_eq!(
                    resume::execute(&runtime, &stale, &AtomicBool::new(false))
                        .err()
                        .unwrap()
                        .to_string(),
                    "OBJECT_RECOVERY_RESUME_REVERIFY_REQUIRED"
                );
                if terminal == State::AwaitingGate {
                    let last = mapped_reviews.last().unwrap();
                    request.rework = Some(serde_json::from_value(json!({
                        "reviewRequestId":last.request.request_id, "attemptId":last.request.target.attempt_id,
                        "fineTaskId":last.request.target.fine_task_id, "feedback":"New explicit feedback"
                    }))?);
                    assert_eq!(
                        resume::execute(&runtime, &request, &AtomicBool::new(false))
                            .err()
                            .unwrap()
                            .to_string(),
                        "OBJECT_CANDIDATE_REWORK_SOURCE_MISMATCH"
                    );
                    let current = history
                        .iter()
                        .find(|a| a.id == last.request.target.attempt_id)
                        .unwrap();
                    request.rework.as_mut().unwrap().review_request_id =
                        fixture::review(&runtime, current, "new-review")?
                            .request
                            .request_id;
                }
                let (op, lease) = resume::execute(&runtime, &request, &AtomicBool::new(false))?;
                let lease = lease.unwrap();
                assert!(lease.record().thread_id.is_none() && lease.record().turn_id.is_none());
                assert_eq!(Some(&lease.record().input), terminal_output.as_ref());
                object_attempt::finish(
                    &runtime,
                    lease,
                    State::Failed,
                    None,
                    &AtomicBool::new(false),
                )?;
                let (replayed, lease) =
                    resume::execute(&runtime, &request, &AtomicBool::new(false))?;
                assert_eq!(op, replayed);
                assert!(lease.is_none());
                assert_eq!(
                    object_attempt::list(&runtime, &mapped_run)?.len(),
                    history.len() + 1
                );
                if staged {
                    assert_eq!(
                        object_tasks::snapshot(&runtime, &next_project)?
                            .tasks
                            .iter()
                            .filter(|t| t.status == "accepted")
                            .count(),
                        2
                    );
                }
            }
            drop(runtime);
            assert_eq!(data_backup::inventory(&source)?, source_before);
            assert_eq!(data_backup::inventory(&preparation)?, prepared_before);
            source = destination.join("project");
            project = next_project;
            run = mapped_run;
            task = mapped_task;
            attempts = history;
            reviews = mapped_reviews;
        }
    }
    Ok(())
}
