use anyhow::{ensure, Context, Result};
use beaver_core::files::Files;
use serde_json::json;
use std::{fs, path::PathBuf, time::Instant};

fn main() -> Result<()> {
    let output = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .context("output directory required")?,
    );
    fs::create_dir_all(&output)?;
    let root = output.join(uuid::Uuid::new_v4().to_string());
    let project = root.join("source");
    fs::create_dir_all(&project)?;
    let files = Files::new(root.join("data"));
    let count = 2252;
    let mut bytes = 0;
    for i in 0..count {
        let content = format!("file {i}\n{}", "snapshot fixture\n".repeat(256));
        bytes += content.len();
        fs::write(project.join(format!("file-{i:04}.txt")), content)?;
    }
    let start = Instant::now();
    let baseline = files.capture(&project)?;
    let cold_ms = start.elapsed().as_millis();
    let mut warm_ms = Vec::new();
    for _ in 0..3 {
        let start = Instant::now();
        ensure!(
            files.capture(&project)? == baseline,
            "warm capture changed snapshot"
        );
        warm_ms.push(start.elapsed().as_millis());
    }
    let start = Instant::now();
    files.restore_copy(&baseline, &root.join("copy"))?;
    let restore_ms = start.elapsed().as_millis();
    let proof = json!({"files":count,"bytes":bytes,"coldCaptureMs":cold_ms,"warmCaptureMs":warm_ms,"restoreMs":restore_ms,"root":root,"snapshotStable":true});
    fs::write(
        output.join("measurement.json"),
        serde_json::to_vec_pretty(&proof)?,
    )?;
    println!("{proof}");
    Ok(())
}
