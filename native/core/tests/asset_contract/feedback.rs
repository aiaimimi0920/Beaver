use super::fixture::{acknowledge, stage, Fixture};
use anyhow::Result;
use beaver_core::{asset_feedback, asset_stages, asset_task, asset_tool, asset_withdrawal};
use serde_json::json;

#[test]
fn retries_are_idempotent_and_conflicting_content_cannot_replace_a_receipt() -> Result<()> {
    let mut fixture = Fixture::new()?;
    let input = fixture.input("request", "now")?;
    let accepted = fixture.submit(input.clone())?;
    let retry = fixture.submit(input.clone())?;
    assert_eq!(retry.id, accepted.id);
    assert_eq!(retry.created_at, accepted.created_at);
    assert_eq!(fixture.state()?.feedback.len(), 1);
    let mut changed = input;
    changed.text = "Make the nose larger".into();
    assert!(fixture.submit(changed).is_err());
    assert_eq!(fixture.state()?.feedback[0].text, accepted.text);
    Ok(())
}

#[test]
fn withdrawal_tombstone_wins_over_a_delayed_submit_but_not_delivered_work() -> Result<()> {
    let mut fixture = Fixture::new()?;
    let delayed = fixture.input("delayed", "now")?;
    asset_withdrawal::cancel(&fixture.store, &fixture.id, "delayed")?;
    asset_withdrawal::cancel(&fixture.store, &fixture.id, "delayed")?;
    assert!(fixture.submit(delayed).is_err());
    let input = fixture.input("visible", "now")?;
    fixture.submit(input.clone())?;
    asset_withdrawal::cancel(&fixture.store, &fixture.id, "visible")?;
    assert!(fixture.submit(input).is_err());
    let input = fixture.input("inflight", "now")?;
    fixture.submit(input)?;
    let mut state = fixture.state()?;
    asset_feedback::reserve_delivery(&mut state);
    asset_task::save(&fixture.store, &state)?;
    assert!(asset_withdrawal::cancel(&fixture.store, &fixture.id, "inflight").is_err());
    assert_eq!(fixture.state()?.withdrawn, ["delayed", "visible"]);
    Ok(())
}

#[test]
fn receipts_require_image_identity_impact_checkpoint_and_verification() -> Result<()> {
    let mut fixture = Fixture::new()?;
    let input = fixture.input("edit", "now")?;
    fixture.submit(input)?;
    let mut state = fixture.state()?;
    assert!(acknowledge(&mut state, "edit", &[]).is_err());
    state = fixture.delivered("edit")?;
    for input in [
        json!({}),
        json!({"frameId":"other","imageObservation":"visible","impact":"local","affectedStages":[]}),
        json!({"frameId":"frame","imageObservation":"visible","impact":"local","affectedStages":["missing"]}),
    ] {
        assert!(asset_feedback::receipt(&mut state, "edit", "acknowledge", &input).is_err());
    }
    acknowledge(&mut state, "edit", &[])?;
    assert!(asset_feedback::receipt(&mut state, "edit", "execute", &json!({})).is_err());
    state.feedback[0].checkpoint = Some("recovery.blend".into());
    asset_feedback::receipt(&mut state, "edit", "execute", &json!({}))?;
    assert!(asset_feedback::receipt(
        &mut state,
        "edit",
        "complete",
        &json!({"evidence":"not checked"})
    )
    .is_err());
    asset_feedback::receipt(&mut state, "edit", "check", &json!({}))?;
    assert!(asset_feedback::receipt(&mut state, "edit", "complete", &json!({})).is_err());
    asset_feedback::receipt(
        &mut state,
        "edit",
        "complete",
        &json!({"evidence":"Measured local extent and inspected silhouette"}),
    )?;
    assert!(state.feedback[0].terminal());
    Ok(())
}

#[test]
fn restart_requires_explicit_geometry_verification_before_relative_edit_replay() -> Result<()> {
    let mut fixture = Fixture::new()?;
    let input = fixture.input("relative", "now")?;
    fixture.submit(input)?;
    let mut state = fixture.delivered("relative")?;
    acknowledge(&mut state, "relative", &[])?;
    state.feedback[0].checkpoint = Some("before.blend".into());
    asset_feedback::receipt(&mut state, "relative", "execute", &json!({}))?;
    asset_task::save(&fixture.store, &state)?;
    asset_task::recover_all(&fixture.store)?;
    state = fixture.state()?;
    assert_eq!(state.session_id, None);
    assert_eq!(state.feedback[0].status, "pendingVerification");
    assert!(asset_feedback::receipt(&mut state, "relative", "execute", &json!({})).is_err());
    assert!(
        asset_feedback::receipt(&mut state, "relative", "verifyNotApplied", &json!({})).is_err()
    );
    let mut absent = state.clone();
    asset_feedback::receipt(
        &mut absent,
        "relative",
        "verifyNotApplied",
        &json!({"evidence":"Loaded before.blend; old dimensions remain"}),
    )?;
    assert_eq!(absent.feedback[0].status, "acknowledged");
    asset_feedback::receipt(
        &mut state,
        "relative",
        "verifyApplied",
        &json!({"evidence":"Saved scene already contains target dimensions"}),
    )?;
    assert!(asset_feedback::receipt(&mut state, "relative", "execute", &json!({})).is_err());
    asset_feedback::receipt(
        &mut state,
        "relative",
        "complete",
        &json!({"evidence":"Verified applied result; no second relative edit"}),
    )?;
    Ok(())
}

#[test]
fn deferred_content_stays_out_of_turns_until_the_production_round_finishes() -> Result<()> {
    let mut fixture = Fixture::new()?;
    let input = fixture.input("later", "afterRound")?;
    fixture.submit(input)?;
    let immediate = fixture.input("now", "now")?;
    fixture.submit(immediate)?;
    let state = fixture.state()?;
    assert_eq!(asset_task::eligible(&state).unwrap().id, "now");
    assert_eq!(asset_tool::projection(&state)["deferredCount"], 1);
    asset_withdrawal::cancel(&fixture.store, &fixture.id, "now")?;
    asset_task::recover_all(&fixture.store)?;
    let mut state = fixture.state()?;
    assert!(asset_task::eligible(&state).is_none());
    let projection = asset_tool::projection(&state).to_string();
    assert!(!projection.contains("Make the nose smaller"));
    assert!(!projection.contains("referenceId"));
    assert!(asset_stages::complete_round(&mut state).is_err());
    asset_stages::update(&mut state, 0, vec![stage("body", "completed", &[])])?;
    asset_stages::complete_round(&mut state)?;
    assert_eq!(state.round, 2);
    assert_eq!(state.phase, "adjusting");
    assert_eq!(asset_task::eligible(&state).unwrap().id, "later");
    assert!(asset_stages::complete_round(&mut state).is_err());
    Ok(())
}
