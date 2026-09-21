use crate::setup_runtime;
use anyhow::Result;
use beaver_core::{
    files::Files, journal::Journal, project_runtime::ProjectRuntime,
    project_storage_router::ProjectStorageRouter, scheduler_runtime::TaskRuntime, store::Store,
};

pub(crate) fn recover(store: &mut Store, files: &Files) -> Result<()> {
    Journal::new(store, files).recover()?;
    store.recover_tasks()?;
    beaver_core::validation::repository::recover(store)?;
    setup_runtime::recover(store)
}

pub(crate) fn open_registered(
    router: &ProjectStorageRouter,
    project_id: &str,
) -> Result<ProjectRuntime> {
    router.open_registered_with(project_id, recover)
}

/// Synchronize open runtimes with the host registry: close runtimes whose registration
/// was removed, then open newly registered local projects with recovery.
pub(crate) fn open_registered_local(router: &ProjectStorageRouter) -> Result<Vec<ProjectRuntime>> {
    router.close_unregistered()?;
    for project_id in router.registered_project_ids()? {
        if router.registered_project_uses_local_storage(&project_id)? {
            open_registered(router, &project_id)?;
        }
    }
    router.runtimes()
}

/// Removed registrations may still own workers. Keep their shadow suppression and
/// controls available, but do not reconcile plans or claim new tasks from them.
pub(crate) fn scheduler_runtimes(router: &ProjectStorageRouter) -> Result<Vec<TaskRuntime>> {
    let runtimes = open_registered_local(router)?;
    let registered = router.registered_project_ids()?;
    Ok(runtimes
        .into_iter()
        .map(|runtime| {
            let task_runtime = TaskRuntime::project(
                runtime.project_id().to_owned(),
                runtime.store(),
                runtime.files(),
            )
            .with_work_gate(router.work_gate(runtime.project_id()));
            if registered.iter().any(|id| id == runtime.project_id()) {
                task_runtime
            } else {
                task_runtime.draining()
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::{open_registered, open_registered_local, scheduler_runtimes};
    use beaver_core::{
        project_storage::ProjectStore, project_storage_router::ProjectStorageRouter, store::Store,
        validation,
    };
    use serde_json::{json, Value};
    use std::{
        collections::BTreeMap,
        fs,
        sync::{Arc, Mutex},
    };

    #[test]
    fn imported_project_is_recovered_before_runtime_becomes_visible() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("project");
        let data = temp.path().join("data");
        fs::create_dir(&root)?;
        fs::write(root.join("project.godot"), "config_version=5\n")?;
        let project_id = "imported-project";
        let project = json!({"id":project_id,"name":"Imported","path":root});
        let project_store = ProjectStore::initialize(&root, project_id)?;
        project_store.store().put("project", project_id, &project)?;
        project_store.store().put(
            "task",
            "running-task",
            &json!({"id":"running-task","projectId":project_id,"status":"running"}),
        )?;
        let validation = validation::repository::new_run(
            project_store.store(),
            project_id,
            BTreeMap::new(),
            None,
            Some("running-task".into()),
            None,
        )?;
        drop(project_store);

        let host = Arc::new(Mutex::new(Store::open(&data)?));
        let host_store = host
            .lock()
            .map_err(|_| anyhow::anyhow!("host store lock unavailable"))?;
        let imported =
            beaver_core::projects::import_project(&host_store, &data, &root, &json!({}))?;
        drop(host_store);
        assert_eq!(imported["id"], project_id);
        let router = ProjectStorageRouter::new(host);

        let runtime = open_registered(&router, project_id)?;
        let store_handle = runtime.store();
        let store = store_handle
            .lock()
            .map_err(|_| anyhow::anyhow!("project store lock unavailable"))?;
        let task = store
            .get::<Value>("task", "running-task")?
            .expect("recovered task");
        assert_eq!(task["status"], "interrupted");
        let run = store
            .get::<validation::model::Run>("validationRun", &validation.id)?
            .expect("recovered validation run");
        assert_eq!(run.status, "interrupted");
        assert_eq!(run.verdict, "needsReview");
        drop(store);
        assert_eq!(
            router.runtime_for_task("running-task")?.project_id(),
            project_id
        );
        Ok(())
    }

    #[test]
    fn closed_project_is_recovered_again_before_reopening() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("project");
        let data = temp.path().join("data");
        fs::create_dir(&root)?;
        fs::write(
            root.join("project.godot"),
            "config_version=5
",
        )?;
        let project_id = "reopened-project";
        let project = json!({"id":project_id,"name":"Reopened","path":root});
        let project_store = ProjectStore::initialize(&root, project_id)?;
        project_store.store().put("project", project_id, &project)?;
        drop(project_store);
        let host = Arc::new(Mutex::new(Store::open(&data)?));
        host.lock().unwrap().put("project", project_id, &project)?;
        let router = ProjectStorageRouter::new(host);

        let runtime = open_registered(&router, project_id)?;
        runtime.store().lock().unwrap().put(
            "task",
            "later-task",
            &json!({"id":"later-task","projectId":project_id,"status":"running"}),
        )?;
        router.close(project_id).unwrap_err();
        drop(runtime);
        router.close(project_id)?;
        assert!(router.runtime_for_task("later-task").is_err());
        assert!(router.runtimes()?.is_empty());

        let runtimes = open_registered_local(&router)?;
        assert_eq!(runtimes.len(), 1);
        let task = runtimes[0]
            .store()
            .lock()
            .unwrap()
            .get::<Value>("task", "later-task")?
            .expect("task survives close and reopen");
        assert_eq!(task["status"], "interrupted");
        assert_eq!(
            router.runtime_for_task("later-task")?.project_id(),
            project_id
        );
        drop(runtimes);
        let again = open_registered_local(&router)?;
        assert_eq!(again.len(), 1);
        Ok(())
    }

    #[test]
    fn unregistered_project_disappears_from_enumeration_and_task_routes() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("project");
        let data = temp.path().join("data");
        fs::create_dir(&root)?;
        fs::write(
            root.join("project.godot"),
            "config_version=5
",
        )?;
        let project_id = "removed-project";
        let project = json!({"id":project_id,"name":"Removed","path":root});
        let project_store = ProjectStore::initialize(&root, project_id)?;
        project_store.store().put("project", project_id, &project)?;
        project_store.store().put(
            "task",
            "orphan-task",
            &json!({"id":"orphan-task","projectId":project_id,"status":"queued"}),
        )?;
        drop(project_store);
        let host = Arc::new(Mutex::new(Store::open(&data)?));
        host.lock().unwrap().put("project", project_id, &project)?;
        let router = ProjectStorageRouter::new(host.clone());
        assert_eq!(open_registered_local(&router)?.len(), 1);
        assert!(router.runtime_for_task("orphan-task").is_ok());

        host.lock().unwrap().remove("project", project_id)?;
        assert!(open_registered_local(&router)?.is_empty());
        assert!(router.runtime_for_task("orphan-task").is_err());
        assert!(router.runtime_for_project(project_id).is_err());
        assert!(
            root.join(".beaver").is_dir(),
            "unregistering never deletes project-local storage"
        );
        Ok(())
    }

    #[test]
    fn unregistered_active_runtime_drains_until_released_or_registered_again() -> anyhow::Result<()>
    {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("project");
        fs::create_dir(&root)?;
        fs::write(root.join("project.godot"), "config_version=5\n")?;
        let project = json!({"id":"active-project","path":root});
        let local = ProjectStore::initialize(&root, "active-project")?;
        local.store().put("project", "active-project", &project)?;
        drop(local);
        let host = Arc::new(Mutex::new(Store::open(&temp.path().join("host"))?));
        host.lock()
            .unwrap()
            .put("project", "active-project", &project)?;
        let router = ProjectStorageRouter::new(host.clone());
        let runtime = open_registered(&router, "active-project")?;
        runtime.store().lock().unwrap().put(
            "task",
            "running",
            &json!({
                "id":"running","projectId":"active-project","status":"running"
            }),
        )?;
        assert!(!scheduler_runtimes(&router)?[0].is_draining());

        host.lock().unwrap().remove("project", "active-project")?;
        let draining = scheduler_runtimes(&router)?;
        assert_eq!(draining.len(), 1);
        assert!(draining[0].is_draining());
        assert!(Arc::ptr_eq(&draining[0].store(), &runtime.store()));
        host.lock()
            .unwrap()
            .put("project", "active-project", &project)?;
        assert!(!scheduler_runtimes(&router)?[0].is_draining());
        assert_eq!(
            runtime
                .store()
                .lock()
                .unwrap()
                .get::<Value>("task", "running")?
                .unwrap()["status"],
            "running"
        );

        host.lock().unwrap().remove("project", "active-project")?;
        drop(draining);
        drop(runtime);
        assert!(scheduler_runtimes(&router)?.is_empty());
        assert!(router.runtime_for_task("running").is_err());
        assert!(root.join(".beaver").is_dir());
        Ok(())
    }
}
