//! Read-only snapshots for ordinary files and folders selected for import.
use crate::{files, object_import_snapshot};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

#[cfg(unix)]
use std::os::unix::fs::symlink;

const MAX_ROOTS: usize = 100;
const MAX_FILES: usize = 100_000;
const MAX_PATH_BYTES: usize = 2_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileSource {
    pub kind: String,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileSnapshot {
    pub source_path: String,
    pub relative_path: String,
    pub path: String,
    pub kind: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileSourceSnapshot {
    pub source: FileSource,
    pub files: Vec<FileSnapshot>,
    pub digest: String,
}

/// Scan selected files and directories without creating or modifying source data.
pub fn inspect(paths: &[PathBuf]) -> Result<FileSourceSnapshot> {
    ensure!(
        !paths.is_empty(),
        "IMPORT_FILES_EMPTY: select at least one path"
    );
    ensure!(
        paths.len() <= MAX_ROOTS,
        "IMPORT_FILES_TOO_MANY: at most {MAX_ROOTS} paths"
    );

    let mut roots = BTreeSet::new();
    for path in paths {
        let root = canonical_root(path)?;
        roots.insert(root);
    }

    let roots: Vec<_> = roots.into_iter().collect();
    let source = FileSource {
        kind: "files".into(),
        paths: roots.iter().map(|path| path_text(path)).collect(),
    };
    let mut files_by_path = BTreeMap::new();
    for root in &roots {
        if root.is_file() {
            let snapshot = read_file(root, root, &display_name(root)?)?;
            files_by_path.insert(snapshot.path.clone(), snapshot);
            continue;
        }
        ensure!(root.is_dir(), "IMPORT_SOURCE_NOT_FILE_OR_DIRECTORY");
        let relative_paths = files::list_files(root)
            .with_context(|| format!("IMPORT_SOURCE_SCAN_FAILED: {}", path_text(root)))?;
        for relative in &relative_paths {
            let file = files::safe_path(root, relative)?;
            let snapshot = read_file(root, &file, relative)?;
            files_by_path.insert(snapshot.path.clone(), snapshot);
            ensure!(
                files_by_path.len() <= MAX_FILES,
                "IMPORT_FILES_TOO_MANY: at most {MAX_FILES} files"
            );
        }
        let current = files::list_files(root)?;
        ensure!(
            current == *relative_paths,
            "IMPORT_SOURCE_CHANGED: directory contents changed during inspection"
        );
    }

    let files: Vec<_> = files_by_path.into_values().collect();
    let digest = object_import_snapshot::digest(&(&source, &files))?;
    Ok(FileSourceSnapshot {
        source,
        files,
        digest,
    })
}

fn canonical_root(path: &Path) -> Result<PathBuf> {
    ensure!(path.is_absolute(), "IMPORT_SOURCE_PATH_NOT_ABSOLUTE");
    ensure!(
        path.as_os_str().len() <= MAX_PATH_BYTES,
        "IMPORT_SOURCE_PATH_TOO_LONG"
    );
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("IMPORT_SOURCE_MISSING: {}", path_text(path)))?;
    ensure!(!files::linked(&metadata), "IMPORT_SOURCE_LINK_NOT_ALLOWED");
    reject_links_in_ancestors(path)?;
    let canonical = fs::canonicalize(path)?;
    let canonical_metadata = fs::symlink_metadata(&canonical)?;
    ensure!(
        !files::linked(&canonical_metadata),
        "IMPORT_SOURCE_LINK_NOT_ALLOWED"
    );
    ensure!(
        !has_reserved_component(&canonical),
        "IMPORT_SOURCE_MANAGED_PATH"
    );
    Ok(canonical)
}

fn reject_links_in_ancestors(path: &Path) -> Result<()> {
    let mut current = path.to_path_buf();
    loop {
        if let Ok(metadata) = fs::symlink_metadata(&current) {
            ensure!(!files::linked(&metadata), "IMPORT_SOURCE_LINK_NOT_ALLOWED");
        }
        if !current.pop() {
            break;
        }
    }
    Ok(())
}

fn read_file(source_root: &Path, path: &Path, relative: &str) -> Result<FileSnapshot> {
    let metadata = fs::symlink_metadata(path)?;
    ensure!(
        metadata.is_file(),
        "IMPORT_SOURCE_NOT_A_FILE: {}",
        path_text(path)
    );
    ensure!(!files::linked(&metadata), "IMPORT_SOURCE_LINK_NOT_ALLOWED");
    let before = file_state(&metadata)?;
    let first_hash = files::file_hash(path)?.context("IMPORT_SOURCE_DISAPPEARED")?;
    let after_metadata = fs::symlink_metadata(path)
        .with_context(|| format!("IMPORT_SOURCE_DISAPPEARED: {}", path_text(path)))?;
    ensure!(
        !files::linked(&after_metadata),
        "IMPORT_SOURCE_LINK_NOT_ALLOWED"
    );
    let after = file_state(&after_metadata)?;
    let second_hash = files::file_hash(path)?.context("IMPORT_SOURCE_DISAPPEARED")?;
    ensure!(
        before == after && first_hash == second_hash,
        "IMPORT_SOURCE_CHANGED: {}",
        path_text(path)
    );
    let canonical = fs::canonicalize(path)?;
    Ok(FileSnapshot {
        source_path: path_text(source_root),
        relative_path: relative.replace('\\', "/"),
        path: path_text(&canonical),
        kind: classify(path),
        bytes: after.0,
        sha256: second_hash,
    })
}

fn file_state(metadata: &fs::Metadata) -> Result<(u64, Option<std::time::SystemTime>)> {
    Ok((metadata.len(), metadata.modified().ok()))
}

fn classify(path: &Path) -> String {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let kind = if ["png", "jpg", "jpeg", "gif", "webp", "bmp", "tga", "svg"]
        .contains(&extension.as_str())
    {
        "image"
    } else if ["glb", "gltf", "fbx", "obj", "blend", "dae", "stl"].contains(&extension.as_str()) {
        "model"
    } else if ["wav", "mp3", "ogg", "flac", "m4a"].contains(&extension.as_str()) {
        "audio"
    } else if ["mp4", "mov", "webm", "avi", "mkv"].contains(&extension.as_str()) {
        "video"
    } else if ["md", "txt", "rtf", "pdf", "doc", "docx"].contains(&extension.as_str()) {
        "document"
    } else if ["rs", "ts", "tsx", "js", "jsx", "gd", "py", "lua", "shader"]
        .contains(&extension.as_str())
    {
        "script"
    } else if ["zip", "7z", "tar", "gz", "rar"].contains(&extension.as_str()) {
        "archive"
    } else if ["json", "toml", "yaml", "yml", "csv"].contains(&extension.as_str()) {
        "data"
    } else {
        "other"
    };
    kind.into()
}

fn has_reserved_component(path: &Path) -> bool {
    path.components().any(|component| {
        let Some(name) = component.as_os_str().to_str() else {
            return false;
        };
        [
            ".git",
            ".godot",
            ".beaver",
            ".beaver-context",
            "node_modules",
            "target",
            "release",
            "exports",
        ]
        .contains(&name)
            || name.starts_with(".beaver-write-")
    })
}

fn display_name(path: &Path) -> Result<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .context("IMPORT_SOURCE_INVALID_NAME")
}

fn path_text(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn scans_files_and_directories_with_stable_digest() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let directory = temp.path().join("assets");
        fs::create_dir_all(directory.join("nested"))?;
        fs::write(directory.join("hero.glb"), b"hero")?;
        fs::write(directory.join("nested/readme.md"), b"readme")?;
        let one = inspect(&[directory.clone()])?;
        let two = inspect(&[directory])?;
        assert_eq!(one, two);
        assert_eq!(one.files.len(), 2);
        assert_eq!(one.files[0].kind, "model");
        assert_eq!(one.files[1].kind, "document");
        assert_eq!(one.files[0].bytes, 4);
        Ok(())
    }

    #[test]
    fn deduplicates_canonical_paths_and_rejects_managed_paths() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let file = temp.path().join("hero.png");
        fs::write(&file, b"image")?;
        let snapshot = inspect(&[file.clone(), file.clone()])?;
        assert_eq!(snapshot.source.paths.len(), 1);
        assert_eq!(snapshot.files.len(), 1);
        let managed = temp.path().join(".beaver");
        fs::create_dir(&managed)?;
        assert!(inspect(&[managed]).is_err());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symbolic_links() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let file = temp.path().join("hero.txt");
        let link = temp.path().join("link.txt");
        fs::write(&file, b"hero")?;
        super::symlink(&file, &link)?;
        assert!(inspect(&[link]).is_err());
        Ok(())
    }

    #[test]
    fn detects_file_changes_during_snapshot() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let file = temp.path().join("hero.txt");
        fs::write(&file, b"hero")?;
        let snapshot = inspect(&[file])?;
        fs::write(temp.path().join("hero.txt"), b"changed")?;
        assert_ne!(
            snapshot.digest,
            inspect(&[temp.path().join("hero.txt")])?.digest
        );
        Ok(())
    }
}
