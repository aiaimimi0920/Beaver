//! Host-registered project and task storage routing without host fallbacks.
use crate::{
    project_runtime::ProjectRuntime, project_storage::ProjectStores,
    project_storage_layout as layout, store::Store,
};
use anyhow::{bail, ensure, Context, Result};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[path = "project_derivation_registration.rs"]
mod derivation_registration;
#[path = "project_object_source.rs"]
mod object_source;
#[path = "project_reassociation.rs"]
mod reassociation;
#[path = "project_unregistration.rs"]
mod unregistration;

pub struct ProjectStorageRouter {
    host: Arc<Mutex<Store>>,
    projects: Mutex<ProjectStores>,
    task_projects: Mutex<BTreeMap<String, String>>,
}

impl ProjectStorageRouter {
    pub fn new(host: Arc<Mutex<Store>>) -> Self {
        Self {
            host,
            projects: Mutex::new(ProjectStores::default()),
            task_projects: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn open_registered(&self, project_id: &str) -> Result<ProjectRuntime> {
        self.open_registered_with(project_id, |_, _| Ok(()))
    }

    pub fn work_gate(&self, project_id: &str) -> crate::project_work_gate::ProjectWorkGate {
        crate::project_work_gate::ProjectWorkGate::registered(self.host.clone(), project_id)
    }

    /// Open a registered project and initialize a newly created runtime before publishing it.
    ///
    /// The project registry remains locked while `initialize` runs. Concurrent callers cannot
    /// observe the runtime until initialization and task indexing both succeed.
    pub fn open_registered_with<F>(&self, project_id: &str, initialize: F) -> Result<ProjectRuntime>
    where
        F: FnOnce(&mut Store, &crate::files::Files) -> Result<()>,
    {
        let mut projects = self
            .projects
            .lock()
            .map_err(|_| anyhow::anyhow!("项目注册表锁不可用"))?;
        let (runtime, was_open) = if let Some(runtime) = projects.get(project_id) {
            // An already-open runtime is authoritative when the host path is stale.
            // If the replacement path exists, still reject an ID reassignment.
            if let Ok(registered_root) = self.registered_root(project_id) {
                if registered_root.exists() {
                    let root = layout::root(&registered_root)?;
                    ensure!(
                        runtime.project_root() == root,
                        "同一项目 ID 对应多个位置；请明确重新关联或派生独立项目身份"
                    );
                }
            }
            (runtime, true)
        } else {
            let root = self.registered_root(project_id)?;
            (projects.open(&root, project_id)?, false)
        };
        if !was_open {
            let initialized = (|| {
                let store_handle = runtime.store();
                let files = runtime.files();
                let mut store = store_handle
                    .lock()
                    .map_err(|_| anyhow::anyhow!("项目数据库锁不可用"))?;
                initialize(&mut store, &files)
            })();
            if let Err(error) = initialized {
                cleanup_failed_open(&mut projects, project_id, runtime, was_open);
                return Err(error);
            }
        }
        let tasks = match self.read_tasks(&runtime) {
            Ok(tasks) => tasks,
            Err(error) => {
                cleanup_failed_open(&mut projects, project_id, runtime, was_open);
                return Err(error);
            }
        };
        let mut index = match self.task_projects.lock() {
            Ok(index) => index,
            Err(_) => {
                cleanup_failed_open(&mut projects, project_id, runtime, was_open);
                bail!("任务路由锁不可用");
            }
        };
        for task_id in tasks.keys() {
            if index.get(task_id).is_some_and(|owner| owner != project_id) {
                drop(index);
                cleanup_failed_open(&mut projects, project_id, runtime, was_open);
                bail!("任务 ID 同时归属于多个项目：{task_id}");
            }
        }
        index.retain(|_, owner| owner != project_id);
        index.extend(tasks);
        Ok(runtime)
    }

    pub fn runtime_for_project(&self, project_id: &str) -> Result<ProjectRuntime> {
        self.projects
            .lock()
            .map_err(|_| anyhow::anyhow!("项目注册表锁不可用"))?
            .get(project_id)
            .with_context(|| format!("项目未打开：{project_id}"))
    }

    pub fn runtime_for_task(&self, task_id: &str) -> Result<ProjectRuntime> {
        let project_id = self
            .task_projects
            .lock()
            .map_err(|_| anyhow::anyhow!("任务路由锁不可用"))?
            .get(task_id)
            .cloned()
            .with_context(|| format!("任务未注册：{task_id}"))?;
        let runtime = self.runtime_for_project(&project_id)?;
        let task = runtime
            .store()
            .lock()
            .map_err(|_| anyhow::anyhow!("项目数据库锁不可用"))?
            .get::<Value>("task", task_id)?
            .with_context(|| format!("项目中不存在任务：{task_id}"))?;
        validate_task(task_id, &project_id, &task)?;
        Ok(runtime)
    }

    /// Refresh an already-open project's task index before resolving a task.
    ///
    /// This is used when a task was persisted by another project-owned writer
    /// after the runtime was opened. The open runtime remains authoritative;
    /// callers must not fall back to the host store after this refresh fails.
    pub fn refresh_open_task_route(
        &self,
        project_id: &str,
        task_id: &str,
    ) -> Result<ProjectRuntime> {
        self.runtime_for_project(project_id)?;
        self.open_registered(project_id)?;
        let runtime = self.runtime_for_task(task_id)?;
        ensure!(
            runtime.project_id() == project_id,
            "任务路由项目不一致：{task_id}"
        );
        Ok(runtime)
    }

    pub fn runtimes(&self) -> Result<Vec<ProjectRuntime>> {
        Ok(self
            .projects
            .lock()
            .map_err(|_| anyhow::anyhow!("项目注册表锁不可用"))?
            .runtimes())
    }

    /// Open every host-registered project that owns a local `.beaver` store.
    ///
    /// The host can register a project after the desktop runtime has started
    /// (for example, after an import). Scheduler and validation enumerators use
    /// this method before taking a snapshot so newly registered local projects
    /// become visible without creating a second scheduler.
    pub fn open_registered_local(&self) -> Result<Vec<ProjectRuntime>> {
        for project_id in self.registered_project_ids()? {
            if self.registered_project_uses_local_storage(&project_id)? {
                self.open_registered(&project_id)?;
            }
        }
        self.runtimes()
    }

    /// Close open runtimes whose host registration has been removed.
    ///
    /// Task routes of a closed project are dropped so stale IDs cannot resolve to a
    /// runtime that the host no longer knows. A runtime that is still referenced by an
    /// active worker is kept for now and reported; it is retried on the next call once
    /// the references are released. Registered projects are never touched here.
    pub fn close_unregistered(&self) -> Result<UnregisteredClose> {
        let mut projects = self
            .projects
            .lock()
            .map_err(|_| anyhow::anyhow!("项目注册表锁不可用"))?;
        let host = self
            .host
            .lock()
            .map_err(|_| anyhow::anyhow!("宿主数据库锁不可用"))?;
        let registered = host
            .list_with_ids::<Value>("project")?
            .into_iter()
            .map(|(id, _)| id)
            .collect::<std::collections::BTreeSet<_>>();
        let mut index = self
            .task_projects
            .lock()
            .map_err(|_| anyhow::anyhow!("任务路由锁不可用"))?;
        let mut result = UnregisteredClose::default();
        for project_id in projects.project_ids() {
            if registered.contains(&project_id) {
                continue;
            }
            match projects.close(&project_id) {
                Ok(()) => result.closed.push(project_id),
                Err(_) => result.retained.push(project_id),
            }
        }
        index.retain(|_, owner| !result.closed.contains(owner));
        Ok(result)
    }

    /// Read project-owned records for the state response without host fallbacks.
    /// Task autonomy is evaluated against each project's own settings store.
    pub fn open_state_records(&self) -> Result<(Vec<Value>, Vec<Value>)> {
        let mut projects = BTreeMap::new();
        let mut tasks = BTreeMap::new();
        for runtime in self.runtimes()? {
            let store_handle = runtime.store();
            let store = store_handle
                .lock()
                .map_err(|_| anyhow::anyhow!("项目数据库锁不可用"))?;
            for (key, project) in store.list_with_ids::<Value>("project")? {
                let project_id = project["id"]
                    .as_str()
                    .context("项目实体缺少有效 ID")?
                    .to_owned();
                ensure!(key == project_id, "项目实体 key 与内部 ID 不一致：{key}");
                ensure!(
                    project_id == runtime.project_id(),
                    "项目实体与打开的项目不一致：{project_id}"
                );
                let duplicate = projects.insert(project_id.clone(), project).is_some();
                ensure!(!duplicate, "项目 ID 重复：{project_id}");
            }
            for (key, mut task) in store.list_with_ids::<Value>("task")? {
                let task_id = task["id"]
                    .as_str()
                    .context("任务实体缺少有效 ID")?
                    .to_owned();
                ensure!(key == task_id, "任务实体 key 与内部 ID 不一致：{key}");
                validate_task(&task_id, runtime.project_id(), &task)?;
                task["effectiveAskRatio"] = json!(crate::autonomy::effective(&store, &task)?);
                let duplicate = tasks.insert(task_id.clone(), task).is_some();
                ensure!(!duplicate, "任务 ID 重复：{task_id}");
            }
        }
        Ok((
            projects.into_values().collect(),
            tasks.into_values().collect(),
        ))
    }

    pub fn index_task(&self, task: &Value) -> Result<()> {
        let task_id = task["id"].as_str().context("任务缺少有效 ID")?;
        let project_id = task["projectId"].as_str().context("任务缺少有效项目 ID")?;
        validate_task(task_id, project_id, task)?;
        self.runtime_for_project(project_id)?;
        let mut index = self
            .task_projects
            .lock()
            .map_err(|_| anyhow::anyhow!("任务路由锁不可用"))?;
        if let Some(owner) = index.get(task_id) {
            ensure!(owner == project_id, "任务 ID 同时归属于多个项目：{task_id}");
        }
        index.insert(task_id.to_owned(), project_id.to_owned());
        Ok(())
    }

    pub fn close(&self, project_id: &str) -> Result<()> {
        self.projects
            .lock()
            .map_err(|_| anyhow::anyhow!("项目注册表锁不可用"))?
            .close(project_id)?;
        self.task_projects
            .lock()
            .map_err(|_| anyhow::anyhow!("任务路由锁不可用"))?
            .retain(|_, owner| owner != project_id);
        Ok(())
    }

    pub fn close_all(&self) -> Result<()> {
        let project_ids = self
            .projects
            .lock()
            .map_err(|_| anyhow::anyhow!("项目注册表锁不可用"))?
            .project_ids();
        let mut first_error = None;
        for project_id in project_ids {
            if let Err(error) = self.close(&project_id) {
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }
        if let Some(error) = first_error {
            return Err(error);
        }
        Ok(())
    }

    pub fn registered_project_ids(&self) -> Result<Vec<String>> {
        let host = self
            .host
            .lock()
            .map_err(|_| anyhow::anyhow!("宿主数据库锁不可用"))?;
        host.list_with_ids::<Value>("project")?
            .into_iter()
            .map(|(key, project)| {
                let id = project["id"].as_str().context("项目登记缺少有效 ID")?;
                ensure!(key == id, "项目登记 key 与内部 ID 不一致：{key}");
                Ok(id.to_owned())
            })
            .collect()
    }

    pub fn registered_root(&self, project_id: &str) -> Result<PathBuf> {
        let host = self
            .host
            .lock()
            .map_err(|_| anyhow::anyhow!("宿主数据库锁不可用"))?;
        let project = host
            .list_with_ids::<Value>("project")?
            .into_iter()
            .find_map(|(key, project)| (key == project_id).then_some(project))
            .with_context(|| format!("宿主未登记项目：{project_id}"))?;
        ensure!(project["id"] == project_id, "项目登记 key 与内部 ID 不一致");
        let path = project["path"].as_str().context("项目登记缺少路径")?;
        let root = PathBuf::from(path);
        ensure!(root.is_absolute(), "项目登记路径必须是绝对路径");
        Ok(root)
    }

    pub fn registered_project_uses_local_storage(&self, project_id: &str) -> Result<bool> {
        let root = layout::root(&self.registered_root(project_id)?)?;
        let control = root.join(layout::CONTROL_DIR);
        match fs::symlink_metadata(&control) {
            Ok(metadata) => {
                ensure!(
                    !crate::files::linked(&metadata) && metadata.is_dir(),
                    "项目 .beaver 必须是实际目录，不允许符号链接或目录联接"
                );
                Ok(true)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error.into()),
        }
    }

    fn read_tasks(&self, runtime: &ProjectRuntime) -> Result<BTreeMap<String, String>> {
        let store_handle = runtime.store();
        let store = store_handle
            .lock()
            .map_err(|_| anyhow::anyhow!("项目数据库锁不可用"))?;
        let mut tasks = BTreeMap::new();
        for (key, task) in store.list_with_ids::<Value>("task")? {
            let task_id = task["id"].as_str().context("任务实体缺少有效 ID")?;
            ensure!(key == task_id, "任务实体 key 与内部 ID 不一致：{key}");
            validate_task(task_id, runtime.project_id(), &task)?;
            ensure!(
                tasks
                    .insert(task_id.to_owned(), runtime.project_id().to_owned())
                    .is_none(),
                "任务 ID 重复：{task_id}"
            );
        }
        Ok(tasks)
    }
}

/// Outcome of [`ProjectStorageRouter::close_unregistered`].
#[derive(Debug, Default, PartialEq, Eq)]
pub struct UnregisteredClose {
    /// Runtimes closed because the host registration disappeared.
    pub closed: Vec<String>,
    /// Unregistered runtimes still held by active references; retried later.
    pub retained: Vec<String>,
}

fn cleanup_failed_open(
    projects: &mut ProjectStores,
    project_id: &str,
    runtime: ProjectRuntime,
    was_open: bool,
) {
    drop(runtime);
    if !was_open {
        let _ = projects.close(project_id);
    }
}

fn validate_task(task_id: &str, project_id: &str, task: &Value) -> Result<()> {
    ensure!(task["id"] == task_id, "任务 ID 与路由不一致：{task_id}");
    ensure!(
        task["projectId"] == project_id,
        "任务项目归属与路由不一致：{task_id}"
    );
    Ok(())
}
