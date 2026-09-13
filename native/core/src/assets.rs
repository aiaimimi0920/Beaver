use crate::{files::safe_path, store::Store};
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

pub fn resolve_asset(store: &Store, uri_path: &str) -> Result<PathBuf> {
    let (id, encoded) = uri_path
        .strip_prefix('/')
        .context("无效资源地址")?
        .split_once('/')
        .context("缺少资源路径")?;
    if id.is_empty() || id.contains(['%', ':', '\\']) {
        bail!("无效资源标识");
    }
    let (kind, id, field) = if let Some(task) = id.strip_prefix("task-") {
        ("task", task, "workspace")
    } else {
        ("project", id, "path")
    };
    let entity: Value = store.get(kind, id)?.context("资源所属项目或任务不存在")?;
    let root = entity[field].as_str().context("资源根目录无效")?;
    let relative = percent_encoding::percent_decode_str(encoded).decode_utf8()?;
    safe_path(Path::new(root), &relative)
}

pub fn import_root(store: &Store, id: &str) -> Result<PathBuf> {
    uuid::Uuid::parse_str(id).context("项目标识无效")?;
    crate::documents::project_path(store, id)
}

pub fn import_files(
    store: &mut Store,
    data: &Path,
    id: &str,
    sources: &[PathBuf],
) -> Result<Vec<String>> {
    let root = import_root(store, id)?;
    if crate::journal::Journal::new(store, &crate::files::Files::new(data.to_path_buf()))
        .blocked(id)?
    {
        bail!("项目有尚未恢复的文件操作，禁止继续写入");
    }
    let invalid = regex::Regex::new(r"[^\p{L}\p{N}._ -]")?;
    let mut imported = Vec::new();
    for source in sources {
        if !std::fs::metadata(source)?.is_file() {
            continue;
        }
        let name = source
            .file_name()
            .and_then(|name| name.to_str())
            .context("素材文件名不是有效 UTF-8")?;
        let name = invalid.replace_all(name, "_");
        let relative = format!(
            "references/{}-{name}",
            &uuid::Uuid::new_v4().to_string()[..8]
        );
        let destination = safe_path(&root, &relative)?;
        std::fs::create_dir_all(destination.parent().context("素材目录无效")?)?;
        let mut input = File::open(source)?;
        // Exclusive creation preserves existing assets; recheck links after mkdir.
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(safe_path(&root, &relative)?)?;
        std::io::copy(&mut input, &mut output)?;
        output.sync_all()?;
        imported.push(relative);
    }
    // Match the original batch contract: earlier copies survive a later failure.
    Ok(imported)
}

pub fn save_capture(store: &mut Store, data: &Path, id: &str, png: &[u8]) -> Result<String> {
    use std::io::Write;
    let root = import_root(store, id)?;
    if crate::journal::Journal::new(store, &crate::files::Files::new(data.to_owned()))
        .blocked(id)?
    {
        bail!("项目有尚未恢复的文件操作，禁止继续写入");
    }
    if !png.starts_with(b"\x89PNG\r\n\x1a\n") || png.len() > 16 * 1024 * 1024 {
        bail!("截图 PNG 数据无效");
    }
    let relative = format!("references/capture-{}.png", uuid::Uuid::new_v4());
    let destination = safe_path(&root, &relative)?;
    std::fs::create_dir_all(destination.parent().context("截图目录无效")?)?;
    let destination = safe_path(&root, &relative)?;
    let mut temporary =
        tempfile::NamedTempFile::new_in(destination.parent().context("截图目录无效")?)?;
    temporary.write_all(png)?;
    temporary.as_file().sync_all()?;
    temporary.persist_noclobber(safe_path(&root, &relative)?)?;
    Ok(relative)
}

pub fn byte_range(value: &str, length: u64) -> Result<(u64, u64)> {
    if length == 0 {
        bail!("空文件没有可读取的范围");
    }
    let (start, end) = value
        .strip_prefix("bytes=")
        .context("无效范围单位")?
        .split_once('-')
        .context("无效范围")?;
    if value.contains(',') {
        bail!("不支持多段范围");
    }
    if start.is_empty() {
        let count: u64 = end.parse()?;
        if count == 0 {
            bail!("无效后缀长度");
        }
        return Ok((length.saturating_sub(count), length - 1));
    }
    let start: u64 = start.parse()?;
    let end: u64 = if end.is_empty() {
        length - 1
    } else {
        end.parse()?
    };
    if start >= length || end < start {
        bail!("范围超出文件");
    }
    Ok((start, end.min(length - 1)))
}

pub struct AssetResponse {
    pub status: u16,
    pub content_type: String,
    pub content_range: Option<String>,
    pub length: u64,
    pub bytes: Vec<u8>,
}
pub fn read_asset(path: &Path, range: Option<&str>, head: bool) -> Result<AssetResponse> {
    let mut file = File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        bail!("不是文件");
    }
    let length = metadata.len();
    let (status, start, end) = if let Some(range) = range {
        match byte_range(range, length) {
            Ok((start, end)) => (206, start, end),
            Err(_) => {
                return Ok(AssetResponse {
                    status: 416,
                    content_type: "text/plain".into(),
                    content_range: Some(format!("bytes */{length}")),
                    length: 0,
                    bytes: Vec::new(),
                })
            }
        }
    } else {
        (200, 0, length.saturating_sub(1))
    };
    let size = if length == 0 { 0 } else { end - start + 1 };
    if size > 256 * 1024 * 1024 && !head {
        bail!("单次预览读取超过 256 MiB，请使用范围读取或外部工具");
    }
    let mut bytes = Vec::new();
    if !head {
        file.seek(SeekFrom::Start(start))?;
        file.take(size).read_to_end(&mut bytes)?;
        if bytes.len() as u64 != size {
            bail!("读取时资源大小发生变化");
        }
    }
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    let mime = match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "mp3" => "audio/mpeg",
        "flac" => "audio/flac",
        "m4a" => "audio/mp4",
        "glb" => "model/gltf-binary",
        "gltf" => "model/gltf+json",
        "json" => "application/json",
        "obj" | "md" | "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    };
    Ok(AssetResponse {
        status,
        content_type: mime.into(),
        content_range: if status == 206 {
            Some(format!("bytes {start}-{end}/{length}"))
        } else {
            None
        },
        length: size,
        bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn capture_is_published_once_under_registered_project() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("project");
        std::fs::create_dir(&root)?;
        let data = temp.path().join("data");
        let mut store = Store::open(&data)?;
        let id = uuid::Uuid::new_v4().to_string();
        store.put("project", &id, &json!({"path":root}))?;
        let mut encoded = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(2, 2).write_to(&mut encoded, image::ImageFormat::Png)?;
        let png = encoded.into_inner();
        assert!(save_capture(&mut store, &data, "invalid", &png).is_err());
        assert!(save_capture(&mut store, &data, &id, b"invalid png").is_err());
        assert!(!root.join("references").exists());
        let first = save_capture(&mut store, &data, &id, &png)?;
        let second = save_capture(&mut store, &data, &id, &png)?;
        assert_ne!(first, second);
        for relative in [first, second] {
            assert!(relative.starts_with("references/capture-") && relative.ends_with(".png"));
            assert_eq!(std::fs::read(safe_path(&root, &relative)?)?, png);
        }
        assert_eq!(std::fs::read_dir(root.join("references"))?.count(), 2);
        Ok(())
    }
    #[test]
    fn ranges_and_head_have_correct_lengths() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let file = temp.path().join("sample.wav");
        std::fs::write(&file, b"0123456789")?;
        assert_eq!(byte_range("bytes=2-4", 10)?, (2, 4));
        assert_eq!(byte_range("bytes=-3", 10)?, (7, 9));
        assert_eq!(byte_range("bytes=8-99", 10)?, (8, 9));
        assert!(byte_range("bytes=0-1,3-4", 10).is_err());
        let response = read_asset(&file, Some("bytes=2-4"), false)?;
        assert_eq!(response.status, 206);
        assert_eq!(response.bytes, b"234");
        assert_eq!(response.content_range.as_deref(), Some("bytes 2-4/10"));
        let head = read_asset(&file, None, true)?;
        assert_eq!(head.length, 10);
        assert!(head.bytes.is_empty());
        assert_eq!(read_asset(&file, Some("bytes=10-"), false)?.status, 416);
        Ok(())
    }
    #[test]
    fn imports_keep_bytes_names_and_previous_success_on_failure() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let data = temp.path().join("data");
        let root = temp.path().join("project");
        std::fs::create_dir(&root)?;
        let mut store = Store::open(&data)?;
        let id = uuid::Uuid::new_v4().to_string();
        store.put("project", &id, &json!({"path":root}))?;
        let source = temp.path().join("角色 🎮.png");
        let bytes = [0, 255, 10, 13, 128];
        std::fs::write(&source, bytes)?;
        let imported = import_files(
            &mut store,
            &data,
            &id,
            &[source.clone(), root.clone(), source.clone()],
        )?;
        assert_eq!(imported.len(), 2);
        assert_ne!(imported[0], imported[1]);
        for relative in &imported {
            assert!(relative.ends_with("-角色 _.png"));
            assert_eq!(std::fs::read(safe_path(&root, relative)?)?, bytes);
        }
        assert_eq!(std::fs::read(&source)?, bytes);
        assert!(import_files(
            &mut store,
            &data,
            &id,
            &[source, temp.path().join("missing")]
        )
        .is_err());
        assert_eq!(std::fs::read_dir(root.join("references"))?.count(), 3);
        assert!(import_files(&mut store, &data, "invalid", &[]).is_err());
        assert!(import_files(&mut store, &data, &uuid::Uuid::new_v4().to_string(), &[]).is_err());
        Ok(())
    }
    #[test]
    fn import_rejects_linked_destination() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let data = temp.path().join("data");
        let root = temp.path().join("project");
        let outside = temp.path().join("outside");
        std::fs::create_dir(&root)?;
        std::fs::create_dir(&outside)?;
        let link = root.join("references");
        #[cfg(windows)]
        {
            let status = std::process::Command::new("cmd.exe")
                .args(["/d", "/c", "mklink", "/J"])
                .arg(&link)
                .arg(&outside)
                .output()?;
            assert!(status.status.success());
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, &link)?;
        let mut store = Store::open(&data)?;
        let id = uuid::Uuid::new_v4().to_string();
        store.put("project", &id, &json!({"path":root}))?;
        let source = temp.path().join("source.png");
        std::fs::write(&source, b"fixture")?;
        assert!(import_files(&mut store, &data, &id, &[source]).is_err());
        assert!(save_capture(&mut store, &data, &id, b"\x89PNG\r\n\x1a\n").is_err());
        assert_eq!(std::fs::read_dir(&outside)?.count(), 0);
        #[cfg(windows)]
        std::fs::remove_dir(&link)?;
        #[cfg(unix)]
        std::fs::remove_file(&link)?;
        Ok(())
    }
    #[test]
    fn only_registered_roots_are_readable() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let store = Store::open(&temp.path().join("data"))?;
        store.put("project", "p", &json!({"path":temp.path()}))?;
        store.put("task", "t", &json!({"workspace":temp.path()}))?;
        assert_eq!(
            resolve_asset(&store, "/p/%E4%B8%96%E7%95%8C.md")?,
            std::fs::canonicalize(temp.path())?.join("世界.md")
        );
        assert!(resolve_asset(&store, "/task-t/a.wav").is_ok());
        for invalid in [
            "/unknown/a",
            "/p/%2e%2e/a",
            "/p/C%3A/a",
            "/p/a%5Cb",
            "/p/%2Fa",
            "/p/%ff",
        ] {
            assert!(resolve_asset(&store, invalid).is_err(), "{invalid}");
        }
        Ok(())
    }
}
