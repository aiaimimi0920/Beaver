//! Ephemeral viewer ownership; no task or acceptance state is mutated.
use super::{
    live_preview_worker, operations, repository,
    service::{State, ToolContext},
    settings,
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::JoinHandle,
    time::Instant,
};

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Camera {
    pub yaw: f64,
    pub pitch: f64,
    pub pan_x: f64,
    pub pan_y: f64,
    pub zoom: f64,
}
impl Camera {
    fn validate(&self) -> Result<()> {
        ensure!(
            [self.yaw, self.pitch, self.pan_x, self.pan_y, self.zoom]
                .iter()
                .all(|v| v.is_finite()),
            "PREVIEW_CAMERA_INVALID"
        );
        ensure!(
            self.yaw.abs() <= 360.0
                && self.pitch.abs() <= 85.0
                && self.pan_x.abs() <= 10.0
                && self.pan_y.abs() <= 10.0
                && self.zoom.abs() <= 4.0,
            "PREVIEW_CAMERA_OUT_OF_RANGE"
        );
        Ok(())
    }
}
pub(super) struct View {
    pub touched: Instant,
    pub revision: u64,
    pub frozen: bool,
    pub camera: Camera,
    pub width: u32,
    pub height: u32,
    pub status: String,
    pub error: Option<String>,
    pub frame: Value,
    pub pick_request: Value,
}
struct Session {
    id: String,
    project: String,
    run: String,
    request: String,
    snapshot: String,
    stop: Arc<AtomicBool>,
    view: Arc<Mutex<View>>,
    worker: Option<JoinHandle<()>>,
    close_error: Option<String>,
}

impl Session {
    fn confirm_finished(&mut self) -> Result<bool> {
        if let Some(error) = &self.close_error {
            anyhow::bail!("{error}");
        }
        if self
            .worker
            .as_ref()
            .is_some_and(|worker| !worker.is_finished())
        {
            return Ok(false);
        }
        if let Some(worker) = self.worker.take() {
            if worker.join().is_err() {
                self.close_error = Some("PREVIEW_WORKER_FAILED".into());
                anyhow::bail!("PREVIEW_WORKER_FAILED");
            }
        }
        Ok(true)
    }
}
#[derive(Default)]
pub(super) struct Sessions(Mutex<Option<Session>>);

impl Sessions {
    pub fn call(&self, state: &State, method: &str, input: &Value) -> Result<Value> {
        let project = operations::string(input, "projectId")?;
        if method == "validation.preview.close" {
            return self.close(project, operations::string(input, "sessionId")?);
        }
        if matches!(
            method,
            "validation.preview.capture" | "validation.preview.saved"
        ) {
            if let Some(result) = super::preview_frames::call(state, method, input, None)? {
                return Ok(result);
            }
        }
        let mut slot = self
            .0
            .lock()
            .map_err(|_| anyhow::anyhow!("Preview lock unavailable"))?;
        if method == "validation.preview.open" {
            let run_id = operations::string(input, "runId")?;
            let request = operations::string(input, "requestId")?;
            ensure!(!request.is_empty(), "PREVIEW_REQUEST_REQUIRED");
            if let Some(session) = slot.as_mut() {
                if session.project == project && session.run == run_id && session.request == request
                {
                    return response(session, true);
                }
                ensure!(session.confirm_finished()?, "PREVIEW_SESSION_LIMIT");
            }
            let storage = (state.storage)(project)?;
            ensure!(!storage.draining, "PROJECT_UNAVAILABLE");
            let permit = storage
                .work_gate
                .acquire()?
                .context("PROJECT_UNAVAILABLE")?;
            let (run, context) = {
                let store = storage
                    .store
                    .lock()
                    .map_err(|_| anyhow::anyhow!("Store unavailable"))?;
                let run = operations::owned_run(&store, input)?;
                ensure!(
                    run.kind == "objectPreview" && run.status == "completed",
                    "PREVIEW_REQUIRES_COMPLETED_CAPTURE"
                );
                super::evidence::validate_capture(&storage.files, &run)?;
                let context = ToolContext {
                    engine: super::blender_preview::engine(&run),
                    project: repository::project(&store, project)?,
                    settings: settings::read(&store, project)?,
                };
                (run, context)
            };
            drop(permit);
            let tools = (state.resolve)(&context)?;
            let config = &run
                .flow
                .as_ref()
                .context("PREVIEW_FLOW_MISSING")?
                .definition
                .config;
            let stop = Arc::new(AtomicBool::new(false));
            let view = Arc::new(Mutex::new(View {
                touched: Instant::now(),
                revision: 0,
                frozen: false,
                camera: Camera::default(),
                width: config.width,
                height: config.height,
                status: "starting".into(),
                error: None,
                frame: Value::Null,
                pick_request: Value::Null,
            }));
            let id = repository::id();
            let session_id = id.clone();
            let worker_view = view.clone();
            let worker_stop = stop.clone();
            let snapshot = run.snapshot_id.clone();
            let worker = std::thread::Builder::new()
                .name("scene-preview".into())
                .spawn(move || {
                    let result = live_preview_worker::execute(
                        &storage.files,
                        &tools.engine,
                        &run,
                        &session_id,
                        &worker_stop,
                        &worker_view,
                    );
                    if let Ok(mut view) = worker_view.lock() {
                        view.status = "closed".into();
                        if let Err(error) = result {
                            if !worker_stop.load(Ordering::SeqCst) {
                                view.error = Some(error.to_string());
                            }
                        }
                    }
                })?;
            *slot = Some(Session {
                id,
                project: project.into(),
                run: run_id.into(),
                request: request.into(),
                snapshot,
                stop,
                view,
                worker: Some(worker),
                close_error: None,
            });
            return response(slot.as_ref().unwrap(), true);
        }
        let id = operations::string(input, "sessionId")?;
        let session = slot.as_ref().context("PREVIEW_SESSION_NOT_FOUND")?;
        ensure!(
            session.id == id && session.project == project,
            "PREVIEW_SESSION_MISMATCH"
        );
        match method {
            "validation.preview.pick" => {
                ensure!(
                    !session.stop.load(Ordering::SeqCst),
                    "PREVIEW_SESSION_CLOSED"
                );
                let mut view = session
                    .view
                    .lock()
                    .map_err(|_| anyhow::anyhow!("Preview view unavailable"))?;
                super::live_preview_pick::request(&mut view, input)
            }
            "validation.preview.capture" => {
                ensure!(input["runId"] == session.run, "PREVIEW_SESSION_MISMATCH");
                let view = session
                    .view
                    .lock()
                    .map_err(|_| anyhow::anyhow!("Preview view unavailable"))?;
                ensure!(
                    view.status == "ready" && !session.stop.load(Ordering::SeqCst),
                    "PREVIEW_SESSION_NOT_READY"
                );
                ensure!(
                    view.frame["revision"] == view.revision,
                    "PREVIEW_VIEW_PENDING"
                );
                ensure!(
                    input["revision"] == view.revision,
                    "PREVIEW_REVISION_CONFLICT"
                );
                let frame = view.frame.clone();
                drop(view);
                super::preview_frames::call(state, method, input, Some(frame))?
                    .context("PREVIEW_SAVE_FAILED")
            }
            "validation.preview.read" => response(session, true),
            "validation.preview.view" => {
                let revision = input["revision"]
                    .as_u64()
                    .context("PREVIEW_REVISION_REQUIRED")?;
                let camera: Camera = serde_json::from_value(input["camera"].clone())?;
                let frozen = match input.get("frozen") {
                    Some(value) => value.as_bool().context("PREVIEW_FREEZE_INVALID")?,
                    None => false,
                };
                camera.validate()?;
                let width = input["width"]
                    .as_u64()
                    .context("PREVIEW_RESOLUTION_REQUIRED")?;
                let height = input["height"]
                    .as_u64()
                    .context("PREVIEW_RESOLUTION_REQUIRED")?;
                let mut view = session
                    .view
                    .lock()
                    .map_err(|_| anyhow::anyhow!("Preview view unavailable"))?;
                ensure!(
                    matches!((width, height), (960, 540) | (1280, 720) | (1920, 1080))
                        || (width == u64::from(view.width) && height == u64::from(view.height)),
                    "PREVIEW_RESOLUTION_INVALID"
                );
                ensure!(
                    view.status != "closed" && !session.stop.load(Ordering::SeqCst),
                    "PREVIEW_SESSION_CLOSED"
                );
                ensure!(
                    revision > view.revision
                        || (revision == view.revision
                            && camera == view.camera
                            && frozen == view.frozen
                            && width == u64::from(view.width)
                            && height == u64::from(view.height)),
                    "PREVIEW_REVISION_CONFLICT"
                );
                if revision != view.revision {
                    view.pick_request = Value::Null;
                }
                view.revision = revision;
                view.frozen = frozen;
                view.camera = camera;
                view.width = width as u32;
                view.height = height as u32;
                Ok(json!({"revision":revision}))
            }
            _ => anyhow::bail!("Unknown preview operation"),
        }
    }
    fn close(&self, project: &str, id: &str) -> Result<Value> {
        let mut slot = self
            .0
            .lock()
            .map_err(|_| anyhow::anyhow!("Preview lock unavailable"))?;
        let closed = match slot.as_mut().filter(|session| session.id == id) {
            Some(session) => {
                ensure!(session.project == project, "PREVIEW_SESSION_MISMATCH");
                session.stop.store(true, Ordering::SeqCst);
                session.confirm_finished()?
            }
            // An old retry cannot stop the replacement session.
            None => true,
        };
        Ok(json!({"closed":closed,"projectId":project,"sessionId":id}))
    }
    pub fn shutdown(&self) -> Result<()> {
        let mut slot = self
            .0
            .lock()
            .map_err(|_| anyhow::anyhow!("Preview lock unavailable"))?;
        if let Some(mut session) = slot.take() {
            session.stop.store(true, Ordering::SeqCst);
            if let Some(worker) = session.worker.take() {
                worker
                    .join()
                    .map_err(|_| anyhow::anyhow!("PREVIEW_WORKER_FAILED"))?;
            }
            if let Some(error) = session.close_error {
                anyhow::bail!("{error}");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "live_preview_close_tests.rs"]
mod close_tests;
fn response(session: &Session, touch: bool) -> Result<Value> {
    let mut view = session
        .view
        .lock()
        .map_err(|_| anyhow::anyhow!("Preview view unavailable"))?;
    if touch {
        view.touched = Instant::now();
    }
    Ok(
        json!({"sessionId":session.id,"projectId":session.project,"runId":session.run,
        "snapshotId":session.snapshot,"status":view.status,"error":view.error,"frame":view.frame}),
    )
}
