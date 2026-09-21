use crate::{project_runtime_lifecycle, Backend};
use anyhow::{ensure, Context, Result};
use beaver_core::{
    files::Files, project_runtime::ProjectRuntime, project_storage_router::ProjectStorageRouter,
    store::Store,
};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

pub(crate) struct GameStorage {
    pub(crate) project: Value,
    pub(crate) store: Arc<Mutex<Store>>,
    pub(crate) files: Arc<Files>,
    pub(crate) root: PathBuf,
    pub(crate) scratch: PathBuf,
    pub(crate) project_id: String,
    _runtime: Option<ProjectRuntime>,
}

impl GameStorage {
    pub(crate) fn for_project(backend: &Backend, project_id: &str) -> Result<Self> {
        if let Some((project, root, runtime)) =
            opened_runtime_project(&backend.project_storage, project_id)?
        {
            return Self::from_parts(backend, project_id.to_owned(), project, root, Some(runtime));
        }
        let registered = host_project(&backend.store, project_id)?;
        let root = project_root(&registered)?;
        Self::from_registered(backend, project_id, registered, root)
    }

    pub(crate) fn for_export_path(backend: &Backend, export_path: &str) -> Result<Option<Self>> {
        let Some(selection) =
            select_export_storage(&backend.project_storage, backend.store.clone(), export_path)?
        else {
            return Ok(None);
        };
        Ok(Some(Self::from_parts(
            backend,
            selection.project_id,
            selection.project,
            selection.root,
            selection.runtime,
        )?))
    }

    fn from_registered(
        backend: &Backend,
        project_id: &str,
        registered: Value,
        root: PathBuf,
    ) -> Result<Self> {
        let local = backend
            .project_storage
            .registered_project_uses_local_storage(project_id)?;
        let runtime = local.then(|| {
            project_runtime_lifecycle::open_registered(&backend.project_storage, project_id)
        });
        let runtime = runtime.transpose()?;
        let (project, root) = if let Some(runtime) = &runtime {
            let project = runtime
                .store()
                .lock()
                .map_err(|_| anyhow::anyhow!("项目数据库锁不可用"))?
                .get::<Value>("project", project_id)?
                .context("项目数据库缺少项目实体")?;
            (project, runtime.project_root().to_path_buf())
        } else {
            (registered, root)
        };
        Self::from_parts(backend, project_id.to_owned(), project, root, runtime)
    }

    fn from_parts(
        backend: &Backend,
        project_id: String,
        project: Value,
        root: PathBuf,
        runtime: Option<ProjectRuntime>,
    ) -> Result<Self> {
        ensure!(project["id"] == project_id, "项目实体与登记 ID 不一致");
        if let Some(runtime) = runtime {
            return Ok(Self {
                project,
                store: runtime.store(),
                files: runtime.files(),
                root,
                scratch: runtime
                    .project_root()
                    .join(beaver_core::project_storage_layout::CONTROL_DIR),
                project_id,
                _runtime: Some(runtime),
            });
        }
        let scratch = backend.root.clone();
        Ok(Self {
            project,
            store: backend.store.clone(),
            files: Arc::new(Files::new(backend.root.clone())),
            root,
            scratch,
            project_id,
            _runtime: None,
        })
    }
}

/// Resolve the project that recorded `export_path` as its delivery without
/// touching a `Backend`, so call logging can bind to the same store that
/// `for_export_path` will later write the verification receipt into.
pub(crate) fn project_id_for_export_path(
    router: &ProjectStorageRouter,
    host_store: Arc<Mutex<Store>>,
    export_path: &str,
) -> Result<Option<String>> {
    Ok(select_export_storage(router, host_store, export_path)?
        .map(|selection| selection.project_id))
}

struct ExportStorageSelection {
    project_id: String,
    project: Value,
    root: PathBuf,
    runtime: Option<ProjectRuntime>,
}

fn select_export_storage(
    router: &beaver_core::project_storage_router::ProjectStorageRouter,
    host_store: Arc<Mutex<Store>>,
    export_path: &str,
) -> Result<Option<ExportStorageSelection>> {
    let requested = Path::new(export_path);
    for project_id in router.registered_project_ids()? {
        if let Some((project, root, runtime)) = opened_runtime_project(router, &project_id)? {
            if project["delivery"]["path"]
                .as_str()
                .is_some_and(|previous| same_path(previous, requested))
            {
                return Ok(Some(ExportStorageSelection {
                    project_id,
                    project,
                    root,
                    runtime: Some(runtime),
                }));
            }
            continue;
        }
        let registered = host_project(&host_store, &project_id)?;
        let registered_root = project_root(&registered)?;
        let local = router.registered_project_uses_local_storage(&project_id)?;
        let (project, root, runtime) = if local {
            let runtime = project_runtime_lifecycle::open_registered(router, &project_id)?;
            let project = runtime
                .store()
                .lock()
                .map_err(|_| anyhow::anyhow!("项目数据库锁不可用"))?
                .get::<Value>("project", &project_id)?
                .context("项目数据库缺少项目实体")?;
            (project, runtime.project_root().to_path_buf(), Some(runtime))
        } else {
            (registered, registered_root, None)
        };
        if project["delivery"]["path"]
            .as_str()
            .is_some_and(|previous| same_path(previous, requested))
        {
            return Ok(Some(ExportStorageSelection {
                project_id,
                project,
                root,
                runtime,
            }));
        }
    }
    Ok(None)
}

fn opened_runtime_project(
    router: &ProjectStorageRouter,
    project_id: &str,
) -> Result<Option<(Value, PathBuf, ProjectRuntime)>> {
    let Some(runtime) = router
        .runtimes()?
        .into_iter()
        .find(|runtime| runtime.project_id() == project_id)
    else {
        return Ok(None);
    };
    let project = runtime
        .store()
        .lock()
        .map_err(|_| anyhow::anyhow!("项目数据库锁不可用"))?
        .get::<Value>("project", project_id)?
        .context("项目数据库缺少项目实体")?;
    Ok(Some((
        project,
        runtime.project_root().to_path_buf(),
        runtime,
    )))
}

fn host_project(host_store: &Arc<Mutex<Store>>, project_id: &str) -> Result<Value> {
    host_store
        .lock()
        .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?
        .get("project", project_id)?
        .with_context(|| format!("项目不存在：{project_id}"))
}

fn project_root(project: &Value) -> Result<PathBuf> {
    let path = project["path"].as_str().context("项目路径无效")?;
    Ok(fs::canonicalize(path).with_context(|| format!("项目路径无效：{path}"))?)
}

fn same_path(left: &str, right: &Path) -> bool {
    left == right.to_string_lossy()
        || fs::canonicalize(left)
            .ok()
            .zip(fs::canonicalize(right).ok())
            .is_some_and(|(a, b)| a == b)
}

#[cfg(test)]
mod tests {
    use super::{opened_runtime_project, select_export_storage};
    use beaver_core::{
        project_storage::ProjectStore, project_storage_router::ProjectStorageRouter, store::Store,
    };
    use serde_json::json;
    use std::{
        fs,
        sync::{Arc, Mutex},
    };

    fn project_root(parent: &std::path::Path, id: &str) -> anyhow::Result<std::path::PathBuf> {
        let root = parent.join(id);
        fs::create_dir(&root)?;
        fs::write(root.join("project.godot"), "config_version=5\n")?;
        Ok(root)
    }

    fn host_store(parent: &std::path::Path) -> anyhow::Result<Arc<Mutex<Store>>> {
        Ok(Arc::new(Mutex::new(Store::open(&parent.join("host"))?)))
    }

    fn register(
        host: &Arc<Mutex<Store>>,
        id: &str,
        root: &std::path::Path,
        delivery: &std::path::Path,
    ) -> anyhow::Result<()> {
        host.lock()
            .map_err(|_| anyhow::anyhow!("host store lock unavailable"))?
            .put(
                "project",
                id,
                &json!({"id":id,"name":id,"path":root,"delivery":{"path":delivery}}),
            )?;
        Ok(())
    }

    #[test]
    fn export_path_selects_the_matching_project_runtime() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let host = host_store(temp.path())?;
        let first_root = project_root(temp.path(), "first")?;
        let second_root = project_root(temp.path(), "second")?;
        let first_delivery = temp.path().join("exports/first");
        let second_delivery = temp.path().join("exports/second");
        for (id, root, delivery) in [
            ("first", &first_root, &first_delivery),
            ("second", &second_root, &second_delivery),
        ] {
            let project = json!({
                "id": id,
                "name": id,
                "path": root,
                "delivery": {"path": delivery}
            });
            let project_store = ProjectStore::initialize(root, id)?;
            project_store.store().put("project", id, &project)?;
            drop(project_store);
            register(&host, id, root, delivery)?;
        }
        let router = ProjectStorageRouter::new(host.clone());

        let selection = select_export_storage(&router, host, &second_delivery.to_string_lossy())?
            .expect("matching export path");
        assert_eq!(selection.project_id, "second");
        assert_eq!(selection.root, fs::canonicalize(&second_root)?);
        assert!(selection.runtime.is_some());
        Ok(())
    }

    #[test]
    fn local_export_selection_uses_runtime_root_and_project_entity() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let host = host_store(temp.path())?;
        let root = project_root(temp.path(), "project-a")?;
        let delivery = temp.path().join("exports/project-a");
        let decoy = temp.path().join("decoy");
        fs::create_dir(&decoy)?;
        let project = json!({
            "id": "project-a",
            "name": "Runtime project",
            "path": decoy,
            "delivery": {"path": delivery}
        });
        let project_store = ProjectStore::initialize(&root, "project-a")?;
        project_store
            .store()
            .put("project", "project-a", &project)?;
        drop(project_store);
        register(&host, "project-a", &root, &delivery)?;
        let router = ProjectStorageRouter::new(host.clone());

        let selection = select_export_storage(&router, host, &delivery.to_string_lossy())?
            .expect("matching local export path");
        assert_eq!(selection.root, fs::canonicalize(&root)?);
        assert_eq!(
            selection.project["path"],
            decoy.to_string_lossy().to_string()
        );
        Ok(())
    }

    #[test]
    fn opened_runtime_ignores_invalid_host_project_path() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let host = host_store(temp.path())?;
        let root = project_root(temp.path(), "project-a")?;
        let project = json!({
            "id": "project-a",
            "name": "Runtime project",
            "path": root,
            "delivery": {"path": temp.path().join("exports/project-a")}
        });
        let project_store = ProjectStore::initialize(&root, "project-a")?;
        project_store
            .store()
            .put("project", "project-a", &project)?;
        drop(project_store);
        register(
            &host,
            "project-a",
            &root,
            &temp.path().join("exports/project-a"),
        )?;
        let router = ProjectStorageRouter::new(host.clone());
        router.open_registered("project-a")?;
        host.lock().unwrap().put(
            "project",
            "project-a",
            &json!({
                "id": "project-a",
                "name": "Stale host record",
                "path": temp.path().join("missing-project"),
                "delivery": {"path": temp.path().join("exports/project-a")}
            }),
        )?;

        let (loaded, resolved_root, _) =
            opened_runtime_project(&router, "project-a")?.expect("open runtime");
        assert_eq!(loaded["name"], "Runtime project");
        assert_eq!(resolved_root, std::fs::canonicalize(root)?);
        Ok(())
    }

    #[test]
    fn legacy_export_selection_keeps_host_storage() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let host = host_store(temp.path())?;
        let root = project_root(temp.path(), "legacy")?;
        let delivery = temp.path().join("exports/legacy");
        register(&host, "legacy", &root, &delivery)?;
        let router = ProjectStorageRouter::new(host.clone());

        let selection = select_export_storage(&router, host, &delivery.to_string_lossy())?
            .expect("matching legacy export path");
        assert_eq!(selection.project_id, "legacy");
        assert_eq!(selection.root, fs::canonicalize(&root)?);
        assert!(selection.runtime.is_none());
        Ok(())
    }

    #[test]
    fn broken_local_runtime_returns_error_instead_of_host_fallback() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let host = host_store(temp.path())?;
        let root = project_root(temp.path(), "project-a")?;
        let delivery = temp.path().join("exports/project-a");
        let project_store = ProjectStore::initialize(&root, "project-a")?;
        project_store.store().put(
            "project",
            "project-a",
            &json!({"id":"project-a","path":root,"delivery":{"path":delivery}}),
        )?;
        drop(project_store);
        register(&host, "project-a", &root, &delivery)?;
        fs::remove_file(root.join(".beaver/project.sqlite"))?;
        let router = ProjectStorageRouter::new(host.clone());

        let error = match select_export_storage(&router, host, &delivery.to_string_lossy()) {
            Ok(_) => panic!("broken local runtime must return an error"),
            Err(error) => error.to_string(),
        };
        assert!(!error.is_empty());
        Ok(())
    }
}
