use super::{
    confirmation,
    model::{Release, Run},
    release, repository, settings,
    test_support::Fixture,
};
use anyhow::Result;
use serde_json::{json, Value};
use std::fs;

fn start(fixture: &mut Fixture, request: &str) -> Result<Release> {
    let result = release::start(
        &mut fixture.store,
        &fixture.files,
        &json!({
            "projectId":"p", "requestId":request, "preset":"Windows Desktop"
        }),
    )?;
    repository::get(
        &fixture.store,
        "validationRelease",
        result["id"].as_str().unwrap(),
    )
}

fn export_input(release: &Release) -> Value {
    json!({"id":release.project_id,"releaseCheckId":release.id,
        "snapshotId":release.snapshot_id,"scopeId":release.scope_id,"preset":release.preset})
}

fn finish_runs(fixture: &mut Fixture, release: &Release) -> Result<()> {
    for id in &release.run_ids {
        let mut run: Run = repository::get(&fixture.store, "validationRun", id)?;
        if run.kind == "code" {
            fixture.passing_code(&mut run)?;
        } else {
            fixture.complete_visual(&mut run)?;
            let ids = run
                .evidence
                .iter()
                .map(|e| e.id.clone())
                .collect::<Vec<_>>();
            confirmation::confirm(
                &fixture.files,
                &mut fixture.store,
                &run.id,
                &run.snapshot_id,
                &ids,
                &format!("approve-{}", run.id),
                "ui",
            )?;
        }
    }
    Ok(())
}

#[test]
fn disabling_visual_checks_changes_only_new_candidates_and_never_waives_code() -> Result<()> {
    let mut fixture = Fixture::new()?;
    fixture.flow()?;
    let on = start(&mut fixture, "on")?;
    assert!(on.visual_required);
    assert_eq!(on.run_ids.len(), 2);
    let code_id = &on.run_ids[0];
    let mut code: Run = repository::get(&fixture.store, "validationRun", code_id)?;
    fixture.passing_code(&mut code)?;
    assert_eq!(release::inspect(&fixture.store, &on)?["ready"], false);
    settings::save(
        &mut fixture.store,
        &json!({"projectId":"p","requestId":"switch-off",
        "expectedRevision":0,"settings":{"visualRequired":false}}),
    )?;
    let repeated = start(&mut fixture, "on")?;
    assert_eq!(repeated.id, on.id);
    assert!(repeated.visual_required);
    let off = start(&mut fixture, "off")?;
    assert!(!off.visual_required);
    assert_eq!(off.run_ids.len(), 1);
    assert!(release::authorize(&fixture.store, &fixture.files, &export_input(&off)).is_err());
    finish_runs(&mut fixture, &off)?;
    assert!(release::authorize(&fixture.store, &fixture.files, &export_input(&off)).is_ok());
    assert_eq!(release::inspect(&fixture.store, &on)?["ready"], false);

    let mut code: Run = repository::get(&fixture.store, "validationRun", &off.run_ids[0])?;
    code.status = "failed".into();
    code.error = Some("A required assertion failed".into());
    fixture.save(&code)?;
    assert!(release::authorize(&fixture.store, &fixture.files, &export_input(&off)).is_err());
    Ok(())
}

#[test]
fn human_visual_approval_cannot_clear_code_failures_or_missing_task_coverage() -> Result<()> {
    let mut fixture = Fixture::new()?;
    let flow = fixture.flow()?;
    fixture.store.put(
        "validationCoverage",
        "task-1",
        &json!({
            "projectId":"p","taskId":"task-1","title":"Inventory","status":"missing"
        }),
    )?;
    let missing = start(&mut fixture, "missing")?;
    assert!(missing.missing.iter().any(|text| text.contains("task-1")));
    finish_runs(&mut fixture, &missing)?;
    assert!(release::authorize(&fixture.store, &fixture.files, &export_input(&missing)).is_err());

    let mut definition = flow.definition.clone();
    definition.task_ids.push("task-1".into());
    repository::save_flow(&mut fixture.store, "p", definition, flow.revision)?;
    let complete = start(&mut fixture, "complete")?;
    assert!(complete.missing.is_empty());
    finish_runs(&mut fixture, &complete)?;
    assert!(release::authorize(&fixture.store, &fixture.files, &export_input(&complete)).is_ok());
    assert!(release::authorize(&fixture.store, &fixture.files, &export_input(&missing)).is_err());
    let mut code: Run = repository::get(&fixture.store, "validationRun", &complete.run_ids[0])?;
    code.code.as_mut().unwrap().exit_code = Some(1);
    fixture.save(&code)?;
    assert_eq!(release::inspect(&fixture.store, &complete)?["ready"], false);
    assert!(release::authorize(&fixture.store, &fixture.files, &export_input(&complete)).is_err());
    Ok(())
}

#[test]
fn formal_export_is_bound_to_the_frozen_candidate_scope_and_artifacts() -> Result<()> {
    assert_frozen_candidate(Fixture::new()?)
}

#[test]
fn project_release_checks_use_project_evidence_and_reject_tampering() -> Result<()> {
    assert_frozen_candidate(Fixture::project()?)
}

fn assert_frozen_candidate(mut fixture: Fixture) -> Result<()> {
    fixture.flow()?;
    let release = start(&mut fixture, "candidate")?;
    finish_runs(&mut fixture, &release)?;
    let input = export_input(&release);
    fs::write(
        fixture.project.join("game.gd"),
        "extends Node\nvar value = 3\n",
    )?;
    let approved = release::authorize(&fixture.store, &fixture.files, &input)?;
    assert_eq!(approved.snapshot, release.snapshot);
    assert_ne!(approved.snapshot, fixture.files.capture(&fixture.project)?);
    for field in ["id", "snapshotId", "scopeId", "preset"] {
        let mut mismatched = input.clone();
        mismatched[field] = json!("different");
        assert!(
            release::authorize(&fixture.store, &fixture.files, &mismatched).is_err(),
            "{field}"
        );
    }
    let next = start(&mut fixture, "updated-candidate")?;
    assert_ne!(next.snapshot_id, release.snapshot_id);
    assert!(next.run_ids.iter().all(|id| !release.run_ids.contains(id)));
    assert!(release::authorize(&fixture.store, &fixture.files, &export_input(&next)).is_err());

    let code_path = repository::run_dir(&fixture.files, &release.run_ids[0])?.join("gut.xml");
    let original = fs::read(&code_path)?;
    fs::write(&code_path, b"truncated")?;
    assert!(release::authorize(&fixture.store, &fixture.files, &input).is_err());
    fs::write(&code_path, original)?;
    let visual: Run = repository::get(&fixture.store, "validationRun", &release.run_ids[1])?;
    let image = repository::run_dir(&fixture.files, &visual.id)?.join(&visual.evidence[0].file);
    fs::remove_file(image)?;
    assert!(release::authorize(&fixture.store, &fixture.files, &input).is_err());
    Ok(())
}

#[test]
fn absent_flows_and_foreign_receipts_cannot_silently_reduce_release_scope() -> Result<()> {
    let mut fixture = Fixture::new()?;
    let empty = start(&mut fixture, "empty")?;
    finish_runs(&mut fixture, &empty)?;
    assert!(!empty.missing.is_empty());
    assert_eq!(release::inspect(&fixture.store, &empty)?["ready"], false);
    fixture.flow()?;
    let complete = start(&mut fixture, "scoped")?;
    finish_runs(&mut fixture, &complete)?;
    let mut missing_run = complete.clone();
    missing_run.run_ids.pop();
    assert!(release::inspect(&fixture.store, &missing_run).is_err());
    let mut foreign_run = complete.clone();
    foreign_run.run_ids[0] = empty.run_ids[0].clone();
    assert!(release::inspect(&fixture.store, &foreign_run).is_err());
    let mut wrong_engine: Run =
        repository::get(&fixture.store, "validationRun", &complete.run_ids[1])?;
    wrong_engine.engine_version = "4.5.stable".into();
    fixture.save(&wrong_engine)?;
    assert_eq!(release::inspect(&fixture.store, &complete)?["ready"], false);
    Ok(())
}
