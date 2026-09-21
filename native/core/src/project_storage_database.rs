//! SQLite identity validation, isolated from application-data initialization.
use crate::{project_storage_layout as layout, store::Store, store_schema};
use anyhow::{ensure, Result};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use serde_json::Value;
use std::{collections::BTreeSet, fs, path::Path, sync::Arc, time::Duration};

const APPLICATION_ID: i64 = 0x42455652;
const IDENTITY_SQL: &str = "CREATE TABLE project_identity (
    singleton INTEGER PRIMARY KEY CHECK(singleton=1),
    project_id TEXT NOT NULL, schema_version INTEGER NOT NULL,
    storage_version INTEGER NOT NULL)";

fn connect(path: &Path, flags: OpenFlags) -> Result<Connection> {
    let connection = Connection::open_with_flags(path, flags)?;
    connection.busy_timeout(Duration::from_secs(5))?;
    connection.execute_batch("PRAGMA trusted_schema=OFF;")?;
    Ok(connection)
}

pub(crate) fn initialize(
    directory: &Path,
    manifest: &layout::Manifest,
    lock: Arc<fs::File>,
) -> Result<Store> {
    let path = directory.join(layout::DATABASE);
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?
        .sync_all()?;
    let connection = connect(&path, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    connection
        .execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; BEGIN IMMEDIATE;")?;
    // Initialization has a filesystem pending marker until all files are durable.
    store_schema::initialize(&connection)?;
    connection.execute_batch(IDENTITY_SQL)?;
    connection.execute(
        "INSERT INTO project_identity VALUES(1,?,?,?)",
        params![
            manifest.project_id,
            manifest.schema_version,
            manifest.storage_version
        ],
    )?;
    connection.pragma_update(None, "application_id", APPLICATION_ID)?;
    connection.pragma_update(None, "user_version", layout::STORAGE_VERSION)?;
    connection.execute_batch("COMMIT;")?;
    validate(&connection, manifest)?;
    let busy: i64 =
        connection.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| row.get(0))?;
    ensure!(busy == 0, "项目初始化检查点未完成");
    Ok(Store::project(connection, lock))
}

fn validate(connection: &Connection, manifest: &layout::Manifest) -> Result<()> {
    let application: i64 =
        connection.pragma_query_value(None, "application_id", |row| row.get(0))?;
    let version: u32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    ensure!(
        application == APPLICATION_ID && version == layout::STORAGE_VERSION,
        "不支持的项目数据库版本"
    );
    let mode: String = connection.pragma_query_value(None, "journal_mode", |row| row.get(0))?;
    ensure!(mode.eq_ignore_ascii_case("wal"), "项目数据库日志模式不匹配");
    for (name, sql) in store_schema::DEFINITIONS {
        store_schema::validate_definition(connection, name, sql)?;
    }
    store_schema::validate_definition(connection, "project_identity", IDENTITY_SQL)?;
    let mut expected: BTreeSet<String> = store_schema::DEFINITIONS
        .iter()
        .map(|(name, _)| (*name).to_owned())
        .collect();
    expected.insert("project_identity".into());
    let mut statement =
        connection.prepare("SELECT name FROM sqlite_schema WHERE name NOT GLOB 'sqlite_*'")?;
    let actual = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<BTreeSet<_>>>()?;
    ensure!(actual == expected, "项目数据库包含未知结构，不能直接打开");
    let identities: i64 =
        connection.query_row("SELECT count(*) FROM project_identity", [], |row| {
            row.get(0)
        })?;
    let (id, schema, storage): (String, u32, u32) = connection.query_row(
        "SELECT project_id,schema_version,storage_version FROM project_identity WHERE singleton=1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    ensure!(
        identities == 1
            && id == manifest.project_id
            && schema == manifest.schema_version
            && storage == manifest.storage_version,
        "项目数据库身份与清单不一致"
    );
    let mut check = connection.prepare("PRAGMA quick_check")?;
    let results = check
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    ensure!(results == ["ok"], "项目数据库完整性检查失败");
    Ok(())
}

/// The caller holds the project lock. Only the temporary copy is opened for preflight:
/// a read-only SQLite connection against the source could still create WAL/SHM files.
pub(crate) fn snapshot(directory: &Path, manifest: &layout::Manifest) -> Result<Snapshot> {
    let copy = tempfile::tempdir()?;
    for suffix in ["", "-wal", "-shm"] {
        let name = format!("{}{suffix}", layout::DATABASE);
        let source = directory.join(&name);
        if !layout::absent(&source)? {
            layout::ordinary(&source, false)?;
            fs::copy(&source, copy.path().join(name))?;
        }
    }
    let connection = connect(
        &copy.path().join(layout::DATABASE),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    validate(&connection, manifest)?;
    Ok(Snapshot {
        connection,
        _directory: copy,
    })
}

/// Close SQLite before deleting its isolated files. No source connection is opened.
pub(crate) struct Snapshot {
    connection: Connection,
    _directory: tempfile::TempDir,
}

impl Snapshot {
    pub(crate) fn derive(
        &self,
        prepared: &crate::project_derivation_copy::Prepared,
    ) -> Result<crate::project_derivation_database::Staged> {
        crate::project_derivation_database::compose_prepared(&self.connection, prepared)
    }

    pub(crate) fn derivation_identities(
        &self,
        request: &crate::project_derivation_copy::Request,
    ) -> Result<crate::project_derivation_identity::IdentityMap> {
        crate::project_derivation_identity::build(&self.connection, request)
    }

    pub(crate) fn project(&self, id: &str) -> Result<Option<Value>> {
        let json: Option<String> = self
            .connection
            .query_row(
                "SELECT value FROM entities WHERE kind='project' AND id=?",
                [id],
                |row| row.get(0),
            )
            .optional()?;
        json.map(|json| serde_json::from_str(&json).map_err(Into::into))
            .transpose()
    }

    pub(crate) fn tasks(&self) -> Result<Vec<(String, Value)>> {
        let mut statement = self
            .connection
            .prepare("SELECT id,value FROM entities WHERE kind='task' ORDER BY id")?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.map(|row| {
            let (id, value) = row?;
            Ok((id, serde_json::from_str(&value)?))
        })
        .collect()
    }
}

pub(crate) fn open(
    directory: &Path,
    manifest: &layout::Manifest,
    lock: Arc<fs::File>,
) -> Result<Store> {
    snapshot(directory, manifest)?;
    // No CREATE flag or DDL here. Invalid or missing project stores never fall back to host data.
    let connection = connect(
        &directory.join(layout::DATABASE),
        OpenFlags::SQLITE_OPEN_READ_WRITE,
    )?;
    connection.execute_batch("PRAGMA synchronous=FULL;")?;
    Ok(Store::project(connection, lock))
}
