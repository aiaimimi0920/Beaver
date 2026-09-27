//! Reserve scheduler capacity before scanning; receipt delivery never owns the worker.
use super::{Active, Message, Scheduler};
use crate::{
    object_run_recovery::resume,
    project_runtime::ProjectRuntime,
    scheduler_runtime::{RuntimeSource, TaskRuntime},
    scheduler_work::Finished,
};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinSet,
};

type Reply = oneshot::Sender<Result<resume::Operation, String>>;
pub(super) struct Request(pub ProjectRuntime, pub resume::Request, pub Reply);

impl Scheduler {
    pub async fn resume_object_recovery(
        &self,
        runtime: ProjectRuntime,
        request: resume::Request,
    ) -> Result<resume::Operation, String> {
        let (reply, receiver) = oneshot::channel();
        if self
            .sender
            .send(Message::Resume(Request(
                runtime.clone(),
                request.clone(),
                reply,
            )))
            .is_err()
        {
            return resume::replay(&runtime, &request)
                .map_err(|error| error.to_string())?
                .ok_or_else(|| "OBJECT_RECOVERY_SCHEDULER_STOPPED".into());
        }
        receiver
            .await
            .map_err(|_| "OBJECT_RECOVERY_CONTROL_UNAVAILABLE".to_string())?
    }
}

pub(super) struct Context<'a> {
    pub active: &'a mut HashMap<String, Active>,
    pub jobs: &'a mut JoinSet<(String, Finished)>,
    pub runtimes: &'a RuntimeSource,
    pub objects: &'a Option<crate::object_attempt_launch::Factory>,
    pub limit: &'a super::ParallelLimit,
    pub closing: &'a AtomicBool,
    pub changed: &'a Arc<dyn Fn() + Send + Sync>,
}

fn runtime(
    context: &Context<'_>,
    project: &ProjectRuntime,
    request: &resume::Request,
) -> Result<TaskRuntime, String> {
    if context.closing.load(Ordering::SeqCst) {
        return Err("OBJECT_RECOVERY_SCHEDULER_STOPPED".into());
    }
    if context.objects.is_none() {
        return Err("OBJECT_ATTEMPT_EXECUTOR_UNAVAILABLE".into());
    }
    if context.active.contains_key(&request.target.task_id) {
        return Err("OBJECT_RECOVERY_WRITER_ACTIVE".into());
    }
    if context.active.len() >= (context.limit)()? {
        return Err("OBJECT_RECOVERY_CAPACITY_UNAVAILABLE".into());
    }
    (context.runtimes)()?
        .into_iter()
        .find(|runtime| {
            !runtime.draining
                && runtime
                    .project
                    .as_ref()
                    .is_some_and(|candidate| candidate.project_id() == project.project_id())
                && Arc::ptr_eq(&runtime.store, &project.store())
        })
        .ok_or_else(|| "OBJECT_RECOVERY_PROJECT_UNAVAILABLE".into())
}

pub(super) fn handle(Request(project, request, reply): Request, context: Context<'_>) {
    match resume::replay(&project, &request) {
        Ok(Some(operation)) => {
            let _ = reply.send(Ok(operation));
            return;
        }
        Err(error) => {
            let _ = reply.send(Err(error.to_string()));
            return;
        }
        Ok(None) => {}
    }
    let selected = runtime(&context, &project, &request);
    let selected = match selected {
        Ok(runtime) => runtime,
        Err(error) => {
            let _ = reply.send(Err(error));
            return;
        }
    };
    let prepared = project
        .store()
        .lock()
        .map_err(|_| "OBJECT_STORE_UNAVAILABLE".to_string())
        .and_then(|store| {
            crate::object_run_preparation::read_record(
                &store.connection,
                project.project_id(),
                &request.target.run_id,
            )
            .map_err(|error| error.to_string())
            .and_then(|record| record.ok_or_else(|| "OBJECT_RUN_PREPARATION_MISSING".into()))
        });
    let prepared = match prepared {
        Ok(value) => value,
        Err(error) => {
            let _ = reply.send(Err(error));
            return;
        }
    };
    let id = request.target.task_id.clone();
    let (controls, input) = mpsc::channel(16);
    let cancelled = Arc::new(AtomicBool::new(false));
    context.active.insert(
        id.clone(),
        Active {
            runtime: selected.clone(),
            object: Some(prepared),
            worker: None,
            controls,
            cancelled: cancelled.clone(),
            waiters: vec![],
        },
    );
    let work_id = id.clone();
    let factory = context.objects.clone().unwrap();
    let changed = context.changed.clone();
    let worker = context.jobs.spawn(async move {
        let scan_project = project.clone();
        let stop = cancelled.clone();
        let result = tokio::task::spawn_blocking(move || {
            let _permit = selected
                .work_gate
                .acquire()
                .map_err(|error| error.to_string())?
                .ok_or_else(|| "OBJECT_RECOVERY_PROJECT_UNAVAILABLE".to_string())?;
            resume::execute(&scan_project, &request, &stop).map_err(|error| error.to_string())
        })
        .await;
        let (operation, lease) = match result {
            Ok(Ok(value)) => value,
            Ok(Err(error)) => {
                let _ = reply.send(Err(error));
                return (work_id, Finished::Object(Ok(())));
            }
            Err(error) => {
                let _ = reply.send(Err(error.to_string()));
                return (
                    work_id,
                    Finished::Object(Err("OBJECT_RECOVERY_WORKER_FAILED".into())),
                );
            }
        };
        changed();
        let _ = reply.send(Ok(operation));
        let result = if let Some(lease) = lease {
            crate::object_attempt_worker::execute_lease(project, lease, factory, cancelled, input)
                .await
        } else {
            Ok(())
        };
        (work_id, Finished::Object(result))
    });
    context.active.get_mut(&id).unwrap().worker = Some(worker.id());
}
