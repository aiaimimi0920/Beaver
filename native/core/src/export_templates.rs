use anyhow::{bail, Context, Result};
use std::{
    io::Read,
    path::{Path, PathBuf},
};

pub const REQUIRED: [&str; 2] = ["windows_release_x86_64.exe", "windows_debug_x86_64.exe"];

pub fn version(output: &str) -> Result<String> {
    let expression =
        regex::Regex::new(r"^(4\.\d+(?:\.\d+)?)\.(stable|rc\d+|beta\d+|dev\d*)(?:\.|$)")?;
    let parts = expression
        .captures(output.trim())
        .context("无法识别 Godot 4 导出模板版本")?;
    Ok(format!("{}.{}", &parts[1], &parts[2]))
}

pub struct Release {
    pub version: String,
    pub filename: String,
    pub base: String,
}
pub fn release(output: &str) -> Result<Release> {
    let version = version(output)?;
    if !regex::Regex::new(r"^4\.\d+(?:\.\d+)?\.stable\.official\.")?.is_match(output.trim()) {
        bail!("自动下载仅支持官方稳定版 Godot；自定义引擎请导入匹配的模板包");
    }
    let tag = format!("{}-stable", version.trim_end_matches(".stable"));
    Ok(Release {
        version,
        filename: format!("Godot_v{tag}_export_templates.tpz"),
        base: format!("https://github.com/godotengine/godot-builds/releases/download/{tag}"),
    })
}

pub fn checksum(sums: &str, filename: &str) -> Result<String> {
    let expression = regex::Regex::new(r"(?i)^([a-f0-9]{128})\s+\*?(.+)$")?;
    let hashes: Vec<_> = sums
        .lines()
        .filter_map(|line| expression.captures(line.trim()))
        .filter(|parts| &parts[2] == filename)
        .map(|parts| parts[1].to_ascii_lowercase())
        .collect();
    if hashes.len() != 1 {
        bail!("官方校验清单中没有唯一匹配的模板包");
    }
    Ok(hashes[0].clone())
}

pub fn directory(executable: &Path, value: &str, app_data: Option<&Path>) -> Result<PathBuf> {
    if version(value)? != value {
        bail!("无效模板目录版本");
    }
    let folder = executable.parent().context("引擎目录无效")?;
    for marker in ["._sc_", "_sc_"] {
        if folder.join(marker).is_file() {
            return Ok(folder.join("editor_data/export_templates").join(value));
        }
    }
    let app_data = app_data
        .filter(|path| path.is_absolute())
        .context("无法确定 Godot 数据目录")?;
    Ok(app_data.join("Godot/export_templates").join(value))
}

pub fn ready(directory: &Path) -> bool {
    REQUIRED.iter().all(|name| -> bool {
        let Ok(path) = crate::files::safe_path(directory, name) else {
            return false;
        };
        let Ok(metadata) = std::fs::symlink_metadata(&path) else {
            return false;
        };
        if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() < 1024 {
            return false;
        }
        let mut header = [0; 2];
        std::fs::File::open(path)
            .and_then(|mut file| file.read_exact(&mut header))
            .is_ok()
            && header == *b"MZ"
    })
}

pub fn install_archive(
    archive: &Path,
    directory: &Path,
    expected: &str,
    cancelled: &std::sync::atomic::AtomicBool,
) -> Result<()> {
    use base64::Engine;
    use std::sync::atomic::Ordering;
    if !cfg!(windows) {
        bail!("模板自动安装当前仅支持 Windows");
    }
    if cancelled.load(Ordering::SeqCst) {
        bail!("模板准备已取消");
    }
    if version(expected)? != expected {
        bail!("无效模板版本");
    }
    match std::fs::symlink_metadata(directory) {
        Ok(_) => bail!("模板目录已经存在，未覆盖；请保留并检查已有模板"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let archive = std::fs::canonicalize(archive)?;
    if !archive.is_file() {
        bail!("模板包不是文件");
    }
    let parent = directory.parent().context("模板目录无效")?;
    std::fs::create_dir_all(parent)?;
    let parent = std::fs::canonicalize(parent)?;
    let destination = parent.join(directory.file_name().context("模板目录无效")?);
    let stage = tempfile::Builder::new()
        .prefix(".beaver-templates-")
        .tempdir_in(&parent)?;
    let quote = |value: &str| format!("'{}'", value.replace('\'', "''"));
    let script = format!(
        "$Archive = {}\n$Stage = {}\n$Expected = {}\n{}",
        quote(&crate::reveal::windows_path(&archive)?),
        quote(&crate::reveal::windows_path(stage.path())?),
        quote(expected),
        include_str!("template_extract.ps1")
    );
    let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    let system = std::env::var_os("SystemRoot").context("无法定位系统目录")?;
    let executable = PathBuf::from(system).join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let output = crate::process::run_cancellable(
        &executable,
        &["-NoProfile", "-NonInteractive", "-EncodedCommand", &encoded],
        None,
        std::time::Duration::from_secs(180),
        cancelled,
    )?;
    if output.code != 0 {
        bail!("模板包安装失败：{}", output.text);
    }
    if !ready(stage.path()) {
        bail!("模板程序验证失败");
    }
    if cancelled.load(Ordering::SeqCst) {
        bail!("模板准备已取消");
    }
    if std::fs::symlink_metadata(&destination).is_ok() {
        bail!("模板目录已经存在，未覆盖");
    }
    std::fs::rename(stage.path(), &destination)?;
    // TempDir only owns the original random staging path, never the published directory.
    Ok(())
}

async fn download_from(
    client: &reqwest::Client,
    release: &Release,
    destination: &Path,
    max_bytes: usize,
) -> Result<()> {
    use sha2::{Digest, Sha512};
    use std::io::Write;
    let mut response = client
        .get(format!("{}/SHA512-SUMS.txt", release.base))
        .send()
        .await?;
    if !response.status().is_success() {
        bail!("获取官方模板校验清单失败：HTTP {}", response.status());
    }
    let mut sums = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if chunk.len() > (1024 * 1024usize).saturating_sub(sums.len()) {
            bail!("校验清单过大");
        }
        sums.extend_from_slice(&chunk);
    }
    let expected = checksum(
        std::str::from_utf8(&sums).context("校验清单编码无效")?,
        &release.filename,
    )?;
    let mut response = client
        .get(format!("{}/{}", release.base, release.filename))
        .send()
        .await?;
    if !response.status().is_success() {
        bail!("下载模板失败：HTTP {}", response.status());
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    let mut size = 0usize;
    let mut hash = Sha512::new();
    while let Some(chunk) = response.chunk().await? {
        if chunk.len() > max_bytes.saturating_sub(size) {
            bail!("模板包超过 2 GiB 限制");
        }
        size += chunk.len();
        hash.update(&chunk);
        file.write_all(&chunk)?;
    }
    if format!("{:x}", hash.finalize()) != expected {
        bail!("模板包 SHA-512 校验失败，未安装");
    }
    file.sync_all()?;
    Ok(())
}

pub async fn download(
    output: &str,
    destination: &Path,
    cancelled: &std::sync::atomic::AtomicBool,
) -> Result<()> {
    use std::sync::atomic::Ordering;
    if cancelled.load(Ordering::SeqCst) {
        bail!("模板准备已取消");
    }
    let release = release(output)?;
    let client = reqwest::Client::builder()
        .https_only(true)
        .connect_timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()?;
    let stopped = async {
        loop {
            if cancelled.load(Ordering::SeqCst) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    };
    tokio::select! {
        _ = stopped => bail!("模板准备已取消"),
        result = tokio::time::timeout(std::time::Duration::from_secs(20 * 60),
            download_from(&client, &release, destination, 2 * 1024 * 1024 * 1024)) => {
            result.context("模板下载超时")?
        }
    }
}

pub async fn prepare(
    executable: &Path,
    archive: Option<&Path>,
    app_data: Option<&Path>,
    cancelled: &std::sync::atomic::AtomicBool,
) -> Result<serde_json::Value> {
    use std::sync::atomic::Ordering;
    if !cfg!(windows) {
        bail!("模板准备当前仅支持 Windows x86_64 导出");
    }
    if cancelled.load(Ordering::SeqCst) {
        bail!("模板准备已取消");
    }
    if let Some(bundle) = crate::godot_bundle::discover(executable, cancelled)? {
        anyhow::ensure!(
            archive.is_none(),
            "本地引擎已锚定配套模板，不接受其他模板包"
        );
        return Ok(
            serde_json::json!({"directory":bundle.directory,"version":bundle.version,
            "reused":true,"toolchain":bundle.receipt(executable)}),
        );
    }
    let probe = crate::process::run_cancellable(
        executable,
        &["--version"],
        None,
        std::time::Duration::from_secs(15),
        cancelled,
    )?;
    if probe.code != 0 {
        bail!("无法读取 Godot 版本");
    }
    let version = version(&probe.text)?;
    let directory = directory(executable, &version, app_data)?;
    if ready(&directory) {
        return Ok(serde_json::json!({"directory":directory,"version":version,"reused":true}));
    }
    match std::fs::symlink_metadata(&directory) {
        Ok(_) => bail!("已有模板目录不完整，未覆盖；请先检查并保留原有文件"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let temporary;
    let downloaded;
    let archive = if let Some(archive) = archive {
        archive
    } else {
        temporary = tempfile::Builder::new()
            .prefix("beaver-templates-")
            .tempdir()?;
        downloaded = temporary.path().join("templates.tpz");
        download(&probe.text, &downloaded, cancelled).await?;
        &downloaded
    };
    install_archive(archive, &directory, &version, cancelled)?;
    Ok(serde_json::json!({"directory":directory,"version":version,"reused":false}))
}

#[cfg(test)]
#[path = "export_templates_tests.rs"]
mod tests;
