use crate::{
    object_task_storage::{self as storage, DRAFT_KIND},
    object_task_types::{valid_id, Draft, PlanProposal, UnlockDraftRequest, MAX_REVISION},
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};

const RECEIPT_KIND: &str = "object_task_draft_unlock_receipt";

#[derive(Serialize, Deserialize)]
pub(crate) struct Receipt {
    pub(crate) request: UnlockDraftRequest,
    pub(crate) draft: Draft,
}

/// Start an empty draft without modifying the already committed task definitions.
pub(crate) fn unlock(runtime: &ProjectRuntime, request: &UnlockDraftRequest) -> Result<Draft> {
    ensure!(
        request.project_id == runtime.project_id(),
        "PROJECT_RUNTIME_MISMATCH"
    );
    ensure!(
        valid_id(&request.draft_id) && valid_id(&request.request_id),
        "INVALID_OBJECT_TASK_ID: unlock"
    );
    ensure!(
        request.expected_revision <= MAX_REVISION && request.expected_plan_revision <= MAX_REVISION,
        "INVALID_OBJECT_TASK_REVISION"
    );
    let handle = runtime.store();
    let mut store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("object task store lock poisoned"))?;
    store
        .transaction(|db| {
            if let Some(receipt) = storage::read::<Receipt>(db, RECEIPT_KIND, &request.request_id)?
            {
                ensure!(
                    receipt.request == *request,
                    "OBJECT_TASK_REQUEST_ID_CONFLICT"
                );
                return Ok(receipt.draft);
            }
            let mut draft = storage::read::<Draft>(db, DRAFT_KIND, &request.draft_id)?
                .context("OBJECT_TASK_DRAFT_NOT_FOUND")?;
            ensure!(
                draft.id == request.draft_id && draft.project_id == request.project_id,
                "OBJECT_TASK_DRAFT_IDENTITY_MISMATCH"
            );
            ensure!(
                draft.revision == request.expected_revision,
                "OBJECT_TASK_DRAFT_REVISION_CONFLICT"
            );
            let state = storage::plan_state(db, &request.project_id)?;
            ensure!(
                state.revision == request.expected_plan_revision,
                "OBJECT_TASK_PLAN_REVISION_CONFLICT"
            );
            ensure!(
                draft.committed_request_id.is_some(),
                "OBJECT_TASK_DRAFT_NOT_LOCKED"
            );
            ensure!(
                draft.revision < MAX_REVISION,
                "OBJECT_TASK_REVISION_EXHAUSTED"
            );
            draft.revision += 1;
            draft.plan_revision = state.revision;
            draft.plan = PlanProposal::default();
            draft.committed_request_id = None;
            storage::replace(db, DRAFT_KIND, &draft.id, &draft)?;
            storage::insert(
                db,
                RECEIPT_KIND,
                &request.request_id,
                &Receipt {
                    request: request.clone(),
                    draft: draft.clone(),
                },
            )?;
            Ok(draft)
        })
        .context("OBJECT_TASK_DRAFT_UNLOCK_FAILED")
}
