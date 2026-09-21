//! Explicit project storage lifecycle. Desktop routing is enabled only after migration is wired.
use crate::{
    project_runtime::ProjectRuntime, project_storage_database as database,
    project_storage_layout as layout, store::Store,
};
use anyhow::{ensure, Context, Result};
use fs2::FileExt;
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct ProjectStore {
    // Rust drops fields in declaration order: close SQLite before releasing the writer lock.
    store: Store,
    lock: Arc<File>,
    root: PathBuf,
    manifest: layout::Manifest,
}

pub(crate) fn lock(directory: &Path, create: bool) -> Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true);
    if create {
        options.create_new(true);
    }
    let lock = options.open(directory.join(layout::LOCK))?;
    lock.try_lock_exclusive()
        .context("项目正在被另一个宿主使用")?;
    Ok(lock)
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new().create_new(true).write(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

impl ProjectStore {
    /// Only for an explicitly new/converted project. Never repairs or overwrites existing storage.
    pub fn initialize(project_root: &Path, project_id: &str) -> Result<Self> {
        Self::initialize_at(layout::root(project_root)?, project_id)
    }

    /// Build a partition inside a pending recovery copy; the copy's marker still blocks `open`.
    pub(crate) fn initialize_partition(project_root: &Path, project_id: &str) -> Result<Self> {
        Self::initialize_at(layout::partition_root(project_root)?, project_id)
    }

    fn initialize_at(root: PathBuf, project_id: &str) -> Result<Self> {
        layout::valid_id(project_id)?;
        layout::ordinary(&root.join("project.godot"), false)?;
        let directory = root.join(layout::CONTROL_DIR);
        fs::create_dir(&directory).context("不能初始化已有 .beaver 目录；保留原数据")?;
        let manifest = layout::Manifest {
            schema_version: layout::SCHEMA_VERSION,
            storage_version: layout::STORAGE_VERSION,
            project_id: project_id.to_owned(),
        };
        write_new(
            &directory.join(layout::PENDING),
            &serde_json::to_vec(&manifest)?,
        )?;
        let lock = Arc::new(lock(&directory, true)?);
        for name in layout::DIRECTORIES {
            fs::create_dir(directory.join(name))?;
        }
        let store = database::initialize(&directory, &manifest, lock.clone())?;
        write_new(
            &directory.join(layout::MANIFEST),
            &serde_json::to_vec_pretty(&manifest)?,
        )?;
        layout::validate_files(&root)?;
        fs::remove_file(directory.join(layout::PENDING))?;
        Ok(Self {
            store,
            lock,
            root,
            manifest,
        })
    }

    pub fn open(project_root: &Path, project_id: &str) -> Result<Self> {
        Self::open_at(layout::root(project_root)?, project_id)
    }

    /// Registration inspects an idle portable store without SQLite touching source files.
    pub fn read_project(project_root: &Path, project_id: &str) -> Result<serde_json::Value> {
        let root = layout::root(project_root)?;
        layout::manifest_in(&root, Some(project_id))?;
        layout::validate_files(&root)?;
        let directory = root.join(layout::CONTROL_DIR);
        let _lock = lock(&directory, false)?;
        let manifest = layout::manifest_in(&root, Some(project_id))?;
        layout::validate_files(&root)?;
        let snapshot = database::snapshot(&directory, &manifest)?;
        let project = snapshot
            .project(project_id)?
            .context("项目本地存储缺少项目实体")?;
        ensure!(project["id"] == project_id, "项目本地实体 ID 与清单不一致");
        Ok(project)
    }

    /// Activation checks a partition while the recovery copy is still pending.
    pub(crate) fn open_partition(project_root: &Path, project_id: &str) -> Result<Self> {
        Self::open_at(layout::partition_root(project_root)?, project_id)
    }

    fn open_at(root: PathBuf, project_id: &str) -> Result<Self> {
        let manifest = layout::manifest_in(&root, Some(project_id))?;
        layout::validate_files(&root)?;
        let directory = root.join(layout::CONTROL_DIR);
        let lock = Arc::new(lock(&directory, false)?);
        // Recheck after acquiring ownership; another initializer may have completed while opening.
        let current = layout::manifest_in(&root, Some(project_id))?;
        layout::validate_files(&root)?;
        ensure!(
            current.project_id == manifest.project_id,
            "项目身份在打开期间发生变化"
        );
        let store = database::open(&directory, &current, lock.clone())?;
        Ok(Self {
            store,
            lock,
            root,
            manifest: current,
        })
    }

    pub fn project_id(&self) -> &str {
        &self.manifest.project_id
    }

    pub fn project_root(&self) -> &Path {
        &self.root
    }

    pub fn store(&self) -> &Store {
        &self.store
    }

    pub fn store_mut(&mut self) -> &mut Store {
        &mut self.store
    }

    pub fn into_runtime(self) -> ProjectRuntime {
        ProjectRuntime::new(self.store, self.root, self.manifest.project_id, self.lock)
    }
}

/// One owning handle per registered project; selection in a UI does not transfer storage ownership.
#[derive(Default)]
pub struct ProjectStores {
    opened: BTreeMap<String, ProjectRuntime>,
}

impl ProjectStores {
    pub fn open(&mut self, root: &Path, project_id: &str) -> Result<ProjectRuntime> {
        let root = layout::root(root)?;
        if let Some(existing) = self.opened.get(project_id) {
            ensure!(
                existing.project_root() == root,
                "同一项目 ID 对应多个位置；请明确重新关联或派生独立项目身份"
            );
            layout::read_manifest(&root, Some(project_id))?;
            layout::validate_files(&root)?;
        } else {
            self.opened.insert(
                project_id.to_owned(),
                ProjectStore::open(&root, project_id)?.into_runtime(),
            );
        }
        self.opened
            .get(project_id)
            .cloned()
            .context("项目存储未打开")
    }

    pub fn close(&mut self, project_id: &str) -> Result<()> {
        self.ensure_can_close(project_id)?;
        self.opened.remove(project_id);
        Ok(())
    }

    pub(crate) fn ensure_can_close(&self, project_id: &str) -> Result<()> {
        if let Some(runtime) = self.opened.get(project_id) {
            ensure!(!runtime.in_use(), "项目仍有运行时引用，不能关闭或重新关联");
        }
        Ok(())
    }

    pub fn get(&self, project_id: &str) -> Option<ProjectRuntime> {
        self.opened.get(project_id).cloned()
    }

    pub fn runtimes(&self) -> Vec<ProjectRuntime> {
        self.opened.values().cloned().collect()
    }

    pub(crate) fn project_ids(&self) -> Vec<String> {
        self.opened.keys().cloned().collect()
    }
}
