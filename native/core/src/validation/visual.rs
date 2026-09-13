use super::{
    flow::Action,
    model::{Evidence, Run},
    repository,
    sandbox::{check_output, engine_path, Sandbox},
};
use crate::files::{file_hash, safe_path};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{fs, path::Path, sync::atomic::AtomicBool, thread, time::Duration};

pub fn execute(
    engine: &Path,
    ffmpeg: Option<&Path>,
    sandbox: &Sandbox,
    run: &mut Run,
    cancelled: &AtomicBool,
    progress: &impl Fn(&Run),
) -> Result<()> {
    let flow = run.flow.as_ref().context("Visual run has no flow")?.clone();
    flow.definition.validate()?;
    if flow.definition.video && ffmpeg.is_none() {
        bail!("Video requires FFmpeg; configure its executable in Tests & Visuals settings");
    }
    let runtime = sandbox.project.join("beaver_validation_runtime");
    if runtime.exists() {
        bail!("Reserved validation runtime path already exists in project");
    }
    fs::create_dir(&runtime)?;
    fs::write(
        runtime.join("driver.gd"),
        include_str!("../../../../resources/validation/driver.gd"),
    )?;
    fs::write(
        runtime.join("capture.gd"),
        include_str!("../../../../resources/validation/capture.gd"),
    )?;
    let mut spec = serde_json::to_value(&flow.definition)?;
    spec["output"] = json!(engine_path(&sandbox.output));
    fs::write(runtime.join("run.json"), serde_json::to_vec_pretty(&spec)?)?;
    let mut overrides = fs::read_to_string(sandbox.project.join("override.cfg"))?;
    overrides.push_str(
        "\n[autoload]\nBeaverValidation=\"*res://beaver_validation_runtime/driver.gd\"\n",
    );
    if !flow.definition.entry.is_empty() {
        let entry = super::flow::relative(&flow.definition.entry)?;
        overrides.push_str(&format!(
            "\n[application]\nrun/main_scene={}\n",
            serde_json::to_string(&format!("res://{entry}"))?
        ));
    }
    fs::write(sandbox.project.join("override.cfg"), overrides)?;
    run.phase = "importing".into();
    progress(run);
    run.log = sandbox.import(engine, cancelled)?;
    let fps = flow.definition.config.fps.to_string();
    let resolution = format!(
        "{}x{}",
        flow.definition.config.width, flow.definition.config.height
    );
    let movie = engine_path(&sandbox.output.join("recording.avi"));
    let mut args = vec![
        "--path",
        ".",
        "--rendering-method",
        "gl_compatibility",
        "--audio-driver",
        "Dummy",
        "--resolution",
        &resolution,
        "--fixed-fps",
        &fps,
        "--disable-vsync",
    ];
    if flow.definition.video {
        args.extend(["--write-movie", &movie]);
    }
    run.phase = "capturing".into();
    progress(run);
    let result = thread::scope(|scope| {
        let process = scope.spawn(|| sandbox.execute(engine, &args, 900, cancelled));
        while !process.is_finished() {
            thread::sleep(Duration::from_millis(300));
            // record.json is replaced atomically. A transient sharing violation on Windows
            // is retried here; final collection below reports persistent capture errors.
            if collect(sandbox, run).is_ok() {
                progress(run);
            }
        }
        process
            .join()
            .map_err(|_| anyhow::anyhow!("Visual process worker failed"))?
    });
    collect(sandbox, run)?;
    let output = result?;
    fs::write(sandbox.output.join("visual.log"), &output.text)?;
    run.log.push_str(&output.text);
    check_output(&output)?;
    let record: Value = serde_json::from_slice(&fs::read(sandbox.output.join("record.json"))?)?;
    if record["complete"] != true
        || record["completedSteps"].as_u64() != Some(flow.definition.steps.len() as u64)
    {
        bail!("Visual flow incomplete: {}", record["error"]);
    }
    for step in &flow.definition.steps {
        if matches!(step.action, Action::Capture)
            && !run
                .evidence
                .iter()
                .any(|e| e.point == format!("{}:capture", step.id))
        {
            bail!("Required capture point missing: {}", step.id);
        }
    }
    if flow.definition.video {
        encode(
            ffmpeg.context("FFmpeg missing")?,
            sandbox,
            run,
            &record,
            cancelled,
            progress,
        )?;
    }
    Ok(())
}

pub fn collect(sandbox: &Sandbox, run: &mut Run) -> Result<()> {
    let file = sandbox.output.join("record.json");
    if !file.is_file() {
        return Ok(());
    }
    let record: Value = serde_json::from_slice(&fs::read(file)?)?;
    run.completed_steps = record["completedSteps"].as_u64().unwrap_or(0) as usize;
    let items = record["evidence"]
        .as_array()
        .context("Missing capture records")?;
    for item in items {
        let file = item["file"].as_str().context("Missing capture filename")?;
        if run.evidence.iter().any(|e| e.file == file) {
            continue;
        }
        let path = safe_path(&sandbox.output, file)?;
        let image = image::ImageReader::open(&path)?.decode()?;
        let config = &run.flow.as_ref().context("Missing flow")?.definition.config;
        if image.width() != config.width || image.height() != config.height {
            bail!("Captured resolution differs from configured viewport");
        }
        run.evidence.push(Evidence {
            id: repository::id(),
            file: file.into(),
            sha256: file_hash(&path)?.context("Capture missing")?,
            kind: "image".into(),
            point: item["point"]
                .as_str()
                .context("Missing capture point")?
                .into(),
            start: item["start"].as_f64().context("Missing capture time")?,
            end: item["end"].as_f64().context("Missing capture time")?,
            references: serde_json::from_value(item["references"].clone())?,
            state: item["state"].clone(),
        });
    }
    Ok(())
}

fn encode(
    ffmpeg: &Path,
    sandbox: &Sandbox,
    run: &mut Run,
    record: &Value,
    cancelled: &AtomicBool,
    progress: &impl Fn(&Run),
) -> Result<()> {
    run.phase = "encoding".into();
    progress(run);
    let source = engine_path(&sandbox.output.join("recording.avi"));
    let target = engine_path(&sandbox.output.join("recording.webm"));
    let output = sandbox.execute(
        ffmpeg,
        &[
            "-nostdin",
            "-y",
            "-i",
            &source,
            "-c:v",
            "libvpx-vp9",
            "-deadline",
            "realtime",
            "-cpu-used",
            "8",
            "-crf",
            "32",
            "-b:v",
            "0",
            "-c:a",
            "libopus",
            "-progress",
            "pipe:1",
            &target,
        ],
        900,
        cancelled,
    )?;
    fs::write(sandbox.output.join("encoding.log"), &output.text)?;
    if output.code != 0
        || !output.text.contains("progress=end")
        || fs::metadata(&target)?.len() < 1000
    {
        bail!("Video encoding did not complete");
    }
    let duration = record["duration"]
        .as_f64()
        .context("Video duration missing")?;
    let encoded = output
        .text
        .lines()
        .filter_map(|line| line.strip_prefix("out_time_us=")?.parse::<f64>().ok())
        .last()
        .unwrap_or(0.0)
        / 1_000_000.0;
    if duration <= 0.0 || (encoded - duration).abs() > 1.0 {
        bail!("Encoded video is truncated or its timeline is inconsistent");
    }
    let mut references = run
        .flow
        .as_ref()
        .context("Missing flow")?
        .definition
        .references
        .clone();
    for reference in run.evidence.iter().flat_map(|frame| &frame.references) {
        if !references.contains(reference) {
            references.push(reference.clone());
        }
    }
    run.evidence.push(Evidence {
        id: repository::id(),
        file: "recording.webm".into(),
        sha256: file_hash(Path::new(&target))?.context("Video missing")?,
        kind: "video".into(),
        point: "recording".into(),
        start: 0.0,
        end: duration,
        references,
        state: json!({"events":record["events"],"duration":duration}),
    });
    // The encoded, verified recording is durable; the intermediate can be very large.
    fs::remove_file(sandbox.output.join("recording.avi"))?;
    Ok(())
}
