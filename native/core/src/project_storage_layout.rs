//! Non-mutating project identity and filesystem checks.
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

pub const SCHEMA_VERSION: u32 = 1;
pub const STORAGE_VERSION: u32 = 1;
pub const CONTROL_DIR: &str = ".beaver";
pub const MANIFEST: &str = "project.json";
pub const DATABASE: &str = "project.sqlite";
pub const DIRECTORIES: &[&str] = &[
    "content",
    "workspaces",
    "previews",
    "evidence",
    "operations",
    "cache",
];
pub(crate) const LOCK: &str = ".project.lock";
pub(crate) const PENDING: &str = ".storage-pending";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub schema_version: u32,
    pub storage_version: u32,
    pub project_id: String,
}

pub(crate) fn valid_id(id: &str) -> Result<()> {
    ensure!(
        !id.is_empty()
            && id.len() <= 128
            && id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"_.:-".contains(&c)),
        "非法项目 ID"
    );
    Ok(())
}

pub(crate) fn absent(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(false),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(error) => Err(error.into()),
    }
}

pub(crate) fn ordinary(path: &Path, directory: bool) -> Result<()> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("无法读取项目存储路径：{}", path.display()))?;
    ensure!(
        !crate::files::linked(&metadata)
            && if directory {
                metadata.is_dir()
            } else {
                metadata.is_file()
            },
        "项目存储路径必须是实际的{}，不允许符号链接或目录联接：{}",
        if directory { "目录" } else { "文件" },
        path.display()
    );
    Ok(())
}

/// A recovery copy keeps its ancestor pending marker until activation; partitions are
/// built inside it, but never opened through the ordinary `root` check before activation.
pub(crate) fn partition_root(path: &Path) -> Result<PathBuf> {
    ensure!(path.is_absolute(), "项目位置必须是绝对路径");
    ordinary(path, true)?;
    Ok(fs::canonicalize(path)?)
}

pub(crate) fn root(path: &Path) -> Result<PathBuf> {
    let root = partition_root(path)?;
    for ancestor in root.ancestors() {
        ensure!(
            absent(&ancestor.join(".beaver-migration-pending"))?,
            "项目副本尚未完成迁移启用"
        );
    }
    Ok(root)
}

pub fn read_manifest(path: &Path, expected_id: Option<&str>) -> Result<Manifest> {
    manifest_in(&root(path)?, expected_id)
}

/// `root` must already be canonical: either `root(..)` or `partition_root(..)` produced it.
pub(crate) fn manifest_in(root: &Path, expected_id: Option<&str>) -> Result<Manifest> {
    let directory = root.join(CONTROL_DIR);
    ordinary(&directory, true)?;
    ensure!(
        absent(&directory.join(PENDING))?,
        "项目存储初始化未完成，请保留目录并恢复"
    );
    let path = directory.join(MANIFEST);
    ordinary(&path, false)?;
    let mut bytes = Vec::new();
    fs::File::open(path)?.take(65537).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= 65536, "项目清单超过 64 KiB");
    let manifest: Manifest = serde_json::from_slice(&bytes).context("项目清单格式无效")?;
    ensure!(
        manifest.schema_version == SCHEMA_VERSION && manifest.storage_version == STORAGE_VERSION,
        "不支持的项目存储版本"
    );
    valid_id(&manifest.project_id)?;
    if let Some(id) = expected_id {
        ensure!(manifest.project_id == id, "项目清单 ID 与登记不一致");
    }
    Ok(manifest)
}

pub(crate) fn validate_files(root: &Path) -> Result<()> {
    ordinary(&root.join("project.godot"), false)?;
    let directory = root.join(CONTROL_DIR);
    ordinary(&directory, true)?;
    for name in DIRECTORIES {
        ordinary(&directory.join(name), true)?;
    }
    for name in [DATABASE, LOCK] {
        ordinary(&directory.join(name), false)?;
    }
    for suffix in ["-wal", "-shm"] {
        let path = directory.join(format!("{DATABASE}{suffix}"));
        if !absent(&path)? {
            ordinary(&path, false)?;
        }
    }
    ensure!(
        absent(&directory.join(format!("{DATABASE}-journal")))?,
        "项目数据库需要日志恢复，不能直接打开"
    );
    Ok(())
}
