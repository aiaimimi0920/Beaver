//! Remap only receipt-proven generated feedback suffixes; never search/replace user text.
use super::super::{rework, storage};
use crate::{
    object_task_types::TaskRecord, project_derivation_copy::Request,
    project_derivation_plan_rewrite::PlanIds,
};
use anyhow::{ensure, Context, Result};
use rusqlite::Connection;
use std::collections::BTreeMap;

pub(super) fn text_only(approval: &rework::Approval) -> Result<()> {
    ensure!(
        approval.image.is_none()
            && approval.preview_frame.is_none()
            && approval.relocation.is_none(),
        "DERIVATION_REWORK_TEXT_ONLY"
    );
    Ok(())
}

pub(super) fn approval(request: &Request, value: &mut rework::Approval) -> Result<()> {
    text_only(value)?;
    let ids = PlanIds(request);
    ids.id(&mut value.review_request_id, "object_candidate_request")?;
    ids.id(&mut value.attempt_id, "object_attempt")?;
    ids.id(&mut value.fine_task_id, "object_task")
}

#[derive(Default)]
pub(crate) struct Prompts(BTreeMap<(String, String), String>);

impl Prompts {
    /// Called only after full closed-world source validation, including historical reviews.
    /// Recovery generation orders repeated reworks even when their request IDs sort differently.
    pub(crate) fn read(connection: &Connection, request: &Request) -> Result<Self> {
        let mut statement = connection.prepare(
            "SELECT json_extract(value,'$.operation.request.requestId') FROM entities
             WHERE kind='object_recovery_resume'
             AND json_extract(value,'$.operation.request.projectId')=?
             AND json_type(value,'$.operation.request.rework')='object'
             ORDER BY json_extract(value,'$.operation.request.target.runId'),
                      json_extract(value,'$.operation.request.target.recoveryGeneration')",
        )?;
        let ids = statement
            .query_map([&request.source_project_id], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut prompts = Self::default();
        for id in ids {
            let saved = storage::read(connection, &request.source_project_id, &id)?
                .context("DERIVATION_RECOVERY_RESUME_INCOMPLETE")?;
            ensure!(
                saved.operation.result.is_some(),
                "DERIVATION_RECOVERY_RESUME_INCOMPLETE"
            );
            let source = saved.operation.request.rework.as_ref().unwrap();
            text_only(source)?;
            let previous = saved
                .verification
                .records
                .fine
                .as_ref()
                .context("DERIVATION_REWORK_FINE_MISSING")?;
            let next = saved
                .next
                .as_ref()
                .context("DERIVATION_REWORK_FINE_MISSING")?;
            ensure!(
                next.prompt == format!("{}{}", previous.prompt, rework::text_suffix(source)),
                "DERIVATION_REWORK_PROMPT_MISMATCH"
            );
            let mut mapped = source.clone();
            approval(request, &mut mapped)?;
            let prefix = prompts.get(previous);
            let prompt = format!("{prefix}{}", rework::text_suffix(&mapped));
            // Mapping long legacy IDs can grow a prompt. Refuse before publishing an
            // unreadable target rather than weakening the production 20 KB contract.
            ensure!(
                prompt.len() <= 20_000,
                "OBJECT_CANDIDATE_REWORK_PROMPT_TOO_LONG"
            );
            let key = (next.id.clone(), next.prompt.clone());
            if let Some(existing) = prompts.0.insert(key, prompt.clone()) {
                ensure!(existing == prompt, "DERIVATION_REWORK_PROMPT_AMBIGUOUS");
            }
        }
        Ok(prompts)
    }

    fn get<'a>(&'a self, task: &'a TaskRecord) -> &'a str {
        self.0
            .get(&(task.id.clone(), task.prompt.clone()))
            .map(String::as_str)
            .unwrap_or(&task.prompt)
    }

    /// All frozen/live task copies use the same exact whole-prompt mapping before IDs change.
    pub(crate) fn task(&self, task: &mut TaskRecord) {
        task.prompt = self.get(task).to_owned();
    }
}
