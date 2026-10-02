use super::*;
use crate::project_runtime::ProjectRuntime;

pub(super) struct ReworkFixture {
    pub candidate: CandidateFixture,
    pub attempts: Vec<object_attempt::Attempt>,
    pub reviews: Vec<candidate::Report>,
}

pub(super) fn verify(runtime: &ProjectRuntime, task: &str, id: &str) -> Result<recovery::View> {
    let project = runtime.project_id();
    let view = recovery::get(runtime, project, task)?.unwrap();
    recovery::verify(
        runtime,
        &recovery::VerifyRequest {
            project_id: project.into(),
            request_id: id.into(),
            target: view.target,
        },
        false,
    )?;
    Ok(recovery::get(runtime, project, task)?.unwrap())
}

pub(super) fn review(
    runtime: &ProjectRuntime,
    attempt: &object_attempt::Attempt,
    id: &str,
) -> Result<candidate::Report> {
    let check = checks::run(
        runtime,
        &checks::Request {
            project_id: runtime.project_id().into(),
            request_id: format!("{id}-check"),
            target: crate::object_attempt_view::Target::from_record(attempt),
        },
    )?;
    assert!(check.passed);
    candidate::prepare(
        runtime,
        &candidate::Request {
            project_id: runtime.project_id().into(),
            request_id: id.into(),
            check_request_id: check.request.request_id,
            target: check.request.target,
        },
    )
}

impl ReworkFixture {
    pub fn new(staged: bool, count: usize, terminal: State, schema: u32) -> Result<Self> {
        let candidate = if staged {
            CandidateFixture::staged(schema)?
        } else {
            CandidateFixture::new(true, "pinned", schema)?
        };
        let runtime =
            ProjectStore::open(&candidate.execution.base.source, "original")?.into_runtime();
        let mut previous = candidate.terminal.clone();
        let mut reviews = vec![candidate.review.clone()];
        for index in 0..count {
            // Reverse lexical request order protects generation-based prompt mapping.
            let name = if index == 0 { "z-rework" } else { "a-rework" };
            for blocked in [true, false] {
                let verify_id = format!("{name}-verify-{blocked}");
                let view = verify(&runtime, "build", &verify_id)?;
                let feedback = format!("Reduce movement speed {index}; preserve original/source-review/{}\n\nOwner rework feedback (candidate review source-review, attempt {}):\n用户原文，不改写", previous.id, previous.id);
                let request: resume::Request = serde_json::from_value(json!({
                    "projectId":"original", "requestId":format!("{name}-{blocked}"),
                    "target":view.target, "verificationRequestId":verify_id,
                    "rework":{"reviewRequestId":reviews.last().unwrap().request.request_id,
                        "attemptId":previous.id, "fineTaskId":previous.fine.id, "feedback":feedback}
                }))?;
                let (receipt, lease) =
                    resume::execute(&runtime, &request, &AtomicBool::new(blocked))?;
                if blocked {
                    assert!(lease.is_none());
                    assert!(matches!(
                        receipt.result,
                        Some(resume::Outcome::Blocked { .. })
                    ));
                } else {
                    let lease = lease.unwrap();
                    let id = lease.record().id.clone();
                    std::fs::write(
                        candidate.execution.workspace().join("partial.txt"),
                        format!("reworked {index}"),
                    )?;
                    object_attempt::finish(
                        &runtime,
                        lease,
                        if index + 1 == count {
                            terminal.clone()
                        } else {
                            State::AwaitingGate
                        },
                        None,
                        &AtomicBool::new(false),
                    )?;
                    previous = object_attempt::list(&runtime, &previous.preparation.run.id)?
                        .into_iter()
                        .find(|a| a.id == id)
                        .unwrap();
                }
            }
            if previous.state == State::AwaitingGate {
                reviews.push(review(
                    &runtime,
                    &previous,
                    &format!("review-after-{index}"),
                )?);
            }
        }
        verify(&runtime, "build", "rework-terminal-verify")?;
        let attempts = object_attempt::list(&runtime, &previous.preparation.run.id)?;
        Ok(Self {
            candidate,
            attempts,
            reviews,
        })
    }
}
