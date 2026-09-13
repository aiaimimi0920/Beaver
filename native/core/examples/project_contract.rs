use anyhow::{ensure, Context, Result};
use beaver_core::{projects, store::Store};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

fn main() -> Result<()> {
    let output = PathBuf::from(std::env::args_os().nth(1).context("output required")?);
    fs::create_dir_all(&output)?;
    let data = output.join("data");
    let store = Store::open(&data)?;
    let designs: Value =
        serde_json::from_str(include_str!("../../../dist-native/design-catalog.json"))?;
    let plans: Value =
        serde_json::from_str(include_str!("../../../dist-native/blueprint-catalog.json"))?;
    let templates: Value =
        serde_json::from_str(include_str!("../../../dist-native/templates.json"))?;
    let configured = std::env::args().nth(2).unwrap_or_default();
    let godot = beaver_core::tools::find("godot", &configured)?;
    let mut checks = Vec::new();
    for template in ["blank", "nightbar"] {
        let project = projects::create_project(
            &store,
            &data,
            json!({"parent":output,"name":format!("{template}-{}",uuid::Uuid::new_v4()),"template":template,"blueprint":plans["default"]}),
            &designs,
            &plans,
            &templates,
        )?;
        let path = Path::new(project["path"].as_str().context("project path required")?);
        // Isolate the engine's user-data name; gameplay scripts remain byte-identical.
        let config = fs::read_to_string(path.join("project.godot"))?;
        let config = config
            .lines()
            .map(|line| {
                if line.starts_with("config/name=") {
                    format!(
                        "config/name=\"Beaver validation {}\"",
                        project["id"].as_str().unwrap()
                    )
                } else {
                    line.to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(path.join("project.godot"), config)?;
        for arguments in [
            vec![
                "--headless",
                "--editor",
                "--path",
                path.to_str().unwrap(),
                "--quit",
            ],
            vec!["--headless", "--path", path.to_str().unwrap(), "--quit"],
        ] {
            let result =
                beaver_core::process::run(&godot, &arguments, Some(path), Duration::from_secs(45))?;
            ensure!(
                result.code == 0
                    && !result.text.contains("SCRIPT ERROR")
                    && !result.text.contains("Parse Error")
                    && !result.text.contains("ERROR:"),
                "{template} engine validation failed (exit {}): {}",
                result.code,
                result.text
            );
            checks.push(json!({"template":template,"arguments":arguments,"exitCode":result.code,"log":result.text}));
        }
    }
    fs::write(
        output.join("proof.json"),
        serde_json::to_vec_pretty(
            &json!({"godot":godot,"checks":checks,"passed":true,"projects":store.list::<Value>("project")?}),
        )?,
    )?;
    println!("{}", output.join("proof.json").display());
    Ok(())
}
