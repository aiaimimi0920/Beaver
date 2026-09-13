use anyhow::{bail, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn detect(configured: &serde_json::Value, secrets: &[String]) -> Result<serde_json::Value> {
    let names = ["codex", "godot", "blender", "node"];
    for name in names {
        if configured[name]
            .as_str()
            .is_none_or(|s| s.encode_utf16().count() > 2000)
        {
            bail!("工具配置格式无效");
        }
    }
    let results = std::thread::scope(|scope| {
        let handles: Vec<_> = names
            .into_iter()
            .map(|name| {
                scope.spawn(move || {
                    detect_one(
                        name,
                        configured[name].as_str().unwrap_or(""),
                        secrets,
                        &std::sync::atomic::AtomicBool::new(false),
                    )
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| {
                handle
                    .join()
                    .map_err(|_| anyhow::anyhow!("工具检测线程异常"))?
            })
            .collect::<Result<Vec<_>>>()
    })?;
    Ok(serde_json::Value::Array(results))
}

pub fn detect_one(
    name: &str,
    configured: &str,
    secrets: &[String],
    cancelled: &std::sync::atomic::AtomicBool,
) -> Result<serde_json::Value> {
    use std::sync::atomic::Ordering;
    if !["codex", "godot", "blender", "node"].contains(&name)
        || configured.encode_utf16().count() > 2000
    {
        bail!("工具配置格式无效");
    }
    if cancelled.load(Ordering::SeqCst) {
        bail!("工具操作已取消");
    }
    #[cfg(windows)]
    crate::windows_tools::refresh();
    let mut path = String::new();
    let result = find(name, configured).and_then(|file| {
        path = file.to_string_lossy().into_owned();
        crate::process::run_cancellable(
            &file,
            &["--version"],
            None,
            std::time::Duration::from_secs(15),
            cancelled,
        )
    });
    if cancelled.load(Ordering::SeqCst) {
        bail!("工具操作已取消");
    }
    let (available, mut version) = match result {
        Ok(output) => {
            let version = output.text.lines().next().unwrap_or("").trim();
            let expression = match name {
                "node" => r"^v\d+\.\d+\.\d+(?:\s|$)",
                "codex" => r"(?i)^codex(?:-cli)?\s+\d+\.\d+",
                "godot" => r"(?i)^(?:Godot Engine v)?4\.\d+(?:\.|\s|$)",
                _ => r"(?i)^Blender\s+\d+\.\d+",
            };
            let valid = output.code == 0 && regex::Regex::new(expression)?.is_match(version);
            (
                valid,
                if valid {
                    version.to_owned()
                } else {
                    format!("版本验证失败：{version}")
                },
            )
        }
        Err(error) => (false, error.to_string()),
    };
    let mut secrets: Vec<_> = secrets.iter().filter(|s| !s.is_empty()).collect();
    secrets.sort_by_key(|s| std::cmp::Reverse(s.len()));
    for secret in secrets {
        version = version.replace(secret, "[REDACTED_SECRET]");
    }
    Ok(serde_json::json!({"name":name,"path":path,"available":available,"version":version}))
}

pub fn native_codex(directory: &Path) -> Option<PathBuf> {
    let (package, target, exe) = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => ("codex-win32-x64", "x86_64-pc-windows-msvc", "codex.exe"),
        ("windows", "aarch64") => ("codex-win32-arm64", "aarch64-pc-windows-msvc", "codex.exe"),
        ("macos", "aarch64") => ("codex-darwin-arm64", "aarch64-apple-darwin", "codex"),
        ("macos", "x86_64") => ("codex-darwin-x64", "x86_64-apple-darwin", "codex"),
        ("linux", "aarch64") => ("codex-linux-arm64", "aarch64-unknown-linux-musl", "codex"),
        ("linux", "x86_64") => ("codex-linux-x64", "x86_64-unknown-linux-musl", "codex"),
        _ => return None,
    };
    for base in [
        directory.join("node_modules/@openai/codex"),
        directory.join("../lib/node_modules/@openai/codex"),
    ] {
        for path in [
            base.parent()?
                .join(format!("{package}/vendor/{target}/bin/{exe}")),
            base.join(format!(
                "node_modules/@openai/{package}/vendor/{target}/bin/{exe}"
            )),
            base.join(format!("vendor/{target}/bin/{exe}")),
        ] {
            if path.is_file() {
                return Some(path);
            }
        }
    }
    None
}

fn candidate(name: &str, path: &Path) -> Option<PathBuf> {
    if name == "codex"
        && (!path.is_file()
            || (cfg!(windows)
                && path
                    .extension()
                    .and_then(|s| s.to_str())
                    .is_none_or(|s| !s.eq_ignore_ascii_case("exe"))))
    {
        // npm's JS shim is not a native app-server. Resolve its vendored executable.
        if let Some(native) = path.parent().and_then(native_codex) {
            return Some(native);
        }
    }
    if !path.is_file() {
        return None;
    }
    if cfg!(windows)
        && path
            .extension()
            .and_then(|s| s.to_str())
            .is_none_or(|s| !s.eq_ignore_ascii_case("exe"))
    {
        return None;
    }
    if name == "godot" {
        let file = path.file_name()?.to_string_lossy().to_ascii_lowercase();
        if file
            .split(['.', '_', '-'])
            .any(|s| ["template", "headless", "server"].contains(&s))
        {
            return None;
        }
    }
    Some(path.into())
}

pub fn find(name: &str, configured: &str) -> Result<PathBuf> {
    if !["codex", "godot", "blender", "node"].contains(&name) {
        bail!("未知工具");
    }
    if !configured.is_empty() {
        #[cfg(windows)]
        if ["godot", "blender"].contains(&name)
            && ["cmd", "bat"].iter().any(|extension| {
                configured
                    .to_ascii_lowercase()
                    .ends_with(&format!(".{extension}"))
            })
        {
            let path = crate::windows_tools::resolve(name, &[configured.to_owned()])
                .ok_or_else(|| anyhow::anyhow!("无法解析工具启动脚本，请选择实际的 EXE 文件"))?;
            return Ok(fs::canonicalize(path)?);
        }
        let path = candidate(name, Path::new(configured))
            .ok_or_else(|| anyhow::anyhow!("工具路径无效，请选择实际的原生可执行程序"))?;
        return Ok(fs::canonicalize(path)?);
    }
    let executable = format!("{name}{}", if cfg!(windows) { ".exe" } else { "" });
    let mut paths: Vec<PathBuf> =
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| p.join(&executable))
            .collect();
    if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
        let home = PathBuf::from(home);
        paths.push(home.join(format!("scoop/apps/{name}/current/{executable}")));
        if name == "codex" {
            paths.push(home.join(format!("AppData/Roaming/npm/{executable}")));
            paths.push(home.join(format!(".local/bin/{executable}")));
        }
    }
    if name == "godot" {
        if let Some(path) = std::env::var_os("GODOT_PATH") {
            paths.insert(0, path.into());
        }
        paths.push("/Applications/Godot.app/Contents/MacOS/Godot".into());
    }
    if name == "blender" {
        if let Some(path) = std::env::var_os("BLENDER_PATH") {
            paths.insert(0, path.into());
        }
        paths.push("/Applications/Blender.app/Contents/MacOS/Blender".into());
    }
    #[cfg(windows)]
    {
        let program_files = PathBuf::from(
            std::env::var_os("ProgramFiles").unwrap_or_else(|| "C:/Program Files".into()),
        );
        if name == "node" {
            paths.push(program_files.join("nodejs/node.exe"));
        }
        if name == "blender" {
            for entry in fs::read_dir(program_files.join("Blender Foundation"))
                .into_iter()
                .flatten()
                .flatten()
            {
                paths.push(entry.path().join("blender.exe"));
            }
        }
        if ["godot", "blender"].contains(&name) {
            for directory in std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
                .filter(|p| !p.as_os_str().is_empty())
            {
                paths.extend([
                    directory.join(format!("{name}.cmd")),
                    directory.join(format!("{name}.bat")),
                    directory,
                ]);
            }
            if let Some(home) = std::env::var_os("USERPROFILE") {
                paths.push(PathBuf::from(home).join(format!("scoop/apps/{name}/current")));
            }
            if let Some(local) = std::env::var_os("LOCALAPPDATA") {
                paths.push(PathBuf::from(local).join("Microsoft/WinGet/Links"));
            }
            let hints: Vec<String> = paths
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect();
            let found = crate::windows_tools::resolve(name, &hints).or_else(|| {
                crate::windows_tools::resolve(name, &crate::windows_tools::registered())
            });
            if let Some(path) = found {
                return Ok(fs::canonicalize(path)?);
            }
            bail!("未找到 {name}，请安装或在设置中选择程序路径");
        }
    }
    for path in paths {
        if let Some(path) = candidate(name, &path) {
            return Ok(fs::canonicalize(path)?);
        }
    }
    bail!("未找到 {name}，请安装或在设置中选择程序路径")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn configured_path_is_checked_without_running_it() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let exe = temp.path().join("godot.exe");
        fs::write(&exe, b"fixture")?;
        assert_eq!(
            find("godot", exe.to_str().unwrap())?,
            fs::canonicalize(exe)?
        );
        let template = temp.path().join("godot.template.exe");
        fs::write(&template, b"fixture")?;
        assert!(find("godot", template.to_str().unwrap()).is_err());
        assert!(find("unknown", "").is_err());
        Ok(())
    }
}
