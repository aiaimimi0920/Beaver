use super::{
    code, comparison, confirmation, evidence, feedback_context, operations, repository,
    sandbox::Sandbox, service::Service, test_support::Fixture,
};
use crate::project_storage::ProjectStore;
use anyhow::Result;
use serde_json::json;
use std::{
    fs,
    sync::{Arc, Mutex},
};

#[test]
fn sandbox_and_reports_use_project_content_after_close_and_relocation() -> Result<()> {
    let fixture = Fixture::project()?;
    fs::write(
        fixture.project.join("beaver.validation.json"),
        r#"{"code":{"directories":["tests"]}}"#,
    )?;
    let mut run = fixture.run(None)?;
    fixture.passing_code(&mut run)?;
    fs::write(fixture.project.join("game.gd"), "changed live source")?;
    fs::write(
        fixture.project.join("beaver.validation.json"),
        "invalid live config",
    )?;
    code::validate(&fixture.files, &run)?;
    let expected = fs::canonicalize(&fixture.project)?
        .join(".beaver/evidence")
        .join(&run.id);
    let sandbox = Sandbox::prepare(&fixture.files, &run)?;
    assert_eq!(sandbox.output, expected);
    assert_eq!(
        fs::read_to_string(sandbox.project.join("game.gd"))?,
        "extends Node\nvar value = 1\n"
    );
    assert!(!fixture.project.join(".beaver/blobs").exists());
    assert!(!fixture.project.join(".beaver/validation").exists());
    drop(sandbox);

    let Fixture {
        store,
        files,
        project,
        _temp,
    } = fixture;
    drop(store);
    drop(files);
    let moved = _temp.path().join("relocated");
    fs::rename(&project, &moved)?;
    let runtime = ProjectStore::open(&moved, "p")?.into_runtime();
    let stored = repository::get(&runtime.store().lock().unwrap(), "validationRun", &run.id)?;
    code::validate(&runtime.files(), &stored)?;
    let sandbox = Sandbox::prepare(&runtime.files(), &stored)?;
    assert!(sandbox.output.starts_with(fs::canonicalize(&moved)?));
    assert_eq!(
        fs::read_to_string(sandbox.project.join("game.gd"))?,
        "extends Node\nvar value = 1\n"
    );
    Ok(())
}

#[test]
fn evidence_reads_are_isolated_non_creating_and_never_fall_back_to_legacy() -> Result<()> {
    let mut first = Fixture::project()?;
    let second = Fixture::project()?;
    let flow = first.flow()?;
    let run = first.visual(&flow)?;
    evidence::validate(&first.files, &run)?;
    let absent = repository::run_dir(&second.files, &run.id)?;
    assert!(!absent.exists());
    let legacy = second.project.join(".beaver/validation").join(&run.id);
    fs::create_dir_all(&legacy)?;
    for item in &run.evidence {
        fs::copy(
            evidence::media_path(&first.files, &run, &item.id)?,
            legacy.join(&item.file),
        )?;
    }
    assert!(evidence::validate(&second.files, &run).is_err());
    assert!(evidence::media_path(&second.files, &run, &run.evidence[0].id).is_err());
    assert!(!absent.exists());
    for id in ["bad", "../outside", "/absolute", "..\\outside"] {
        assert!(repository::run_dir(&second.files, id).is_err());
    }
    let legacy_fixture = Fixture::new()?;
    let path = repository::run_dir(&legacy_fixture.files, &run.id)?;
    assert_eq!(
        path,
        fs::canonicalize(legacy_fixture.files.root())?
            .join("validation")
            .join(&run.id)
    );
    assert!(!path.exists());
    Ok(())
}

#[test]
fn project_baselines_and_confirmations_reject_changed_media() -> Result<()> {
    let mut fixture = Fixture::project()?;
    let flow = fixture.flow()?;
    let baseline = fixture.visual(&flow)?;
    let ids: Vec<_> = baseline
        .evidence
        .iter()
        .map(|item| item.id.clone())
        .collect();
    confirmation::confirm(
        &fixture.files,
        &mut fixture.store,
        &baseline.id,
        &baseline.snapshot_id,
        &ids,
        "approve",
        "ui",
    )?;
    let mut candidate = fixture.visual(&flow)?;
    comparison::compare(&fixture.files, &fixture.store, &mut candidate)?;
    assert_eq!(candidate.verdict, "autoPassed");
    fs::write(
        evidence::media_path(&fixture.files, &baseline, &ids[0])?,
        "changed",
    )?;
    assert!(comparison::compare(&fixture.files, &fixture.store, &mut candidate).is_err());
    assert!(confirmation::confirm(
        &fixture.files,
        &mut fixture.store,
        &baseline.id,
        &baseline.snapshot_id,
        &ids,
        "approve",
        "ui"
    )
    .is_err());
    Ok(())
}

#[test]
fn feedback_and_repairs_freeze_project_media_and_recorded_sources() -> Result<()> {
    let mut fixture = Fixture::project()?;
    let flow = fixture.flow()?;
    let run = fixture.visual(&flow)?;
    fs::write(fixture.project.join("game.gd"), "changed live source")?;
    let source = operations::source(
        &fixture.store,
        &fixture.files,
        &json!({
            "projectId":"p", "runId":run.id, "path":"game.gd"
        }),
    )?;
    assert_eq!(source["historical"], "extends Node\nvar value = 1\n");
    assert_eq!(source["current"], "changed live source");
    let workspace = fixture.files.workspace("feedback")?;
    fs::create_dir_all(&workspace)?;
    let feedback =
        json!({"text":"Change the selection", "selection":{"evidenceId":run.evidence[0].id}});
    feedback_context::freeze_feedback(&fixture.files, &workspace, &run, &feedback)?;
    feedback_context::freeze_repair(&fixture.files, &workspace, &run, &feedback)?;
    let context = workspace.join(".beaver-context/validation");
    for directory in [context.clone(), context.join("repairs").join(&run.id)] {
        assert_eq!(
            fs::read_to_string(directory.join("source/game.gd"))?,
            source["historical"].as_str().unwrap()
        );
        assert_eq!(
            fs::read(directory.join(format!("media/{}.png", run.evidence[0].id)))?,
            fs::read(evidence::media_path(
                &fixture.files,
                &run,
                &run.evidence[0].id
            )?)?
        );
    }
    fs::write(
        fixture.files.blob(&run.snapshot["game.gd"])?,
        "changed blob",
    )?;
    assert!(operations::source(
        &fixture.store,
        &fixture.files,
        &json!({
            "projectId":"p", "runId":run.id, "path":"game.gd"
        })
    )
    .is_err());
    Ok(())
}

#[test]
fn validation_service_retains_project_files_until_workers_are_released() -> Result<()> {
    let Fixture {
        store,
        files,
        project,
        _temp,
    } = Fixture::project()?;
    let files = Arc::new(files);
    let weak = Arc::downgrade(&files);
    let service = Service::start(
        Arc::new(Mutex::new(store)),
        files.clone(),
        Arc::new(|_| anyhow::bail!("No engine work was enqueued")),
        Arc::new(|| {}),
    )?;
    assert!(Arc::strong_count(&files) >= 2);
    drop(files);
    assert!(weak.upgrade().is_some());
    assert!(ProjectStore::open(&project, "p").is_err());
    service.shutdown()?;
    drop(service);
    assert!(weak.upgrade().is_none());
    drop(ProjectStore::open(&project, "p")?);
    Ok(())
}
