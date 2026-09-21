use anyhow::Result;
use beaver_core::{
    autonomy, preferences, project_storage_router::ProjectStorageRouter, store::Store,
};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
};

pub(crate) fn state_operation(
    host_store: Arc<Mutex<Store>>,
    router: &ProjectStorageRouter,
) -> Result<Value> {
    let (runtime_projects, runtime_tasks) = router.open_state_records()?;
    let (host_projects, host_tasks, settings) = {
        let store = host_store
            .lock()
            .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
        let mut tasks = store.list::<Value>("task")?;
        for task in &mut tasks {
            task["effectiveAskRatio"] = json!(autonomy::effective(&store, task)?);
        }
        let settings = preferences::read(
            &store,
            serde_json::from_str(include_str!("../../../dist-native/default-settings.json"))?,
        )?;
        (store.list::<Value>("project")?, tasks, settings)
    };
    let local_projects = local_project_ids(router, &host_projects)?;
    let host_tasks = host_tasks
        .into_iter()
        .filter(|task| {
            task["projectId"]
                .as_str()
                .is_none_or(|id| !local_projects.contains(id))
        })
        .collect();
    let projects = merge_records(host_projects, runtime_projects);
    let mut tasks = merge_records(host_tasks, runtime_tasks);
    let deliveries = projects
        .iter()
        .filter_map(|project| {
            project["id"]
                .as_str()
                .map(|id| (id.to_owned(), project["delivery"].clone()))
        })
        .collect::<BTreeMap<_, _>>();
    for task in &mut tasks {
        task["delivery"] = task["projectId"]
            .as_str()
            .and_then(|id| deliveries.get(id).cloned())
            .unwrap_or(Value::Null);
    }
    Ok(json!({
        "projects": projects,
        "tasks": tasks,
        "settings": settings,
        "features": serde_json::from_str::<Value>(include_str!("../../../dist-native/features.json"))?
    }))
}

/// Registered projects whose task records are owned by project storage.
///
/// Open runtimes and registered projects with an existing `.beaver` directory own their
/// tasks. Host shadow tasks of such projects are never returned, even when the runtime is
/// closed; reading state must not open a runtime and bypass the recovery order. Legacy
/// projects without local storage, or registrations the router cannot resolve, keep their
/// host records.
fn local_project_ids(
    router: &ProjectStorageRouter,
    host_projects: &[Value],
) -> Result<BTreeSet<String>> {
    let mut ids = router
        .runtimes()?
        .iter()
        .map(|runtime| runtime.project_id().to_owned())
        .collect::<BTreeSet<_>>();
    for project in host_projects {
        let Some(id) = project["id"].as_str() else {
            continue;
        };
        if ids.contains(id) {
            continue;
        }
        if router
            .registered_project_uses_local_storage(id)
            .unwrap_or(false)
        {
            ids.insert(id.to_owned());
        }
    }
    Ok(ids)
}

fn merge_records(mut host: Vec<Value>, runtime: Vec<Value>) -> Vec<Value> {
    let positions = host
        .iter()
        .enumerate()
        .filter_map(|(index, record)| record["id"].as_str().map(|id| (id.to_owned(), index)))
        .collect::<BTreeMap<_, _>>();
    let mut additions = BTreeMap::new();
    for record in runtime {
        if let Some(id) = record["id"].as_str() {
            if let Some(index) = positions.get(id) {
                host[*index] = record;
            } else {
                additions.insert(id.to_owned(), record);
            }
        }
    }
    host.extend(additions.into_values());
    host
}
