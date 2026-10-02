use super::*;
use crate::{
    object_attempt_control as control, object_attempt_trace as trace,
    object_task_dispatch as dispatch,
};
use std::{fs, path::PathBuf, sync::atomic::AtomicBool};

pub(super) struct ExecutionFixture {
    pub base: objects::Fixture,
    pub attempt: object_attempt::Attempt,
    pub unpaused: dispatch::Receipt,
}

pub(super) fn pause(
    runtime: &crate::project_runtime::ProjectRuntime,
    task: &str,
    request: &str,
    paused: bool,
) -> Result<dispatch::Receipt> {
    let snapshot = object_tasks::snapshot(runtime, runtime.project_id())?;
    let task = snapshot.tasks.iter().find(|t| t.id == task).unwrap();
    let control_revision = snapshot
        .dispatch_controls
        .iter()
        .find(|c| c.task_id == task.id)
        .map_or(0, |c| c.revision);
    dispatch::set_paused(
        runtime,
        &dispatch::SetPausedRequest {
            project_id: runtime.project_id().into(),
            task_id: task.id.clone(),
            object_id: task.object_id.clone().unwrap(),
            run_id: task.run_id.clone().unwrap(),
            request_id: request.into(),
            expected_task_revision: task.revision,
            expected_control_revision: control_revision,
            paused,
        },
    )
}

impl ExecutionFixture {
    pub fn new(state: object_attempt::State, baseline: &str) -> Result<Self> {
        Self::with_successor(state, baseline, true)
    }

    pub fn with_successor(
        state: object_attempt::State,
        baseline: &str,
        successor: bool,
    ) -> Result<Self> {
        Self::with_stages(state, baseline, if successor { 2 } else { 1 })
    }

    pub fn with_stages(
        state: object_attempt::State,
        baseline: &str,
        stages: usize,
    ) -> Result<Self> {
        let mut base = objects::Fixture::new()?;
        let runtime = ProjectStore::open(&base.source, "original")?.into_runtime();
        // The generic object fixture intentionally changes metadata after acceptance.
        // Execution must start from a version matching the current object metadata.
        let captured = objects::capture(&runtime, &base.child, "capture-execution-child")?;
        let version = captured.version_id.as_deref().unwrap();
        base.child = objects::accept(
            &runtime,
            &captured.object,
            version,
            "accept-execution-child",
        )?
        .object;
        let policy = match baseline {
            "empty" => json!({"basePolicy":"empty"}),
            "latest" => json!({"basePolicy":"latestAccepted"}),
            "pinned" => json!({"basePolicy":"pinnedVersion","selectedVersionId":version}),
            _ => panic!("unknown baseline"),
        };
        let mut draft = json!({
            "projectId":"original","draftId":"execution-plan","expectedRevision":0,"expectedPlanRevision":0,
            "plan":{"tasks":[
                {"id":"build","position":0,"granularity":"medium","objectId":base.child.id,"title":"Build","prompt":"Build","acceptance":"","baseline":policy},
                {"id":"fine","position":0,"granularity":"fine","parentTaskId":"build","objectId":base.child.id,"stageId":"files","title":"Files","prompt":"Keep source text","acceptance":""},
                {"id":"later-fine","position":1,"granularity":"fine","parentTaskId":"build","objectId":base.child.id,"stageId":"review","dependsOn":["fine"],"title":"Review","prompt":"Review","acceptance":""},
                {"id":"next","position":1,"granularity":"medium","objectId":base.child.id,"title":"Next","prompt":"Next","acceptance":""}
            ]}
        });
        if stages == 1 {
            draft["plan"]["tasks"]
                .as_array_mut()
                .unwrap()
                .retain(|t| t["id"] != "later-fine");
        }
        if stages == 3 {
            draft["plan"]["tasks"].as_array_mut().unwrap().push(json!({
                "id":"final-fine","position":2,"granularity":"fine","parentTaskId":"build",
                "objectId":base.child.id,"stageId":"final","dependsOn":["later-fine"],
                "title":"Final","prompt":"Finalize","acceptance":""
            }));
        }
        object_tasks::save_draft(&runtime, &serde_json::from_value(draft)?)?;
        object_tasks::commit(
            &runtime,
            &serde_json::from_value(json!({
                "projectId":"original","requestId":"execution-commit","draftId":"execution-plan","expectedDraftRevision":1,"expectedPlanRevision":0
            }))?,
        )?;
        object_tasks::enqueue(&runtime, "original", &["build".into(), "next".into()])?;
        pause(&runtime, "build", "pause-source", true)?;
        let unpaused = pause(&runtime, "build", "unpause-source", false)?;
        let claim =
            crate::object_run_preparation::claim_next(&runtime, "original", "source-worker")?
                .unwrap();
        let mut lease = object_attempt::start(&runtime, claim)?.unwrap();
        object_attempt::bind(
            &runtime,
            &mut lease,
            "historical-thread",
            Some("historical-turn"),
        )?;
        object_attempt::record_event(
            &runtime,
            &lease,
            trace::Entry {
                sequence: 0,
                operation: "codex".into(),
                phase: "launch".into(),
                status: "failed".into(),
            },
        )?;
        let workspace = base.source.join(&lease.record().preparation.workspace);
        fs::write(workspace.join("partial.txt"), "retained stopped output")?;
        fs::create_dir(workspace.join(".godot"))?;
        fs::write(workspace.join(".godot/cache"), "ignored cache")?;
        fs::create_dir_all(
            base.source
                .join(".beaver/workspaces/.codex")
                .join(&lease.record().id),
        )?;
        if state == object_attempt::State::Interrupted {
            let view =
                crate::object_attempt_view::list(&runtime, &lease.record().preparation.run.id)?
                    .remove(0);
            control::request(
                &runtime,
                &control::InterruptRequest {
                    project_id: "original".into(),
                    request_id: "interrupt".into(),
                    target: view.target,
                    expected_task_revision: view.task_revision,
                },
                true,
            )?;
        }
        let run = lease.record().preparation.run.id.clone();
        let error =
            (state != object_attempt::State::AwaitingGate).then(|| "simulated failure".into());
        object_attempt::finish(&runtime, lease, state, error, &AtomicBool::new(false))?;
        let attempt = object_attempt::list(&runtime, &run)?.remove(0);
        crate::object_attempt_checks::run(
            &runtime,
            &crate::object_attempt_checks::Request {
                project_id: "original".into(),
                request_id: "check".into(),
                target: crate::object_attempt_view::Target::from_record(&attempt),
            },
        )?;
        drop(runtime);
        Ok(Self {
            base,
            attempt,
            unpaused,
        })
    }

    pub fn mutate(&self, kind: &str, id: &str, work: impl FnOnce(&mut Value)) -> Result<()> {
        let storage = ProjectStore::open(&self.base.source, "original")?;
        let mut value: Value = storage.store().get(kind, id)?.unwrap();
        work(&mut value);
        storage.store().put(kind, id, &value)
    }

    pub fn request_with_reordered_closure(&self) -> Result<copy::Request> {
        let versions = &self.attempt.preparation.baseline.as_ref().unwrap().versions;
        let mut request = self.base.request();
        if versions.len() < 2 {
            return Ok(request);
        }
        // Exercise the real remapper with IDs whose canonical ordering changes.
        for index in 0..100 {
            request.request_id = format!("derive-reordered-{index}");
            let mapped = versions
                .iter()
                .map(|v| {
                    crate::project_derivation_identity::generated(&request, "object", &v.object_id)
                })
                .collect::<Result<Vec<_>>>()?;
            if mapped.windows(2).any(|pair| pair[0] > pair[1]) {
                return Ok(request);
            }
        }
        anyhow::bail!("fixture could not produce a reordered reference closure")
    }

    pub fn preparation(&self, pointer: &str, value: Value) -> Result<()> {
        self.mutate(
            "object_run_preparation",
            &self.attempt.preparation.id,
            |p| {
                *p.pointer_mut(pointer).unwrap() = value.clone();
            },
        )?;
        self.mutate("object_attempt", &self.attempt.id, |a| {
            *a["preparation"].pointer_mut(pointer).unwrap() = value;
        })
    }

    pub fn rejected(&self, label: &str, expected: &str) -> Result<()> {
        let before = data_backup::inventory(&self.base.source)?;
        let preparation = self.base.temp.path().join(label);
        for error in [
            copy::inspect_source(&self.base.source).unwrap_err(),
            copy::prepare(self.base.request(), &preparation).unwrap_err(),
        ] {
            assert!(
                format!("{error:#}").contains(expected),
                "{label}: {error:#}"
            );
        }
        assert!(!preparation.exists());
        assert_eq!(data_backup::inventory(&self.base.source)?, before);
        Ok(())
    }

    pub fn workspace(&self) -> PathBuf {
        self.base.source.join(&self.attempt.preparation.workspace)
    }
}
