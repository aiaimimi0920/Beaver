use super::*;
use crate::{
    object_framework::{Baseline, Identity, VERSION},
    object_task_storage::RUN_KIND,
    object_task_types::{Granularity, RunRecord},
    store_schema,
};
use rusqlite::Connection;

fn task(id: &str, object_id: Option<&str>, position: u64) -> TaskRecord {
    TaskRecord {
        id: id.into(),
        position,
        project_id: "project-1".into(),
        granularity: Granularity::Medium,
        title: id.into(),
        prompt: "prompt".into(),
        acceptance: "done".into(),
        requirement: Default::default(),
        pending_planning: String::new(),
        object_id: object_id.map(str::to_owned),
        parent_task_id: None,
        depends_on: vec![],
        run_id: Some(format!("run-{id}")),
        stage_id: None,
        identity: Identity::Medium {
            schema_version: VERSION,
            object_id: object_id.unwrap().into(),
            baseline: Baseline::LatestAccepted {},
        },
        status: "planned".into(),
        revision: 0,
    }
}

fn insert_task(connection: &Connection, task: &TaskRecord) -> Result<()> {
    object_task_storage::insert(connection, TASK_KIND, &task.id, task)?;
    let run = RunRecord {
        id: task.run_id.clone().unwrap(),
        project_id: task.project_id.clone(),
        object_id: task.object_id.clone().unwrap(),
        medium_task_id: task.id.clone(),
        baseline_version_id: None,
        status: task.status.clone(),
        revision: 0,
    };
    object_task_storage::insert(connection, RUN_KIND, &run.id, &run)
}

fn insert_queue(connection: &Connection, task: &TaskRecord) -> Result<()> {
    enqueue_in(connection, "project-1", std::slice::from_ref(&task.id))?;
    Ok(())
}

#[test]
fn enqueue_is_idempotent_and_same_object_claim_is_exclusive() -> Result<()> {
    let connection = Connection::open_in_memory()?;
    store_schema::initialize(&connection)?;
    insert_task(&connection, &task("a", Some("hero"), 1))?;
    insert_task(&connection, &task("b", Some("hero"), 2))?;
    let ids = vec!["a".into(), "b".into()];
    assert_eq!(enqueue_in(&connection, "project-1", &ids)?.len(), 2);
    assert_eq!(enqueue_in(&connection, "project-1", &ids)?.len(), 2);
    assert!(claim_next_in(&connection, "project-1", "worker-a")?.is_some());
    assert!(claim_next_in(&connection, "project-1", "worker-b")?.is_none());
    Ok(())
}

#[test]
fn different_objects_can_claim_in_parallel() -> Result<()> {
    let connection = Connection::open_in_memory()?;
    store_schema::initialize(&connection)?;
    let first = task("first", Some("hero"), 1);
    let second = task("second", Some("villain"), 2);
    insert_task(&connection, &first)?;
    insert_task(&connection, &second)?;
    insert_queue(&connection, &first)?;
    insert_queue(&connection, &second)?;

    assert_eq!(
        claim_next_in(&connection, "project-1", "worker-a")?
            .unwrap()
            .task
            .id,
        "first"
    );
    assert_eq!(
        claim_next_in(&connection, "project-1", "worker-b")?
            .unwrap()
            .task
            .id,
        "second"
    );
    Ok(())
}

#[test]
fn cancelled_task_cannot_be_enqueued_or_claimed() -> Result<()> {
    let connection = Connection::open_in_memory()?;
    store_schema::initialize(&connection)?;
    let mut cancelled = task("cancelled", Some("hero"), 1);
    cancelled.status = "cancelled".into();
    insert_task(&connection, &cancelled)?;
    assert!(enqueue_in(&connection, "project-1", &[cancelled.id]).is_err());
    Ok(())
}

#[test]
fn missing_or_incomplete_dependency_blocks_until_ready() -> Result<()> {
    for requirement in [
        crate::object_task_types::WorkRequirement::Required,
        crate::object_task_types::WorkRequirement::Optional,
    ] {
        let connection = Connection::open_in_memory()?;
        store_schema::initialize(&connection)?;
        let mut dependent = task("dependent", Some("hero"), 1);
        dependent.depends_on = vec!["missing".into()];
        insert_task(&connection, &dependent)?;
        insert_queue(&connection, &dependent)?;
        assert!(claim_next_in(&connection, "project-1", "worker")?.is_none());

        let mut dependency = task("missing", Some("other"), 0);
        dependency.requirement = requirement;
        dependency.status = "queued".into();
        insert_task(&connection, &dependency)?;
        assert!(claim_next_in(&connection, "project-1", "worker")?.is_none());

        dependency.status = "accepted".into();
        object_task_storage::replace(&connection, TASK_KIND, &dependency.id, &dependency)?;
        assert!(claim_next_in(&connection, "project-1", "worker")?.is_some());
    }
    Ok(())
}

#[test]
fn stale_owner_token_and_generation_cannot_finish_claim() -> Result<()> {
    let connection = Connection::open_in_memory()?;
    store_schema::initialize(&connection)?;
    let queued = task("queued", Some("hero"), 1);
    insert_task(&connection, &queued)?;
    insert_queue(&connection, &queued)?;
    let claim = claim_next_in(&connection, "project-1", "worker")?.unwrap();
    let id = "project-1:queued";
    let entry = object_task_storage::read::<QueueEntry>(&connection, QUEUE_KIND, id)?.unwrap();
    assert_eq!(entry.generation, 1);
    assert_ne!(entry.claim_token.as_deref(), Some("wrong"));
    assert_eq!(claim.generation, 1);
    assert_eq!(claim.task.status, "queued");
    assert!(entry.owner.as_deref() == Some("worker"));
    assert!(finish_claim_in(
        &connection,
        "project-1",
        "queued",
        "other",
        "wrong",
        1,
        true
    )
    .is_err());
    assert!(finish_claim_in(
        &connection,
        "project-1",
        "queued",
        "worker",
        &claim.claim_token,
        2,
        true
    )
    .is_err());
    Ok(())
}

#[test]
fn finish_failure_marks_task_and_entry_failed() -> Result<()> {
    let connection = Connection::open_in_memory()?;
    store_schema::initialize(&connection)?;
    let queued = task("queued", Some("hero"), 1);
    insert_task(&connection, &queued)?;
    insert_queue(&connection, &queued)?;
    let claim = claim_next_in(&connection, "project-1", "worker")?.unwrap();
    let id = "project-1:queued";
    let current = finish_claim_in(
        &connection,
        "project-1",
        "queued",
        "worker",
        &claim.claim_token,
        claim.generation,
        false,
    )?;
    assert_eq!(
        object_task_storage::read::<QueueEntry>(&connection, QUEUE_KIND, id)?
            .unwrap()
            .state,
        "failed"
    );
    assert_eq!(current.status, "failed");
    assert_eq!(claim.generation, 1);
    Ok(())
}

#[test]
fn cancel_claim_marks_owned_work_terminal_and_rejects_stale_callback() -> Result<()> {
    let connection = Connection::open_in_memory()?;
    store_schema::initialize(&connection)?;
    let queued = task("queued", Some("hero"), 1);
    insert_task(&connection, &queued)?;
    insert_queue(&connection, &queued)?;
    let claim = claim_next_in(&connection, "project-1", "worker")?.unwrap();
    let cancelled = cancel_claim_in(
        &connection,
        "project-1",
        "queued",
        "worker",
        &claim.claim_token,
        claim.generation,
    )?;
    assert_eq!(cancelled.status, "cancelled");
    let entry =
        object_task_storage::read::<QueueEntry>(&connection, QUEUE_KIND, "project-1:queued")?
            .unwrap();
    assert_eq!(entry.state, "cancelled");
    assert!(finish_claim_in(
        &connection,
        "project-1",
        "queued",
        "worker",
        &claim.claim_token,
        claim.generation,
        true,
    )
    .is_err());
    Ok(())
}
