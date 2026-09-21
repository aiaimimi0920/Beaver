use crate::asset_task::{self, Stage, State};
use anyhow::{bail, Result};
use std::collections::{HashMap, HashSet};

pub fn update(state: &mut State, revision: u64, stages: Vec<Stage>) -> Result<()> {
    if revision != state.revision {
        bail!("Stage revision changed; read current state before retrying");
    }
    crate::asset_delivery::guard_update(state, &stages)?;
    if stages.is_empty() || stages.len() > 100 {
        bail!("Supply 1 to 100 actual production stages");
    }
    let mut ids = HashSet::new();
    for stage in &stages {
        asset_task::validate_id(&stage.id)?;
        if !ids.insert(stage.id.as_str())
            || stage.name.trim().is_empty()
            || stage.name.len() > 300
            || stage.evidence.len() > 12000
            || stage.objects.len() > 100
            || stage.dependencies.len() > 100
            || stage.round == 0
            || stage.round > state.round
        {
            bail!("Invalid or duplicate production stage");
        }
        if ![
            "pending",
            "running",
            "completed",
            "suspended",
            "deciding",
            "adjusting",
            "checking",
            "failed",
        ]
        .contains(&stage.status.as_str())
        {
            bail!("Invalid stage status");
        }
        if stage.status == "completed" && stage.evidence.trim().is_empty() {
            bail!("Completed stages require result evidence");
        }
    }
    for previous in &state.stages {
        if !ids.contains(previous.id.as_str()) {
            bail!(
                "Keep existing stable stage IDs; revise their states rather than deleting history"
            );
        }
    }
    let graph: HashMap<_, _> = stages.iter().map(|s| (s.id.as_str(), s)).collect();
    let mut visited = HashSet::new();
    for stage in &stages {
        let mut visiting = HashSet::new();
        visit(&stage.id, &graph, &mut visiting, &mut visited)?;
        if stage.status == "running"
            && stage
                .dependencies
                .iter()
                .any(|id| graph[id.as_str()].status != "completed")
        {
            bail!("Running stage has unfinished dependencies");
        }
    }
    let paused: Vec<_> = state
        .stages
        .iter()
        .filter(|previous| {
            previous.status == "running" && graph[previous.id.as_str()].status == "suspended"
        })
        .map(|s| s.id.clone())
        .collect();
    if let Some(feedback) = state
        .feedback
        .iter_mut()
        .find(|f| !f.terminal() && f.round <= state.round)
    {
        if matches!(
            feedback.status.as_str(),
            "waitingSwitch" | "acknowledged" | "deciding"
        ) {
            for id in paused {
                if !feedback.resume_stages.contains(&id) {
                    feedback.resume_stages.push(id);
                }
            }
        }
    }
    state.stages = stages;
    if state.phase == "ready" && state.stages.iter().any(|s| s.status != "completed") {
        state.phase = "adjusting".into();
        state.round += 1;
    }
    state.revision += 1;
    Ok(())
}

fn visit<'a>(
    id: &'a str,
    graph: &HashMap<&'a str, &'a Stage>,
    visiting: &mut HashSet<&'a str>,
    visited: &mut HashSet<&'a str>,
) -> Result<()> {
    if visited.contains(id) {
        return Ok(());
    }
    let Some(stage) = graph.get(id) else {
        bail!("Unknown stage dependency: {id}");
    };
    if !visiting.insert(id) {
        bail!("Stage dependency cycle");
    }
    for dependency in &stage.dependencies {
        visit(dependency, graph, visiting, visited)?;
    }
    visiting.remove(id);
    visited.insert(id);
    Ok(())
}

pub fn complete_round(state: &mut State) -> Result<()> {
    anyhow::ensure!(
        crate::asset_delivery::complete(state),
        "Delivery stages require owner approval before finishing the round"
    );
    if state.phase == "ready" {
        return Ok(());
    }
    if state.stages.is_empty() || state.stages.iter().any(|s| s.status != "completed") {
        bail!("Finish and verify all production stages before closing the round");
    }
    if state
        .feedback
        .iter()
        .any(|f| !f.terminal() && f.round <= state.round)
    {
        bail!("Current-round feedback is not verified");
    }
    if state.feedback.iter().any(|f| !f.terminal()) {
        state.round += 1;
        state.phase = "adjusting".into();
    } else {
        state.phase = "ready".into();
    }
    state.revision += 1;
    Ok(())
}
