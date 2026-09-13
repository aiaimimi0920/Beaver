use regex::Regex;
use std::{
    fs,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

fn executable(name: &str, path: &Path) -> bool {
    let file = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase();
    if name == "blender" {
        return file == "blender.exe";
    }
    (file == "godot.exe"
        || ["godot_", "godot.", "godot-"]
            .iter()
            .any(|s| file.starts_with(s)))
        && file.ends_with(".exe")
        && !file
            .split(['.', '_', '-'])
            .any(|s| ["template", "server", "headless"].contains(&s))
}

fn hint_path(hint: &str) -> PathBuf {
    let hint = hint.trim();
    if let Some(rest) = hint.strip_prefix('"') {
        if let Some(end) = rest.find('"') {
            return rest[..end].into();
        }
    }
    static SUFFIX: OnceLock<Regex> = OnceLock::new();
    SUFFIX
        .get_or_init(|| Regex::new(r#"(?i)(\.exe)(?:,\s*-?\d+|\s+[-"%]).*$"#).unwrap())
        .replace(hint, "$1")
        .as_ref()
        .into()
}

fn launcher(path: &Path, name: &str) -> Option<PathBuf> {
    if fs::metadata(path).ok()?.len() >= 32768 {
        return None;
    }
    let text = fs::read_to_string(path).ok()?;
    let expression =
        Regex::new(r#"(?im)^\s*@?(?:call\s+)?"([^"\r\n]+)"(?:\s+%[*1-9]|\s*$)"#).ok()?;
    let variables = Regex::new(r"%([^%]+)%").ok()?;
    let directory = path.parent()?.to_string_lossy().to_string() + "\\";
    let dp0 = Regex::new("(?i)%~dp0").ok()?;
    for captures in expression.captures_iter(&text) {
        let raw = dp0.replace_all(&captures[1], directory.as_str());
        let expanded = variables.replace_all(&raw, |captures: &regex::Captures<'_>| {
            std::env::vars()
                .find(|(key, _)| key.eq_ignore_ascii_case(&captures[1]))
                .map(|(_, v)| v)
                .unwrap_or_else(|| captures[0].to_owned())
        });
        let target = PathBuf::from(expanded.as_ref());
        if !expanded.contains('%')
            && target.is_absolute()
            && executable(name, &target)
            && target.is_file()
        {
            return Some(target);
        }
    }
    None
}

fn file(name: &str, path: &Path) -> Option<PathBuf> {
    if !path.is_file() {
        return None;
    }
    let extension = path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase();
    if ["cmd", "bat"].contains(&extension.as_str()) {
        return launcher(path, name);
    }
    if executable(name, path) {
        let filename = path.file_name()?.to_string_lossy();
        let gui = Regex::new(r"(?i)(?:_console|\.console)\.exe$")
            .ok()?
            .replace(&filename, ".exe");
        let gui = path.with_file_name(gui.as_ref());
        return Some(if gui.is_file() { gui } else { path.into() });
    }
    if name == "blender"
        && path
            .file_name()?
            .to_string_lossy()
            .eq_ignore_ascii_case("blender-launcher.exe")
    {
        let target = path.parent()?.join("blender.exe");
        if target.is_file() {
            return Some(target);
        }
    }
    None
}

fn natural_cmp(left: &str, right: &str) -> std::cmp::Ordering {
    static PARTS: OnceLock<Regex> = OnceLock::new();
    let parts = PARTS.get_or_init(|| Regex::new(r"\d+|\D+").unwrap());
    let mut left = parts.find_iter(left);
    let mut right = parts.find_iter(right);
    loop {
        match (left.next(), right.next()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(a), Some(b)) => {
                let (a, b) = (a.as_str(), b.as_str());
                let order = if a.bytes().all(|v| v.is_ascii_digit())
                    && b.bytes().all(|v| v.is_ascii_digit())
                {
                    let (a, b) = (a.trim_start_matches('0'), b.trim_start_matches('0'));
                    a.len().cmp(&b.len()).then_with(|| a.cmp(b))
                } else {
                    a.cmp(b)
                };
                if !order.is_eq() {
                    return order;
                }
            }
        }
    }
}

pub fn resolve(name: &str, hints: &[String]) -> Option<PathBuf> {
    for hint in hints {
        let path = hint_path(hint);
        if path.is_dir() {
            for directory in [path.clone(), path.join("bin")] {
                let mut entries: Vec<PathBuf> = fs::read_dir(directory)
                    .into_iter()
                    .flatten()
                    .filter_map(|entry| entry.ok().map(|e| e.path()))
                    .filter(|p| executable(name, p))
                    .collect();
                entries.sort_by(|a, b| {
                    let a = a
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_ascii_lowercase();
                    let b = b
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_ascii_lowercase();
                    a.contains("console")
                        .cmp(&b.contains("console"))
                        .then_with(|| natural_cmp(&b, &a))
                });
                for path in entries {
                    if let Some(path) = file(name, &path) {
                        return Some(path);
                    }
                }
            }
        } else if let Some(path) = file(name, &path) {
            return Some(path);
        }
    }
    None
}

fn collect() -> Option<Vec<String>> {
    use base64::Engine;
    use std::os::windows::process::CommandExt;
    let script = include_str!("../../../dist-native/tool-discovery-windows.ps1");
    let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    let executable =
        PathBuf::from(std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into()))
            .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let mut output = tempfile::tempfile().ok()?;
    let mut process = Command::new(executable)
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-EncodedCommand",
            &encoded,
        ])
        .creation_flags(0x08000000)
        .stdin(Stdio::null())
        .stdout(output.try_clone().ok()?)
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + Duration::from_secs(12);
    let status = loop {
        match process.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(30)),
            _ => {
                let _ = process.kill();
                let _ = process.wait();
                return None;
            }
        }
    };
    if !status.success() {
        return None;
    }
    output.seek(SeekFrom::Start(0)).ok()?;
    let mut bytes = Vec::new();
    output.take(1024 * 1024 + 1).read_to_end(&mut bytes).ok()?;
    if bytes.len() > 1024 * 1024 {
        return None;
    }
    let text = std::str::from_utf8(&bytes)
        .ok()?
        .trim_start_matches('\u{feff}')
        .trim();
    let mut hints: Vec<String> = serde_json::from_str(text).ok()?;
    hints.truncate(500);
    Some(hints)
}

static CACHE: OnceLock<Mutex<Option<(Instant, Vec<String>)>>> = OnceLock::new();

pub fn refresh() {
    if let Some(cache) = CACHE.get() {
        if let Ok(mut cache) = cache.lock() {
            *cache = None;
        }
    }
}

pub fn registered() -> Vec<String> {
    let Ok(mut cache) = CACHE.get_or_init(|| Mutex::new(None)).lock() else {
        return Vec::new();
    };
    if let Some((at, hints)) = cache.as_ref() {
        if at.elapsed() < Duration::from_secs(30) {
            return hints.clone();
        }
    }
    let hints = collect().unwrap_or_default();
    *cache = Some((Instant::now(), hints.clone()));
    hints
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn numeric_versions_are_ordered_naturally() {
        assert!(natural_cmp("godot_v4.10.exe", "godot_v4.9.exe").is_gt());
        assert!(natural_cmp("godot_v4.9.exe", "godot_v4.09.exe").is_eq());
    }
    #[test]
    fn registered_paths_launchers_and_templates() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let bin = temp.path().join("bin");
        fs::create_dir(&bin)?;
        let gui = bin.join("godot.windows.editor.x86_64.exe");
        fs::write(&gui, "fixture")?;
        fs::write(
            bin.join("godot.windows.editor.x86_64.console.exe"),
            "fixture",
        )?;
        fs::write(bin.join("godot.template.exe"), "fixture")?;
        assert_eq!(
            resolve("godot", &[temp.path().to_string_lossy().into_owned()]),
            Some(gui.clone())
        );
        assert_eq!(
            resolve("godot", &[format!("\"{}\" --editor %1", gui.display())]),
            Some(gui.clone())
        );
        let script = temp.path().join("godot.cmd");
        fs::write(
            &script,
            "@\"%~dp0bin\\godot.windows.editor.x86_64.exe\" %*\n",
        )?;
        assert_eq!(
            resolve("godot", &[script.to_string_lossy().into_owned()]),
            Some(gui)
        );
        assert!(resolve(
            "godot",
            &[bin
                .join("godot.template.exe")
                .to_string_lossy()
                .into_owned()]
        )
        .is_none());
        assert!(!executable("godot", Path::new("godot-server.exe")));
        Ok(())
    }
}
