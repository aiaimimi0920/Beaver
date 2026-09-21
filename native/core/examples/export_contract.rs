use anyhow::{ensure, Context, Result};
use beaver_core::{export_bundle, export_templates, game_export, projects, store::Store};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    time::Duration,
};

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let output = PathBuf::from(args.next().context("output required")?);
    fs::create_dir_all(&output)?;
    let godot = fs::canonicalize(PathBuf::from(
        args.next().context("Godot executable required")?,
    ))?;
    let source = args
        .next()
        .context("matching template or --archive required")?;
    let cancelled = AtomicBool::new(false);
    let (template, template_setup) = if source == "--archive" {
        ensure!(
            godot
                .parent()
                .context("editor directory")?
                .join("._sc_")
                .is_file(),
            "archive validation requires an isolated self-contained editor"
        );
        let archive = fs::canonicalize(PathBuf::from(args.next().context("archive required")?))?;
        let installed = export_templates::prepare(&godot, Some(&archive), None, &cancelled).await?;
        ensure!(installed["reused"] == false, "fresh installation required");
        let reused = export_templates::prepare(&godot, None, None, &cancelled).await?;
        ensure!(
            reused["reused"] == true,
            "installed templates were not reused"
        );
        ensure!(
            installed["directory"] == reused["directory"],
            "template directory changed"
        );
        (
            None,
            Some(json!({"installed": installed, "reused": reused})),
        )
    } else {
        (Some(fs::canonicalize(PathBuf::from(source))?), None)
    };
    let data = output.join("data");
    let store = Store::open(&data)?;
    let designs: Value =
        serde_json::from_str(include_str!("../../../dist-native/design-catalog.json"))?;
    let plans: Value =
        serde_json::from_str(include_str!("../../../dist-native/blueprint-catalog.json"))?;
    let templates: Value =
        serde_json::from_str(include_str!("../../../dist-native/templates.json"))?;
    let project = projects::create_project(
        &store,
        &data,
        json!({"parent":output,"name":format!("原创游戏-{}",uuid::Uuid::new_v4()),"template":"nightbar","blueprint":plans["default"]}),
        &designs,
        &plans,
        &templates,
    )?;
    let root = Path::new(project["path"].as_str().context("project path")?);
    let config = fs::read_to_string(root.join("project.godot"))?
        .lines()
        .map(|line| {
            if line.starts_with("config/name=") {
                format!(
                    "config/name=\"Beaver export validation {}\"",
                    project["id"].as_str().unwrap()
                )
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(root.join("project.godot"), config)?;
    if let Some(template) = &template {
        let presets = fs::read_to_string(root.join("export_presets.cfg"))?.replace(
            "custom_template/release=\"\"",
            &format!(
                "custom_template/release={}",
                serde_json::to_string(template.to_str().context("template encoding")?)?
            ),
        );
        fs::write(root.join("export_presets.cfg"), presets)?;
    }
    let files = beaver_core::files::Files::new(data.clone());
    let frozen = files.capture(root)?;
    ensure!(
        game_export::prepare(&files, &data, &project, root, "Windows Desktop").is_err(),
        "nested export accepted"
    );
    let job = game_export::prepare(&files, &data, &project, &output, "Windows Desktop")?;
    let result = game_export::execute(job, &godot, &AtomicBool::new(false))?;
    let folder = Path::new(result["path"].as_str().context("export folder")?);
    let verified = export_bundle::verify(folder)?;
    let run = beaver_core::process::run(
        &folder.join("Game.exe"),
        &["--headless", "--quit"],
        Some(folder),
        Duration::from_secs(45),
    )?;
    ensure!(
        run.code == 0 && !run.text.contains("SCRIPT ERROR") && !run.text.contains("ERROR:"),
        "exported game startup failed ({}): {}",
        run.code,
        run.text
    );
    ensure!(
        files.capture(root)? == frozen,
        "export modified source project"
    );
    fs::write(
        output.join("proof.json"),
        serde_json::to_vec_pretty(
            &json!({"passed":true,"godot":godot,"template":template,"templateSetup":template_setup,"bundle":verified,"startup":{"exitCode":run.code,"log":run.text},"checks":["nested export refused","actual Godot release export","complete bundle verification","exported program starts and exits cleanly","source project unchanged"]}),
        )?,
    )?;
    println!("{}", output.join("proof.json").display());
    Ok(())
}
