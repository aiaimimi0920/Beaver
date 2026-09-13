use super::{
    model::{CodeReport, Run},
    sandbox::{check_output, engine_path, Sandbox},
    GUT_VERSION,
};
use anyhow::{bail, ensure, Context, Result};
use serde_json::Value;
use std::{fs, path::Path, sync::atomic::AtomicBool};

pub use super::code_report::parse_report;

pub fn directories(config: &Value, exists: impl Fn(&str) -> bool) -> Result<Vec<String>> {
    let explicit = !config["code"]["directories"].is_null();
    let values: Vec<String> = if explicit {
        serde_json::from_value(config["code"]["directories"].clone())
            .context("Invalid required GUT directories")?
    } else {
        vec!["tests".into(), "test".into()]
    };
    ensure!(
        !values.is_empty() && values.len() <= 100,
        "No required GUT suites configured"
    );
    let mut dirs = vec![];
    for value in values {
        let directory = super::flow::relative(&value)?.to_owned();
        ensure!(
            !dirs.contains(&directory),
            "Duplicate GUT directory: {directory}"
        );
        if exists(&directory) {
            dirs.push(directory);
        } else if explicit {
            bail!("Required GUT directory is missing: {directory}");
        }
    }
    ensure!(
        !dirs.is_empty(),
        "尚未建立代码验收：请在 tests/ 添加有意义的 GUT test_*.gd 用例"
    );
    Ok(dirs)
}

pub fn execute(
    engine: &Path,
    sandbox: &Sandbox,
    run: &mut Run,
    cancelled: &AtomicBool,
    progress: &impl Fn(&Run),
) -> Result<()> {
    sandbox.install_gut()?;
    run.phase = "importing".into();
    progress(run);
    run.log = sandbox.import(engine, cancelled)?;
    let manifest = sandbox.project.join("beaver.validation.json");
    let config: Value = if manifest.exists() {
        serde_json::from_slice(&fs::read(manifest)?)?
    } else {
        Value::Null
    };
    let dirs = directories(&config, |s| sandbox.project.join(s).is_dir())?;
    let seconds = config["code"]["timeoutSeconds"]
        .as_u64()
        .unwrap_or(300)
        .clamp(10, 1800);
    let report = sandbox.output.join("gut.xml");
    let directory = format!(
        "-gdir={}",
        dirs.iter()
            .map(|d| format!("res://{d}"))
            .collect::<Vec<_>>()
            .join(",")
    );
    let report_arg = format!("-gjunit_xml_file={}", engine_path(&report));
    run.phase = "testing".into();
    progress(run);
    let output = sandbox.execute(
        engine,
        &[
            "--headless",
            "--path",
            ".",
            "-s",
            "addons/gut/gut_cmdln.gd",
            "-gconfig=",
            "-gexit",
            "-gdisable_colors",
            "-ginclude_subdirs",
            &directory,
            &report_arg,
        ],
        seconds,
        cancelled,
    )?;
    fs::write(sandbox.output.join("gut.log"), &output.text)?;
    run.log.push_str(&output.text);
    let cases = parse_report(
        &fs::read_to_string(&report).context("GUT did not produce a complete report")?,
    )?;
    let passed = cases
        .iter()
        .filter(|c| c.status == "pass" && c.assertions > 0)
        .count();
    let failed = cases
        .iter()
        .filter(|c| c.status == "fail" || (c.status == "pass" && c.assertions == 0))
        .count();
    let skipped = cases.len() - passed - failed;
    run.code = Some(CodeReport {
        gut_version: GUT_VERSION.into(),
        directories: dirs,
        cases,
        passed,
        failed,
        skipped,
        report_sha256: crate::files::file_hash(&report)?.context("GUT report missing")?,
        output_sha256: crate::files::file_hash(&sandbox.output.join("gut.log"))?
            .context("GUT log missing")?,
        exit_code: Some(output.code),
    });
    check_output(&output)?;
    if passed == 0 || failed > 0 || skipped > 0 {
        bail!("GUT: {passed} passed, {failed} failed, {skipped} skipped; all required tests must execute assertions and pass");
    }
    run.verdict = "autoPassed".into();
    Ok(())
}

pub fn green(run: &Run) -> bool {
    let Some(report) = &run.code else {
        return false;
    };
    run.kind == "code"
        && run.status == "completed"
        && run.verdict == "autoPassed"
        && run.error.is_none()
        && !run.engine_version.is_empty()
        && report.gut_version == GUT_VERSION
        && report.exit_code == Some(0)
        && report.passed > 0
        && report.passed == report.cases.len()
        && report.failed == 0
        && report.skipped == 0
        && !report.directories.is_empty()
        && report.cases.iter().all(|case| {
            case.status == "pass"
                && case.assertions > 0
                && !case.name.is_empty()
                && report
                    .directories
                    .iter()
                    .any(|dir| case.file.starts_with(&format!("{dir}/")))
        })
}

pub fn validate(data: &Path, run: &Run) -> Result<()> {
    ensure!(green(run), "Required GUT cases have not all passed");
    let report = run.code.as_ref().unwrap();
    let output = super::repository::run_dir(data, &run.id)?;
    for (name, hash) in [
        ("gut.xml", &report.report_sha256),
        ("gut.log", &report.output_sha256),
    ] {
        ensure!(
            crate::files::file_hash(&output.join(name))?.as_ref() == Some(hash),
            "GUT artifact changed or is missing: {name}"
        );
    }
    ensure!(
        parse_report(&fs::read_to_string(output.join("gut.xml"))?)? == report.cases,
        "GUT cases differ from the recorded report"
    );
    let files = crate::files::Files::new(data.into());
    let config: Value = match run.snapshot.get("beaver.validation.json") {
        Some(hash) => serde_json::from_slice(&fs::read(files.blob(hash)?)?)?,
        None => Value::Null,
    };
    let expected = directories(&config, |dir| {
        run.snapshot
            .keys()
            .any(|path| path.starts_with(&format!("{dir}/")))
    })?;
    ensure!(
        expected == report.directories,
        "Required GUT scope differs from the candidate"
    );
    Ok(())
}
