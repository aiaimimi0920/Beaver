use crate::{
    asset_reference,
    asset_task::{self, Annotation, Feedback, Reference, State},
    store::Store,
};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Submission {
    pub id: String,
    pub feedback_id: String,
    pub timing: String,
    pub text: String,
    pub reference_id: String,
    #[serde(default)]
    pub annotations: Vec<Annotation>,
}

impl Submission {
    pub fn fingerprint(&self) -> Result<String> {
        Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(self)?)))
    }
    pub fn validate(&self) -> Result<()> {
        asset_task::validate_id(&self.id)?;
        asset_task::validate_id(&self.reference_id)?;
        asset_task::validate_id(&self.feedback_id)?;
        if !["now", "afterRound"].contains(&self.timing.as_str())
            || self.text.trim().is_empty()
            || self.text.encode_utf16().count() > 12000
        {
            bail!("Invalid asset feedback");
        }
        asset_reference::validate_annotations(&self.annotations)
    }
}

pub fn duplicate(state: &State, input: &Submission) -> Result<Option<Feedback>> {
    if state.withdrawn.contains(&input.feedback_id) {
        bail!("Feedback submission was withdrawn; use a new feedback ID");
    }
    if let Some(feedback) = state.feedback.iter().find(|f| f.id == input.feedback_id) {
        if feedback.source_task_id != input.id || feedback.fingerprint != input.fingerprint()? {
            bail!("Feedback ID already belongs to different content");
        }
        return Ok(Some(feedback.clone()));
    }
    Ok(None)
}

pub fn build(state: &mut State, input: Submission, mut reference: Reference) -> Result<Feedback> {
    input.validate()?;
    if state.feedback.len() >= asset_task::MAX_FEEDBACK {
        bail!("Task feedback limit reached; finish this task before adding more");
    }
    let open_round = state.phase != "ready";
    if !open_round {
        state.round += 1;
        state.phase = "adjusting".into();
    }
    let round = if input.timing == "afterRound" && open_round {
        state.round + 1
    } else {
        state.round
    };
    reference.used = true;
    Ok(Feedback {
        id: input.feedback_id.clone(),
        source_task_id: input.id.clone(),
        task_id: state.task_id.clone(),
        fingerprint: input.fingerprint()?,
        timing: input.timing,
        text: input.text,
        reference,
        annotations: input.annotations,
        round,
        status: "received".into(),
        created_at: asset_task::now(),
        delivered_at: None,
        impact: String::new(),
        affected_stages: Vec::new(),
        resume_stages: Vec::new(),
        image_observation: String::new(),
        evidence: String::new(),
        checkpoint: None,
        history: Vec::new(),
    })
}

pub fn accept(store: &Store, input: Submission, reference: Reference) -> Result<Feedback> {
    let mut state = asset_task::get(store, &input.id)?;
    if let Some(existing) = duplicate(&state, &input)? {
        return Ok(existing);
    }
    let feedback = build(&mut state, input, reference)?;
    state.feedback.push(feedback.clone());
    // Pin first: a crash may retain an unused image, but can never delete an accepted one.
    store.put(
        "asset-reference",
        &feedback.reference.id,
        &feedback.reference,
    )?;
    asset_task::save(store, &state)?;
    Ok(feedback)
}

pub fn reserve_delivery(state: &mut State) {
    let id = asset_task::eligible(state).map(|f| f.id.clone());
    if let Some(feedback) = state
        .feedback
        .iter_mut()
        .find(|f| Some(&f.id) == id.as_ref())
    {
        if feedback.status == "received" {
            feedback.status = "delivering".into();
            feedback
                .history
                .push(json!({"at":asset_task::now(),"status":"delivering"}));
        }
    }
}

pub fn receipt(state: &mut State, id: &str, action: &str, input: &Value) -> Result<()> {
    let current = asset_task::eligible(state).context("No eligible feedback in this round")?;
    if current.id != id {
        bail!("Process the earlier feedback first; scene writes are serialized");
    }
    let known: Vec<_> = state.stages.iter().map(|s| s.id.clone()).collect();
    let unfinished: Vec<_> = state
        .stages
        .iter()
        .filter(|s| s.status != "completed")
        .map(|s| s.id.clone())
        .collect();
    let not_resumed: Vec<_> = state
        .stages
        .iter()
        .filter(|s| !matches!(s.status.as_str(), "running" | "completed"))
        .map(|s| s.id.clone())
        .collect();
    let feedback = state
        .feedback
        .iter_mut()
        .find(|f| f.id == id)
        .context("Unknown feedback")?;
    let evidence = input["evidence"].as_str().unwrap_or("").trim();
    if evidence.len() > 12000 {
        bail!("Evidence too long");
    }
    match action {
        "acknowledge" => {
            if feedback.status != "waitingSwitch" {
                bail!("Read feedback including its image before acknowledging");
            }
            let observation = input["imageObservation"].as_str().unwrap_or("").trim();
            let impact = input["impact"].as_str().unwrap_or("").trim();
            let affected: Vec<String> = serde_json::from_value(input["affectedStages"].clone())?;
            if observation.is_empty()
                || impact.is_empty()
                || observation.len() > 12000
                || impact.len() > 12000
                || input["frameId"] != feedback.reference.frame.id
                || affected.iter().any(|id| !known.contains(id))
            {
                bail!("Identify the received image, describe its visible target and analyze actual stage dependencies");
            }
            feedback.image_observation = observation.into();
            feedback.impact = impact.into();
            feedback.affected_stages = affected;
            feedback.status = "acknowledged".into();
        }
        "deciding" if feedback.status == "acknowledged" => feedback.status = "deciding".into(),
        "execute" if matches!(feedback.status.as_str(), "acknowledged" | "deciding") => {
            if feedback.checkpoint.is_none() {
                bail!("A real recovery scene must be saved before execution");
            }
            feedback.status = "executing".into();
        }
        "check" if feedback.status == "executing" => feedback.status = "checking".into(),
        "complete" if feedback.status == "checking" => {
            if evidence.is_empty() {
                bail!("Modification completion requires verification evidence");
            }
            if feedback
                .affected_stages
                .iter()
                .any(|id| unfinished.contains(id) && !feedback.resume_stages.contains(id))
            {
                bail!("Verify affected stages before completing feedback");
            }
            if feedback
                .resume_stages
                .iter()
                .any(|id| not_resumed.contains(id))
            {
                bail!("Resume the interrupted production stages before completing feedback");
            }
            feedback.status = "completed".into();
        }
        "verifyApplied" if feedback.status == "pendingVerification" => {
            if evidence.is_empty() {
                bail!("Inspect saved scene and results before resolving uncertain execution");
            }
            feedback.status = "checking".into();
        }
        "verifyNotApplied" if feedback.status == "pendingVerification" => {
            if evidence.is_empty() {
                bail!("Explain current geometry and recovery source before retrying a relative modification");
            }
            feedback.status = "acknowledged".into();
        }
        "fail" => {
            if evidence.is_empty() {
                bail!("Failure requires an explanation and recovery location");
            }
            feedback.status = "failed".into();
        }
        "retryFailed" if feedback.status == "failed" => {
            if evidence.is_empty() {
                bail!("Describe failure and recovery source; inspect before retrying");
            }
            feedback.status = "pendingVerification".into();
        }
        _ => bail!(
            "Invalid feedback transition: {} -> {action}",
            feedback.status
        ),
    }
    feedback.evidence = evidence.into();
    feedback
        .history
        .push(json!({"at":asset_task::now(),"status":feedback.status,"evidence":evidence}));
    if feedback.history.len() > 64 {
        feedback.history.remove(0);
    }
    Ok(())
}
