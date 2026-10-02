use super::*;
use crate::project_runtime::ProjectRuntime;
use std::fs;

pub(super) struct RetryFixture {
    pub first: ExecutionFixture,
    pub attempts: Vec<object_attempt::Attempt>,
    pub verifications: Vec<String>,
    pub resumes: Vec<String>,
}

fn verify(runtime: &ProjectRuntime, id: &str) -> Result<recovery::Operation> {
    let current = recovery::get(runtime, "original", "build")?.unwrap();
    recovery::verify(
        runtime,
        &recovery::VerifyRequest {
            project_id: "original".into(),
            request_id: id.into(),
            target: current.target,
        },
        false,
    )
}

fn resume_request(runtime: &ProjectRuntime, id: &str, verified: &str) -> Result<resume::Request> {
    let current = recovery::get(runtime, "original", "build")?.unwrap();
    Ok(resume::Request {
        project_id: "original".into(),
        request_id: id.into(),
        target: current.target,
        verification_request_id: verified.into(),
        advance: None,
        rework: None,
    })
}

impl RetryFixture {
    pub fn new(baseline: &str) -> Result<Self> {
        Self::with_terminal(baseline, State::Interrupted)
    }

    pub fn with_terminal(baseline: &str, terminal: State) -> Result<Self> {
        Self::with_successor(baseline, terminal, true)
    }

    pub fn with_successor(baseline: &str, terminal: State, successor: bool) -> Result<Self> {
        let first = ExecutionFixture::with_successor(State::Interrupted, baseline, successor)?;
        Self::from_first(first, terminal)
    }

    pub fn from_first(first: ExecutionFixture, terminal: State) -> Result<Self> {
        let runtime = ProjectStore::open(&first.base.source, "original")?.into_runtime();
        let mut verifications = Vec::new();
        let mut resumes = Vec::new();
        pause(&runtime, "build", "retry-pause", true)?;
        verify(&runtime, "paused-verify")?;
        verifications.push("paused-verify".into());
        pause(&runtime, "build", "retry-unpause", false)?;
        verify(&runtime, "blocked-verify")?;
        verifications.push("blocked-verify".into());
        let blocked = resume_request(&runtime, "blocked-resume", "blocked-verify")?;
        let (operation, lease) = resume::execute(&runtime, &blocked, &AtomicBool::new(true))?;
        assert!(matches!(
            operation.result,
            Some(resume::Outcome::Blocked { .. })
        ));
        assert!(lease.is_none());
        resumes.push("blocked-resume".into());
        let mut attempts = vec![first.attempt.clone()];
        for index in 0..2 {
            let verified = format!("retry-verify-{index}");
            let request_id = format!("retry-resume-{index}");
            verify(&runtime, &verified)?;
            verifications.push(verified.clone());
            let input = resume_request(&runtime, &request_id, &verified)?;
            let (_, lease) = resume::execute(&runtime, &input, &AtomicBool::new(false))?;
            let mut lease = lease.unwrap();
            let id = lease.record().id.clone();
            object_attempt::bind(
                &runtime,
                &mut lease,
                &format!("retry-thread-{index}"),
                Some(&format!("retry-turn-{index}")),
            )?;
            object_attempt::record_event(
                &runtime,
                &lease,
                trace::Entry {
                    sequence: 0,
                    operation: "codex".into(),
                    phase: "retry".into(),
                    status: "failed".into(),
                },
            )?;
            fs::write(
                first.workspace().join("partial.txt"),
                format!("retry output {index}"),
            )?;
            let state = if index == 0 {
                State::Failed
            } else {
                terminal.clone()
            };
            if state == State::Interrupted {
                let view =
                    crate::object_attempt_view::list(&runtime, &first.attempt.preparation.run.id)?
                        .into_iter()
                        .find(|v| v.target.attempt_id == id)
                        .unwrap();
                control::request(
                    &runtime,
                    &control::InterruptRequest {
                        project_id: "original".into(),
                        request_id: "retry-interrupt".into(),
                        target: view.target,
                        expected_task_revision: view.task_revision,
                    },
                    true,
                )?;
            }
            let error = (state != State::AwaitingGate).then(|| format!("retry failure {index}"));
            object_attempt::finish(&runtime, lease, state, error, &AtomicBool::new(false))?;
            let attempt = object_attempt::list(&runtime, &first.attempt.preparation.run.id)?
                .into_iter()
                .find(|a| a.id == id)
                .unwrap();
            checks::run(
                &runtime,
                &checks::Request {
                    project_id: "original".into(),
                    request_id: format!("retry-check-{index}"),
                    target: crate::object_attempt_view::Target::from_record(&attempt),
                },
            )?;
            attempts.push(attempt);
            resumes.push(request_id);
            if index == 0 {
                let mut child = runtime
                    .store()
                    .lock()
                    .unwrap()
                    .get::<crate::object_catalog::ObjectRecord>("object", &first.base.child.id)?
                    .unwrap();
                child.name = "Renamed between retries".into();
                objects::update(&runtime, &child, "between-retries-metadata")?;
            }
        }
        verify(&runtime, "terminal-verify")?;
        verifications.push("terminal-verify".into());
        // A completed interrupt may legitimately be added after a verifier took its snapshot.
        let terminal = attempts.last().unwrap();
        let view = crate::object_attempt_view::list(&runtime, &terminal.preparation.run.id)?
            .into_iter()
            .find(|v| v.target.attempt_id == terminal.id)
            .unwrap();
        control::request(
            &runtime,
            &control::InterruptRequest {
                project_id: "original".into(),
                request_id: "late-interrupt".into(),
                target: view.target,
                expected_task_revision: view.task_revision,
            },
            false,
        )?;
        drop(runtime);
        Ok(Self {
            first,
            attempts,
            verifications,
            resumes,
        })
    }

    pub fn key(request: &str) -> Result<String> {
        crate::project_derivation_queue_records::receipt_key("original", request)
    }

    /// Keep a nested resume verifier synchronized when exercising deeper snapshot validators.
    pub fn verification(&self, id: &str, work: impl Fn(&mut Value)) -> Result<()> {
        self.first
            .mutate("object_recovery_verification", &Self::key(id)?, &work)?;
        for resume in &self.resumes {
            let storage = ProjectStore::open(&self.first.base.source, "original")?;
            let key = Self::key(resume)?;
            let mut saved: Value = storage
                .store()
                .get("object_recovery_resume", &key)?
                .unwrap();
            if saved["operation"]["request"]["verificationRequestId"] == id {
                work(&mut saved["verification"]);
                storage
                    .store()
                    .put("object_recovery_resume", &key, &saved)?;
            }
        }
        Ok(())
    }
}
