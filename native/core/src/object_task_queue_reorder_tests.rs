use super::queue_fixture::{enqueue, fixture};
use crate::{
    object_task_queue as queue,
    object_task_queue_reorder::{reorder, Request},
    object_task_queue_view::get,
    project_runtime::ProjectRuntime,
    project_storage::ProjectStore,
};
use anyhow::Result;

fn request(
    runtime: &ProjectRuntime,
    id: &str,
    task: &str,
    previous: Option<&str>,
    next: Option<&str>,
) -> Result<Request> {
    Ok(Request {
        project_id: "project-1".into(),
        request_id: id.into(),
        expected_version: get(runtime, "project-1")?.version,
        task_id: task.into(),
        previous_task_id: previous.map(Into::into),
        next_task_id: next.map(Into::into),
    })
}

#[test]
fn reorder_persists_and_replays_original_receipt_after_claim() -> Result<()> {
    let f = fixture(false)?;
    enqueue(&f, &["head", "next", "independent"])?;
    let input = request(&f.runtime, "move", "next", None, Some("head"))?;
    let receipt = reorder(&f.runtime, &input)?;
    assert_eq!(
        receipt
            .result
            .items
            .iter()
            .map(|i| i.task_id.as_str())
            .collect::<Vec<_>>(),
        ["next", "head", "independent"]
    );
    let crate::object_catalog_test_fixture::Fixture { runtime, temp } = f;
    drop(runtime);
    let runtime = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    assert_eq!(get(&runtime, "project-1")?, receipt.result);
    assert_eq!(
        queue::claim_next(&runtime, "project-1", "worker")?
            .unwrap()
            .task
            .id,
        "next"
    );
    let claimed = queue::list(&runtime, "project-1")?;
    assert_eq!(reorder(&runtime, &input)?, receipt);
    assert_eq!(queue::list(&runtime, "project-1")?, claimed);
    let mut changed = input.clone();
    changed.next_task_id = Some("independent".into());
    assert!(reorder(&runtime, &changed)
        .unwrap_err()
        .to_string()
        .contains("REQUEST_CONFLICT"));
    Ok(())
}

#[test]
fn stale_review_and_invalid_anchors_leave_queue_unchanged() -> Result<()> {
    let f = fixture(false)?;
    enqueue(&f, &["head", "next", "independent"])?;
    let stale = request(&f.runtime, "stale", "next", None, Some("head"))?;
    let first = request(&f.runtime, "first", "independent", None, Some("head"))?;
    reorder(&f.runtime, &first)?;
    let before = get(&f.runtime, "project-1")?;
    assert!(reorder(&f.runtime, &stale)
        .unwrap_err()
        .to_string()
        .contains("VERSION_CONFLICT"));
    for (previous, next) in [
        (Some("head"), None),
        (Some("missing"), Some("head")),
        (Some("independent"), None),
    ] {
        let invalid = request(&f.runtime, "invalid", "independent", previous, next)?;
        assert!(reorder(&f.runtime, &invalid).is_err());
        assert_eq!(get(&f.runtime, "project-1")?, before);
    }
    Ok(())
}

#[test]
fn blocked_head_cannot_be_bypassed_but_other_objects_can_move() -> Result<()> {
    let f = fixture(true)?;
    enqueue(&f, &["head", "next", "independent"])?;
    let before = get(&f.runtime, "project-1")?;
    assert_eq!(before.items[0].blockers, ["dependencies"]);
    let bypass = request(&f.runtime, "bypass", "next", None, Some("head"))?;
    assert!(reorder(&f.runtime, &bypass)
        .unwrap_err()
        .to_string()
        .contains("BLOCKED_HEAD"));
    assert_eq!(get(&f.runtime, "project-1")?, before);
    let independent = request(&f.runtime, "independent", "independent", None, Some("head"))?;
    reorder(&f.runtime, &independent)?;
    let claim = queue::claim_next(&f.runtime, "project-1", "worker")?.unwrap();
    assert_eq!(claim.task.id, "independent");
    let owned = queue::list(&f.runtime, "project-1")?;
    let active = request(&f.runtime, "active", "independent", Some("next"), None)?;
    assert!(reorder(&f.runtime, &active)
        .unwrap_err()
        .to_string()
        .contains("NOT_REORDERABLE"));
    assert_eq!(queue::list(&f.runtime, "project-1")?, owned);
    assert!(queue::claim_next(&f.runtime, "project-1", "worker")?.is_none());
    Ok(())
}

#[test]
fn claim_enqueue_and_pause_invalidate_reviews_without_order_changes() -> Result<()> {
    for change in ["claim", "enqueue", "pause"] {
        let f = fixture(false)?;
        enqueue(&f, &["head", "next"])?;
        let stale = request(&f.runtime, "move", "next", None, Some("head"))?;
        match change {
            "claim" => {
                queue::claim_next(&f.runtime, "project-1", "worker")?;
            }
            "enqueue" => enqueue(&f, &["independent"])?,
            _ => {
                let pause = super::object_task_dispatch::request(&f, "pause", true)?;
                crate::object_task_dispatch::set_paused(&f.runtime, &pause)?;
            }
        }
        let before = get(&f.runtime, "project-1")?;
        assert!(reorder(&f.runtime, &stale)
            .unwrap_err()
            .to_string()
            .contains("VERSION_CONFLICT"));
        assert_eq!(get(&f.runtime, "project-1")?, before);
    }
    Ok(())
}

#[test]
fn returning_to_old_order_still_invalidates_old_review_and_overflow_rolls_back() -> Result<()> {
    let f = fixture(false)?;
    enqueue(&f, &["head", "next", "independent"])?;
    let normalize = request(&f.runtime, "normalize", "head", None, Some("next"))?;
    reorder(&f.runtime, &normalize)?;
    let before = get(&f.runtime, "project-1")?;
    let stale = request(&f.runtime, "stale", "next", None, Some("head"))?;
    reorder(
        &f.runtime,
        &request(&f.runtime, "move", "independent", None, Some("head"))?,
    )?;
    reorder(
        &f.runtime,
        &request(&f.runtime, "back", "independent", Some("next"), None)?,
    )?;
    let after = get(&f.runtime, "project-1")?;
    assert_eq!(after.items, before.items);
    assert_ne!(after.version, before.version);
    assert!(reorder(&f.runtime, &stale).is_err());
    f.runtime.store().lock().unwrap().put(
        crate::object_task_queue_view::ORDER_KIND,
        "project-1",
        &u64::MAX,
    )?;
    let before = get(&f.runtime, "project-1")?;
    let input = request(&f.runtime, "overflow", "independent", None, Some("head"))?;
    assert!(reorder(&f.runtime, &input)
        .unwrap_err()
        .to_string()
        .contains("REVISION_EXHAUSTED"));
    assert_eq!(get(&f.runtime, "project-1")?, before);
    Ok(())
}

#[test]
fn paused_heads_and_active_leases_survive_reorder() -> Result<()> {
    let f = fixture(false)?;
    enqueue(&f, &["head", "next", "independent"])?;
    let pause = super::object_task_dispatch::request(&f, "pause", true)?;
    crate::object_task_dispatch::set_paused(&f.runtime, &pause)?;
    assert!(reorder(
        &f.runtime,
        &request(&f.runtime, "bypass", "next", None, Some("head"))?
    )
    .is_err());
    queue::claim_next(&f.runtime, "project-1", "worker")?;
    let before = queue::list(&f.runtime, "project-1")?;
    reorder(
        &f.runtime,
        &request(&f.runtime, "keep-head", "next", Some("head"), None)?,
    )?;
    let after = queue::list(&f.runtime, "project-1")?;
    assert_eq!(
        before.iter().find(|e| e.task_id == "independent"),
        after.iter().find(|e| e.task_id == "independent")
    );
    assert!(queue::claim_next(&f.runtime, "project-1", "worker")?.is_none());
    Ok(())
}
