use super::commit::{commit, save, task};
use crate::{
    object_attempt::{self, Attempt, Lease},
    object_catalog_test_fixture::Fixture,
    object_framework::Baseline,
    object_run_preparation,
    object_task_types::{Granularity, ObjectProposal, PlanProposal, TaskRecord},
};
use anyhow::Result;

pub(super) fn fixture() -> Result<Fixture> {
    let fixture = Fixture::new()?;
    let mut tasks = vec![
        task("head", Granularity::Medium, Some("hero"), None, None),
        task(
            "fine-a",
            Granularity::Fine,
            Some("hero"),
            Some("head"),
            Some("files"),
        ),
        task(
            "fine-b",
            Granularity::Fine,
            Some("hero"),
            Some("head"),
            Some("review"),
        ),
        task("next", Granularity::Medium, Some("hero"), None, None),
        task(
            "independent",
            Granularity::Medium,
            Some("rival"),
            None,
            None,
        ),
        task(
            "rival-fine",
            Granularity::Fine,
            Some("rival"),
            Some("independent"),
            Some("files"),
        ),
    ];
    tasks[2].depends_on.push("fine-a".into());
    for (position, task) in tasks.iter_mut().enumerate() {
        task.position = position as u64;
        if task.granularity == Granularity::Medium {
            task.baseline = Some(Baseline::Empty {});
        }
    }
    save(
        &fixture,
        "attempt-plan",
        0,
        PlanProposal {
            objects: ["hero", "rival"]
                .into_iter()
                .map(|id| ObjectProposal {
                    id: id.into(),
                    name: id.into(),
                    category: "character".into(),
                })
                .collect(),
            tasks,
            assumptions: vec![],
        },
    )?;
    commit(&fixture, "attempt-commit", "attempt-plan", 0)?;
    Ok(fixture)
}

pub(super) fn start(fixture: &Fixture) -> Result<Lease> {
    super::queue_fixture::enqueue(fixture, &["head", "next"])?;
    let claim =
        object_run_preparation::claim_next(&fixture.runtime, "project-1", "worker")?.unwrap();
    Ok(object_attempt::start(&fixture.runtime, claim)?.unwrap())
}

pub(super) fn task_record(fixture: &Fixture, id: &str) -> Result<TaskRecord> {
    Ok(fixture
        .runtime
        .store()
        .lock()
        .unwrap()
        .get("object_task", id)?
        .unwrap())
}

pub(super) fn attempts(fixture: &Fixture, medium: &str) -> Result<Vec<Attempt>> {
    object_attempt::list(
        &fixture.runtime,
        task_record(fixture, medium)?.run_id.as_deref().unwrap(),
    )
}

pub(super) fn interrupt_request(
    fixture: &Fixture,
    medium: &str,
) -> Result<crate::object_attempt_control::InterruptRequest> {
    let run = task_record(fixture, medium)?.run_id.unwrap();
    let view = crate::object_attempt_view::list(&fixture.runtime, &run)?.remove(0);
    Ok(crate::object_attempt_control::InterruptRequest {
        project_id: view.project_id,
        request_id: "interrupt-1".into(),
        target: view.target,
        expected_task_revision: view.task_revision,
    })
}

pub(super) fn workspace(fixture: &Fixture, attempt: &Attempt) -> Result<std::path::PathBuf> {
    fixture.runtime.files().resolve_workspace(
        &attempt.preparation.run.id,
        std::path::Path::new(&attempt.preparation.workspace),
    )
}

pub(super) fn mutate(
    fixture: &Fixture,
    kind: &str,
    id: &str,
    pointer: &str,
    value: serde_json::Value,
) -> Result<()> {
    let handle = fixture.runtime.store();
    let store = handle.lock().unwrap();
    let mut record: serde_json::Value = store.get(kind, id)?.unwrap();
    *record.pointer_mut(pointer).unwrap() = value;
    store.put(kind, id, &record)
}
