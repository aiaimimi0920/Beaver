//! Append planned work from an immutable publication; never enqueue or rewrite its source.
use super::{storage, transact, State};
use crate::{
    object_framework::Baseline,
    object_task_commit_records, object_task_storage as tasks,
    object_task_types::{
        valid_id, Granularity, PlanProposal, TaskProposal, WorkRequirement, MAX_REVISION,
    },
    object_task_validation,
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};

const KIND: &str = "object_publication_followup";

#[path = "object_publication_frames.rs"]
pub mod frames;
#[path = "object_publication_feedback_image.rs"]
pub(crate) mod image;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub project_id: String,
    pub request_id: String,
    pub publication_request_id: String,
    pub version_id: String,
    pub title: String,
    pub feedback: String,
    pub acceptance: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview_frame: Option<frames::Reference>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Receipt {
    pub request: Request,
    pub source: crate::object_attempt_view::Target,
    pub medium_task_id: String,
    pub fine_task_id: String,
    pub run_id: String,
    pub plan_revision: u64,
}

fn validate(runtime: &ProjectRuntime, project: &str, publication: &str) -> Result<()> {
    ensure!(project == runtime.project_id(), "PROJECT_RUNTIME_MISMATCH");
    ensure!(
        valid_id(project) && valid_id(publication),
        "INVALID_OBJECT_TASK_ID"
    );
    Ok(())
}

pub fn list(runtime: &ProjectRuntime, project: &str, publication: &str) -> Result<Vec<Receipt>> {
    validate(runtime, project, publication)?;
    transact(runtime, |db| {
        Ok(tasks::read_all::<Receipt>(db, KIND)?
            .into_iter()
            .filter(|item| {
                item.request.project_id == project
                    && item.request.publication_request_id == publication
            })
            .collect())
    })
}

pub fn create(runtime: &ProjectRuntime, request: &Request) -> Result<Receipt> {
    validate(
        runtime,
        &request.project_id,
        &request.publication_request_id,
    )?;
    ensure!(
        valid_id(&request.request_id) && valid_id(&request.version_id),
        "INVALID_OBJECT_TASK_ID"
    );
    for (value, limit) in [
        (&request.title, 300),
        (&request.feedback, 16000),
        (&request.acceptance, 10000),
    ] {
        ensure!(
            !value.trim().is_empty() && value.len() <= limit,
            "INVALID_PUBLICATION_FOLLOWUP_TEXT"
        );
    }
    let key = crate::framework_checks::digest(&(&request.project_id, &request.request_id))?;
    transact(runtime, |db| {
        if let Some(receipt) = tasks::read::<Receipt>(db, KIND, &key)? {
            ensure!(
                receipt.request == *request,
                "OBJECT_TASK_REQUEST_ID_CONFLICT"
            );
            return Ok(receipt);
        }
        let saved = storage::read(db, &request.project_id, &request.publication_request_id)?
            .context("OBJECT_PUBLICATION_NOT_FOUND")?;
        ensure!(
            saved.operation.state == State::Published,
            "OBJECT_PUBLICATION_NOT_PUBLISHED"
        );
        ensure!(
            saved.operation.version_id == request.version_id,
            "OBJECT_PUBLICATION_VERSION_MISMATCH"
        );
        let source = saved.operation.request.target;
        if let Some(reference) = &request.preview_frame {
            frames::resolve(
                db,
                &request.project_id,
                &source.object_id,
                &request.version_id,
                reference,
            )?;
        }
        insert(db, request, source)
    })
}

pub(super) fn insert(
    db: &rusqlite::Connection,
    request: &Request,
    source: crate::object_attempt_view::Target,
) -> Result<Receipt> {
    let key = crate::framework_checks::digest(&(&request.project_id, &request.request_id))?;
    let mut state = tasks::plan_state(db, &request.project_id)?;
    ensure!(
        state.revision < MAX_REVISION,
        "OBJECT_TASK_REVISION_EXHAUSTED"
    );
    let medium_id = format!("feedback-{}", uuid::Uuid::new_v4());
    let fine_id = format!("feedback-{}", uuid::Uuid::new_v4());
    let position = tasks::all_tasks(db)?
        .iter()
        .filter(|task| task.project_id == request.project_id)
        .map(|task| task.position)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    let prompt = format!(
        "Follow-up to publication {} (version {}, object {}, run {}, attempt {}).\nFeedback:\n{}",
        request.publication_request_id,
        request.version_id,
        source.object_id,
        source.run_id,
        source.attempt_id,
        request.feedback
    );
    let medium = TaskProposal {
        id: medium_id.clone(),
        position,
        granularity: Granularity::Medium,
        title: request.title.clone(),
        prompt: prompt.clone(),
        acceptance: request.acceptance.clone(),
        requirement: WorkRequirement::Required,
        pending_planning: String::new(),
        object_id: Some(source.object_id.clone()),
        parent_task_id: None,
        depends_on: vec![source.task_id.clone()],
        stage_id: None,
        baseline: Some(Baseline::PinnedVersion {
            selected_version_id: request.version_id.clone(),
        }),
    };
    let fine = TaskProposal {
        id: fine_id.clone(),
        granularity: Granularity::Fine,
        parent_task_id: Some(medium_id.clone()),
        depends_on: vec![],
        stage_id: Some("feedback".into()),
        baseline: None,
        ..medium.clone()
    };
    let plan = PlanProposal {
        tasks: vec![medium, fine],
        ..Default::default()
    };
    object_task_validation::validate_plan(db, &request.project_id, &plan, false)?;
    let records = object_task_commit_records::prepare(db, &request.project_id, &plan)?;
    for run in &records.new_runs {
        tasks::insert(db, tasks::RUN_KIND, &run.id, run)?;
    }
    for task in &records.new_tasks {
        tasks::insert(db, tasks::TASK_KIND, &task.id, task)?;
    }
    state.revision += 1;
    tasks::replace(db, tasks::STATE_KIND, &request.project_id, &state)?;
    let receipt = Receipt {
        request: request.clone(),
        source,
        medium_task_id: medium_id,
        fine_task_id: fine_id,
        run_id: records.runs[0].id.clone(),
        plan_revision: state.revision,
    };
    tasks::insert(db, KIND, &key, &receipt)?;
    Ok(receipt)
}
