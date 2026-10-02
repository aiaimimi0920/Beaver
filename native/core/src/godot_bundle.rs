use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    time::Duration,
};

pub const DEBUG: &str = "godot.windows.template_debug.x86_64.exe";
pub const RELEASE: &str = "godot.windows.template_release.x86_64.exe";

#[derive(Debug)]
pub struct Bundle {
    pub directory: PathBuf,
    pub debug: PathBuf,
    pub release: PathBuf,
    pub version: String,
}

fn require_version(expected: &str, actual: &str) -> Result<()> {
    anyhow::ensure!(
        actual == expected,
        "本地导出模板与编辑器版本不一致，未回退其他版本"
    );
    Ok(())
}

pub fn probe(executable: &Path, cancelled: &AtomicBool) -> Result<String> {
    let result = crate::process::run_cancellable(
        executable,
        &["--version"],
        None,
        Duration::from_secs(15),
        cancelled,
    )?;
    anyhow::ensure!(
        result.code == 0,
        "无法读取 Godot 版本：{}",
        executable.display()
    );
    crate::export_templates::version(&result.text)?;
    Ok(result.text.trim().to_owned())
}

// Source-built Windows editors keep their matching templates beside the editor.
// Official installations retain their existing TPZ/default-directory behavior.
pub fn discover(executable: &Path, cancelled: &AtomicBool) -> Result<Option<Bundle>> {
    let name = executable
        .file_name()
        .context("引擎文件名无效")?
        .to_string_lossy();
    if ![
        "godot.windows.editor.x86_64.exe",
        "godot.windows.editor.x86_64.console.exe",
    ]
    .iter()
    .any(|expected| name.eq_ignore_ascii_case(expected))
    {
        return Ok(None);
    }
    let directory = fs::canonicalize(executable.parent().context("引擎目录无效")?)?;
    let debug = directory.join(DEBUG);
    let release = directory.join(RELEASE);
    for path in [&debug, &release] {
        let metadata = fs::symlink_metadata(path)
            .with_context(|| format!("本地配套导出模板缺失，未回退其他版本：{}", path.display()))?;
        anyhow::ensure!(
            metadata.is_file() && !metadata.file_type().is_symlink() && metadata.len() >= 1024,
            "本地导出模板不是有效文件：{}",
            path.display()
        );
    }
    let version = probe(executable, cancelled)?;
    for path in [&debug, &release] {
        require_version(&version, &probe(path, cancelled)?)
            .with_context(|| format!("本地模板版本验证失败：{}", path.display()))?;
    }
    Ok(Some(Bundle {
        directory,
        debug,
        release,
        version,
    }))
}

impl Bundle {
    pub fn receipt(&self, executable: &Path) -> Value {
        json!({"source":"editor-siblings", "engine":executable, "version":self.version,
            "debugTemplate":self.debug, "releaseTemplate":self.release})
    }

    pub fn preset(&self, config: &str, preset: &str, root: &Path) -> Result<String> {
        let mut section = "";
        let mut selected = Vec::new();
        let header = regex::Regex::new(r"^\[preset\.\d+\]$")?;
        for line in config.lines().map(str::trim) {
            if line.starts_with('[') {
                section = line;
            }
            if let Some((key, value)) = line.split_once('=') {
                if header.is_match(section)
                    && key.trim() == "name"
                    && serde_json::from_str::<String>(value.trim()).ok().as_deref() == Some(preset)
                {
                    selected.push(format!("{}.options]", section.trim_end_matches(']')));
                }
            }
        }
        anyhow::ensure!(selected.len() == 1, "Godot 导出预设不存在或名称重复");
        let options = &selected[0];
        let mut output = Vec::new();
        let mut found = false;
        for line in config.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                section = trimmed;
                if section == options {
                    found = true;
                    output.push(line.to_owned());
                    for (key, path) in [("debug", &self.debug), ("release", &self.release)] {
                        let path = crate::reveal::windows_path(path)?.replace('\\', "/");
                        output.push(format!(
                            "custom_template/{key}={}",
                            serde_json::to_string(&path)?
                        ));
                    }
                    continue;
                }
            }
            if section == options {
                if let Some((key, value)) = trimmed.split_once('=') {
                    let expected = match key.trim() {
                        "custom_template/debug" => Some(&self.debug),
                        "custom_template/release" => Some(&self.release),
                        _ => None,
                    };
                    if let Some(expected) = expected {
                        let value: String = serde_json::from_str(value.trim())?;
                        if !value.is_empty() {
                            let path = if let Some(relative) = value.strip_prefix("res://") {
                                crate::files::safe_path(root, relative)?
                            } else if Path::new(&value).is_absolute() {
                                PathBuf::from(value)
                            } else {
                                crate::files::safe_path(root, &value)?
                            };
                            if fs::canonicalize(path)? != fs::canonicalize(expected)? {
                                bail!("项目自定义模板与当前本地配套模板冲突；请显式调整预设，未覆盖原项目");
                            }
                        }
                        continue;
                    }
                    if key.trim() == "binary_format/architecture" {
                        anyhow::ensure!(
                            serde_json::from_str::<String>(value.trim())? == "x86_64",
                            "本地配套模板仅支持 Windows x86_64"
                        );
                    }
                }
            }
            output.push(line.to_owned());
        }
        if !found {
            output.push(options.to_owned());
            for (key, path) in [("debug", &self.debug), ("release", &self.release)] {
                let path = crate::reveal::windows_path(path)?.replace('\\', "/");
                output.push(format!(
                    "custom_template/{key}={}",
                    serde_json::to_string(&path)?
                ));
            }
        }
        Ok(output.join("\n") + "\n")
    }
}

#[cfg(test)]
#[path = "godot_bundle_tests.rs"]
mod tests;
