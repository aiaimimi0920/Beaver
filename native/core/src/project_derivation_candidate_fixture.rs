use super::*;
use crate::project_runtime::ProjectRuntime;

pub(super) struct CandidateFixture {
    pub execution: ExecutionFixture,
    pub terminal: object_attempt::Attempt,
    pub review: candidate::Report,
}

impl CandidateFixture {
    pub fn new(retry: bool, baseline: &str, schema: u32) -> Result<Self> {
        let (execution, terminal) = if retry {
            let f = RetryFixture::with_successor(baseline, State::AwaitingGate, false)?;
            (f.first, f.attempts.last().unwrap().clone())
        } else {
            let f = ExecutionFixture::with_successor(State::AwaitingGate, baseline, false)?;
            let terminal = f.attempt.clone();
            (f, terminal)
        };
        Self::review(
            execution,
            terminal,
            if retry { "retry-check-1" } else { "check" },
            schema,
        )
    }

    pub fn staged(schema: u32) -> Result<Self> {
        let f = super::super::stages::stage_fixture::StageFixture::new(2, State::AwaitingGate)?;
        let terminal = f.retry.attempts.last().unwrap().clone();
        Self::review(f.retry.first, terminal, "stage-check-1", schema)
    }

    fn review(
        execution: ExecutionFixture,
        terminal: object_attempt::Attempt,
        check: &str,
        schema: u32,
    ) -> Result<Self> {
        let runtime = ProjectStore::open(&execution.base.source, "original")?.into_runtime();
        let request = candidate::Request {
            project_id: "original".into(),
            request_id: "source-review".into(),
            check_request_id: check.into(),
            target: crate::object_attempt_view::Target::from_record(&terminal),
        };
        candidate::prepare(&runtime, &request)?;
        drop(runtime);
        if schema == 1 {
            execution.mutate(KIND, "source-review", |saved| {
                saved["report"]["schemaVersion"] = json!(1);
                let blockers = saved["report"]["blockers"].as_array_mut().unwrap();
                blockers.retain(|b| b != "PUBLICATION_CONFIRMATION_REQUIRED");
                blockers.insert(1, json!("FEEDBACK_REVIEW_UNAVAILABLE"));
                blockers.insert(2, json!("PUBLICATION_NOT_IMPLEMENTED"));
            })?;
        }
        let runtime = ProjectStore::open(&execution.base.source, "original")?.into_runtime();
        let review = candidate::prepare(&runtime, &request)?;
        Ok(Self {
            execution,
            terminal,
            review,
        })
    }

    pub fn mutate(&self, work: impl FnOnce(&mut Value)) -> Result<()> {
        self.execution.mutate(KIND, "source-review", work)
    }
}

pub(super) fn verify(runtime: &ProjectRuntime, task: &str) -> Result<recovery::View> {
    let project = runtime.project_id();
    let view = recovery::get(runtime, project, task)?.unwrap();
    recovery::verify(
        runtime,
        &recovery::VerifyRequest {
            project_id: project.into(),
            request_id: "fresh-review-verify".into(),
            target: view.target,
        },
        false,
    )?;
    Ok(recovery::get(runtime, project, task)?.unwrap())
}
