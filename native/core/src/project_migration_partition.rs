//! Partition a restored recovery copy into project stores; the copy stays pending afterwards.
use crate::{
    data_backup,
    files::safe_path,
    migration_bundle::{self, RestoreReceipt},
    project_migration_inventory::{self, Inventory},
    project_migration_ownership::Entity,
    project_migration_plan::{self, FileMove, Plan, ProjectPlan, Retained},
    project_migration_rewrite::Rewriter,
    project_migration_session_rewrite,
    project_storage::ProjectStore,
};
use anyhow::{ensure, Context, Result};
use fs2::FileExt;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

pub(crate) const RECEIPT: &str = "PROJECT-MIGRATION.json";
pub(crate) const FORMAT: &str = "beaver-project-partition-v1";
const PENDING: &str = ".beaver-migration-pending";

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCounts {
    pub entities: usize,
    pub events: usize,
    pub calls: usize,
    pub files: usize,
    pub fields_rewritten: usize,
    pub session_paths_rewritten: usize,
    pub root: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Receipt {
    pub format: String,
    pub restored_data: PathBuf,
    pub backup_manifest_sha256: String,
    pub application_manifest_sha256: String,
    pub projects: BTreeMap<String, ProjectCounts>,
    pub retained: Vec<Retained>,
    pub retained_files: Vec<String>,
    pub ready_to_activate: bool,
    pub activation_blockers: Vec<String>,
}

struct HostRows {
    entities: Vec<Entity>,
    raw: BTreeMap<(String, String), String>,
    events: Vec<(i64, String)>,
    calls: Vec<(i64, Option<String>)>,
}

fn read_host(connection: &Connection) -> Result<HostRows> {
    let mut rows = HostRows {
        entities: Vec::new(),
        raw: BTreeMap::new(),
        events: Vec::new(),
        calls: Vec::new(),
    };
    let mut statement =
        connection.prepare("SELECT kind,id,value FROM entities ORDER BY kind,id")?;
    for row in statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })? {
        let (kind, id, raw) = row?;
        rows.entities.push(Entity {
            kind: kind.clone(),
            id: id.clone(),
            value: serde_json::from_str(&raw).ok(),
        });
        rows.raw.insert((kind, id), raw);
    }
    let mut statement = connection.prepare("SELECT seq,task FROM events ORDER BY seq")?;
    rows.events = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let mut statement = connection.prepare("SELECT seq,task FROM calls ORDER BY seq")?;
    rows.calls = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(rows)
}

fn copy_files(data: &Path, root: &Path, files: &[FileMove]) -> Result<()> {
    for file in files {
        let source = safe_path(data, &file.source)?;
        let target = safe_path(root, &file.target)?;
        if file.sha256.is_none() {
            fs::create_dir_all(&target)?;
            continue;
        }
        fs::create_dir_all(target.parent().context("partition file has no parent")?)?;
        if target.exists() {
            // Shared blobs are content-addressed; a second project copy is byte-identical.
            ensure!(
                file.target.starts_with(".beaver/content/blobs/")
                    && crate::files::file_hash(&target)? == file.sha256,
                "partition target already exists: {}",
                file.target
            );
            continue;
        }
        let mut input = fs::File::open(&source)?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)?;
        let bytes = std::io::copy(&mut input, &mut output)?;
        output.sync_all()?;
        ensure!(
            bytes == file.bytes,
            "recovery copy changed while partitioning"
        );
        ensure!(
            crate::files::file_hash(&target)? == file.sha256,
            "partition copy differs from archive: {}",
            file.source
        );
    }
    Ok(())
}

fn write_rows(
    host: &Connection,
    store: &mut ProjectStore,
    rows: &HostRows,
    plan: &ProjectPlan,
    original: &Path,
) -> Result<usize> {
    let mut rewriter = Rewriter::new(original);
    let mut entities = Vec::with_capacity(plan.entities.len());
    for (kind, id) in &plan.entities {
        let raw = rows
            .raw
            .get(&(kind.clone(), id.clone()))
            .context("inventory entity missing from recovery copy")?;
        entities.push((kind, id, rewriter.entity(kind, id, raw)?));
    }
    store.store_mut().transaction(|db| {
        for (kind, id, value) in &entities {
            db.execute(
                "INSERT INTO entities(kind,id,value) VALUES(?,?,?)",
                params![kind, id, value],
            )?;
        }
        for seq in &plan.events {
            let (task, time, kind, text): (String, String, String, String) = host.query_row(
                "SELECT task,time,kind,text FROM events WHERE seq=?",
                [seq],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )?;
            db.execute(
                "INSERT INTO events(seq,task,time,kind,text) VALUES(?,?,?,?,?)",
                params![seq, task, time, kind, text],
            )?;
        }
        for seq in &plan.calls {
            let (id, task, project, method, value): (
                String,
                Option<String>,
                Option<String>,
                String,
                String,
            ) = host.query_row(
                "SELECT id,task,project,method,value FROM calls WHERE seq=?",
                [seq],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )?;
            db.execute(
                "INSERT INTO calls(seq,id,task,project,method,value) VALUES(?,?,?,?,?,?)",
                params![seq, id, task, project, method, value],
            )?;
        }
        Ok(())
    })?;
    Ok(rewriter.rewritten)
}

fn remove_moved(host: &mut Connection, data: &Path, plan: &Plan) -> Result<()> {
    let transaction = host.transaction()?;
    for project in plan.projects.values() {
        // The host keeps the project entity as its registration; the partition owns the copy.
        for (kind, id) in project
            .entities
            .iter()
            .filter(|(kind, _)| kind != "project")
        {
            let removed = transaction.execute(
                "DELETE FROM entities WHERE kind=? AND id=?",
                params![kind, id],
            )?;
            ensure!(removed == 1, "moved entity vanished from the host copy");
        }
        for seq in &project.events {
            ensure!(
                transaction.execute("DELETE FROM events WHERE seq=?", [seq])? == 1,
                "moved event vanished from the host copy"
            );
        }
        for seq in &project.calls {
            ensure!(
                transaction.execute("DELETE FROM calls WHERE seq=?", [seq])? == 1,
                "moved call vanished from the host copy"
            );
        }
    }
    transaction.commit()?;
    let mut moved: Vec<&FileMove> = plan
        .projects
        .values()
        .flat_map(|project| &project.files)
        .filter(|file| !file.source.starts_with("blobs/"))
        .collect();
    moved.sort_by(|a, b| b.source.cmp(&a.source));
    moved.dedup_by(|a, b| a.source == b.source);
    for file in moved {
        let path = safe_path(data, &file.source)?;
        if file.sha256.is_none() {
            fs::remove_dir(&path)
                .with_context(|| format!("host copy directory not emptied: {}", file.source))?;
        } else {
            fs::remove_file(&path)?;
        }
    }
    Ok(())
}

fn restored_copy(backup: &Path, restored: &Path) -> Result<(RestoreReceipt, Inventory)> {
    let restored = fs::canonicalize(restored)?;
    ensure!(
        restored.join(PENDING).is_file(),
        "restored copy is not a pending recovery root"
    );
    ensure!(
        !restored.join(RECEIPT).exists(),
        "restored copy has already been partitioned"
    );
    let receipt: RestoreReceipt =
        serde_json::from_slice(&fs::read(restored.join("RESTORE.json"))?)?;
    ensure!(
        !receipt.paths_rewritten && !receipt.credentials_converted,
        "partitioning requires an unconverted restore"
    );
    ensure!(
        fs::canonicalize(&receipt.restored_data)? == restored.join("data"),
        "restore receipt does not describe this copy"
    );
    let inventory = project_migration_inventory::inspect(backup)?;
    let application = data_backup::verify(&safe_path(backup, "application")?)?;
    ensure!(
        data_backup::inventory(&receipt.restored_data)? == application.entries,
        "restored data differs from the archive"
    );
    let manifest = migration_bundle::verify(backup)?;
    for project in &manifest.projects {
        let restored_project = receipt
            .projects
            .iter()
            .find(|candidate| candidate.id == project.id)
            .context("restored copy lacks an archived project")?;
        ensure!(
            data_backup::inventory(&restored_project.restored_path)? == project.entries,
            "restored project differs from the archive"
        );
    }
    Ok((receipt, inventory))
}

/// Blobs are content-addressed and stay in the host copy for retained history.
pub fn partition(backup: &Path, restored: &Path) -> Result<Receipt> {
    let (receipt, inventory) = restored_copy(backup, restored)?;
    let data = receipt.restored_data.clone();
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(safe_path(&data, ".beaver-native.lock")?)?;
    lock.try_lock_exclusive()
        .context("recovery copy is in use")?;
    let mut host = Connection::open(data.join("beaver.sqlite"))?;
    let rows = read_host(&host)?;
    let plan =
        project_migration_plan::build(&inventory, &rows.entities, &rows.events, &rows.calls)?;
    let mut projects = BTreeMap::new();
    for restored_project in &receipt.projects {
        let project_plan = plan
            .projects
            .get(&restored_project.id)
            .context("plan lacks an archived project")?;
        let root = &restored_project.restored_path;
        let mut store = ProjectStore::initialize_partition(root, &restored_project.id)?;
        copy_files(&data, root, &project_plan.files)?;
        let fields = write_rows(
            &host,
            &mut store,
            &rows,
            project_plan,
            &receipt.original_data,
        )?;
        drop(store);
        let sessions = project_migration_session_rewrite::rewrite(
            &inventory.session_indexes.records,
            &receipt,
            restored_project,
            project_plan,
        )?;
        projects.insert(
            restored_project.id.clone(),
            ProjectCounts {
                entities: project_plan.entities.len(),
                events: project_plan.events.len(),
                calls: project_plan.calls.len(),
                files: project_plan.files.len(),
                fields_rewritten: fields,
                session_paths_rewritten: sessions,
                root: root.clone(),
            },
        );
    }
    remove_moved(&mut host, &data, &plan)?;
    drop(host);
    migration_bundle::verify(backup)?;
    let mut blockers = vec![
        "run activate-import with the original verified bundle to check destination tools and enable this copy".to_owned(),
    ];
    if !plan.retained.is_empty() || !plan.retained_files.is_empty() {
        blockers
            .push("retained legacy history stays in the host copy; activation marks its tasks migrationRetained and they need explicit conversion before use".into());
    }
    let receipt = Receipt {
        format: FORMAT.to_owned(),
        restored_data: data,
        backup_manifest_sha256: crate::files::file_hash(&backup.join("BEAVER-MIGRATION.json"))?
            .context("backup manifest missing")?,
        application_manifest_sha256: crate::files::file_hash(
            &backup.join("application/BEAVER-BACKUP.json"),
        )?
        .context("application manifest missing")?,
        projects,
        retained: plan.retained,
        retained_files: plan.retained_files,
        ready_to_activate: false,
        activation_blockers: blockers,
    };
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(fs::canonicalize(restored)?.join(RECEIPT))?;
    file.write_all(&serde_json::to_vec_pretty(&receipt)?)?;
    file.sync_all()?;
    Ok(receipt)
}
