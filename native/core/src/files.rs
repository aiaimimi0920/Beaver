use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, Metadata},
    io::{self, Read},
    path::{Path, PathBuf},
};

pub type Snapshot = BTreeMap<String, String>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Change {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
}

fn linked(metadata: &Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

pub fn safe_path(root: &Path, relative: &str) -> Result<PathBuf> {
    if relative.is_empty()
        || relative.contains(['\\', ':', '\0'])
        || relative
            .split('/')
            .any(|s| s.is_empty() || s == "." || s == "..")
    {
        bail!("非法项目相对路径");
    }
    let mut current = fs::canonicalize(root)?;
    for component in relative.split('/') {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if linked(&metadata) => bail!("不允许通过符号链接或目录联接访问项目文件"),
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(current)
}

pub fn list_files(root: &Path) -> Result<Vec<String>> {
    fn visit(root: &Path, relative: &str, output: &mut Vec<String>) -> Result<()> {
        let directory = if relative.is_empty() {
            fs::canonicalize(root)?
        } else {
            safe_path(root, relative)?
        };
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| anyhow::anyhow!("文件名不是有效 UTF-8"))?;
            if [
                ".git",
                ".godot",
                ".beaver",
                ".beaver-context",
                "node_modules",
                "target",
                "release",
                "exports",
            ]
            .contains(&name.as_str())
                || name.starts_with(".beaver-write-")
            {
                continue;
            }
            let file = if relative.is_empty() {
                name
            } else {
                format!("{relative}/{name}")
            };
            let metadata = fs::symlink_metadata(safe_path(root, &file)?)?;
            if linked(&metadata) {
                bail!("项目含符号链接，无法创建任务副本：{file}");
            }
            if metadata.is_dir() {
                visit(root, &file, output)?;
            } else if metadata.is_file() {
                output.push(file);
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    visit(root, "", &mut files)?;
    files.sort();
    Ok(files)
}

pub fn file_hash(path: &Path) -> Result<Option<String>> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let length = file.read(&mut buffer)?;
        if length == 0 {
            break;
        }
        hasher.update(&buffer[..length]);
    }
    Ok(Some(format!("{:x}", hasher.finalize())))
}

pub struct Files {
    root: PathBuf,
}
impl Files {
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
    pub fn blob(&self, hash: &str) -> Result<PathBuf> {
        if hash.len() != 64
            || !hash
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            bail!("非法内容摘要");
        }
        Ok(self.root.join("blobs").join(hash))
    }
    pub fn capture(&self, project: &Path) -> Result<Snapshot> {
        let blobs = self.root.join("blobs");
        fs::create_dir_all(&blobs)?;
        let reuse_existing = fs::read_dir(&blobs)?.next().transpose()?.is_some();
        let mut snapshot = Snapshot::new();
        for relative in list_files(project)? {
            let source = safe_path(project, &relative)?;
            let mut expected_hash = None;
            // An empty content store needs no extra source hash pass for cache lookup.
            if reuse_existing {
                let hash = file_hash(&source)?.context("快照源文件读取失败")?;
                let destination = self.blob(&hash)?;
                match fs::symlink_metadata(&destination) {
                    Ok(metadata) => {
                        if linked(&metadata)
                            || !metadata.is_file()
                            || file_hash(&destination)?.as_ref() != Some(&hash)
                        {
                            bail!("已有快照内容损坏");
                        }
                        if file_hash(&safe_path(project, &relative)?)?.as_ref() != Some(&hash) {
                            bail!("文件在快照时发生变化：{relative}");
                        }
                        // Existing content is verified, not rewritten or fsynced per capture.
                        snapshot.insert(relative, hash);
                        continue;
                    }
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error.into()),
                }
                expected_hash = Some(hash);
            }
            let mut temporary = tempfile::Builder::new()
                .prefix(".tmp-")
                .tempfile_in(&blobs)?;
            io::copy(&mut File::open(&source)?, &mut temporary)?;
            temporary.as_file().sync_all()?;
            let hash = file_hash(temporary.path())?.context("快照文件读取失败")?;
            if expected_hash
                .as_ref()
                .is_some_and(|expected| expected != &hash)
                || file_hash(&safe_path(project, &relative)?)?.as_ref() != Some(&hash)
            {
                bail!("文件在快照时发生变化：{relative}");
            }
            let destination = self.blob(&hash)?;
            if let Err(error) = temporary.persist_noclobber(&destination) {
                if error.error.kind() != io::ErrorKind::AlreadyExists {
                    return Err(error.error.into());
                }
                let metadata = fs::symlink_metadata(&destination)?;
                if linked(&metadata)
                    || !metadata.is_file()
                    || file_hash(&destination)?.as_ref() != Some(&hash)
                {
                    bail!("已有快照内容损坏");
                }
            }
            snapshot.insert(relative, hash);
        }
        Ok(snapshot)
    }
    pub fn restore_copy(&self, snapshot: &Snapshot, target: &Path) -> Result<()> {
        fs::create_dir_all(target)?;
        for (relative, hash) in snapshot {
            self.write(target, relative, Some(hash))?;
        }
        Ok(())
    }
    pub fn changes(before: &Snapshot, after: &Snapshot) -> Vec<Change> {
        before
            .keys()
            .chain(after.keys())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .filter(|name| before.get(*name) != after.get(*name))
            .map(|name| Change {
                path: name.clone(),
                before: before.get(name).cloned(),
                after: after.get(name).cloned(),
            })
            .collect()
    }
    pub fn conflicts(&self, project: &Path, changes: &[Change]) -> Result<Vec<String>> {
        let mut conflicts = Vec::new();
        for change in changes {
            if file_hash(&safe_path(project, &change.path)?)? != change.before {
                conflicts.push(change.path.clone());
            }
        }
        Ok(conflicts)
    }
    pub fn apply(&self, project: &Path, changes: &[Change]) -> Result<()> {
        let conflicts = self.conflicts(project, changes)?;
        if !conflicts.is_empty() {
            bail!("文件冲突，未覆盖：{}", conflicts.join(", "));
        }
        let mut applied: Vec<&Change> = Vec::new();
        let result = (|| -> Result<()> {
            for change in changes {
                if file_hash(&safe_path(project, &change.path)?)? != change.before {
                    bail!("写入前文件发生变化：{}", change.path);
                }
                self.write(project, &change.path, change.after.as_deref())?;
                applied.push(change);
            }
            Ok(())
        })();
        if result.is_err() {
            for change in applied.into_iter().rev() {
                if file_hash(&safe_path(project, &change.path)?)? == change.after {
                    self.write(project, &change.path, change.before.as_deref())?;
                }
            }
        }
        result
    }
    pub fn write(&self, project: &Path, relative: &str, hash: Option<&str>) -> Result<()> {
        let destination = safe_path(project, relative)?;
        let Some(hash) = hash else {
            return match fs::remove_file(destination) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(error.into()),
            };
        };
        let blob = self.blob(hash)?;
        if file_hash(&blob)?.as_deref() != Some(hash) {
            bail!("快照内容缺失或损坏");
        }
        let parent = destination.parent().context("文件没有父目录")?;
        fs::create_dir_all(parent)?;
        let mut temporary = tempfile::Builder::new()
            .prefix(".beaver-write-")
            .tempfile_in(parent)?;
        io::copy(&mut File::open(blob)?, &mut temporary)?;
        temporary.as_file().sync_all()?;
        temporary.persist(&destination).map_err(|e| e.error)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cached_capture_rejects_corruption_and_detects_same_size_same_mtime_edits() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let project = temp.path().join("project");
        fs::create_dir(&project)?;
        let source = project.join("a.txt");
        fs::write(&source, "before")?;
        let modified = fs::metadata(&source)?.modified()?;
        let files = Files::new(temp.path().join("data"));
        let before = files.capture(&project)?;
        let blob = files.blob(&before["a.txt"])?;
        assert_eq!(files.capture(&project)?, before);
        fs::write(&blob, "broken")?;
        assert!(files
            .capture(&project)
            .unwrap_err()
            .to_string()
            .contains("损坏"));
        fs::write(&blob, "before")?;
        fs::write(&source, "after!")?;
        File::options()
            .write(true)
            .open(&source)?
            .set_times(fs::FileTimes::new().set_modified(modified))?;
        let after = files.capture(&project)?;
        assert_ne!(before, after);
        assert_eq!(fs::read_to_string(blob)?, "before");
        let copy = temp.path().join("copy");
        files.restore_copy(&after, &copy)?;
        fs::write(copy.join("a.txt"), "edited copy")?;
        assert_eq!(fs::read_to_string(files.blob(&after["a.txt"])?)?, "after!");
        Ok(())
    }
    #[test]
    fn snapshots_conflicts_and_reversal() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let project = temp.path().join("project");
        fs::create_dir(&project)?;
        fs::write(project.join("世界观.md"), "before")?;
        let files = Files::new(temp.path().join("data"));
        let before = files.capture(&project)?;
        fs::write(project.join("世界观.md"), "after")?;
        fs::write(project.join("new.txt"), "new")?;
        let after = files.capture(&project)?;
        let changes = Files::changes(&before, &after);
        let copy = temp.path().join("copy");
        files.restore_copy(&before, &copy)?;
        fs::write(copy.join("世界观.md"), "human edit")?;
        assert!(files.apply(&copy, &changes).is_err());
        assert!(!copy.join("new.txt").exists());
        files.restore_copy(&before, &copy)?;
        files.apply(&copy, &changes)?;
        assert_eq!(files.capture(&copy)?, after);
        files.apply(&copy, &Files::changes(&after, &before))?;
        assert_eq!(files.capture(&copy)?, before);
        Ok(())
    }
    #[test]
    fn failed_batch_restores_earlier_writes() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let project = temp.path().join("project");
        fs::create_dir(&project)?;
        fs::write(project.join("a"), "old")?;
        let files = Files::new(temp.path().join("data"));
        let before = files.capture(&project)?;
        let changes = vec![
            Change {
                path: "a".into(),
                before: before.get("a").cloned(),
                after: None,
            },
            Change {
                path: "b".into(),
                before: None,
                after: Some("0".repeat(64)),
            },
        ];
        assert!(files.apply(&project, &changes).is_err());
        assert_eq!(files.capture(&project)?, before);
        Ok(())
    }
    #[test]
    fn rejects_escapes_and_ignores_generated_directories() -> Result<()> {
        let temp = tempfile::tempdir()?;
        for invalid in ["", "/a", "a/../b", "a\\b", "C:/a", "a//b", "./a"] {
            assert!(safe_path(temp.path(), invalid).is_err());
        }
        fs::create_dir(temp.path().join(".godot"))?;
        fs::write(temp.path().join(".godot/cache"), "ignore")?;
        fs::write(temp.path().join("keep.txt"), "keep")?;
        assert_eq!(list_files(temp.path())?, vec!["keep.txt"]);
        Ok(())
    }
}
