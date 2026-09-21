//! Real engine adapter contract, distinct from Beaver game-creation acceptance.
use anyhow::{ensure, Context, Result};
use beaver_core::{
    files::Files,
    store::Store,
    validation::{comparison, flow::Definition, repository, runner},
};
use serde_json::json;
use std::{fs, path::PathBuf, sync::atomic::AtomicBool};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let output = PathBuf::from(args.next().context("output directory required")?);
    ensure!(!output.exists(), "Use a fresh adapter contract directory");
    let engine = PathBuf::from(args.next().context("Godot executable required")?);
    let ffmpeg = args.next().map(PathBuf::from);
    let data = output.join("data");
    let project = output.join("project");
    fs::create_dir_all(project.join("tests"))?;
    for (path, content) in [
        (
            "project.godot",
            include_str!("../../../tests/fixtures/validation/project.godot"),
        ),
        (
            "main.tscn",
            include_str!("../../../tests/fixtures/validation/main.tscn"),
        ),
        (
            "main.gd",
            include_str!("../../../tests/fixtures/validation/main.gd"),
        ),
        (
            "counter.gd",
            include_str!("../../../tests/fixtures/validation/counter.gd"),
        ),
        (
            "tests/test_counter.gd",
            include_str!("../../../tests/fixtures/validation/test_counter.gd"),
        ),
    ] {
        fs::write(project.join(path), content)?;
    }
    let mut store = Store::open(&data)?;
    store.put(
        "project",
        "contract",
        &json!({"id":"contract","path":project}),
    )?;
    let files = Files::new(data.clone());
    let snapshot = files.capture(&project)?;
    let cancelled = AtomicBool::new(false);
    let mut code = repository::new_run(&store, "contract", snapshot.clone(), None, None, None)?;
    runner::execute(&files, &engine, None, &mut code, &cancelled, |run| {
        let _ = store.put("validationRun", &run.id, run);
    });
    fs::write(output.join("code.json"), serde_json::to_vec_pretty(&code)?)?;
    ensure!(
        code.status == "completed" && code.verdict == "autoPassed",
        "GUT did not pass: {:?}",
        code.error
    );
    let definition: Definition = serde_json::from_value(json!({
        "key":"adapter","name":"Input and capture","category":"ui","purpose":"Verify real input and frame capture",
        "config":{"width":320,"height":180},"video":ffmpeg.is_some(),
        "steps":[
            {"id":"ready","kind":"waitFor","node":"/root/Main/Button","property":"visible","equals":true,"timeout":120},
            {"id":"before","kind":"capture"},
            {"id":"click","kind":"click","node":"/root/Main/Button"},
            {"id":"clicked","kind":"waitFor","node":"/root/Main","property":"count","equals":1,"timeout":120},
            {"id":"move","kind":"action","name":"ui_right","pressed":true},
            {"id":"settle","kind":"wait","frames":40},
            {"id":"release","kind":"action","name":"ui_right","pressed":false},
            {"id":"after","kind":"capture"}
        ]
    }))?;
    let flow = repository::save_flow(&mut store, "contract", definition, 0)?;
    let mut visual = repository::new_run(&store, "contract", snapshot, Some(flow), None, None)?;
    runner::execute(
        &files,
        &engine,
        ffmpeg.as_deref(),
        &mut visual,
        &cancelled,
        |run| {
            let _ = store.put("validationRun", &run.id, run);
        },
    );
    fs::write(
        output.join("visual.json"),
        serde_json::to_vec_pretty(&visual)?,
    )?;
    ensure!(
        visual.status == "completed",
        "Visual flow failed: {:?}",
        visual.error
    );
    comparison::validate_evidence(&files, &visual)?;
    let before = visual
        .evidence
        .iter()
        .find(|e| e.point == "before:capture")
        .context("before missing")?;
    let after = visual
        .evidence
        .iter()
        .find(|e| e.point == "after:capture")
        .context("after missing")?;
    ensure!(
        before.sha256 != after.sha256,
        "Input did not change rendered pixels"
    );
    for evidence in &visual.evidence {
        ensure!(
            evidence.references.iter().any(|reference| {
                reference.path == "res://main.gd" && reference.source == "runtime"
            }),
            "Media lost its runtime source reference: {}",
            evidence.point
        );
    }
    let directory = repository::run_dir(&files, &visual.id)?;
    let record: serde_json::Value =
        serde_json::from_slice(&fs::read(directory.join("record.json"))?)?;
    let user = PathBuf::from(record["userData"].as_str().context("userData missing")?);
    ensure!(
        fs::canonicalize(user)?.starts_with(fs::canonicalize(&directory)?),
        "User data escaped run directory"
    );
    fs::write(
        project.join("counter.gd"),
        "extends RefCounted\nvar value: int = 0\nfunc increment() -> void:\n\tvalue += 3\n",
    )?;
    let mut failed = repository::new_run(
        &store,
        "contract",
        files.capture(&project)?,
        None,
        None,
        None,
    )?;
    runner::execute(&files, &engine, None, &mut failed, &cancelled, |_| {});
    fs::write(
        output.join("failure.json"),
        serde_json::to_vec_pretty(&failed)?,
    )?;
    ensure!(
        failed.status == "failed" && failed.code.as_ref().is_some_and(|c| c.failed == 1),
        "GUT false-green contract failed"
    );
    println!(
        "GUT pass/fail, real input, PNG{} and isolated user data verified: {}",
        if ffmpeg.is_some() { ", WebM" } else { "" },
        output.display()
    );
    Ok(())
}
