use crate::{resources, runtime, Host};
use anyhow::{bail, Context, Result};
use beaver_core::{journal::Journal, project_runtime::ProjectRuntime, workflows};
use serde_json::{json, Value};

pub fn call(host: &Host, method: &str, mut input: Value) -> Result<Value> {
    let options = if method == "project.create" {
        input
            .as_object_mut()
            .context("Invalid project input")?
            .remove("npr")
            .map(serde_json::from_value::<workflows::NprOptions>)
            .transpose()?
    } else if method == "project.npr.install" {
        Some(workflows::NprOptions {
            godot: input["godot"].as_str().unwrap().into(),
        })
    } else {
        None
    };
    if let Some(options) = &options {
        workflows::engine_path(&options.godot)?;
    }
    let (mut project, runtime) = if method == "project.create" {
        let project = {
            let store = host
                .store
                .lock()
                .map_err(|_| anyhow::anyhow!("Host store lock unavailable"))?;
            beaver_core::projects::create_project(
                &store,
                &host.root,
                input.clone(),
                &resources::designs()?,
                &resources::blueprints()?,
                &resources::templates()?,
            )?
        };
        let runtime = runtime::open(
            &host.router,
            project["id"].as_str().context("Invalid project ID")?,
        )?;
        (project, runtime)
    } else {
        load(host, input["id"].as_str().unwrap(), method)?
    };
    if let Some(options) = options {
        project["npr"] = json!({"status":"installing", "godot":options.godot});
        persist(&runtime, &project)?;
        let result = workflows::install(runtime.project_root(), &options.godot, &host.closing);
        project["npr"] = match &result {
            Ok(config) => json!({"status":"ready", "runtime":config}),
            Err(error) => {
                json!({"status":"failed", "godot":options.godot, "error":error.to_string()})
            }
        };
        persist(&runtime, &project)?;
        if let Err(error) = result {
            bail!(
                "Project {} preserved; NPR installation failed: {error}",
                project["id"]
            );
        }
    }
    if method.starts_with("project.") {
        return Ok(project);
    }
    if method == "workflow.list" {
        return workflows::list(runtime.project_root());
    }
    input
        .as_object_mut()
        .context("Invalid workflow input")?
        .remove("id");
    workflows::run(runtime.project_root(), input, &host.closing)
}

fn load(host: &Host, id: &str, method: &str) -> Result<(Value, ProjectRuntime)> {
    let runtime = host.router.runtime_for_project(id)?;
    let handle = runtime.store();
    let files = runtime.files();
    let mut store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("Project store lock unavailable"))?;
    let project = store.get("project", id)?.context("Project not found")?;
    if Journal::new(&mut store, &files).blocked(id)? {
        bail!("Project recovery pending");
    }
    if method != "workflow.list"
        && store.list::<Value>("task")?.iter().any(|task| {
            task["projectId"] == id
                && matches!(
                    task["status"].as_str(),
                    Some("running" | "queued" | "waitingChildren")
                )
        })
    {
        bail!("Use task-scoped workflow MCP while creation is active");
    }
    drop(store);
    Ok((project, runtime))
}

fn persist(runtime: &ProjectRuntime, project: &Value) -> Result<()> {
    let handle = runtime.store();
    let store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("Project store lock unavailable"))?;
    store.put("project", runtime.project_id(), project)
}
