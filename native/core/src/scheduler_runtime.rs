use crate::{
    files::Files, project_runtime::ProjectRuntime, project_work_gate::ProjectWorkGate, store::Store,
};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeOwner {
    Project(String),
    Host,
}

#[derive(Clone)]
pub struct TaskRuntime {
    pub(crate) store: Arc<Mutex<Store>>,
    pub(crate) files: Arc<Files>,
    pub(crate) owner: RuntimeOwner,
    pub(crate) draining: bool,
    pub(crate) work_gate: ProjectWorkGate,
    pub(crate) project: Option<ProjectRuntime>,
}

pub type RuntimeSource = Arc<dyn Fn() -> Result<Vec<TaskRuntime>, String> + Send + Sync>;

impl TaskRuntime {
    pub fn host(store: Arc<Mutex<Store>>, files: Arc<Files>) -> Self {
        Self {
            store,
            files,
            owner: RuntimeOwner::Host,
            draining: false,
            work_gate: ProjectWorkGate::default(),
            project: None,
        }
    }

    pub fn project(
        project_id: impl Into<String>,
        store: Arc<Mutex<Store>>,
        files: Arc<Files>,
    ) -> Self {
        Self {
            store,
            files,
            owner: RuntimeOwner::Project(project_id.into()),
            draining: false,
            work_gate: ProjectWorkGate::default(),
            project: None,
        }
    }

    pub fn store(&self) -> Arc<Mutex<Store>> {
        self.store.clone()
    }

    pub fn from_project(runtime: ProjectRuntime) -> Self {
        let mut task_runtime =
            Self::project(runtime.project_id(), runtime.store(), runtime.files());
        task_runtime.project = Some(runtime);
        task_runtime
    }

    pub fn with_work_gate(mut self, gate: ProjectWorkGate) -> Self {
        self.work_gate = gate;
        self
    }

    /// Retain ownership and active-task controls without starting more queued work.
    pub fn draining(mut self) -> Self {
        self.draining = true;
        self
    }

    pub fn is_draining(&self) -> bool {
        self.draining
    }

    pub fn files(&self) -> Arc<Files> {
        self.files.clone()
    }

    pub fn owner(&self) -> &RuntimeOwner {
        &self.owner
    }
}
