use super::definition_fixture::{fixture, rejected, request};
use crate::object_tasks;
use anyhow::Result;

#[test]
fn history_write_failure_rolls_back_task_plan_and_receipt_then_allows_retry() -> Result<()> {
    let f = fixture()?;
    let request = request(&f, "medium", "retry")?;
    let handle = f.runtime.store();
    handle.lock().unwrap().connection.execute_batch(
        "CREATE TRIGGER fail_revision BEFORE INSERT ON entities
         WHEN NEW.kind = 'object_task_definition_revision'
         BEGIN SELECT RAISE(ABORT, 'forced revision failure'); END;",
    )?;
    rejected(&f, &request, "OBJECT_TASK_REVISE_FAILED")?;
    assert!(object_tasks::revisions(&f.runtime, "project-1", "medium")?.is_empty());
    handle
        .lock()
        .unwrap()
        .connection
        .execute_batch("DROP TRIGGER fail_revision;")?;
    let receipt = object_tasks::revise_planned(&f.runtime, &request)?;
    assert_eq!(receipt.task_revision, 1);
    assert_eq!(receipt.plan_revision, 2);
    assert_eq!(object_tasks::revise_planned(&f.runtime, &request)?, receipt);
    assert_eq!(f.count("object_task_definition_revision")?, 1);
    Ok(())
}

#[test]
fn concurrent_revisions_accept_only_one_writer_from_the_same_plan() -> Result<()> {
    let f = fixture()?;
    let requests = [
        request(&f, "fine", "first")?,
        request(&f, "medium", "second")?,
    ];
    let barrier = std::sync::Barrier::new(2);
    let results = std::thread::scope(|scope| {
        let handles: Vec<_> = requests
            .iter()
            .map(|request| {
                let barrier = &barrier;
                let runtime = &f.runtime;
                scope.spawn(move || {
                    barrier.wait();
                    object_tasks::revise_planned(runtime, request)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    let error = results.into_iter().find_map(Result::err).unwrap();
    assert!(format!("{error:#}").contains("OBJECT_TASK_PLAN_REVISION_CONFLICT"));
    assert_eq!(
        object_tasks::snapshot(&f.runtime, "project-1")?.plan_revision,
        2
    );
    assert_eq!(f.count("object_task_definition_revision")?, 1);
    Ok(())
}
