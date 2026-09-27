use crate::{
    object_task_storage::{self, DRAFT_KIND},
    object_task_types::{self, Draft, SaveDraftRequest, MAX_REVISION},
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Context, Result};

pub fn save(runtime: &ProjectRuntime, request: &SaveDraftRequest) -> Result<Draft> {
    ensure!(
        request.project_id == runtime.project_id(),
        "PROJECT_RUNTIME_MISMATCH"
    );
    ensure!(
        request.expected_revision <= MAX_REVISION,
        "INVALID_DRAFT_REVISION"
    );
    ensure!(
        object_task_types::valid_id(&request.draft_id),
        "INVALID_OBJECT_TASK_ID: draft id"
    );
    let handle = runtime.store();
    let mut store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("object task store lock poisoned"))?;
    store
        .transaction(|connection| {
            crate::object_task_validation::validate_plan(
                connection,
                &request.project_id,
                &request.plan,
                true,
            )?;
            let plan_state = object_task_storage::plan_state(connection, &request.project_id)?;
            ensure!(
                request.expected_plan_revision == plan_state.revision,
                "OBJECT_TASK_PLAN_REVISION_CONFLICT"
            );
            let previous =
                object_task_storage::read::<Draft>(connection, DRAFT_KIND, &request.draft_id)?;
            if let Some(draft) = &previous {
                ensure!(
                    draft.committed_request_id.is_none(),
                    "OBJECT_TASK_DRAFT_LOCKED"
                );
                ensure!(
                    draft.id == request.draft_id,
                    "OBJECT_TASK_DRAFT_IDENTITY_MISMATCH"
                );
            }
            let previous_revision = previous.as_ref().map_or(0, |draft| draft.revision);
            ensure!(
                previous
                    .as_ref()
                    .is_none_or(|draft| draft.project_id == request.project_id),
                "OBJECT_TASK_DRAFT_PROJECT_MISMATCH"
            );
            ensure!(
                previous_revision == request.expected_revision,
                "OBJECT_TASK_DRAFT_REVISION_CONFLICT"
            );
            ensure!(
                previous_revision < MAX_REVISION,
                "OBJECT_TASK_REVISION_EXHAUSTED"
            );
            let draft = Draft {
                project_id: request.project_id.clone(),
                id: request.draft_id.clone(),
                revision: previous_revision + 1,
                plan_revision: request.expected_plan_revision,
                plan: request.plan.clone(),
                committed_request_id: None,
            };
            object_task_storage::replace(connection, DRAFT_KIND, &draft.id, &draft)?;
            Ok(draft)
        })
        .context("OBJECT_TASK_DRAFT_SAVE_FAILED")
}

pub fn get(runtime: &ProjectRuntime, project_id: &str, draft_id: &str) -> Result<Option<Draft>> {
    ensure!(
        project_id == runtime.project_id(),
        "PROJECT_RUNTIME_MISMATCH"
    );
    ensure!(
        object_task_types::valid_id(draft_id),
        "INVALID_OBJECT_TASK_ID: draft id"
    );
    let handle = runtime.store();
    let store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("object task store lock poisoned"))?;
    let draft = store.get::<Draft>(DRAFT_KIND, draft_id)?;
    if let Some(draft) = &draft {
        ensure!(draft.id == draft_id, "OBJECT_TASK_DRAFT_IDENTITY_MISMATCH");
    }
    Ok(draft.filter(|draft| draft.project_id == project_id))
}
