use super::{attempt_fixture::fixture, object_task_dispatch::request};
use crate::{
    object_task_dispatch as dispatch, object_task_types::CancelPlannedRequest, object_tasks,
};
use anyhow::Result;

#[test]
fn receipt_failure_rolls_back_control_and_allows_exact_retry() -> Result<()> {
    let f = fixture()?;
    let input = request(&f, "pause", true)?;
    let before = object_tasks::snapshot(&f.runtime, "project-1")?;
    let handle = f.runtime.store();
    handle.lock().unwrap().connection.execute_batch(
        "CREATE TRIGGER fail_dispatch_receipt BEFORE INSERT ON entities
         WHEN NEW.kind = 'object_task_dispatch_receipt'
         BEGIN SELECT RAISE(ABORT, 'forced receipt failure'); END;",
    )?;
    assert!(dispatch::set_paused(&f.runtime, &input).is_err());
    assert_eq!(object_tasks::snapshot(&f.runtime, "project-1")?, before);
    assert_eq!(f.count("object_task_dispatch_control")?, 0);
    assert_eq!(f.count("object_task_dispatch_receipt")?, 0);
    handle
        .lock()
        .unwrap()
        .connection
        .execute_batch("DROP TRIGGER fail_dispatch_receipt;")?;
    let receipt = dispatch::set_paused(&f.runtime, &input)?;
    assert_eq!(receipt.result.revision, 1);
    assert_eq!(dispatch::set_paused(&f.runtime, &input)?, receipt);
    Ok(())
}

#[test]
fn concurrent_control_writers_share_a_single_revision_winner() -> Result<()> {
    let f = fixture()?;
    let inputs = [
        request(&f, "pause", true)?,
        request(&f, "leave-unpaused", false)?,
    ];
    let barrier = std::sync::Barrier::new(2);
    let results = std::thread::scope(|scope| {
        let handles: Vec<_> = inputs
            .iter()
            .map(|input| {
                let runtime = &f.runtime;
                let barrier = &barrier;
                scope.spawn(move || {
                    barrier.wait();
                    dispatch::set_paused(runtime, input)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .find_map(|result| result.as_ref().err())
            .unwrap()
            .to_string(),
        "OBJECT_TASK_DISPATCH_REVISION_CONFLICT"
    );
    let winner = results.into_iter().find_map(Result::ok).unwrap();
    assert_eq!(
        object_tasks::snapshot(&f.runtime, "project-1")?.dispatch_controls,
        vec![winner.result]
    );
    assert_eq!(f.count("object_task_dispatch_receipt")?, 1);
    Ok(())
}

#[test]
fn cancelled_medium_rejects_new_control_but_keeps_historical_receipt() -> Result<()> {
    let f = fixture()?;
    let input = request(&f, "pause", true)?;
    let receipt = dispatch::set_paused(&f.runtime, &input)?;
    object_tasks::cancel_planned(
        &f.runtime,
        &CancelPlannedRequest {
            project_id: "project-1".into(),
            task_id: "head".into(),
            request_id: "cancel".into(),
            expected_task_revision: input.expected_task_revision,
            expected_plan_revision: object_tasks::snapshot(&f.runtime, "project-1")?.plan_revision,
        },
    )?;
    let cancelled = object_tasks::snapshot(&f.runtime, "project-1")?;
    assert_eq!(dispatch::set_paused(&f.runtime, &input)?, receipt);
    let fresh = request(&f, "unpause-cancelled", false)?;
    assert_eq!(
        dispatch::set_paused(&f.runtime, &fresh)
            .unwrap_err()
            .to_string(),
        "OBJECT_TASK_DISPATCH_NOT_CONTROLLABLE"
    );
    assert_eq!(object_tasks::snapshot(&f.runtime, "project-1")?, cancelled);
    assert_eq!(f.count("object_task_dispatch_receipt")?, 1);
    Ok(())
}
