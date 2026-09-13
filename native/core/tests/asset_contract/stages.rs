use super::fixture::{acknowledge, stage, Fixture};
use anyhow::Result;
use beaver_core::{asset_feedback, asset_stages, asset_task};
use serde_json::json;

#[test]
fn stage_updates_preserve_history_and_reject_stale_or_invalid_graphs() -> Result<()> {
    let fixture = Fixture::new()?;
    let mut state = fixture.state()?;
    asset_stages::update(
        &mut state,
        0,
        vec![
            stage("body", "completed", &[]),
            stage("hair", "running", &["body"]),
        ],
    )?;
    let before = serde_json::to_value(&state)?;
    for (revision, proposed) in [
        (0, state.stages.clone()),
        (1, vec![stage("body", "completed", &[])]),
        (
            1,
            vec![
                stage("body", "completed", &["hair"]),
                stage("hair", "pending", &["body"]),
            ],
        ),
        (
            1,
            vec![
                stage("body", "completed", &[]),
                stage("hair", "pending", &["missing"]),
            ],
        ),
        (
            1,
            vec![
                stage("body", "checking", &[]),
                stage("hair", "running", &["body"]),
            ],
        ),
        (
            1,
            vec![
                stage("body", "completed", &[]),
                stage("body", "pending", &[]),
            ],
        ),
    ] {
        assert!(asset_stages::update(&mut state, revision, proposed).is_err());
        assert_eq!(serde_json::to_value(&state)?, before);
    }
    let mut no_evidence = state.stages.clone();
    no_evidence[0].evidence.clear();
    assert!(asset_stages::update(&mut state, 1, no_evidence).is_err());
    Ok(())
}

#[test]
fn nose_feedback_finishes_only_after_interrupted_hair_resumes() -> Result<()> {
    let mut fixture = Fixture::new()?;
    let mut state = fixture.state()?;
    asset_stages::update(
        &mut state,
        0,
        vec![
            stage("body", "completed", &[]),
            stage("hair", "running", &["body"]),
        ],
    )?;
    asset_task::save(&fixture.store, &state)?;
    let input = fixture.input("nose", "now")?;
    fixture.submit(input)?;
    state = fixture.delivered("nose")?;
    acknowledge(&mut state, "nose", &["body", "hair"])?;
    let mut stages = state.stages.clone();
    stages[1].status = "suspended".into();
    stages.push(stage("nose", "running", &["body"]));
    asset_stages::update(&mut state, 1, stages)?;
    assert_eq!(state.feedback[0].resume_stages, ["hair"]);
    state.feedback[0].checkpoint = Some("saved-before-nose.blend".into());
    asset_feedback::receipt(&mut state, "nose", "execute", &json!({}))?;
    asset_feedback::receipt(&mut state, "nose", "check", &json!({}))?;
    let mut stages = state.stages.clone();
    stages[2] = stage("nose", "completed", &["body"]);
    asset_stages::update(&mut state, 2, stages)?;
    let evidence = json!({"evidence":"Nose extent reduced, face intact, hair work resumed"});
    assert!(asset_feedback::receipt(&mut state, "nose", "complete", &evidence).is_err());
    let mut stages = state.stages.clone();
    stages[1].status = "running".into();
    asset_stages::update(&mut state, 3, stages)?;
    asset_feedback::receipt(&mut state, "nose", "complete", &evidence)?;
    assert_eq!(state.feedback[0].status, "completed");
    assert_eq!(state.stages[1].status, "running");
    assert!(asset_stages::complete_round(&mut state).is_err());
    Ok(())
}

#[test]
fn completed_clothing_cannot_skip_dependency_revalidation_as_suspended_work() -> Result<()> {
    let mut fixture = Fixture::new()?;
    let mut state = fixture.state()?;
    asset_stages::update(
        &mut state,
        0,
        vec![
            stage("body", "completed", &[]),
            stage("clothing", "completed", &["body"]),
        ],
    )?;
    asset_task::save(&fixture.store, &state)?;
    let input = fixture.input("proportions", "now")?;
    fixture.submit(input)?;
    state = fixture.delivered("proportions")?;
    acknowledge(&mut state, "proportions", &["body", "clothing"])?;
    let mut stages = state.stages.clone();
    stages[1].status = "suspended".into();
    asset_stages::update(&mut state, 1, stages)?;
    assert!(state.feedback[0].resume_stages.is_empty());
    state.feedback[0].checkpoint = Some("saved-proportions.blend".into());
    asset_feedback::receipt(&mut state, "proportions", "execute", &json!({}))?;
    asset_feedback::receipt(&mut state, "proportions", "check", &json!({}))?;
    let evidence = json!({"evidence":"Body adjusted; clothing still needs clearance checks"});
    assert!(asset_feedback::receipt(&mut state, "proportions", "complete", &evidence).is_err());
    let mut stages = state.stages.clone();
    stages[1] = stage("clothing", "running", &["body"]);
    asset_stages::update(&mut state, 2, stages)?;
    assert!(asset_feedback::receipt(&mut state, "proportions", "complete", &evidence).is_err());
    let mut stages = state.stages.clone();
    stages[1] = stage("clothing", "completed", &["body"]);
    asset_stages::update(&mut state, 3, stages)?;
    asset_feedback::receipt(
        &mut state,
        "proportions",
        "complete",
        &json!({"evidence":"Adjusted clothing fits the new body without intersections"}),
    )?;
    Ok(())
}
