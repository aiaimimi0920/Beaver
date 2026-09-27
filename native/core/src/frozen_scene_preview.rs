//! Persistent capture scheduling shared by frozen versions and attempt checkpoints.
use crate::{
    files::Snapshot,
    project_runtime::ProjectRuntime,
    store::Store,
    validation::{
        flow::Definition,
        model::{Flow, Run},
        operations, repository,
        requests::Request,
    },
};
use anyhow::{ensure, Result};
use serde_json::{json, Value};

#[derive(Clone, Copy, serde::Deserialize, serde::Serialize)]
pub(crate) enum Resolution {
    #[serde(rename = "540p")]
    Standard,
    #[serde(rename = "720p")]
    Hd,
    #[serde(rename = "1080p")]
    FullHd,
}
impl Resolution {
    fn dimensions(self) -> (u32, u32) {
        match self {
            Self::Standard => (960, 540),
            Self::Hd => (1280, 720),
            Self::FullHd => (1920, 1080),
        }
    }
}
fn dimensions(run: &Run) -> Result<(u32, u32)> {
    let flow = run
        .flow
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("PREVIEW_FLOW_MISSING"))?;
    Ok((flow.definition.config.width, flow.definition.config.height))
}

pub(crate) fn latest(runtime: &ProjectRuntime, target: &Value, key: &str) -> Result<Value> {
    let project = operations::string(target, "projectId")?;
    ensure!(project == runtime.project_id(), "PROJECT_RUNTIME_MISMATCH");
    let handle = runtime.store();
    let store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("Preview store unavailable"))?;
    let Some(record) = store.get::<Value>("objectScenePreview", key)? else {
        return Ok(Value::Null);
    };
    ensure!(record["target"] == *target, "PREVIEW_TARGET_MISMATCH");
    let run: Run = repository::get(
        &store,
        "validationRun",
        operations::string(&record, "runId")?,
    )?;
    ensure!(
        run.project_id == project && run.kind == "objectPreview",
        "PREVIEW_RUN_MISMATCH"
    );
    let integrity_error = if run.status == "completed" {
        crate::validation::evidence::validate_capture(&runtime.files(), &run)
            .err()
            .map(|error| error.to_string())
    } else {
        None
    };
    let (width, height) = dimensions(&run)?;
    Ok(
        json!({"resolution":{"width":width,"height":height},"target":target,"sourceDigest":record["sourceDigest"],
        "projectConfig":record["projectConfig"],"integrityError":integrity_error,"run":operations::public_run(&run)?}),
    )
}

pub(crate) fn enqueue(
    runtime: &ProjectRuntime,
    input: &Value,
    method: &str,
    key: String,
    resolve: impl FnOnce(&Store) -> Result<(Snapshot, String)>,
) -> Result<Value> {
    let target = &input["target"];
    let project = operations::string(target, "projectId")?;
    ensure!(
        operations::string(input, "projectId")? == project && project == runtime.project_id(),
        "PROJECT_RUNTIME_MISMATCH"
    );
    let path = operations::string(target, "path")?;
    ensure!(
        path.ends_with(".tscn") || path.ends_with(".scn") || path.ends_with(".blend"),
        "PREVIEW_REQUIRES_GODOT_OR_BLENDER_SCENE"
    );
    let resolution: Option<Resolution> =
        serde_json::from_value(input.get("resolution").cloned().unwrap_or(Value::Null))?;
    let (width, height) = resolution.unwrap_or(Resolution::Standard).dimensions();
    let request = Request::new(method, input)?;
    let handle = runtime.store();
    let mut store = handle
        .lock()
        .map_err(|_| anyhow::anyhow!("Preview store unavailable"))?;
    if let Some(receipt) = request.replay(&store)? {
        return Ok(receipt);
    }
    if let Some(previous) = store.get::<Value>("objectScenePreview", &key)? {
        let run: Run = repository::get(
            &store,
            "validationRun",
            operations::string(&previous, "runId")?,
        )?;
        ensure!(
            previous["target"] == *target
                && run.project_id == project
                && run.kind == "objectPreview",
            "PREVIEW_RUN_MISMATCH"
        );
        if ["queued", "running"].contains(&run.status.as_str()) {
            ensure!(
                dimensions(&run)? == (width, height),
                "PREVIEW_RESOLUTION_ACTIVE_CONFLICT"
            );
            return request.finish(&mut store, json!({"runId":run.id}), vec![]);
        }
    }
    let (snapshot, source_digest) = resolve(&store)?;
    let project_config = if path.ends_with(".blend") {
        "blender-frozen-v1"
    } else if snapshot.contains_key("project.godot") {
        "frozen"
    } else {
        "minimal-v1"
    };
    let steps = if path.ends_with(".blend") {
        json!([{"id":"preview","kind":"capture"}])
    } else {
        json!([{"id":"settle","kind":"wait","frames":30},{"id":"preview","kind":"capture"}])
    };
    let definition: Definition = serde_json::from_value(json!({
        "key":"object-preview","name":"Frozen object scene","category":"feature",
        "purpose":"Object preview only; not acceptance evidence","entry":path,
        "config":{"width":width,"height":height},
        "steps":steps
    }))?;
    definition.validate()?;
    let flow = Flow {
        id: repository::id(),
        project_id: project.into(),
        revision: 1,
        definition,
        updated_at: repository::now(),
        reason: "Frozen object preview".into(),
    };
    let mut run = repository::build_run(&store, project, snapshot, Some(flow), None, None)?;
    run.kind = "objectPreview".into();
    run.managed = true;
    let record = json!({"target":target,"runId":run.id,"sourceDigest":source_digest,"projectConfig":project_config});
    request.finish(
        &mut store,
        json!({"runId":run.id}),
        vec![
            ("objectScenePreviewSource", run.id.clone(), record.clone()),
            ("objectScenePreview", key, record),
            ("validationRun", run.id.clone(), serde_json::to_value(&run)?),
        ],
    )
}
