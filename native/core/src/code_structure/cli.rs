use super::{read, scan, Baseline, Scope};
use anyhow::{bail, Context, Result};
use std::{ffi::OsString, fs, io::Write, path::PathBuf};

pub fn run(arguments: Vec<OsString>) -> Result<bool> {
    let mut root = std::env::current_dir()?;
    let mut baseline_path = None;
    let mut output = None;
    let mut strict = false;
    let mut scope = Scope::Game;
    let mut args = arguments.into_iter();
    while let Some(argument) = args.next() {
        match argument
            .to_str()
            .context("Invalid code-structure argument")?
        {
            "--root" => root = PathBuf::from(args.next().context("--root needs a directory")?),
            "--baseline" => {
                baseline_path = Some(PathBuf::from(
                    args.next().context("--baseline needs a file")?,
                ))
            }
            "--json" => output = Some(PathBuf::from(args.next().context("--json needs a file")?)),
            "--strict" => strict = true,
            "--repo" => scope = Scope::Repository,
            _ => bail!("Usage: --root DIR [--repo] [--baseline FILE] [--strict] [--json FILE]"),
        }
    }
    let baseline = match baseline_path {
        Some(path) => Baseline::parse(&read(&path, 8 * 1024 * 1024)?)?,
        None => Baseline::empty(),
    };
    let report = scan(&root, scope, &baseline, strict)?;
    if let Some(output) = output {
        let output = std::path::absolute(output)?;
        let parent = output.parent().context("Report path has no parent")?;
        fs::create_dir_all(parent)?;
        let mut temp = tempfile::NamedTempFile::new_in(parent)?;
        temp.write_all(&serde_json::to_vec_pretty(&report)?)?;
        temp.as_file().sync_all()?;
        temp.persist(output)?;
    }
    let debt = report
        .files
        .iter()
        .filter(|entry| entry.status == "legacy")
        .count();
    println!(
        "Code structure: {} sources, {} unchanged legacy files, {} violations",
        report.files.len(),
        debt,
        report.violations.len()
    );
    for entry in report
        .files
        .iter()
        .filter(|entry| entry.stamp.effective_lines > 500 && entry.status != "immutable")
    {
        println!(
            "{}: {} effective lines ({})",
            entry.path, entry.stamp.effective_lines, entry.status
        );
    }
    for violation in &report.violations {
        eprintln!("{violation}");
    }
    Ok(report.ok)
}
