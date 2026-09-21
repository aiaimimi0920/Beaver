use crate::asset_task::{self, Stage, State};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Template {
    pub id: String,
    pub version: u32,
    pub stages: Vec<Definition>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Workflow {
    pub template_id: String,
    pub template_version: u32,
    pub approved: Vec<String>,
    pub pending: Option<String>,
    pub history: Vec<String>,
}

pub fn configure(state: &mut State, revision: u64, template: &Template) -> Result<()> {
    ensure!(state.revision == revision, "Asset revision changed");
    ensure!(
        state.delivery.is_none() && state.stages.is_empty(),
        "Define delivery before creating production stages; the template cannot be replaced"
    );
    ensure!(
        state.feedback.iter().all(|f| f.terminal()),
        "Resolve existing feedback before defining delivery"
    );
    asset_task::validate_id(&template.id)?;
    ensure!(
        template.version > 0 && (2..=20).contains(&template.stages.len()),
        "Supply a versioned template with 2 to 20 linear stages"
    );
    let mut ids = std::collections::HashSet::new();
    let mut stages = Vec::new();
    for (index, definition) in template.stages.iter().enumerate() {
        asset_task::validate_id(&definition.id)?;
        ensure!(
            ids.insert(&definition.id)
                && !definition.name.trim().is_empty()
                && definition.name.len() <= 300,
            "Invalid or duplicate delivery stage"
        );
        stages.push(Stage {
            id: definition.id.clone(),
            name: definition.name.clone(),
            status: if index == 0 { "running" } else { "pending" }.into(),
            dependencies: index
                .checked_sub(1)
                .map(|i| vec![template.stages[i].id.clone()])
                .unwrap_or_default(),
            objects: vec![],
            evidence: String::new(),
            round: state.round,
        });
    }
    state.stages = stages;
    state.delivery = Some(Workflow {
        template_id: template.id.clone(),
        template_version: template.version,
        ..Workflow::default()
    });
    state.phase = "producing".into();
    state.revision += 1;
    Ok(())
}

pub fn complete(state: &State) -> bool {
    state
        .delivery
        .as_ref()
        .is_none_or(|flow| flow.pending.is_none() && flow.approved.len() == state.stages.len())
}

/// Shared by both asset tools: only the current stage is writable by Codex.
pub fn guard_update(state: &State, stages: &[Stage]) -> Result<()> {
    let Some(flow) = &state.delivery else {
        return Ok(());
    };
    ensure!(
        stages.len() == state.stages.len(),
        "Delivery template stages are fixed"
    );
    for (index, (old, next)) in state.stages.iter().zip(stages).enumerate() {
        ensure!(
            old.id == next.id && old.name == next.name && old.dependencies == next.dependencies,
            "Delivery stage identity, order and dependencies are fixed"
        );
        if index != flow.approved.len() || flow.pending.is_some() {
            ensure!(old == next, "Only the current unsubmitted stage can change; request owner rework for approved stages");
        } else {
            ensure!(
                next.status != "completed",
                "Submit files for owner review; textual completion cannot approve a delivery stage"
            );
        }
    }
    Ok(())
}

pub fn can_submit(state: &State, revision: u64, stage: &str, inputs: &[String]) -> Result<()> {
    ensure!(
        state.revision == revision,
        "Asset revision changed; inspect before submitting"
    );
    let flow = state
        .delivery
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Define a delivery template first"))?;
    ensure!(
        flow.pending.is_none(),
        "A candidate is already awaiting review"
    );
    ensure!(flow.history.len() < 200, "Delivery history limit reached");
    ensure!(
        state
            .stages
            .get(flow.approved.len())
            .is_some_and(|s| s.id == stage),
        "Only the current linear stage may submit"
    );
    ensure!(
        inputs == flow.approved,
        "Input candidate versions changed; use the approved versions from state"
    );
    ensure!(
        state.feedback.iter().all(|f| f.terminal()),
        "Resolve feedback before submitting files"
    );
    crate::asset_work::completed_attempts(state, stage)?;
    Ok(())
}
