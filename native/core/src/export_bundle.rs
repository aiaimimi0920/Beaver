use crate::files::{file_hash, safe_path};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    io::Read,
    path::Path,
};

#[derive(Deserialize, Serialize, PartialEq, Debug)]
pub struct Record {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}

pub fn capture(root: &Path) -> Result<Vec<Record>> {
    fn visit(root: &Path, prefix: &str, out: &mut Vec<Record>) -> Result<()> {
        let directory = if prefix.is_empty() {
            root.to_path_buf()
        } else {
            safe_path(root, prefix)?
        };
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| anyhow::anyhow!("导出文件名不是 UTF-8"))?;
            let relative = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            let path = safe_path(root, &relative)?;
            let metadata = fs::symlink_metadata(&path)?;
            if metadata.is_dir() {
                visit(root, &relative, out)?;
            } else if metadata.is_file() {
                if ["export-manifest.json", "export.log"].contains(&relative.as_str()) {
                    continue;
                }
                out.push(Record {
                    path: relative,
                    bytes: metadata.len(),
                    sha256: file_hash(&path)?.context("导出文件在读取时消失")?,
                });
                if out.len() > 100000 {
                    bail!("导出文件数量超出限制");
                }
            } else {
                bail!("导出包包含不支持的文件类型");
            }
        }
        Ok(())
    }
    let root = fs::canonicalize(root)?;
    let mut files = Vec::new();
    visit(&root, "", &mut files)?;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

#[derive(Deserialize)]
struct Manifest {
    version: u8,
    entry: String,
    platform: String,
    files: Vec<Record>,
    #[serde(default)]
    validation: Value,
}

pub fn verify(directory: &Path) -> Result<Value> {
    let root = fs::canonicalize(directory)?;
    let mut bytes = Vec::new();
    fs::File::open(safe_path(&root, "export-manifest.json")?)?
        .take(32 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 32 * 1024 * 1024 {
        bail!("导出清单过大");
    }
    let manifest: Manifest =
        serde_json::from_slice(&bytes).context("不是完整的 v2 导出清单，请重新导出")?;
    if manifest.version != 2
        || manifest.files.is_empty()
        || manifest.files.len() > 100000
        || !["Windows Desktop", "Linux", "macOS"].contains(&manifest.platform.as_str())
    {
        bail!("导出清单版本或目标平台无效");
    }
    let mut declared = HashSet::new();
    for record in &manifest.files {
        safe_path(&root, &record.path)?;
        let key = if cfg!(windows) {
            record.path.to_lowercase()
        } else {
            record.path.clone()
        };
        if !declared.insert(key) {
            bail!("导出清单包含重复路径");
        }
        if record.sha256.len() != 64
            || !record
                .sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            bail!("导出摘要格式无效");
        }
    }
    if !manifest
        .files
        .iter()
        .any(|file| file.path == manifest.entry)
    {
        bail!("入口程序未包含在完整性清单中");
    }
    let actual = capture(&root)?;
    let expected: BTreeMap<_, _> = manifest
        .files
        .iter()
        .map(|file| (&file.path, file))
        .collect();
    if actual.len() != expected.len() {
        bail!("导出文件数量发生变化，请恢复完整目录或重新导出");
    }
    for file in &actual {
        if expected.get(&file.path).copied() != Some(file) {
            bail!("导出文件缺失或内容改变：{}", file.path);
        }
    }
    let mut entry = fs::File::open(safe_path(&root, &manifest.entry)?)?;
    let mut magic = [0u8; 4];
    entry.read_exact(&mut magic)?;
    let valid = match manifest.platform.as_str() {
        "Windows Desktop" => magic[..2] == *b"MZ",
        "Linux" => magic == *b"\x7fELF",
        _ => magic == *b"PK\x03\x04",
    };
    if !valid || entry.metadata()?.len() < 1024 {
        bail!("入口文件不是有效的目标平台程序或归档");
    }
    Ok(
        json!({"path":root,"entry":manifest.entry,"files":actual.len(),"bytes":actual.iter().map(|f|f.bytes).sum::<u64>(),"reportedValidation":manifest.validation}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_bundle_rejects_missing_modified_extra_and_undeclared_entry() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let mut exe = vec![0u8; 2048];
        exe[..2].copy_from_slice(b"MZ");
        fs::write(root.join("Game.exe"), exe)?;
        fs::write(root.join("Game.pck"), "assets")?;
        let manifest = json!({"version":2,"entry":"Game.exe","platform":"Windows Desktop","files":capture(root)?});
        let write = |value: &Value| fs::write(root.join("export-manifest.json"), value.to_string());
        write(&manifest)?;
        assert_eq!(verify(root)?["files"], 2);
        fs::write(root.join("export.log"), "ignored build log")?;
        assert!(verify(root).is_ok());
        fs::write(root.join("Game.pck"), "changed")?;
        assert!(verify(root).is_err());
        fs::write(root.join("Game.pck"), "assets")?;
        fs::write(root.join("extra.dll"), "extra")?;
        assert!(verify(root).is_err());
        fs::remove_file(root.join("extra.dll"))?;
        let mut wrong = manifest.clone();
        wrong["entry"] = json!("../escape.exe");
        write(&wrong)?;
        assert!(verify(root).is_err());
        write(&manifest)?;
        fs::remove_file(root.join("Game.pck"))?;
        assert!(verify(root).is_err());
        Ok(())
    }
}
