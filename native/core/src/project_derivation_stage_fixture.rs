use super::*;
use crate::project_runtime::ProjectRuntime;
use std::fs;

pub(in super::super) struct StageFixture {
    pub retry: RetryFixture,
}

pub(super) fn verify(runtime: &ProjectRuntime, request: &str) -> Result<recovery::View> {
    let project = runtime.project_id();
    let task = object_tasks::snapshot(runtime, project)?
        .tasks
        .into_iter()
        .find(|t| t.title == "Build")
        .unwrap()
        .id;
    let view = recovery::get(runtime, project, &task)?.unwrap();
    recovery::verify(
        runtime,
        &recovery::VerifyRequest {
            project_id: project.into(),
            request_id: request.into(),
            target: view.target,
        },
        false,
    )?;
    Ok(recovery::get(runtime, project, &task)?.unwrap())
}

impl StageFixture {
    pub fn new(advances: usize, terminal: State) -> Result<Self> {
        let first = ExecutionFixture::with_stages(State::Interrupted, "empty", 3)?;
        let mut retry = RetryFixture::from_first(first, State::AwaitingGate)?;
        let runtime = ProjectStore::open(&retry.first.base.source, "original")?.into_runtime();
        for index in 0..advances {
            let next = ["later-fine", "final-fine"][index];
            // Preserve a real completed blocked advance as well as successful ones.
            for blocked in [true, false] {
                let verify_id = format!("stage-verify-{index}-{blocked}");
                let view = verify(&runtime, &verify_id)?;
                retry.verifications.push(verify_id.clone());
                let attempt = retry.attempts.last().unwrap();
                let check_id = if index == 0 {
                    "retry-check-1".into()
                } else {
                    format!("stage-check-{}", index - 1)
                };
                let request = resume::Request {
                    project_id: "original".into(),
                    request_id: format!("stage-advance-{index}-{blocked}"),
                    target: view.target,
                    verification_request_id: verify_id,
                    rework: None,
                    advance: Some(resume::advance::Approval {
                        attempt_id: attempt.id.clone(),
                        check_request_id: check_id,
                        next_fine_task_id: next.into(),
                        next_fine_revision: 0,
                        acceptance_note: format!("Reviewed source stage {index}"),
                    }),
                };
                let (receipt, lease) =
                    resume::execute(&runtime, &request, &AtomicBool::new(blocked))?;
                retry.resumes.push(request.request_id);
                if blocked {
                    assert!(lease.is_none());
                    assert!(matches!(
                        receipt.result,
                        Some(resume::Outcome::Blocked { .. })
                    ));
                    continue;
                }
                let lease = lease.unwrap();
                let id = lease.record().id.clone();
                fs::write(
                    retry.first.workspace().join("partial.txt"),
                    format!("stage output {index}"),
                )?;
                object_attempt::finish(
                    &runtime,
                    lease,
                    if index + 1 == advances {
                        terminal.clone()
                    } else {
                        State::AwaitingGate
                    },
                    None,
                    &AtomicBool::new(false),
                )?;
                let attempt =
                    object_attempt::list(&runtime, &retry.first.attempt.preparation.run.id)?
                        .into_iter()
                        .find(|a| a.id == id)
                        .unwrap();
                checks::run(
                    &runtime,
                    &checks::Request {
                        project_id: "original".into(),
                        request_id: format!("stage-check-{index}"),
                        target: crate::object_attempt_view::Target::from_record(&attempt),
                    },
                )?;
                retry.attempts.push(attempt);
            }
        }
        verify(&runtime, "stage-terminal-verify")?;
        retry.verifications.push("stage-terminal-verify".into());
        Ok(Self { retry })
    }
}
