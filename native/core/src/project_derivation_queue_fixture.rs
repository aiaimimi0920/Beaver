use super::*;
use crate::{
    object_task_coarse_dispatch as coarse, object_task_dispatch as dispatch,
    object_task_queue_reorder as reorder, object_task_queue_view as view,
};

pub(super) struct QueueFixture {
    pub base: objects::Fixture,
    pub reordered: reorder::Receipt,
    pub unpaused: dispatch::Receipt,
    pub free_paused: dispatch::Receipt,
    pub coarse_unpaused: coarse::Receipt,
}

pub(super) fn pause(
    runtime: &crate::project_runtime::ProjectRuntime,
    task_id: &str,
    request_id: &str,
    revision: u64,
    paused: bool,
) -> Result<dispatch::Receipt> {
    let task = object_tasks::get_task(runtime, runtime.project_id(), task_id)?.unwrap();
    dispatch::set_paused(
        runtime,
        &dispatch::SetPausedRequest {
            project_id: task.project_id,
            task_id: task.id,
            object_id: task.object_id.unwrap(),
            run_id: task.run_id.unwrap(),
            request_id: request_id.into(),
            expected_task_revision: task.revision,
            expected_control_revision: revision,
            paused,
        },
    )
}

impl QueueFixture {
    pub fn new() -> Result<Self> {
        let base = objects::Fixture::new()?;
        let runtime = ProjectStore::open(&base.source, "original")?.into_runtime();
        save(
            &runtime,
            "queued-plan",
            0,
            json!({
                "objects":[{"id":"discard-object","name":"Unused"}],
                "tasks":[
                    {"id":"root","granularity":"coarse","title":"Root","prompt":"Root","acceptance":""},
                    {"id":"build","position":0,"granularity":"medium","title":"Build","prompt":"Build","acceptance":"",
                        "objectId":base.parent.id,"parentTaskId":"root"},
                    {"id":"second","position":1,"granularity":"medium","title":"Second","prompt":"Second","acceptance":"",
                        "objectId":base.parent.id,"parentTaskId":"root"},
                    {"id":"discard","position":2,"granularity":"medium","title":"Discard","prompt":"Discard","acceptance":"",
                        "objectId":"discard-object","parentTaskId":"root","dependsOn":["build"]},
                    {"id":"free","position":3,"granularity":"medium","title":"Free","prompt":"Free","acceptance":"",
                        "objectId":base.child.id,"parentTaskId":"root"}
                ]
            }),
        )?;
        object_tasks::commit(
            &runtime,
            &serde_json::from_value(json!({
                "projectId":"original","requestId":"queue-commit","draftId":"queued-plan",
                "expectedDraftRevision":1,"expectedPlanRevision":0
            }))?,
        )?;
        object_tasks::enqueue(
            &runtime,
            "original",
            &[
                "build".into(),
                "second".into(),
                "discard".into(),
                "free".into(),
            ],
        )?;
        let current = view::get(&runtime, "original")?;
        let reordered = reorder::reorder(
            &runtime,
            &reorder::Request {
                project_id: "original".into(),
                request_id: "move-free".into(),
                expected_version: current.version,
                task_id: "free".into(),
                previous_task_id: None,
                next_task_id: Some("build".into()),
            },
        )?;
        pause(&runtime, "build", "pause-build", 0, true)?;
        let unpaused = pause(&runtime, "build", "resume-build", 1, false)?;
        let free_paused = pause(&runtime, "free", "pause-free", 0, true)?;
        let mut coarse_request: coarse::SetPausedRequest = serde_json::from_value(json!({
            "projectId":"original","taskId":"root","requestId":"pause-root","expectedTaskRevision":0,
            "expectedControlRevision":0,"paused":true
        }))?;
        coarse::set_paused(&runtime, &coarse_request)?;
        coarse_request.request_id = "resume-root".into();
        coarse_request.expected_control_revision = 1;
        coarse_request.paused = false;
        let coarse_unpaused = coarse::set_paused(&runtime, &coarse_request)?;
        object_tasks::revise_planned(
            &runtime,
            &serde_json::from_value(json!({
                "projectId":"original","taskId":"build","requestId":"revise-build","expectedTaskRevision":0,"expectedPlanRevision":1,
                "definition":{"title":"Revised build","prompt":"Build","acceptance":"","dependsOn":[]},"reason":"Before execution"
            }))?,
        )?;
        object_tasks::cancel_planned(
            &runtime,
            &serde_json::from_value(json!({
                "projectId":"original","taskId":"discard","requestId":"cancel-discard","expectedTaskRevision":0,"expectedPlanRevision":2
            }))?,
        )?;
        drop(runtime);
        Ok(Self {
            base,
            reordered,
            unpaused,
            free_paused,
            coarse_unpaused,
        })
    }
}
