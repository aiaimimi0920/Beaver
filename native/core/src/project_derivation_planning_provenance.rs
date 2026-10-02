//! Only typed assumption source links are identities; arbitrary narrative text is not.
use crate::{
    object_task_planning_types::Record, object_task_types::PlanProposal,
    project_derivation_plan_validation::Plans,
    project_derivation_planning_context::PlanningContext,
};
use anyhow::{ensure, Context, Result};
use std::collections::BTreeMap;

fn detail(value: &Option<String>, sessions: &BTreeMap<String, Record>) -> Result<()> {
    let Some(link) = value.as_deref().and_then(|s| s.strip_prefix("planning:")) else {
        return Ok(());
    };
    let (id, decision) = link
        .split_once("/decision:")
        .map_or((link, None), |(id, d)| (id, Some(d)));
    let session = &sessions
        .get(id)
        .context("DERIVATION_PLANNING_SOURCE_SESSION_MISSING")?
        .session;
    if let Some(decision) = decision {
        ensure!(
            session.decisions.iter().any(|d| d.id == decision),
            "DERIVATION_PLANNING_SOURCE_DECISION_MISSING"
        );
    }
    Ok(())
}

fn plan(value: &PlanProposal, sessions: &BTreeMap<String, Record>) -> Result<()> {
    for assumption in &value.assumptions {
        detail(&assumption.source_detail, sessions)?;
    }
    Ok(())
}

pub(crate) fn validate(plans: &Plans, sessions: &BTreeMap<String, Record>) -> Result<()> {
    for assumption in &plans.state.assumptions {
        detail(&assumption.source_detail, sessions)?;
    }
    for draft in plans.drafts.values() {
        plan(&draft.plan, sessions)?;
    }
    for receipt in plans.unlocks.values() {
        plan(&receipt.draft.plan, sessions)?;
    }
    for record in sessions.values() {
        plan(&record.baseline, sessions)?;
        if let Some(proposal) = &record.session.proposal {
            plan(proposal, sessions)?;
        }
        if let Some(draft) = &record.session.adopted_draft {
            plan(&draft.plan, sessions)?;
        }
        let context: PlanningContext = serde_json::from_value(record.context.clone())?;
        for assumption in context.assumptions {
            detail(&assumption.source_detail, sessions)?;
        }
    }
    Ok(())
}
