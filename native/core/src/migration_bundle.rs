//! Offline application + registered project archive. Restore does not activate migration.
use crate::{data_backup, files::safe_path};
use anyhow::{ensure, Context, Result};
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

const MANIFEST: &str = "BEAVER-MIGRATION.json";
const FORMAT: &str = "beaver-migration-bundle-v1";
pub const MODE_ARGUMENT: &str = "--migration-bundle";
const PENDING: &str = ".beaver-migration-pending";

pub fn ensure_activated(data: &Path) -> Result<()> {
    // Resolve aliases; reject both the recovery root and any data/project descendant.
    let absolute = std::path::absolute(data)?;
    let existing = absolute
        .ancestors()
        .find(|directory| directory.exists())
        .context("data directory has no existing ancestor")?;
    let data = fs::canonicalize(existing)?.join(absolute.strip_prefix(existing)?);
    for directory in data.ancestors() {
        let marker = directory.join(PENDING);
        ensure!(!marker.try_exists()?, "recovery copy is not activated: project/workspace paths and credentials still require migration");
    }
    Ok(())
}

pub fn command(arguments: Vec<std::ffi::OsString>) -> Result<Value> {
    let usage =
        "usage: --migration-bundle create|verify|restore|prepare-import SOURCE [NEW_DESTINATION]; activate-import BACKUP PREPARED_DIRECTORY [TOOL_PATHS_JSON]";
    let mode = arguments
        .first()
        .and_then(|value| value.to_str())
        .context(usage)?;
    let source = Path::new(arguments.get(1).context(usage)?);
    match (mode, arguments.len()) {
        ("create", 3) => {
            let manifest = create(source, Path::new(&arguments[2]))?;
            Ok(serde_json::json!({"format":manifest.format,"projects":manifest.projects.len()}))
        }
        ("verify", 2) => {
            let manifest = verify(source)?;
            Ok(serde_json::json!({"format":manifest.format,"projects":manifest.projects.len()}))
        }
        ("restore", 3) => Ok(serde_json::to_value(restore(
            source,
            Path::new(&arguments[2]),
        )?)?),
        ("prepare-import", 3) => Ok(serde_json::to_value(crate::migration_import::prepare(
            source,
            Path::new(&arguments[2]),
        )?)?),
        ("activate-import", 3 | 4) => crate::migration_activation::activate(
            source,
            Path::new(&arguments[2]),
            arguments.get(3).map(Path::new),
        ),
        _ => anyhow::bail!(usage),
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub id: String,
    pub stored_path: String,
    pub source: PathBuf,
    pub entries: Vec<data_backup::Entry>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub format: String,
    pub projects: Vec<Project>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RestoredProject {
    pub id: String,
    pub original_path: String,
    pub restored_path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RestoreReceipt {
    pub format: String,
    pub original_data: PathBuf,
    pub restored_data: PathBuf,
    pub projects: Vec<RestoredProject>,
    pub paths_rewritten: bool,
    pub credentials_converted: bool,
    pub ready_to_activate: bool,
}

/// Never open the source SQLite database: even read-only WAL access can create SHM.
fn projects_from_copy(data: &Path) -> Result<BTreeMap<String, String>> {
    let temp = tempfile::tempdir()?;
    for name in ["beaver.sqlite", "beaver.sqlite-wal", "beaver.sqlite-shm"] {
        let file = safe_path(data, name)?;
        if file.try_exists()? {
            fs::copy(file, temp.path().join(name))?;
        }
    }
    let connection = Connection::open_with_flags(
        temp.path().join("beaver.sqlite"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    let integrity: String = connection.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    ensure!(integrity == "ok", "backup database integrity check failed");
    let mut statement =
        connection.prepare("SELECT id,value FROM entities WHERE kind='project' ORDER BY id")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut projects = BTreeMap::new();
    for row in rows {
        let (id, raw) = row?;
        ensure!(
            uuid::Uuid::parse_str(&id).is_ok(),
            "invalid project identifier"
        );
        let value: Value = serde_json::from_str(&raw).context("invalid stored project")?;
        ensure!(
            value["id"].as_str() == Some(&id),
            "project identifier mismatch"
        );
        let path = value["path"].as_str().context("project path missing")?;
        ensure!(
            Path::new(path).is_absolute(),
            "project path must be absolute"
        );
        projects.insert(id, path.into());
    }
    Ok(projects)
}

fn exact_names(root: &Path, mut expected: Vec<String>) -> Result<()> {
    let mut actual = fs::read_dir(root)?
        .map(|entry| {
            entry?
                .file_name()
                .into_string()
                .map_err(|_| anyhow::anyhow!("invalid archive filename"))
        })
        .collect::<Result<Vec<_>>>()?;
    actual.sort();
    expected.sort();
    ensure!(
        actual == expected,
        "archive has missing or unexpected entries"
    );
    Ok(())
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(&serde_json::to_vec_pretty(value)?)?;
    file.sync_all()?;
    Ok(())
}

pub fn create(source: &Path, destination: &Path) -> Result<Manifest> {
    let source = fs::canonicalize(source)?;
    let _database = data_backup::hold_database(&source)?;
    let records = projects_from_copy(&source)?;
    let target = fs::canonicalize(destination.parent().context("destination needs parent")?)?
        .join(destination.file_name().context("destination needs name")?);
    let mut projects: Vec<Project> = Vec::new();
    for (id, stored_path) in records {
        let project = fs::canonicalize(&stored_path)
            .context("registered project is unavailable; full backup cannot omit it")?;
        ensure!(project.is_dir(), "project path is not a directory");
        ensure!(
            !source.starts_with(&project) && !project.starts_with(&source),
            "application and project roots overlap"
        );
        ensure!(
            !target.starts_with(&project),
            "backup destination is inside a project"
        );
        for previous in &projects {
            ensure!(
                !project.starts_with(&previous.source) && !previous.source.starts_with(&project),
                "registered project roots overlap"
            );
        }
        let entries = data_backup::inventory(&project)?;
        ensure!(
            entries
                .iter()
                .any(|entry| entry.path == "project.godot" && entry.sha256.is_some()),
            "registered project has no project.godot"
        );
        projects.push(Project {
            id,
            stored_path,
            source: project,
            entries,
        });
    }
    let target = data_backup::new_destination(&source, &target)?;
    data_backup::create(&source, &target.join("application"))?;
    let projects_root = target.join("projects");
    fs::create_dir(&projects_root)?;
    for project in &projects {
        let copy = safe_path(&projects_root, &project.id)?;
        fs::create_dir(&copy)?;
        data_backup::copy_entries(&project.source, &copy, &project.entries)?;
    }
    // Recheck earlier projects after the whole batch, not only their individual copies.
    for project in &projects {
        ensure!(
            data_backup::inventory(&project.source)? == project.entries,
            "project changed during backup"
        );
    }
    let application = data_backup::verify(&target.join("application"))?;
    ensure!(
        data_backup::inventory(&source)? == application.entries,
        "application data changed during backup"
    );
    let manifest = Manifest {
        format: FORMAT.into(),
        projects,
    };
    write_json(&target.join(MANIFEST), &manifest)?;
    verify(&target)?;
    Ok(manifest)
}

pub fn verify(root: &Path) -> Result<Manifest> {
    exact_names(
        root,
        vec![MANIFEST.into(), "application".into(), "projects".into()],
    )?;
    let path = safe_path(root, MANIFEST)?;
    ensure!(
        fs::metadata(&path)?.len() <= 64 * 1024 * 1024,
        "migration manifest too large"
    );
    let manifest: Manifest = serde_json::from_reader(File::open(path)?)?;
    ensure!(
        manifest.format == FORMAT,
        "unsupported migration bundle format"
    );
    let application = safe_path(root, "application")?;
    data_backup::verify(&application)?;
    let records = projects_from_copy(&safe_path(&application, "data")?)?;
    let projects = safe_path(root, "projects")?;
    exact_names(
        &projects,
        manifest
            .projects
            .iter()
            .map(|project| project.id.clone())
            .collect(),
    )?;
    let mut covered = BTreeMap::new();
    for project in &manifest.projects {
        ensure!(
            uuid::Uuid::parse_str(&project.id).is_ok(),
            "invalid project identifier"
        );
        ensure!(
            covered
                .insert(project.id.clone(), project.stored_path.clone())
                .is_none(),
            "duplicate project identifier"
        );
        let copy = safe_path(&projects, &project.id)?;
        ensure!(
            data_backup::inventory(&copy)? == project.entries,
            "project backup is incomplete or modified"
        );
        ensure!(
            project
                .entries
                .iter()
                .any(|entry| entry.path == "project.godot" && entry.sha256.is_some()),
            "project backup has no project.godot"
        );
    }
    ensure!(
        records == covered,
        "archive does not cover all registered projects"
    );
    Ok(manifest)
}

/// Materialize recovery copies and an explicit relocation map; do not activate a new host.
pub fn restore(backup: &Path, destination: &Path) -> Result<RestoreReceipt> {
    let backup = fs::canonicalize(backup)?;
    let manifest = verify(&backup)?;
    let target = data_backup::new_destination(&backup, destination)?;
    write_json(
        &target.join(PENDING),
        &serde_json::json!({"readyToActivate":false}),
    )?;
    let data = target.join("data");
    let application = data_backup::restore(&backup.join("application"), &data)?;
    let projects_root = target.join("projects");
    fs::create_dir(&projects_root)?;
    let mut projects = Vec::new();
    for project in manifest.projects {
        let restored_path = safe_path(&projects_root, &project.id)?;
        fs::create_dir(&restored_path)?;
        data_backup::copy_entries(
            &safe_path(&backup.join("projects"), &project.id)?,
            &restored_path,
            &project.entries,
        )?;
        projects.push(RestoredProject {
            id: project.id,
            original_path: project.stored_path,
            restored_path,
        });
    }
    verify(&backup)?;
    let receipt = RestoreReceipt {
        format: "beaver-migration-restore-v1".into(),
        original_data: application.source,
        restored_data: data,
        projects,
        paths_rewritten: false,
        credentials_converted: false,
        ready_to_activate: false,
    };
    write_json(&target.join("RESTORE.json"), &receipt)?;
    Ok(receipt)
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use crate::store::Store;
    use serde_json::json;

    fn fixture(root: &Path) -> Result<(PathBuf, PathBuf, String)> {
        let data = root.join("old-data");
        let game = root.join("original-game");
        fs::create_dir_all(game.join(".git/empty"))?;
        fs::create_dir(game.join(".godot"))?;
        fs::write(game.join("project.godot"), "config_version=5")?;
        fs::write(game.join(".godot/imported.bin"), [0, 255, 3])?;
        fs::write(game.join(".git/config"), "preserve repository")?;
        let id = uuid::Uuid::new_v4().to_string();
        let store = Store::open(&data)?;
        store.put(
            "project",
            &id,
            &json!({"id":id,"path":game,"unknown":"preserved"}),
        )?;
        store.put("secret", "code", &"opaque-secret")?;
        drop(store);
        Ok((data, game, id))
    }

    #[test]
    fn complete_bundle_restores_without_originals_and_keeps_database_bytes() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let (data, game, id) = fixture(temp.path())?;
        let data_before = data_backup::inventory(&data)?;
        let game_before = data_backup::inventory(&game)?;
        let backup = temp.path().join("bundle");
        create(&data, &backup)?;
        ensure!(data_backup::inventory(&data)? == data_before);
        ensure!(data_backup::inventory(&game)? == game_before);
        fs::rename(&data, temp.path().join("old-data-unavailable"))?;
        fs::rename(&game, temp.path().join("old-game-unavailable"))?;
        verify(&backup)?;
        let destination = temp.path().join("restored");
        let receipt = restore(&backup, &destination)?;
        ensure!(ensure_activated(&destination).is_err());
        ensure!(ensure_activated(&destination.join("not-created-yet/child")).is_err());
        ensure!(ensure_activated(&receipt.restored_data).is_err());
        ensure!(ensure_activated(&receipt.restored_data.join(".")).is_err());
        ensure!(
            !receipt.ready_to_activate
                && !receipt.paths_rewritten
                && !receipt.credentials_converted
        );
        ensure!(data_backup::inventory(&receipt.restored_data)? == data_before);
        ensure!(data_backup::inventory(&receipt.projects[0].restored_path)? == game_before);
        ensure!(receipt.projects[0].id == id);
        ensure!(restore(&backup, &destination).is_err());
        fs::write(
            backup.join("projects").join(id).join("project.godot"),
            "tampered",
        )?;
        ensure!(verify(&backup).is_err());
        let rejected = temp.path().join("rejected");
        ensure!(restore(&backup, &rejected).is_err() && !rejected.exists());
        Ok(())
    }

    #[test]
    fn incomplete_or_overlapping_projects_never_silently_omitted() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let (data, game, id) = fixture(temp.path())?;
        ensure!(create(&data, &game.join("backup")).is_err());
        ensure!(!game.join("backup").exists());
        let store = Store::open(&data)?;
        let missing_id = uuid::Uuid::new_v4().to_string();
        store.put(
            "project",
            &missing_id,
            &json!({"id":missing_id,"path":temp.path().join("missing")}),
        )?;
        drop(store);
        let backup = temp.path().join("bundle");
        ensure!(create(&data, &backup).is_err() && !backup.exists());
        let store = Store::open(&data)?;
        store.remove("project", &missing_id)?;
        store.put(
            "project",
            &missing_id,
            &json!({"id":missing_id,"path":game}),
        )?;
        drop(store);
        ensure!(create(&data, &backup).is_err() && !backup.exists());
        let store = Store::open(&data)?;
        store.remove("project", &missing_id)?;
        drop(store);
        create(&data, &backup)?;
        let manifest_path = backup.join(MANIFEST);
        let mut manifest: Value = serde_json::from_slice(&fs::read(&manifest_path)?)?;
        manifest["projects"] = json!([]);
        fs::write(&manifest_path, serde_json::to_vec(&manifest)?)?;
        fs::remove_dir_all(backup.join("projects").join(id))?;
        ensure!(verify(&backup).is_err());
        Ok(())
    }
}
