use super::{
    cancel::request,
    queue_fixture::{enqueue, fixture},
};
use crate::{
    object_catalog_test_fixture::Fixture, object_task_queue as queue, object_tasks,
    project_storage::ProjectStore,
};
use anyhow::Result;

#[test]
fn blocked_head_reserves_its_object_but_allows_independent_work() -> Result<()> {
    let fixture = fixture(true)?;
    enqueue(&fixture, &["head", "next", "independent"])?;
    let claim = queue::claim_next(&fixture.runtime, "project-1", "worker")?.unwrap();
    assert_eq!(claim.task.id, "independent");
    assert!(queue::claim_next(&fixture.runtime, "project-1", "other")?.is_none());
    Ok(())
}

#[test]
fn finish_retains_ownership_across_reopen_until_cancellation_closes_the_run() -> Result<()> {
    for (success, status) in [(true, "awaitingAcceptance"), (false, "failed")] {
        let fixture = fixture(false)?;
        enqueue(&fixture, &["head", "next", "independent"])?;
        let claim = queue::claim_next(&fixture.runtime, "project-1", "worker")?.unwrap();
        assert_eq!(claim.task.id, "head");
        let finished = queue::finish_claim(
            &fixture.runtime,
            "project-1",
            "head",
            "worker",
            &claim.claim_token,
            claim.generation,
            success,
        )?;
        assert_eq!(finished.status, status);
        let snapshot = object_tasks::snapshot(&fixture.runtime, "project-1")?;
        let run = snapshot
            .runs
            .iter()
            .find(|run| run.medium_task_id == "head")
            .unwrap();
        assert_eq!((run.status.as_str(), run.revision), (status, 2));
        let Fixture { runtime, temp } = fixture;
        drop(runtime);
        let runtime = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
        assert_eq!(
            queue::claim_next(&runtime, "project-1", "other")?
                .unwrap()
                .task
                .id,
            "independent",
        );
        assert!(queue::claim_next(&runtime, "project-1", "another")?.is_none());
        queue::cancel_claim(
            &runtime,
            "project-1",
            "head",
            "worker",
            &claim.claim_token,
            claim.generation,
        )?;
        let cancelled = object_tasks::snapshot(&runtime, "project-1")?;
        assert_eq!(cancelled.plan_revision, 2);
        for id in ["head", "fine"] {
            assert_eq!(
                cancelled
                    .tasks
                    .iter()
                    .find(|task| task.id == id)
                    .unwrap()
                    .status,
                "cancelled"
            );
        }
        let run = cancelled
            .runs
            .iter()
            .find(|run| run.medium_task_id == "head")
            .unwrap();
        assert_eq!((run.status.as_str(), run.revision), ("cancelled", 3));
        assert_eq!(
            queue::claim_next(&runtime, "project-1", "next-worker")?
                .unwrap()
                .task
                .id,
            "next",
        );
        let before = object_tasks::snapshot(&runtime, "project-1")?;
        assert!(queue::finish_claim(
            &runtime,
            "project-1",
            "head",
            "worker",
            &claim.claim_token,
            claim.generation,
            true,
        )
        .is_err());
        assert_eq!(object_tasks::snapshot(&runtime, "project-1")?, before);
    }
    Ok(())
}

#[test]
fn planned_cancellation_removes_owned_queue_entries_and_keeps_the_next_task() -> Result<()> {
    let fixture = fixture(false)?;
    enqueue(&fixture, &["head", "next"])?;
    object_tasks::cancel_planned(&fixture.runtime, &request("head", "cancel-head", 0, 1))?;
    let entries = queue::list(&fixture.runtime, "project-1")?;
    assert_eq!(
        entries
            .iter()
            .find(|entry| entry.task_id == "head")
            .unwrap()
            .state,
        "cancelled"
    );
    assert_eq!(
        entries
            .iter()
            .find(|entry| entry.task_id == "next")
            .unwrap()
            .state,
        "queued"
    );
    assert_eq!(
        queue::claim_next(&fixture.runtime, "project-1", "worker")?
            .unwrap()
            .task
            .id,
        "next",
    );
    Ok(())
}

#[test]
fn coarse_and_fine_tasks_cannot_acquire_independent_writer_claims() -> Result<()> {
    let fixture = fixture(false)?;
    let before = object_tasks::snapshot(&fixture.runtime, "project-1")?;
    for id in ["root", "fine"] {
        assert!(enqueue(&fixture, &["head", id]).is_err());
        assert!(queue::list(&fixture.runtime, "project-1")?.is_empty());
        assert_eq!(
            object_tasks::snapshot(&fixture.runtime, "project-1")?,
            before
        );
    }
    Ok(())
}

#[test]
fn dependency_requires_the_same_project_and_accepted_task_identity() -> Result<()> {
    let fixture = fixture(true)?;
    enqueue(&fixture, &["head", "next"])?;
    for (id, project, status) in [
        ("gate", "project-1", "completed"),
        ("gate", "project-1", "succeeded"),
        ("gate", "project-1", "awaitingAcceptance"),
        ("gate", "project-1", "failed"),
        ("gate", "project-2", "accepted"),
        ("other", "project-1", "accepted"),
    ] {
        fixture.runtime.store().lock().unwrap().connection.execute(
            "UPDATE entities SET value=json_set(value,'$.id',?,'$.projectId',?,'$.status',?)
             WHERE kind='object_task' AND id='gate'",
            [id, project, status],
        )?;
        assert!(queue::claim_next(&fixture.runtime, "project-1", "worker")?.is_none());
    }
    fixture.runtime.store().lock().unwrap().connection.execute(
        "UPDATE entities SET value=json_set(value,'$.id','gate','$.projectId','project-1','$.status','accepted')
         WHERE kind='object_task' AND id='gate'", [],
    )?;
    assert_eq!(
        queue::claim_next(&fixture.runtime, "project-1", "worker")?
            .unwrap()
            .task
            .id,
        "head"
    );
    Ok(())
}

#[test]
fn started_fine_work_blocks_cancellation_and_retains_the_object() -> Result<()> {
    let fixture = fixture(false)?;
    enqueue(&fixture, &["head", "next"])?;
    let claim = queue::claim_next(&fixture.runtime, "project-1", "worker")?.unwrap();
    fixture.runtime.store().lock().unwrap().connection.execute(
        "UPDATE entities SET value=json_set(value,'$.status','queued')
         WHERE kind='object_task' AND id='fine'",
        [],
    )?;
    let snapshot = object_tasks::snapshot(&fixture.runtime, "project-1")?;
    let entries = queue::list(&fixture.runtime, "project-1")?;
    assert!(queue::cancel_claim(
        &fixture.runtime,
        "project-1",
        "head",
        "worker",
        &claim.claim_token,
        claim.generation,
    )
    .is_err());
    assert_eq!(
        object_tasks::snapshot(&fixture.runtime, "project-1")?,
        snapshot
    );
    assert_eq!(queue::list(&fixture.runtime, "project-1")?, entries);
    assert!(queue::claim_next(&fixture.runtime, "project-1", "other")?.is_none());
    Ok(())
}
