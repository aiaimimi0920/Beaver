//! Byte-preserving offline application-data backup. External projects are not included.
use crate::files::{file_hash, safe_path};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

const MANIFEST: &str = "BEAVER-BACKUP.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub path: String,
    pub bytes: u64,
    pub sha256: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub format: String,
    pub scope: String,
    pub source: PathBuf,
    pub entries: Vec<Entry>,
}

pub fn inventory(root: &Path) -> Result<Vec<Entry>> {
    inventory_without(root, &[])
}

pub(crate) fn inventory_without(root: &Path, excluded: &[&str]) -> Result<Vec<Entry>> {
    fn visit(
        root: &Path,
        directory: &Path,
        excluded: &[&str],
        entries: &mut Vec<Entry>,
    ) -> Result<()> {
        for item in fs::read_dir(directory)? {
            let item = item?;
            let relative = item.path().strip_prefix(root)?.to_owned();
            let relative = relative
                .to_str()
                .context("backup filename is not UTF-8")?
                .replace('\\', "/");
            if excluded.contains(&relative.as_str()) {
                continue;
            }
            let file = safe_path(root, &relative)?;
            let metadata = fs::symlink_metadata(&file)?;
            ensure!(
                metadata.is_file() || metadata.is_dir(),
                "unsupported backup entry"
            );
            entries.push(Entry {
                path: relative,
                bytes: if metadata.is_file() {
                    metadata.len()
                } else {
                    0
                },
                sha256: if metadata.is_file() {
                    file_hash(&file)?
                } else {
                    None
                },
            });
            if metadata.is_dir() {
                visit(root, &file, excluded, entries)?;
            }
        }
        Ok(())
    }
    let root = fs::canonicalize(root)?;
    let mut entries = Vec::new();
    visit(&root, &root, excluded, &mut entries)?;
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}

pub(crate) fn new_destination(source: &Path, destination: &Path) -> Result<PathBuf> {
    let parent = fs::canonicalize(destination.parent().context("destination needs parent")?)?;
    let name = destination.file_name().context("destination needs name")?;
    let target = parent.join(name);
    ensure!(
        !target.starts_with(source),
        "destination must be outside source"
    );
    // create_dir, not create_dir_all: existing/partial backups are never overwritten.
    fs::create_dir(&target).context("backup destination must be new")?;
    Ok(target)
}

pub(crate) fn copy_entries(source: &Path, target: &Path, entries: &[Entry]) -> Result<()> {
    for entry in entries {
        let input = safe_path(source, &entry.path)?;
        let output = safe_path(target, &entry.path)?;
        if entry.sha256.is_none() {
            fs::create_dir(&output)?;
        } else {
            let mut input = File::open(input)?;
            let mut output = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(output)?;
            let bytes = std::io::copy(&mut input, &mut output)?;
            output.sync_all()?;
            ensure!(bytes == entry.bytes, "source changed during backup");
        }
    }
    ensure!(
        inventory(target)? == entries,
        "copied bytes differ from backup inventory"
    );
    ensure!(
        inventory(source)? == entries,
        "source changed during backup"
    );
    Ok(())
}

/// Deny sharing with existing/future SQLite writers without opening SQLite itself.
/// The user must also stop external tools writing workspaces before creating a backup.
pub(crate) fn hold_database(source: &Path) -> Result<Vec<File>> {
    hold_named_database(source, "beaver.sqlite")
}

#[cfg(windows)]
pub(crate) fn hold_named_database(source: &Path, database: &str) -> Result<Vec<File>> {
    use std::os::windows::fs::OpenOptionsExt;
    let mut handles = Vec::new();
    for suffix in ["", "-wal", "-shm"] {
        let file = safe_path(source, &format!("{database}{suffix}"))?;
        if !suffix.is_empty() && !file.try_exists()? {
            continue;
        }
        handles.push(
            OpenOptions::new()
                .read(true)
                .share_mode(1)
                .open(file)
                .context("close Beaver and all database readers/writers before backup")?,
        );
    }
    Ok(handles)
}

#[cfg(not(windows))]
pub(crate) fn hold_named_database(_source: &Path, _database: &str) -> Result<Vec<File>> {
    anyhow::bail!("offline database writer exclusion is currently implemented only on Windows")
}

pub fn create(source: &Path, destination: &Path) -> Result<Manifest> {
    let source = fs::canonicalize(source)?;
    let _handles = hold_database(&source)?;
    let entries = inventory(&source)?;
    let target = new_destination(&source, destination)?;
    let data = target.join("data");
    fs::create_dir(&data)?;
    copy_entries(&source, &data, &entries)?;
    let manifest = Manifest {
        format: "beaver-data-backup-v1".into(),
        scope: "application-data-only; external projects require separate backup".into(),
        source,
        entries,
    };
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target.join(MANIFEST))?;
    file.write_all(&serde_json::to_vec_pretty(&manifest)?)?;
    file.sync_all()?;
    verify(&target)?;
    Ok(manifest)
}

pub fn verify(root: &Path) -> Result<Manifest> {
    let manifest_path = safe_path(root, MANIFEST)?;
    ensure!(
        fs::metadata(&manifest_path)?.len() <= 64 * 1024 * 1024,
        "backup manifest too large"
    );
    let manifest: Manifest = serde_json::from_reader(File::open(manifest_path)?)?;
    ensure!(
        manifest.format == "beaver-data-backup-v1",
        "unsupported backup format"
    );
    ensure!(
        manifest.scope == "application-data-only; external projects require separate backup",
        "unsupported backup scope"
    );
    let mut names = fs::read_dir(root)?
        .map(|entry| Ok(entry?.file_name()))
        .collect::<Result<Vec<_>>>()?;
    names.sort();
    ensure!(
        names
            == [
                std::ffi::OsString::from(MANIFEST),
                std::ffi::OsString::from("data")
            ],
        "unexpected backup root entry"
    );
    let data = safe_path(root, "data")?;
    let entries = inventory(&data)?;
    ensure!(
        entries == manifest.entries,
        "backup is incomplete or modified"
    );
    ensure!(
        entries
            .iter()
            .any(|entry| entry.path == "beaver.sqlite" && entry.sha256.is_some()),
        "backup has no database"
    );
    Ok(manifest)
}

/// Restore bytes to a NEW directory. Does not rewrite paths, decrypt keys or run recovery.
pub fn restore(backup: &Path, destination: &Path) -> Result<Manifest> {
    let backup = fs::canonicalize(backup)?;
    let manifest = verify(&backup)?;
    let data = safe_path(&backup, "data")?;
    let _handles = hold_database(&data)?;
    let target = new_destination(&backup, destination)?;
    copy_entries(&data, &target, &manifest.entries)?;
    verify(&backup)?;
    Ok(manifest)
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use crate::store::Store;
    use serde_json::json;

    #[test]
    fn offline_backup_restores_exact_bytes_and_rejects_drift() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let source = temp.path().join("source");
        let store = Store::open(&source)?;
        store.put("secret", "code", &"opaque-cipher-not-decrypted")?;
        store.put("task", "task", &json!({"status":"running", "unknown":true}))?;
        let backup = temp.path().join("backup");
        ensure!(create(&source, &backup).is_err());
        ensure!(!backup.exists());
        drop(store);
        fs::create_dir_all(source.join("workspaces/task/.beaver-context/empty"))?;
        fs::create_dir_all(source.join("codex/task/sessions"))?;
        fs::write(
            source.join("codex/task/sessions/session.jsonl"),
            "unchanged session",
        )?;
        fs::write(source.join("Local State"), "opaque profile")?;
        fs::write(source.join("workspaces/task/中文.png"), [0, 255, 1, 128])?;
        let original = inventory(&source)?;
        let manifest = create(&source, &backup)?;
        ensure!(manifest.entries == original);
        ensure!(inventory(&source)? == original);
        ensure!(create(&source, &backup).is_err());
        ensure!(create(&source, &source.join("nested")).is_err());
        let restored = temp.path().join("restored");
        restore(&backup, &restored)?;
        ensure!(inventory(&restored)? == original);
        ensure!(restore(&backup, &restored).is_err());
        let read = Store::open(&restored)?;
        ensure!(
            read.get::<String>("secret", "code")? == Some("opaque-cipher-not-decrypted".into())
        );
        ensure!(read.get::<serde_json::Value>("task", "task")?.unwrap()["status"] == "running");
        drop(read);
        fs::write(backup.join("data/Local State"), "tampered")?;
        ensure!(verify(&backup).is_err());
        let rejected = temp.path().join("rejected");
        ensure!(restore(&backup, &rejected).is_err());
        ensure!(!rejected.exists());
        ensure!(inventory(&source)? == original);
        Ok(())
    }

    #[test]
    fn backup_rejects_directory_links_without_touching_target() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let source = temp.path().join("source");
        drop(Store::open(&source)?);
        let outside = temp.path().join("outside");
        fs::create_dir(&outside)?;
        fs::write(outside.join("keep.txt"), "keep")?;
        use std::os::windows::process::CommandExt;
        let status = std::process::Command::new("cmd.exe")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(source.join("linked"))
            .arg(&outside)
            .creation_flags(0x08000000)
            .output()?;
        ensure!(
            status.status.success(),
            "could not prepare junction fixture"
        );
        ensure!(create(&source, &temp.path().join("backup")).is_err());
        ensure!(fs::read(outside.join("keep.txt"))? == b"keep");
        fs::remove_dir(source.join("linked"))?;
        Ok(())
    }
}
