//! Complete immutable dispatch chains; current controls cannot exist without their commands.
use crate::{
    object_task_coarse_dispatch as coarse, object_task_dispatch as medium,
    object_task_types::{valid_id, Granularity},
    project_derivation_plan_validation::{records, Plans},
    project_derivation_queue_records::receipt_key,
    project_migration_ownership::Entity,
};
use anyhow::{ensure, Context, Result};
use std::collections::BTreeMap;

type Controls = BTreeMap<String, (u64, bool)>;
type History = BTreeMap<String, BTreeMap<u64, (u64, bool)>>;

fn command(
    key: &str,
    project: &str,
    request: &str,
    expected_task: u64,
    expected_control: u64,
    current_task: u64,
    cancelled: bool,
) -> Result<()> {
    ensure!(
        valid_id(request)
            && key == receipt_key(project, request)?
            && expected_task <= current_task - u64::from(cancelled)
            && expected_control < medium::MAX_CONTROL_REVISION,
        "DERIVATION_DISPATCH_COMMAND_MISMATCH"
    );
    Ok(())
}

fn insert(
    history: &mut History,
    id: &str,
    revision: u64,
    task_revision: u64,
    paused: bool,
) -> Result<()> {
    ensure!(
        history
            .entry(id.into())
            .or_default()
            .insert(revision, (task_revision, paused))
            .is_none(),
        "DERIVATION_DISPATCH_DUPLICATE_REVISION"
    );
    Ok(())
}

fn chains(controls: Controls, mut history: History) -> Result<()> {
    for (id, (revision, paused)) in controls {
        let chain = history
            .remove(&id)
            .context("DERIVATION_DISPATCH_HISTORY_GAP")?;
        ensure!(
            (1..=medium::MAX_CONTROL_REVISION).contains(&revision)
                && chain.len() as u64 == revision
                && chain.keys().enumerate().all(|(i, r)| *r == i as u64 + 1)
                && chain.get(&revision).map(|(_, value)| *value) == Some(paused),
            "DERIVATION_DISPATCH_HISTORY_HEAD_MISMATCH"
        );
        let task_revisions: Vec<_> = chain.values().map(|(task, _)| *task).collect();
        ensure!(
            task_revisions.windows(2).all(|pair| pair[0] <= pair[1]),
            "DERIVATION_DISPATCH_TASK_REVISION_REVERSED"
        );
    }
    ensure!(history.is_empty(), "DERIVATION_DISPATCH_CONTROL_MISSING");
    Ok(())
}

fn medium_identity(plans: &Plans, control: &medium::Control) -> Result<()> {
    let task = plans.task(&control.task_id)?;
    let run = plans.run(&control.run_id)?;
    ensure!(
        control.schema_version == 1
            && control.project_id == plans.project
            && task.granularity == Granularity::Medium
            && task.run_id.as_ref() == Some(&control.run_id)
            && run.object_id == control.object_id
            && run.medium_task_id == task.id,
        "DERIVATION_DISPATCH_IDENTITY_MISMATCH"
    );
    Ok(())
}

fn medium(entities: &[Entity], plans: &Plans) -> Result<()> {
    let controls = records::<medium::Control>(entities, "object_task_dispatch_control")?;
    let receipts = records::<medium::Receipt>(entities, "object_task_dispatch_receipt")?;
    let mut heads = Controls::new();
    let mut history = History::new();
    for (key, control) in controls {
        medium_identity(plans, &control)?;
        ensure!(key == control.run_id, "DERIVATION_DISPATCH_KEY_MISMATCH");
        heads.insert(key, (control.revision, control.paused));
    }
    for (key, receipt) in receipts {
        let request = receipt.request;
        let result = receipt.result;
        medium_identity(plans, &result)?;
        let task = plans.task(&request.task_id)?;
        command(
            &key,
            &plans.project,
            &request.request_id,
            request.expected_task_revision,
            request.expected_control_revision,
            task.revision,
            task.status == "cancelled",
        )?;
        ensure!(
            request.project_id == plans.project
                && result.task_id == request.task_id
                && result.object_id == request.object_id
                && result.run_id == request.run_id
                && result.paused == request.paused
                && result.revision == request.expected_control_revision + 1,
            "DERIVATION_DISPATCH_RECEIPT_MISMATCH"
        );
        insert(
            &mut history,
            &result.run_id,
            result.revision,
            request.expected_task_revision,
            result.paused,
        )?;
    }
    chains(heads, history)
}

fn coarse_identity(plans: &Plans, control: &coarse::Control) -> Result<()> {
    ensure!(
        control.schema_version == 1
            && control.project_id == plans.project
            && plans.task(&control.task_id)?.granularity == Granularity::Coarse,
        "DERIVATION_DISPATCH_IDENTITY_MISMATCH"
    );
    Ok(())
}

fn coarse(entities: &[Entity], plans: &Plans) -> Result<()> {
    let controls = records::<coarse::Control>(entities, "object_task_coarse_dispatch_control")?;
    let receipts = records::<coarse::Receipt>(entities, "object_task_coarse_dispatch_receipt")?;
    let mut heads = Controls::new();
    let mut history = History::new();
    for (key, control) in controls {
        coarse_identity(plans, &control)?;
        ensure!(key == control.task_id, "DERIVATION_DISPATCH_KEY_MISMATCH");
        heads.insert(key, (control.revision, control.paused));
    }
    for (key, receipt) in receipts {
        let request = receipt.request;
        let result = receipt.result;
        coarse_identity(plans, &result)?;
        let task = plans.task(&request.task_id)?;
        command(
            &key,
            &plans.project,
            &request.request_id,
            request.expected_task_revision,
            request.expected_control_revision,
            task.revision,
            task.status == "cancelled",
        )?;
        ensure!(
            request.project_id == plans.project
                && result.task_id == request.task_id
                && result.paused == request.paused
                && result.revision == request.expected_control_revision + 1,
            "DERIVATION_DISPATCH_RECEIPT_MISMATCH"
        );
        insert(
            &mut history,
            &result.task_id,
            result.revision,
            request.expected_task_revision,
            result.paused,
        )?;
    }
    chains(heads, history)
}

pub(crate) fn validate(entities: &[Entity], plans: &Plans) -> Result<()> {
    medium(entities, plans)?;
    coarse(entities, plans)
}
