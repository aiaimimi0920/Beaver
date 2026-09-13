use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};

fn main() -> Result<()> {
    let path = std::env::args().nth(1).context("fixture path required")?;
    let fixture: Value = serde_json::from_slice(&std::fs::read(path)?)?;
    let cases = fixture["cases"].as_array().context("cases required")?;
    for (index, case) in cases.iter().enumerate() {
        let actual = beaver_core::task_brief::format(&case["task"], &fixture["catalog"]);
        ensure!(
            actual == case["expected"].as_str().context("expected required")?,
            "brief case {index} differs: {actual}"
        );
    }
    println!("{}", json!({"checks":cases.len(),"passed":true}));
    Ok(())
}
