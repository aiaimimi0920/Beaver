//! Typed object controls never fall through to legacy task interruption.
use super::{cancel, Active, Message, Reply, Scheduler};
use crate::{
    object_attempt::State,
    object_attempt_control::{self as control, InterruptRequest, Receipt},
    object_attempt_view::{self, View},
    object_run_recovery::{self as recovery, disposition, Operation, VerifyRequest},
    project_runtime::ProjectRuntime,
    scheduler_work,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::oneshot;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Availability {
    Active,
    RecoveryRequired,
    Finished,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Execution {
    pub attempt: View,
    pub availability: Availability,
    pub definition: object_attempt_view::Definition,
    pub checkpoints: object_attempt_view::Checkpoints,
}

pub(super) enum ObjectMessage {
    Query(
        ProjectRuntime,
        String,
        oneshot::Sender<Result<Vec<Execution>, String>>,
    ),
    Interrupt(ProjectRuntime, InterruptRequest, Reply),
    Verify(
        ProjectRuntime,
        VerifyRequest,
        oneshot::Sender<Result<Operation, String>>,
    ),
    Dispose(
        ProjectRuntime,
        disposition::Request,
        oneshot::Sender<Result<disposition::Operation, String>>,
    ),
}

impl Scheduler {
    pub async fn dispose_object_recovery(
        &self,
        runtime: ProjectRuntime,
        request: disposition::Request,
    ) -> Result<disposition::Operation, String> {
        let (reply, receiver) = oneshot::channel();
        let result = if self
            .sender
            .send(Message::Object(ObjectMessage::Dispose(
                runtime.clone(),
                request.clone(),
                reply,
            )))
            .is_err()
        {
            tokio::task::spawn_blocking(move || {
                disposition::dispose(&runtime, &request, false)
                    .map_err(|error| format!("{error:#}"))
            })
            .await
            .map_err(|error| error.to_string())??
        } else {
            receiver
                .await
                .map_err(|_| "OBJECT_RECOVERY_CONTROL_UNAVAILABLE".to_string())??
        };
        if matches!(
            result.result,
            Some(
                disposition::Outcome::CancelledAndRetained { .. }
                    | disposition::Outcome::CancelledAndWorkspaceRemoved { .. }
            )
        ) {
            // Cancellation commits off-loop. Wake only after durable ownership release.
            let _ = self.wake();
        }
        Ok(result)
    }

    pub async fn verify_object_recovery(
        &self,
        runtime: ProjectRuntime,
        request: VerifyRequest,
    ) -> Result<Operation, String> {
        let (reply, receiver) = oneshot::channel();
        if self
            .sender
            .send(Message::Object(ObjectMessage::Verify(
                runtime.clone(),
                request.clone(),
                reply,
            )))
            .is_err()
        {
            return tokio::task::spawn_blocking(move || {
                recovery::verify(&runtime, &request, false).map_err(|error| format!("{error:#}"))
            })
            .await
            .map_err(|error| error.to_string())?;
        }
        receiver
            .await
            .map_err(|_| "OBJECT_RECOVERY_CONTROL_UNAVAILABLE".to_string())?
    }

    pub async fn object_attempts(
        &self,
        runtime: ProjectRuntime,
        run: String,
    ) -> Result<Vec<Execution>, String> {
        let (reply, receiver) = oneshot::channel();
        if self
            .sender
            .send(Message::Object(ObjectMessage::Query(
                runtime.clone(),
                run.clone(),
                reply,
            )))
            .is_err()
        {
            return query(&runtime, &run, &HashMap::new());
        }
        receiver
            .await
            .unwrap_or_else(|_| query(&runtime, &run, &HashMap::new()))
    }

    pub async fn interrupt_attempt(
        &self,
        runtime: ProjectRuntime,
        request: InterruptRequest,
    ) -> Result<Receipt, String> {
        let (reply, receiver) = oneshot::channel();
        if self
            .sender
            .send(Message::Object(ObjectMessage::Interrupt(
                runtime.clone(),
                request.clone(),
                reply,
            )))
            .is_err()
        {
            control::request(&runtime, &request, false).map_err(|error| error.to_string())?;
        } else {
            receiver
                .await
                .map_err(|_| "OBJECT_ATTEMPT_CONTROL_UNAVAILABLE")??;
        }
        control::result(&runtime, &request).map_err(|error| error.to_string())
    }
}

fn matches(entry: &Active, runtime: &ProjectRuntime, view: &View) -> bool {
    Arc::ptr_eq(&entry.runtime.store, &runtime.store())
        && entry
            .object
            .as_ref()
            .is_some_and(|prepared| view.target.matches_writer(&view.project_id, prepared))
}

fn query(
    runtime: &ProjectRuntime,
    run: &str,
    active: &HashMap<String, Active>,
) -> Result<Vec<Execution>, String> {
    object_attempt_view::list_with_details(runtime, run)
        .map_err(|error| error.to_string())
        .map(|views| {
            views
                .into_iter()
                .map(|(attempt, definition, checkpoints)| {
                    let availability = if attempt.state != State::Running {
                        Availability::Finished
                    } else if active
                        .get(&scheduler_work::object_key(
                            &attempt.project_id,
                            &attempt.target.task_id,
                        ))
                        .is_some_and(|entry| matches(entry, runtime, &attempt))
                    {
                        Availability::Active
                    } else {
                        Availability::RecoveryRequired
                    };
                    Execution {
                        attempt,
                        availability,
                        definition,
                        checkpoints,
                    }
                })
                .collect()
        })
}

fn recovery_live(
    active: &HashMap<String, Active>,
    runtime: &ProjectRuntime,
    project: &str,
    target: &recovery::Target,
) -> bool {
    active
        .get(&scheduler_work::object_key(project, &target.task_id))
        .is_some_and(|entry| {
            Arc::ptr_eq(&entry.runtime.store, &runtime.store())
                && entry.object.as_ref().is_some_and(|prepared| {
                    prepared.project_id == project
                        && prepared.medium.id == target.task_id
                        && prepared.run.id == target.run_id
                        && prepared.run.object_id == target.object_id
                        && prepared.owner == target.owner
                        && prepared.claim_token == target.claim_token
                        && prepared.generation == target.writer_generation
                })
        })
}

pub(super) fn handle(message: ObjectMessage, active: &mut HashMap<String, Active>) {
    match message {
        ObjectMessage::Dispose(runtime, request, reply) => {
            let live = recovery_live(active, &runtime, &request.project_id, &request.target);
            tokio::task::spawn_blocking(move || {
                let _ = reply.send(
                    disposition::dispose(&runtime, &request, live)
                        .map_err(|error| format!("{error:#}")),
                );
            });
        }
        ObjectMessage::Verify(runtime, request, reply) => {
            let live = recovery_live(active, &runtime, &request.project_id, &request.target);
            // Keep filesystem I/O off the scheduler loop. A claimed preparation
            // is live even before it has created an attempt.
            tokio::task::spawn_blocking(move || {
                let _ = reply.send(
                    recovery::verify(&runtime, &request, live)
                        .map_err(|error| format!("{error:#}")),
                );
            });
        }
        ObjectMessage::Query(runtime, run, reply) => {
            let _ = reply.send(query(&runtime, &run, active));
        }
        ObjectMessage::Interrupt(runtime, request, reply) => {
            let entry = active.get_mut(&scheduler_work::object_key(
                &request.project_id,
                &request.target.task_id,
            ));
            let live = entry.as_ref().is_some_and(|entry| {
                Arc::ptr_eq(&entry.runtime.store, &runtime.store())
                    && entry.object.as_ref().is_some_and(|prepared| {
                        request.target.matches_writer(&request.project_id, prepared)
                    })
            });
            match control::request(&runtime, &request, live) {
                Ok(true) => {
                    let entry = entry.expect("validated live object writer");
                    cancel(entry);
                    entry.waiters.push(reply);
                }
                Ok(false) => {
                    let _ = reply.send(Ok(()));
                }
                Err(error) => {
                    let _ = reply.send(Err(error.to_string()));
                }
            }
        }
    }
}
