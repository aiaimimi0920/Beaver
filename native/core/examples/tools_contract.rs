use anyhow::{ensure, Context, Result};
use serde_json::json;
fn main() -> Result<()> {
    let output =
        std::path::PathBuf::from(std::env::args_os().nth(1).context("proof path required")?);
    let results =
        beaver_core::tools::detect(&json!({"codex":"","godot":"","blender":"","node":""}), &[])?;
    for name in ["godot", "blender", "codex", "node"] {
        let item = results
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["name"] == name)
            .context("tool result missing")?;
        ensure!(item["available"] == true, "{name} not discovered: {item}");
        ensure!(
            std::path::Path::new(item["path"].as_str().unwrap()).is_file(),
            "discovered path missing"
        );
    }
    std::fs::create_dir_all(output.parent().context("proof directory required")?)?;
    std::fs::write(
        &output,
        serde_json::to_vec_pretty(&json!({"checks":4,"passed":true,"tools":results}))?,
    )?;
    println!("{}", results);
    Ok(())
}
