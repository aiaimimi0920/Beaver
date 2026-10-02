use crate::{
    export_bundle,
    files::{safe_path, Files, Snapshot},
    process,
};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

pub struct Target {
    pub name: String,
    pub platform: String,
    pub entry: &'static str,
}
pub fn targets(config: &str) -> Vec<Target> {
    let header = regex::Regex::new(r"^\[preset\.\d+\]$").unwrap();
    let mut rows = Vec::new();
    let mut current: Option<(String, String)> = None;
    for line in config.lines().chain(std::iter::once("[end]")) {
        let line = line.trim();
        if line.starts_with('[') {
            if let Some((name, platform)) = current.take() {
                let entry = match platform.as_str() {
                    "Windows Desktop" => "Game.exe",
                    "Linux" => "Game.x86_64",
                    "macOS" => "Game.zip",
                    _ => "",
                };
                rows.push(Target {
                    name,
                    platform,
                    entry,
                });
            }
            if header.is_match(line) {
                current = Some((String::new(), String::new()));
            }
        } else if let Some((name, platform)) = &mut current {
            if let Some((key, value)) = line.split_once('=') {
                if let Ok(value) = serde_json::from_str::<String>(value.trim()) {
                    match key.trim() {
                        "name" => *name = value,
                        "platform" => *platform = value,
                        _ => {}
                    }
                }
            }
        }
    }
    rows
}
fn config(root: &Path) -> Result<String> {
    let mut bytes = Vec::new();
    fs::File::open(safe_path(root, "export_presets.cfg")?)?
        .take(4 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 4 * 1024 * 1024 {
        bail!("导出预设文件过大");
    }
    String::from_utf8(bytes).context("导出预设不是 UTF-8")
}
pub fn presets(root: &Path) -> Result<Vec<String>> {
    match config(root) {
        Ok(value) => Ok(targets(&value)
            .into_iter()
            .map(|target| target.name)
            .filter(|name| !name.is_empty())
            .collect()),
        Err(error) => {
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|error| error.kind() == std::io::ErrorKind::NotFound)
            {
                Ok(Vec::new())
            } else {
                Err(error)
            }
        }
    }
}
pub struct Prepared {
    workspace: tempfile::TempDir,
    parent: PathBuf,
    target: Target,
    project: String,
    template_directory: Option<PathBuf>,
    provenance: Value,
}

impl Prepared {
    pub fn with_provenance(mut self, provenance: Value) -> Self {
        self.provenance = provenance;
        self
    }

    fn bind_bundle(&mut self, bundle: &crate::godot_bundle::Bundle) -> Result<()> {
        let config = bundle.preset(
            &config(self.workspace.path())?,
            &self.target.name,
            self.workspace.path(),
        )?;
        fs::write(self.workspace.path().join("export_presets.cfg"), config)?;
        self.template_directory = Some(bundle.directory.clone());
        Ok(())
    }
}

fn custom_template(config: &str, preset: &str) -> Result<Option<String>> {
    let mut section = "";
    let mut selected = None;
    let mut templates = std::collections::BTreeMap::new();
    for line in config.lines().map(str::trim) {
        if line.starts_with('[') {
            section = line;
        } else if let Some((key, value)) = line.split_once('=') {
            if key.trim() == "name" && !section.ends_with(".options]") {
                if serde_json::from_str::<String>(value.trim()).ok().as_deref() == Some(preset) {
                    selected = Some(format!("{}.options]", section.trim_end_matches(']')));
                }
            } else if key.trim() == "custom_template/release" {
                templates.insert(
                    section.to_owned(),
                    serde_json::from_str::<String>(value.trim())?,
                );
            }
        }
    }
    Ok(selected
        .and_then(|section| templates.remove(&section))
        .filter(|path| !path.is_empty()))
}

/// Caller holds the project write lock only while freezing this export snapshot.
pub fn prepare(
    files: &Files,
    scratch: &Path,
    project: &Value,
    destination: &Path,
    preset: &str,
) -> Result<Prepared> {
    let root = Path::new(project["path"].as_str().context("项目路径无效")?);
    let snapshot = files.capture(root)?;
    prepare_snapshot(files, scratch, project, destination, preset, &snapshot)
}

pub fn snapshot_preset(files: &Files, snapshot: &Snapshot, preset: &str) -> Result<()> {
    let hash = snapshot
        .get("export_presets.cfg")
        .context("发布候选缺少导出预设")?;
    let blob = files.blob(hash)?;
    anyhow::ensure!(
        crate::files::file_hash(&blob)?.as_ref() == Some(hash),
        "导出预设快照损坏"
    );
    anyhow::ensure!(
        fs::metadata(&blob)?.len() <= 4 * 1024 * 1024,
        "导出预设文件过大"
    );
    let config = fs::read_to_string(blob)?;
    anyhow::ensure!(
        targets(&config)
            .iter()
            .any(|target| target.name == preset && !target.entry.is_empty()),
        "发布候选中没有对应的桌面导出预设"
    );
    Ok(())
}

/// Formal export supplies the already-validated candidate, never the live project.
pub fn prepare_snapshot(
    files: &Files,
    scratch: &Path,
    project: &Value,
    destination: &Path,
    preset: &str,
    snapshot: &Snapshot,
) -> Result<Prepared> {
    if preset.trim().is_empty() || preset.encode_utf16().count() > 200 {
        bail!("请选择有效的 Godot 导出预设");
    }
    let root = fs::canonicalize(project["path"].as_str().context("项目路径无效")?)?;
    let parent = fs::canonicalize(destination)?;
    if parent.starts_with(&root) {
        bail!("导出目录必须位于项目外，避免把构建产物重新导入资源");
    }
    // After restoration, the job owns its copy and no longer needs the content store.
    let workspace = tempfile::Builder::new()
        .prefix("export-workspace-")
        .tempdir_in(scratch)?;
    files.restore_copy(snapshot, workspace.path())?;
    let export_config = config(workspace.path())?;
    let target = targets(&export_config)
        .into_iter()
        .find(|target| target.name == preset)
        .context("Godot 导出预设不存在")?;
    if target.entry.is_empty() {
        bail!("首版导出仅支持 Windows、Linux、macOS 桌面预设");
    }
    let template_directory = custom_template(&export_config, preset)?
        .map(|path| -> Result<PathBuf> {
            let path = if let Some(relative) = path.strip_prefix("res://") {
                safe_path(workspace.path(), relative)?
            } else if Path::new(&path).is_absolute() {
                PathBuf::from(path)
            } else {
                safe_path(workspace.path(), &path)?
            };
            let path = fs::canonicalize(path).context("自定义导出模板不存在")?;
            if !path.is_file() {
                bail!("自定义导出模板不是文件");
            }
            Ok(path.parent().context("导出模板目录无效")?.to_owned())
        })
        .transpose()?;
    Ok(Prepared {
        workspace,
        parent,
        target,
        project: project["name"].as_str().unwrap_or("Game").to_owned(),
        template_directory,
        provenance: json!({"purpose":"internal"}),
    })
}
pub fn execute(mut job: Prepared, godot: &Path, cancelled: &AtomicBool) -> Result<Value> {
    if cancelled.load(Ordering::SeqCst) {
        bail!("应用正在退出");
    }
    let toolchain = if job.target.platform == "Windows Desktop" {
        if let Some(bundle) = crate::godot_bundle::discover(godot, cancelled)? {
            job.bind_bundle(&bundle)?;
            Some(bundle.receipt(godot))
        } else {
            None
        }
    } else {
        None
    };
    let folder = tempfile::Builder::new()
        .prefix("Beaver-game-")
        .tempdir_in(&job.parent)?
        .keep();
    let entry = folder.join(job.target.entry);
    let engine_entry = engine_path(&entry)?;
    let engine_workspace = engine_path(job.workspace.path())?;
    // Snapshots deliberately exclude .godot. Finish importing this exact snapshot
    // before export reads the generated script/resource caches.
    import_project(
        godot,
        job.workspace.path(),
        &folder.join("import.log"),
        cancelled,
    )?;
    let result = process::run_cancellable(
        godot,
        &[
            "--headless",
            "--path",
            ".",
            "--export-release",
            &job.target.name,
            &engine_entry,
        ],
        Some(Path::new(&engine_workspace)),
        Duration::from_secs(180),
        cancelled,
    );
    let result = match result {
        Ok(result) => result,
        Err(error) => {
            fs::write(folder.join("export.log"), error.to_string())?;
            return Err(error);
        }
    };
    fs::write(folder.join("export.log"), &result.text)?;
    check_stage(
        "导出",
        result.code,
        &result.text,
        &folder.join("export.log"),
    )?;
    if cancelled.load(Ordering::SeqCst) {
        bail!("导出已取消，未写入成功清单");
    }
    if job.target.platform == "Windows Desktop" {
        if let Some(directory) = &job.template_directory {
            crate::export_dependencies::collect(&folder, job.target.entry, directory)?;
        }
    }
    if cancelled.load(Ordering::SeqCst) {
        bail!("导出已取消，未写入成功清单");
    }
    let files = export_bundle::capture(&folder)?;
    let file = files
        .iter()
        .find(|file| file.path == job.target.entry)
        .context("导出产物缺少入口程序")?;
    let manifest = json!({"version":2,"project":job.project,"preset":job.target.name,"platform":job.target.platform,"entry":job.target.entry,"sha256":file.sha256,"files":files,"exportedAt":chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis,true),"runtimeVerified":false,"validation":job.provenance,"toolchain":toolchain});
    fs::write(
        folder.join("export-manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    if let Err(error) = export_bundle::verify(&folder) {
        fs::remove_file(folder.join("export-manifest.json"))?;
        return Err(error);
    }
    Ok(
        json!({"path":folder,"log":result.text,"bundleVerified":true,"runtimeVerified":false,"validation":job.provenance,"toolchain":toolchain}),
    )
}

pub fn import_project(godot: &Path, root: &Path, log: &Path, cancelled: &AtomicBool) -> Result<()> {
    let result = (|| -> Result<_> {
        let help =
            process::run_cancellable(godot, &["--help"], None, Duration::from_secs(15), cancelled)?;
        require_import_support(help.code, &help.text)?;
        let root = engine_path(root)?;
        process::run_cancellable(
            godot,
            &["--headless", "--path", ".", "--import"],
            Some(Path::new(&root)),
            Duration::from_secs(180),
            cancelled,
        )
    })();
    match result {
        Ok(imported) => {
            fs::write(log, &imported.text)?;
            check_stage("导入", imported.code, &imported.text, log)
        }
        Err(error) => {
            fs::write(log, error.to_string())?;
            bail!("导入未完成；日志：{}；{}", log.display(), error);
        }
    }
}

fn engine_path(path: &Path) -> Result<String> {
    let text = path.to_str().context("导出路径编码无效")?;
    // std::fs canonical paths carry Windows namespaces. Godot resource paths do
    // not accept those prefixes; filesystem validation still uses the original.
    Ok(if let Some(unc) = text.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{unc}")
    } else {
        text.strip_prefix(r"\\?\").unwrap_or(text).to_owned()
    })
}

pub fn delivery_record(result: &std::result::Result<Value, String>) -> Value {
    let mut record = json!({"checkedAt":chrono::Utc::now().to_rfc3339(),"scope":"export-snapshot","runtimeVerified":false});
    match result {
        Ok(value) => {
            record["status"] = json!("verified");
            record["path"] = value["path"].clone();
        }
        Err(error) => {
            record["status"] = json!("failed");
            record["message"] = json!(error);
        }
    }
    record
}

fn check_stage(stage: &str, code: i32, text: &str, log: &Path) -> Result<()> {
    if code != 0 || text.contains("SCRIPT ERROR") || text.contains("ERROR:") {
        let relevant: String = text
            .lines()
            .filter(|line| {
                line.contains("ERROR") || line.contains("WARNING") || line.contains(" at:")
            })
            .take(12)
            .collect::<Vec<_>>()
            .join("\n");
        bail!(
            "{stage}失败（退出码 {code}），未通过交付验证；日志：{}\n{}",
            log.display(),
            relevant.chars().take(2000).collect::<String>()
        );
    }
    Ok(())
}

fn require_import_support(code: i32, help: &str) -> Result<()> {
    if code != 0 || !help.contains("--import") {
        bail!("当前 Godot 不支持可靠的无界面导入（--import）。请在工具设置中选择支持 --import 的 Godot 4 编辑器（建议 4.4.1 或更新版本），然后重试操作；未通过验证。");
    }
    Ok(())
}

#[cfg(test)]
#[path = "game_export_project_tests.rs"]
mod project_tests;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn old_editors_fail_before_import_instead_of_hanging() {
        assert!(require_import_support(0, "--editor --quit").is_err());
        assert!(require_import_support(1, "--import").is_err());
        assert!(require_import_support(0, "--import Starts the editor").is_ok());
        assert!(require_import_support(0, "\u{1b}[92m--import\u{1b}[0m Starts the editor").is_ok());
    }
    #[test]
    fn engine_paths_remove_only_windows_namespace_prefix() -> Result<()> {
        assert_eq!(
            engine_path(Path::new(r"\\?\C:\out\Game.exe"))?,
            r"C:\out\Game.exe"
        );
        assert_eq!(
            engine_path(Path::new(r"\\?\UNC\server\share\Game.exe"))?,
            r"\\server\share\Game.exe"
        );
        assert_eq!(engine_path(Path::new("/tmp/Game"))?, "/tmp/Game");
        Ok(())
    }
    #[test]
    fn import_errors_remain_failures_and_delivery_never_claims_runtime() {
        assert!(check_stage(
            "import",
            0,
            "WARNING: headless cursor",
            Path::new("import.log")
        )
        .is_ok());
        let failure = check_stage(
            "import",
            0,
            "ERROR: missing .godot/global_script_class_cache.cfg",
            Path::new("import.log"),
        )
        .unwrap_err()
        .to_string();
        assert!(failure.contains("global_script_class_cache"));
        let record = delivery_record(&Err(failure));
        assert_eq!(record["status"], "failed");
        let record = delivery_record(&Ok(json!({"path":"bundle"})));
        assert_eq!(record["scope"], "export-snapshot");
        assert_eq!(record["runtimeVerified"], false);
    }
    #[test]
    fn preset_sections_do_not_confuse_options_or_platforms() {
        let parsed=targets("[preset.0]\nname=\"Windows Desktop\"\nplatform=\"Windows Desktop\"\n[preset.0.options]\nname=\"ignored\"\n[preset.1]\nname=\"Web\"\nplatform=\"Web\"\n");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].entry, "Game.exe");
        assert_eq!(parsed[0].name, "Windows Desktop");
        assert_eq!(parsed[1].entry, "");
    }

    #[test]
    fn custom_template_belongs_to_selected_preset() -> Result<()> {
        let config = "[preset.0]\nname=\"One\"\n[preset.0.options]\ncustom_template/release=\"res://one.exe\"\n[preset.1]\nname=\"Two\"\n[preset.1.options]\ncustom_template/release=\"\"\n";
        assert_eq!(
            custom_template(config, "One")?,
            Some("res://one.exe".into())
        );
        assert_eq!(custom_template(config, "Two")?, None);
        assert_eq!(custom_template(config, "Missing")?, None);
        Ok(())
    }
}
