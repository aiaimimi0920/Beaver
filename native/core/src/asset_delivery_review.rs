use crate::{
    asset_delivery_files::{self as artifacts, Candidate},
    asset_task,
    files::{Files, Snapshot},
    store::Store,
    task_callback::{get, put},
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Decision {
    pub id: String,
    pub request_id: String,
    pub expected_revision: u64,
    pub candidate_id: String,
    pub decision: String,
    pub note: String,
}

pub struct Prepared {
    input: Decision,
    expected: Snapshot,
    inputs: Vec<crate::asset_work_inputs::Captured>,
    workspace: std::path::PathBuf,
    configuration: crate::framework_contract::Configuration,
}

pub struct Verified(Decision, crate::framework_contract::Configuration);

impl Prepared {
    /// Run after the old executor stops, without holding the database mutex.
    pub fn verify(self, files: &Files) -> Result<Verified> {
        if self.input.decision == "approve" {
            let workspace = files.resolve_workspace(&self.input.id, &self.workspace)?;
            artifacts::verify_workspace(files, &workspace, &self.expected)?;
            crate::asset_work_inputs::verify(files, &workspace, &self.inputs)?;
            for rule in &self.configuration.rules {
                crate::framework_command::verify(&rule.checker)?;
            }
        }
        Ok(Verified(self.input, self.configuration))
    }
}

pub fn prepare(store: &Store, input: Decision) -> Result<Prepared> {
    let candidate = check(store, &input)?;
    let state = asset_task::get(store, &input.id)?;
    let inputs = crate::asset_work_inputs::for_attempts(&state, &candidate.attempt_ids)?;
    let flow = state.delivery.context("Missing workflow")?;
    let mut expected = artifacts::approved_files(store, &input.id, &flow.approved)?;
    expected.extend(candidate.files);
    let task: Value = store.get("task", &input.id)?.context("Missing task")?;
    Ok(Prepared {
        configuration: crate::framework::configuration(store, &input.id)?,
        input,
        expected,
        inputs,
        workspace: task["workspace"]
            .as_str()
            .context("Missing workspace")?
            .into(),
    })
}

pub fn duplicate(store: &Store, input: &Decision) -> Result<Option<Value>> {
    let saved: Option<Value> = store.get(
        &format!("asset-delivery-decisions/{}", input.id),
        &input.request_id,
    )?;
    saved
        .map(|saved| {
            ensure!(
                saved["request"] == json!(input),
                "Decision request ID conflict"
            );
            Ok(saved["response"].clone())
        })
        .transpose()
}

pub fn check(store: &Store, input: &Decision) -> Result<Candidate> {
    crate::framework_operations::idle(store, &input.id)?;
    asset_task::validate_id(&input.id)?;
    asset_task::validate_id(&input.request_id)?;
    ensure!(
        ["approve", "reject", "reopen"].contains(&input.decision.as_str()),
        "Invalid delivery decision"
    );
    ensure!(
        input.note.len() <= 12000 && (input.decision == "approve" || !input.note.trim().is_empty()),
        "Rejection and rework require a reason of at most 12000 bytes"
    );
    let task: Value = store
        .get("task", &input.id)?
        .context("Task does not exist")?;
    crate::object_framework::require_legacy(&task)?;
    ensure!(
        !task["clarifications"].as_array().is_some_and(|items| items
            .iter()
            .any(|q| q.get("answers").is_none_or(Value::is_null))),
        "Resolve the pending clarification first"
    );
    ensure!(
        ["awaitingInput", "interrupted", "failed"].contains(&task["status"].as_str().unwrap_or("")),
        "Wait for submission, or interrupt the task before requesting rework"
    );
    let state = asset_task::get(store, &input.id)?;
    ensure!(
        state.revision == input.expected_revision,
        "Asset revision changed; inspect the current candidate"
    );
    let flow = state.delivery.context("Delivery workflow is not enabled")?;
    if input.decision == "reopen" {
        ensure!(
            flow.pending.is_none() && flow.approved.contains(&input.candidate_id),
            "Reject the pending candidate first, or select a currently approved candidate"
        );
    } else {
        ensure!(
            flow.pending.as_deref() == Some(&input.candidate_id),
            "Candidate is no longer pending review"
        );
    }
    let candidate = artifacts::get(store, &input.id, &input.candidate_id)?;
    if input.decision == "approve" {
        crate::framework_checks::gate(store, &input.id, &candidate)?;
    }
    Ok(candidate)
}

/// Caller synchronizes the executor before preparing the verified decision.
pub fn decide(store: &mut Store, verified: Verified) -> Result<Value> {
    let input = &verified.0;
    if let Some(result) = duplicate(store, input)? {
        return Ok(result);
    }
    check(store, input)?;
    ensure!(
        crate::framework::configuration(store, &input.id)? == verified.1,
        "Framework configuration changed during approval verification"
    );
    store.transaction(|db| {
        let mut state: asset_task::State = get(db, "asset-task", &input.id)?.context("Missing asset state")?;
        let mut task: Value = get(db, "task", &input.id)?.context("Missing task")?;
        crate::object_framework::require_legacy(&task)?;
        let flow = state.delivery.as_mut().context("Missing workflow")?;
        let index = if input.decision == "reopen" {
            flow.approved.iter().position(|id| id == &input.candidate_id).context("Candidate no longer approved")?
        } else { flow.approved.len() };
        if input.decision == "approve" {
            flow.approved.push(input.candidate_id.clone());
            state.stages[index].status = "completed".into();
            state.stages[index].evidence = format!("Owner approved frozen candidate {}", input.candidate_id);
            if let Some(next) = state.stages.get_mut(index + 1) { next.status = "running".into(); }
        } else {
            flow.approved.truncate(index);
            for (position, stage) in state.stages.iter_mut().enumerate().skip(index) {
                stage.status = if position == index { "running" } else { "pending" }.into();
                stage.evidence.clear();
                stage.objects.clear();
            }
        }
        flow.pending = None;
        if input.decision != "approve" {
            crate::asset_work::invalidate(&mut state, index, &input.note);
        }
        state.phase = "producing".into();
        state.revision += 1;
        task.as_object_mut().context("Invalid task")?.remove("waitingDelivery");
        task.as_object_mut().unwrap().remove("turnId");
        task.as_object_mut().unwrap().remove("error");
        task["status"] = json!("queued");
        task["validationOnly"] = json!(false);
        task["validationPrepared"] = json!(false);
        task["updatedAt"] = json!(asset_task::now());
        task["prompt"] = json!(format!("{}\n\nBeaver owner delivery decision: {} for candidate {}. Note: {}\nRead beaver_task state for the current stage and approved input candidates. Inspect saved files and scene before editing. Do not repeat applied changes. Only the current stage may execute. When all stages are approved, finish the asset round without modifying approved files.", task["prompt"].as_str().unwrap_or(""), input.decision, input.candidate_id, input.note));
        let response = json!({"candidateId":input.candidate_id,"decision":input.decision,"assetRevision":state.revision,"status":"queued"});
        let record = json!({"request":input,"response":response,"source":"owner","fileCheck":if input.decision == "approve" { "frozenAndWorkspaceHashes" } else { "notRequired" },"time":asset_task::now()});
        put(db, &format!("asset-delivery-decisions/{}", input.id), &input.request_id, &record)?;
        put(db, "asset-task", &input.id, &state)?;
        put(db, "task", &input.id, &task)?;
        db.execute("INSERT INTO events(task,time,kind,text) VALUES(?,?,?,?)", rusqlite::params![input.id, asset_task::now(), "assetDeliveryDecision", record.to_string()])?;
        Ok(response)
    })
}

pub fn inspect(store: &Store, id: &str) -> Result<Value> {
    let state = asset_task::get(store, id)?;
    let candidates = state
        .delivery
        .as_ref()
        .map(|flow| {
            flow.history
                .iter()
                .rev()
                .map(|id_| artifacts::get(store, id, id_))
                .collect::<Result<Vec<_>>>()
        })
        .transpose()?
        .unwrap_or_default();
    let decisions: Vec<Value> = store.list(&format!("asset-delivery-decisions/{id}"))?;
    Ok(
        json!({"revision":state.revision,"workflow":state.delivery,"work":state.work,"candidates":candidates,"decisions":decisions}),
    )
}
