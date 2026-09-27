use super::{model::Run, repository::run_dir};
use crate::{
    files::{safe_path, Files},
    process,
};
use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    time::Duration,
};

const GUT: &[u8] = include_bytes!("../../../../resources/validation/gut-9.4.0.zip");
const HASH: &str = "c54428c250f55a2945282ce462f1aecffac0133a7af2c459ac2ed286ddd0b83e";

pub struct Sandbox {
    _temporary: tempfile::TempDir,
    pub project: PathBuf,
    pub output: PathBuf,
    pub environment: BTreeMap<String, String>,
}

pub fn engine_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    if let Some(unc) = text.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{unc}")
    } else {
        text.strip_prefix(r"\\?\")
            .unwrap_or(&text)
            .replace('\\', "/")
    }
}

impl Sandbox {
    pub fn prepare(files: &Files, run: &Run) -> Result<Self> {
        let output = run_dir(files, &run.id)?;
        Self::prepare_at(files, run, output)
    }

    pub(crate) fn prepare_at(files: &Files, run: &Run, output: PathBuf) -> Result<Self> {
        fs::create_dir_all(&output)?;
        let temporary = tempfile::Builder::new()
            .prefix("workspace-")
            .tempdir_in(&output)?;
        let project = temporary.path().join("project");
        files.restore_copy(&run.snapshot, &project)?;
        if run.kind == "objectPreview" && !project.join("project.godot").exists() {
            fs::write(project.join("project.godot"), "config_version=5\n")?;
        }
        if !project.join("project.godot").is_file() {
            bail!("Godot project.godot is missing");
        }
        let user = output.join("user");
        fs::create_dir_all(&user)?;
        let environment = [
            "APPDATA",
            "LOCALAPPDATA",
            "XDG_DATA_HOME",
            "XDG_CONFIG_HOME",
            "HOME",
            "USERPROFILE",
        ]
        .into_iter()
        .map(|key| (key.into(), engine_path(&user)))
        .collect();
        let mut overrides = fs::read_to_string(project.join("override.cfg")).unwrap_or_default();
        overrides.push_str(&format!("\n[application]\nconfig/use_custom_user_dir=true\nconfig/custom_user_dir_name=\"BeaverValidation/{}\"\n", run.id));
        fs::write(project.join("override.cfg"), overrides)?;
        Ok(Self {
            _temporary: temporary,
            project,
            output,
            environment,
        })
    }

    pub fn install_gut(&self) -> Result<()> {
        if format!("{:x}", Sha256::digest(GUT)) != HASH {
            bail!("Bundled GUT checksum mismatch");
        }
        let mut archive = zip::ZipArchive::new(Cursor::new(GUT))?;
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index)?;
            let name = entry.name().replace('\\', "/");
            if entry.is_dir() {
                continue;
            }
            if !name.starts_with("addons/gut/") || entry.size() > 20 * 1024 * 1024 {
                bail!("Invalid bundled GUT archive member");
            }
            let path = safe_path(&self.project, &name)?;
            fs::create_dir_all(path.parent().context("Archive member parent missing")?)?;
            std::io::copy(&mut entry, &mut fs::File::create(path)?)?;
        }
        Ok(())
    }

    pub fn execute(
        &self,
        engine: &Path,
        args: &[&str],
        seconds: u64,
        cancelled: &AtomicBool,
    ) -> Result<process::Output> {
        process::run_cancellable_env(
            engine,
            args,
            Some(&self.project),
            Duration::from_secs(seconds),
            cancelled,
            &self.environment,
        )
    }

    pub fn import(&self, engine: &Path, cancelled: &AtomicBool) -> Result<String> {
        let output = self.execute(
            engine,
            &["--headless", "--path", ".", "--import"],
            180,
            cancelled,
        )?;
        fs::write(self.output.join("import.log"), &output.text)?;
        check_output(&output)?;
        Ok(output.text)
    }
}

pub fn check_output(output: &process::Output) -> Result<()> {
    if output.code != 0 || output.text.contains("SCRIPT ERROR") || output.text.contains("ERROR:") {
        bail!(
            "Godot failed (exit {}): {}",
            output.code,
            output
                .text
                .chars()
                .rev()
                .take(6000)
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>()
        );
    }
    Ok(())
}
