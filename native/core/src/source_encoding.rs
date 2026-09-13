use crate::files::{file_hash, list_files, safe_path, Snapshot};
use anyhow::{Context, Result};
use std::{fs, path::Path};

/// Normalize only changed text sources with a recognized BOM. Invalid input is
/// preserved and blocks merge; binaries and unchanged legacy files are untouched.
pub fn normalize(root: &Path, baseline: &Snapshot) -> Result<Vec<String>> {
    let mut normalized = Vec::new();
    for relative in list_files(root)? {
        let ext = Path::new(&relative)
            .extension()
            .and_then(|v| v.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !crate::code_structure::is_source(&relative)
            && ![
                "gd", "tscn", "tres", "godot", "cfg", "md", "json", "toml", "yaml", "yml",
            ]
            .contains(&ext.as_str())
        {
            continue;
        }
        let path = safe_path(root, &relative)?;
        if file_hash(&path)?.as_ref() == baseline.get(&relative) {
            continue;
        }
        let bytes = fs::read(&path)?;
        let text = crate::task_resources::decode_text(&bytes)
            .with_context(|| format!("源文件编码无效：{relative}"))?;
        if text.as_bytes() != bytes {
            fs::write(&path, text.as_bytes())?;
            normalized.push(relative);
        }
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sources_normalize_without_touching_binary_or_invalid_bytes() -> Result<()> {
        let dir = tempfile::tempdir()?;
        fs::write(dir.path().join("main.tscn"), "\u{feff}[gd_scene]\n")?;
        fs::write(dir.path().join("audio.wav"), [0xff, 0xfe, 0])?;
        assert_eq!(normalize(dir.path(), &Snapshot::new())?, vec!["main.tscn"]);
        assert_eq!(fs::read(dir.path().join("main.tscn"))?, b"[gd_scene]\n");
        assert_eq!(fs::read(dir.path().join("audio.wav"))?, vec![0xff, 0xfe, 0]);
        fs::write(
            dir.path().join("shader.gdshader"),
            "\u{feff}shader_type spatial;\n",
        )?;
        assert_eq!(
            normalize(dir.path(), &Snapshot::new())?,
            vec!["shader.gdshader"]
        );
        assert_eq!(
            fs::read(dir.path().join("shader.gdshader"))?,
            b"shader_type spatial;\n"
        );
        fs::write(dir.path().join("bad.gd"), [0xff])?;
        assert!(normalize(dir.path(), &Snapshot::new()).is_err());
        assert_eq!(fs::read(dir.path().join("bad.gd"))?, vec![0xff]);
        Ok(())
    }
}
