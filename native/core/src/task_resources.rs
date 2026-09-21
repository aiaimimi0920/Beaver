use crate::files::{file_hash, list_files, safe_path, Files, Snapshot};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{
    collections::BTreeSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

fn workspace(files: &Files, task: &Value) -> Result<PathBuf> {
    files.resolve_workspace(
        task["id"].as_str().context("任务标识无效")?,
        Path::new(task["workspace"].as_str().context("任务工作副本无效")?),
    )
}

pub fn resources(files: &Files, task: &Value) -> Result<Value> {
    if task["workspacePrepared"] == false {
        return Ok(json!([]));
    }
    let workspace = workspace(files, task)?;
    let baseline: Snapshot = serde_json::from_value(task["baseline"].clone())?;
    let mut paths: Vec<(String, &'static str)> = Vec::new();
    fn add(paths: &mut Vec<(String, &'static str)>, path: &str, origin: &'static str) {
        if let Some(item) = paths.iter_mut().find(|item| item.0 == path) {
            item.1 = origin;
        } else {
            paths.push((path.into(), origin));
        }
    }
    for item in task["references"].as_array().context("任务参考记录无效")? {
        add(
            &mut paths,
            item["path"].as_str().context("参考路径无效")?,
            "参考",
        );
    }
    for item in task["changes"].as_array().context("任务变更记录无效")? {
        add(
            &mut paths,
            item["path"].as_str().context("变更路径无效")?,
            "修改",
        );
    }
    let current: BTreeSet<String> = list_files(&workspace)?.into_iter().collect();
    for path in baseline.keys() {
        if !current.contains(path) {
            add(&mut paths, path, "删除");
        }
    }
    for path in current {
        if !paths.iter().any(|item| item.0 == path)
            && file_hash(&safe_path(&workspace, &path)?)?.as_ref() != baseline.get(&path)
        {
            add(&mut paths, &path, "修改");
        }
    }
    let mut result = Vec::new();
    for (path, origin) in paths {
        let exists = match fs::metadata(safe_path(&workspace, &path)?) {
            Ok(metadata) => metadata.is_file(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) => return Err(error.into()),
        };
        result.push(json!({"path":path,"origin":origin,"exists":exists}));
    }
    Ok(json!(result))
}

fn bytes(files: &Files, task: &Value, relative: &str) -> Result<Vec<u8>> {
    let workspace = workspace(files, task)?;
    let handle = fs::File::open(safe_path(&workspace, relative)?)?;
    if !handle.metadata()?.is_file() {
        bail!("资源不是文件");
    }
    if handle.metadata()?.len() > 512000 {
        bail!("文件超过 500 KB，请在工作副本中查看");
    }
    let mut bytes = Vec::new();
    handle.take(512001).read_to_end(&mut bytes)?;
    if bytes.len() > 512000 {
        bail!("文件超过 500 KB");
    }
    Ok(bytes)
}

pub fn decode_text(bytes: &[u8]) -> Result<String> {
    let text = if bytes.starts_with(&[0xff, 0xfe, 0, 0]) || bytes.starts_with(&[0, 0, 0xfe, 0xff]) {
        bail!("暂不支持 UTF-32，请下载原始文件查看");
    } else if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
        if bytes.len() % 2 != 0 {
            bail!("UTF-16 字节不完整，请下载原始文件查看");
        }
        let little = bytes[0] == 0xff;
        let units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|b| {
                if little {
                    u16::from_le_bytes([b[0], b[1]])
                } else {
                    u16::from_be_bytes([b[0], b[1]])
                }
            })
            .collect();
        String::from_utf16(&units).context("UTF-16 内容损坏，请下载原始文件查看")?
    } else {
        String::from_utf8(bytes.to_vec()).context("无法识别文本编码，请下载原始文件查看")?
    };
    if text.contains('\0') {
        bail!("资源包含二进制数据，请下载原始文件查看");
    }
    Ok(text.strip_prefix('\u{feff}').unwrap_or(&text).into())
}

pub fn text(files: &Files, task: &Value, relative: &str) -> Result<String> {
    decode_text(&bytes(files, task, relative)?)
}

pub fn raw(files: &Files, task: &Value, relative: &str) -> Result<Value> {
    use base64::Engine;
    let bytes = bytes(files, task, relative)?;
    Ok(
        json!({"path":relative,"bytes":bytes.len(),"base64":base64::engine::general_purpose::STANDARD.encode(bytes)}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn known_encodings_are_lossless_and_unknown_bytes_remain_available() -> Result<()> {
        let expected = "日志：完成\n";
        for little in [true, false] {
            let mut bytes = if little {
                vec![0xff, 0xfe]
            } else {
                vec![0xfe, 0xff]
            };
            for unit in expected.encode_utf16() {
                bytes.extend(if little {
                    unit.to_le_bytes()
                } else {
                    unit.to_be_bytes()
                });
            }
            assert_eq!(decode_text(&bytes)?, expected);
        }
        assert_eq!(decode_text("\u{feff}正常".as_bytes())?, "正常");
        assert!(decode_text(&[0xff, 0xfe, 0x41]).is_err());
        assert!(decode_text(&[0xff]).is_err());
        let temp = tempfile::tempdir()?;
        fs::write(temp.path().join("unknown.log"), [0xff, 0x81])?;
        let files = Files::new(temp.path().join("data"));
        let task = json!({"id":"task","workspace":temp.path()});
        assert_eq!(raw(&files, &task, "unknown.log")?["base64"], "/4E=");
        assert!(raw(&files, &task, "../outside").is_err());
        Ok(())
    }
    #[test]
    fn live_changes_deletions_and_references_are_discoverable() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("work");
        fs::create_dir(&workspace)?;
        fs::write(workspace.join("keep.md"), "unchanged")?;
        fs::write(workspace.join("gone.md"), "before")?;
        let files = Files::new(temp.path().join("data"));
        let baseline = files.capture(&workspace)?;
        fs::remove_file(workspace.join("gone.md"))?;
        fs::write(workspace.join("new.md"), "\u{feff}# 新资料")?;
        let mut task = json!({"id":"task","workspace":workspace,"baseline":baseline,"references":[{"path":"missing.png"}],"changes":[]});
        let items = resources(&files, &task)?;
        assert_eq!(
            items,
            json!([{"path":"missing.png","origin":"参考","exists":false},{"path":"gone.md","origin":"删除","exists":false},{"path":"new.md","origin":"修改","exists":true}])
        );
        assert_eq!(text(&files, &task, "new.md")?, "# 新资料");
        assert!(text(&files, &task, "../outside").is_err());
        fs::write(workspace.join("binary"), [0xff])?;
        assert!(text(&files, &task, "binary").is_err());
        fs::write(workspace.join("big.txt"), vec![b'a'; 512001])?;
        assert!(text(&files, &task, "big.txt").is_err());
        task["references"] = json!([{"path":"../outside"}]);
        assert!(resources(&files, &task).is_err());
        Ok(())
    }
}
