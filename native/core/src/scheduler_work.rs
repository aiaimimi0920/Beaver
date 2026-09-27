//! Typed dispatch keeps object attempts out of legacy task validation and merge.
use crate::{
    executor::{Control, Outcome},
    object_attempt_launch::Factory as ObjectFactory,
    object_run_preparation::{self, PreparationClaim},
    scheduler::{ParallelLimit, RuntimeFactory},
    scheduler_runtime::TaskRuntime,
};
use serde_json::Value;
use std::{
    collections::HashSet,
    sync::{atomic::AtomicBool, Arc},
};
use tokio::sync::mpsc;

pub(crate) enum Work {
    Legacy(Value),
    Object(PreparationClaim),
}

pub(crate) struct Claimed {
    pub work: Work,
    pub runtime: TaskRuntime,
}

pub(crate) struct Batch {
    pub claimed: Vec<Claimed>,
    pub failure: Option<String>,
}

pub(crate) enum Finished {
    Legacy(Outcome),
    Object(Result<(), String>),
}

pub(crate) fn object_key(project: &str, medium: &str) -> String {
    format!("object:{}:{project}{medium}", project.len())
}

impl Work {
    pub(crate) fn id(&self) -> String {
        match self {
            Self::Legacy(task) => task["id"].as_str().unwrap_or_default().into(),
            Self::Object(claim) => {
                object_key(&claim.record().project_id, &claim.record().medium.id)
            }
        }
    }
}

pub(crate) fn claim(
    runtimes: &[TaskRuntime],
    active: usize,
    parallel_limit: &ParallelLimit,
    objects_enabled: bool,
    owner: &str,
) -> Result<Batch, String> {
    let (legacy, mut remaining) =
        crate::scheduler_runtime_ops::claim_batch(runtimes, active, parallel_limit)?;
    let mut claimed: Vec<_> = legacy
        .into_iter()
        .map(|task| Claimed {
            work: Work::Legacy(task.task),
            runtime: task.runtime,
        })
        .collect();
    if !objects_enabled {
        return Ok(Batch {
            claimed,
            failure: None,
        });
    }
    // A later project error must not discard capabilities already claimed.
    // The scheduler takes ownership of this batch before draining on failure.
    let result = (|| -> Result<(), String> {
        let mut visited = HashSet::new();
        for runtime in runtimes {
            if runtime.draining || remaining == 0 {
                continue;
            }
            let Some(project) = &runtime.project else {
                continue;
            };
            if !visited.insert(project.project_id()) {
                continue;
            }
            let Some(_permit) = runtime
                .work_gate
                .acquire()
                .map_err(|error| error.to_string())?
            else {
                continue;
            };
            while remaining > 0 {
                let Some(preparation) =
                    object_run_preparation::claim_next(project, project.project_id(), owner)
                        .map_err(|error| error.to_string())?
                else {
                    break;
                };
                claimed.push(Claimed {
                    work: Work::Object(preparation),
                    runtime: runtime.clone(),
                });
                remaining -= 1;
            }
        }
        Ok(())
    })();
    Ok(Batch {
        claimed,
        failure: result.err(),
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn execute(
    claimed: Claimed,
    legacy: RuntimeFactory,
    objects: Option<ObjectFactory>,
    sessions: crate::asset_sessions::Sessions,
    cancelled: Arc<AtomicBool>,
    input: mpsc::Receiver<Control>,
    changed: Arc<dyn Fn() + Send + Sync>,
) -> (String, Finished) {
    let id = claimed.work.id();
    let result = match claimed.work {
        Work::Legacy(task) => Finished::Legacy(
            crate::scheduler_worker::execute(
                task,
                claimed.runtime,
                legacy,
                sessions,
                cancelled,
                input,
                changed,
            )
            .await
            .1,
        ),
        Work::Object(claim) => {
            let result = match (claimed.runtime.project, objects) {
                (Some(runtime), Some(factory)) => {
                    crate::object_attempt_worker::execute(
                        runtime, claim, factory, cancelled, input, changed,
                    )
                    .await
                }
                _ => Err("OBJECT_ATTEMPT_RUNTIME_UNAVAILABLE".into()),
            };
            Finished::Object(result)
        }
    };
    (id, result)
}
