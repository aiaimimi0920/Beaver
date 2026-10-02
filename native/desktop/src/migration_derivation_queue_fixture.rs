use super::*;
use beaver_core::{
    object_task_coarse_dispatch as coarse, object_task_dispatch as dispatch,
    object_task_queue_reorder as reorder, object_task_queue_view as view,
};

pub(super) fn source_history(f: &Fixture) -> Result<Vec<(&'static str, Value)>> {
    let runtime = ProjectStore::open(&f.source, "original")?.into_runtime();
    object_tasks::save_draft(
        &runtime,
        &serde_json::from_value(json!({
            "projectId":"original","draftId":"queue-plan","expectedRevision":0,"expectedPlanRevision":0,
            "plan":{"objects":[{"id":"object","name":"Shared"},{"id":"free-object","name":"Free"}],"tasks":[
                {"id":"root","granularity":"coarse","title":"Root","prompt":"Root","acceptance":""},
                {"id":"build","position":0,"granularity":"medium","title":"Build","prompt":"Build","acceptance":"",
                    "objectId":"object","parentTaskId":"root"},
                {"id":"second","position":1,"granularity":"medium","title":"Second","prompt":"Second","acceptance":"",
                    "objectId":"object","parentTaskId":"root"},
                {"id":"free","position":2,"granularity":"medium","title":"Free","prompt":"Free","acceptance":"",
                    "objectId":"free-object","parentTaskId":"root"}
            ]}
        }))?,
    )?;
    object_tasks::commit(
        &runtime,
        &serde_json::from_value(json!({
            "projectId":"original","requestId":"queue-commit","draftId":"queue-plan",
            "expectedDraftRevision":1,"expectedPlanRevision":0
        }))?,
    )?;
    object_tasks::enqueue(
        &runtime,
        "original",
        &["build".into(), "second".into(), "free".into()],
    )?;
    let reordered = reorder::reorder(
        &runtime,
        &reorder::Request {
            project_id: "original".into(),
            request_id: "move-free".into(),
            expected_version: view::get(&runtime, "original")?.version,
            task_id: "free".into(),
            previous_task_id: None,
            next_task_id: Some("build".into()),
        },
    )?;
    let task = object_tasks::get_task(&runtime, "original", "build")?.unwrap();
    let mut pause = dispatch::SetPausedRequest {
        project_id: "original".into(),
        task_id: task.id,
        object_id: task.object_id.unwrap(),
        run_id: task.run_id.unwrap(),
        request_id: "pause-build".into(),
        expected_task_revision: 0,
        expected_control_revision: 0,
        paused: true,
    };
    dispatch::set_paused(&runtime, &pause)?;
    pause.request_id = "resume-build".into();
    pause.expected_control_revision = 1;
    pause.paused = false;
    let resumed = dispatch::set_paused(&runtime, &pause)?;
    let mut coarse_pause: coarse::SetPausedRequest = serde_json::from_value(json!({
        "projectId":"original","taskId":"root","requestId":"pause-root","expectedTaskRevision":0,
        "expectedControlRevision":0,"paused":true
    }))?;
    coarse::set_paused(&runtime, &coarse_pause)?;
    coarse_pause.request_id = "resume-root".into();
    coarse_pause.expected_control_revision = 1;
    coarse_pause.paused = false;
    let root_resumed = coarse::set_paused(&runtime, &coarse_pause)?;
    object_tasks::revise_planned(
        &runtime,
        &serde_json::from_value(json!({
            "projectId":"original","taskId":"build","requestId":"revise-build","expectedTaskRevision":0,"expectedPlanRevision":1,
            "definition":{"title":"Revised build","prompt":"Build","acceptance":"","dependsOn":[]},"reason":"Reviewed before execution"
        }))?,
    )?;
    object_tasks::cancel_planned(
        &runtime,
        &serde_json::from_value(json!({
            "projectId":"original","taskId":"second","requestId":"cancel-second","expectedTaskRevision":0,"expectedPlanRevision":2
        }))?,
    )?;
    Ok(vec![
        (
            "object_task_queue_reorder_receipt",
            serde_json::to_value(reordered)?,
        ),
        (
            "object_task_dispatch_receipt",
            serde_json::to_value(resumed)?,
        ),
        (
            "object_task_coarse_dispatch_receipt",
            serde_json::to_value(root_resumed)?,
        ),
    ])
}
