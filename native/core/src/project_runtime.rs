//! A database and content pair with shared project ownership. No host fallback or activation.
use crate::{files::Files, store::Store};
use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[derive(Clone)]
pub struct ProjectRuntime {
    store: Arc<Mutex<Store>>,
    files: Arc<Files>,
    root: PathBuf,
    project_id: String,
}

impl ProjectRuntime {
    pub(crate) fn new(store: Store, root: PathBuf, project_id: String, lock: Arc<File>) -> Self {
        let files = Arc::new(Files::project(root.clone(), lock));
        Self {
            store: Arc::new(Mutex::new(store)),
            files,
            root,
            project_id,
        }
    }

    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    pub fn project_root(&self) -> &Path {
        &self.root
    }

    pub fn store(&self) -> Arc<Mutex<Store>> {
        self.store.clone()
    }

    /// Project snapshots use .beaver/content/blobs; legacy Files::new remains host-only.
    pub fn files(&self) -> Arc<Files> {
        self.files.clone()
    }

    pub(crate) fn in_use(&self) -> bool {
        Arc::strong_count(&self.store) > 1 || Arc::strong_count(&self.files) > 1
    }
}
