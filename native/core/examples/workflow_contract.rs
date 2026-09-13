use anyhow::{ensure, Context, Result};
use base64::Engine;
use beaver_core::{game_export, process, workflows};
use serde_json::{json, Value};
use std::{fs, path::PathBuf, sync::atomic::AtomicBool, time::Duration};

// Engine-adapter fixture only. This does not stand in for a Beaver game-creation run.
fn main() -> Result<()> {
    let output = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .context("output directory required")?,
    );
    let engine =
        workflows::engine_path(&std::env::args().nth(2).context("custom engine required")?)?;
    let root = output.join(uuid::Uuid::new_v4().to_string());
    fs::create_dir_all(&root)?;
    let templates: Value =
        serde_json::from_str(include_str!("../../../dist-native/templates.json"))?;
    for (relative, encoded) in templates["blank"]
        .as_object()
        .context("blank template missing")?
    {
        let target = root.join(relative);
        fs::create_dir_all(target.parent().unwrap())?;
        fs::write(
            target,
            base64::engine::general_purpose::STANDARD
                .decode(encoded.as_str().context("invalid template entry")?)?,
        )?;
    }
    let cancelled = AtomicBool::new(false);
    workflows::install(
        &root,
        engine.to_str().context("engine path is not UTF-8")?,
        &cancelled,
    )?;
    game_export::import_project(&engine, &root, &root.join("fixture-import.log"), &cancelled)?;
    // Serialize the repository's existing mannequin factory, without authoring a game.
    fs::write(
        root.join("fixture.gd"),
        r#"extends SceneTree
func _initialize():
	var factory = load("res://addons/npr_characters/examples/mannequin_factory.gd")
	var definition = factory.create_definition()
	assert(ResourceSaver.save(definition.model_scene, "res://unit_model.tscn") == OK)
	definition.model_scene = load("res://unit_model.tscn")
	assert(ResourceSaver.save(definition, "res://unit_definition.tres") == OK)
	quit(0)
"#,
    )?;
    let prepared = process::run_cancellable(
        &engine,
        &["--headless", "--path", ".", "--script", "res://fixture.gd"],
        Some(&root),
        Duration::from_secs(60),
        &cancelled,
    )?;
    fs::write(root.join("fixture.log"), &prepared.text)?;
    ensure!(
        prepared.code == 0 && !prepared.text.contains("SCRIPT ERROR"),
        "fixture setup failed; see fixture.log"
    );
    let first = workflows::run(
        &root,
        json!({"workflow":workflows::WORKFLOW,"action":"preview","definition":"unit_definition.tres"}),
        &cancelled,
    )?;
    ensure!(
        first["ok"] == true,
        "default preview failed: {}",
        first["errors"]
    );
    let second = workflows::run(
        &root,
        json!({"workflow":workflows::WORKFLOW,"action":"preview","definition":"unit_definition.tres","camera":first["camera"],"grayscale":true}),
        &cancelled,
    )?;
    ensure!(
        second["ok"] == true,
        "matched preview failed: {}",
        second["errors"]
    );
    ensure!(
        first["camera"] == second["camera"]
            && first["definitionSha256"] == second["definitionSha256"]
            && first["modelSha256"] == second["modelSha256"]
    );
    let color = second["screenshots"]
        .as_array()
        .context("color outputs missing")?;
    let gray = second["grayscaleScreenshots"]
        .as_array()
        .context("grayscale outputs missing")?;
    ensure!(color.len() == 3 && gray.len() == 3);
    for relative in color.iter().chain(gray) {
        let relative = relative.as_str().context("invalid screenshot path")?;
        let image = image::open(root.join(relative))?.to_rgba8();
        ensure!(image.dimensions() == (768, 768));
        let colors: std::collections::HashSet<_> = image.pixels().map(|pixel| pixel.0).collect();
        ensure!(colors.len() > 8, "preview is blank or flat");
        if relative.ends_with("-gray.png") {
            ensure!(
                image
                    .pixels()
                    .all(|pixel| pixel[0] == pixel[1] && pixel[1] == pixel[2]),
                "grayscale output contains color"
            );
        }
    }
    let proof = json!({"passed":true,"scope":"engine-adapter fixture using the bundled mannequin factory","realModelTaskVerified":false,"gameCreationAcceptance":false,"root":root,"first":first,"second":second,"checks":["actual custom engine renders original default preview","fixed camera survives round trip","definition and model hashes stay stable","three color and three grayscale PNGs are 768x768 and nonblank","grayscale pixels have equal RGB channels"]});
    fs::write(
        output.join("proof.json"),
        serde_json::to_vec_pretty(&proof)?,
    )?;
    println!(
        "{}",
        json!({"passed":true,"proof":output.join("proof.json"),"root":root,"scope":"engine-adapter fixture only"})
    );
    Ok(())
}
