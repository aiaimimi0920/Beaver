//! Object task persistence requires an open project and never uses legacy task storage.
use beaver_core::{
    object_task_dispatch, object_tasks, project_storage_router::ProjectStorageRouter,
};
use serde_json::Value;

pub(super) fn dispatch(
    router: &ProjectStorageRouter,
    method: &str,
    input: Option<&Value>,
) -> Option<Result<Value, String>> {
    if !matches!(
        method,
        "objectTask.saveDraft"
            | "objectTask.unlockDraft"
            | "objectTask.getDraft"
            | "objectTask.commit"
            | "objectTask.cancelPlanned"
            | "objectTask.setPaused"
            | "objectTask.setCoarsePaused"
            | "objectTask.revisePlanned"
            | "objectTask.declarePlanningComplete"
            | "objectTask.revisions"
            | "objectTask.snapshot"
            | "objectTask.get"
            | "objectTask.getRun"
            | "objectTask.enqueue"
            | "objectTask.queue"
            | "objectTask.queueView"
            | "objectTask.claim"
            | "objectTask.reorder"
            | "objectTask.finish"
            | "objectTask.cancelClaim"
    ) {
        return None;
    }
    Some(
        input
            .ok_or_else(|| anyhow::anyhow!("INVALID_INPUT: missing object task input"))
            .and_then(|input| call(router, method, input))
            .map_err(|error| format!("{error:#}")),
    )
}

fn call(router: &ProjectStorageRouter, method: &str, input: &Value) -> anyhow::Result<Value> {
    crate::business_catalog::validate(method, input)
        .map_err(|(code, message)| anyhow::anyhow!("{code}: {message}"))?;
    let project_id = input["projectId"]
        .as_str()
        .filter(|id| !id.is_empty())
        .ok_or_else(|| anyhow::anyhow!("INVALID_INPUT: projectId"))?;
    let runtime = router.runtime_for_project(project_id)?;
    Ok(match method {
        "objectTask.declarePlanningComplete" => {
            serde_json::to_value(beaver_core::object_task_planning_declaration::declare(
                &runtime,
                &serde_json::from_value(input.clone())?,
            )?)?
        }
        "objectTask.unlockDraft" => serde_json::to_value(object_tasks::unlock_draft(
            &runtime,
            &serde_json::from_value(input.clone())?,
        )?)?,
        "objectTask.saveDraft" => serde_json::to_value(object_tasks::save_draft(
            &runtime,
            &serde_json::from_value(input.clone())?,
        )?)?,
        "objectTask.commit" => serde_json::to_value(object_tasks::commit(
            &runtime,
            &serde_json::from_value(input.clone())?,
        )?)?,
        "objectTask.cancelPlanned" => serde_json::to_value(object_tasks::cancel_planned(
            &runtime,
            &serde_json::from_value(input.clone())?,
        )?)?,
        "objectTask.setCoarsePaused" => {
            serde_json::to_value(beaver_core::object_task_coarse_dispatch::set_paused(
                &runtime,
                &serde_json::from_value(input.clone())?,
            )?)?
        }
        "objectTask.setPaused" => serde_json::to_value(object_task_dispatch::set_paused(
            &runtime,
            &serde_json::from_value(input.clone())?,
        )?)?,
        "objectTask.revisePlanned" => serde_json::to_value(object_tasks::revise_planned(
            &runtime,
            &serde_json::from_value(input.clone())?,
        )?)?,
        "objectTask.revisions" => serde_json::to_value(object_tasks::revisions(
            &runtime,
            project_id,
            input["taskId"].as_str().expect("validated taskId"),
        )?)?,
        "objectTask.snapshot" => {
            serde_json::to_value(object_tasks::snapshot(&runtime, project_id)?)?
        }
        "objectTask.getDraft" => serde_json::to_value(object_tasks::get_draft(
            &runtime,
            project_id,
            input["draftId"].as_str().expect("validated draftId"),
        )?)?,
        "objectTask.get" => serde_json::to_value(object_tasks::get_task(
            &runtime,
            project_id,
            input["taskId"].as_str().expect("validated taskId"),
        )?)?,
        "objectTask.getRun" => serde_json::to_value(object_tasks::get_run(
            &runtime,
            project_id,
            input["runId"].as_str().expect("validated runId"),
        )?)?,
        "objectTask.enqueue" => serde_json::to_value(object_tasks::enqueue(
            &runtime,
            project_id,
            &serde_json::from_value::<Vec<String>>(input["taskIds"].clone())?,
        )?)?,
        "objectTask.queue" => serde_json::to_value(object_tasks::queue(&runtime, project_id)?)?,
        "objectTask.queueView" => serde_json::to_value(beaver_core::object_task_queue_view::get(
            &runtime, project_id,
        )?)?,
        "objectTask.claim" => serde_json::to_value(object_tasks::claim_next(
            &runtime,
            project_id,
            input["owner"].as_str().expect("validated owner"),
        )?)?,
        "objectTask.reorder" => {
            serde_json::to_value(beaver_core::object_task_queue_reorder::reorder(
                &runtime,
                &serde_json::from_value(input.clone())?,
            )?)?
        }
        "objectTask.finish" => serde_json::to_value(object_tasks::finish_claim(
            &runtime,
            project_id,
            input["taskId"].as_str().expect("validated taskId"),
            input["owner"].as_str().expect("validated owner"),
            input["claimToken"].as_str().expect("validated claimToken"),
            input["generation"].as_u64().expect("validated generation"),
            input["success"].as_bool().expect("validated success"),
        )?)?,
        "objectTask.cancelClaim" => serde_json::to_value(object_tasks::cancel_claim(
            &runtime,
            project_id,
            input["taskId"].as_str().expect("validated taskId"),
            input["owner"].as_str().expect("validated owner"),
            input["claimToken"].as_str().expect("validated claimToken"),
            input["generation"].as_u64().expect("validated generation"),
        )?)?,
        _ => unreachable!("object task dispatch method"),
    })
}

#[cfg(test)]
#[path = "object_task_dispatch_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "object_task_revision_dispatch_tests.rs"]
mod revision_tests;

#[cfg(test)]
#[path = "object_task_queue_dispatch_tests.rs"]
mod queue_tests;

#[cfg(test)]
#[path = "object_task_pause_dispatch_tests.rs"]
mod pause_tests;

#[cfg(test)]
#[path = "object_task_planning_metadata_dispatch_tests.rs"]
mod planning_metadata_tests;

#[cfg(test)]
#[path = "object_task_planning_declaration_tests.rs"]
mod planning_declaration_tests;
