use super::{attempt_fixture::*, object_attempt_rpc_fixture as rpc, queue_fixture};
use crate::{
    object_attempt::{self, State},
    object_attempt_launch::Factory,
    object_catalog_test_fixture::Fixture,
    object_run_preparation, object_task_queue,
    project_storage::ProjectStore,
    scheduler::{ObjectAvailability, Scheduler},
    scheduler_runtime::{RuntimeSource, TaskRuntime},
    scheduler_work,
};
use anyhow::Result;
use std::{
    fs,
    path::Path,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

fn scheduler(
    fixture: &Fixture,
    factory: Factory,
    limit: usize,
    legacy: Arc<AtomicUsize>,
) -> Scheduler {
    let runtime = TaskRuntime::from_project(fixture.runtime.clone());
    let runtimes: RuntimeSource = Arc::new(move || Ok(vec![runtime.clone()]));
    Scheduler::start_with_objects(
        runtimes,
        Arc::new(move |_, _| {
            legacy.fetch_add(1, Ordering::SeqCst);
            Err("unexpected legacy factory call".into())
        }),
        factory,
        Arc::new(|| {}),
        Arc::new(move || Ok(limit)),
    )
}

fn factory(root: &Path, calls: Arc<AtomicUsize>) -> Factory {
    let root = root.to_path_buf();
    Arc::new(move |attempt, runtime| {
        calls.fetch_add(1, Ordering::SeqCst);
        let cwd = runtime
            .files()
            .resolve_workspace(
                &attempt.preparation.run.id,
                Path::new(&attempt.preparation.workspace),
            )
            .map_err(|error| error.to_string())?;
        rpc::launch(&root.join(&attempt.preparation.medium.id), &cwd, "hold")
            .map_err(|error| error.to_string())
    })
}

#[tokio::test]
async fn independent_objects_share_slots_while_waiting_gate_retains_the_object() -> Result<()> {
    for limit in [1, 2] {
        let fixture = fixture()?;
        queue_fixture::enqueue(&fixture, &["head", "next", "independent"])?;
        let root = fixture.temp.path().join("rpc");
        let calls = Arc::new(AtomicUsize::new(0));
        let legacy = Arc::new(AtomicUsize::new(0));
        let scheduler = scheduler(
            &fixture,
            factory(&root, calls.clone()),
            limit,
            legacy.clone(),
        );
        rpc::wait_until(|| root.join("head/turn-ready").exists()).await;
        let held = attempts(&fixture, "head")?.remove(0).preparation;
        if limit == 2 {
            rpc::wait_until(|| root.join("independent/turn-ready").exists()).await;
            assert_eq!(attempts(&fixture, "independent")?[0].state, State::Running);
        } else {
            assert!(attempts(&fixture, "independent")?.is_empty());
        }
        assert_eq!(attempts(&fixture, "head")?[0].state, State::Running);
        fs::write(root.join("head/release"), "release")?;
        scheduler
            .synchronize(scheduler_work::object_key("project-1", "head"))
            .await
            .map_err(anyhow::Error::msg)?;
        rpc::wait_until(|| root.join("independent/turn-ready").exists()).await;
        rpc::assert_closed(&root.join("head"))?;
        assert_eq!(attempts(&fixture, "head")?[0].state, State::AwaitingGate);
        assert_eq!(task_record(&fixture, "fine-b")?.status, "planned");
        assert_eq!(task_record(&fixture, "next")?.status, "planned");
        let queue = object_task_queue::list(&fixture.runtime, "project-1")?;
        let head = queue.iter().find(|entry| entry.task_id == "head").unwrap();
        assert_eq!(head.owner.as_deref(), Some(held.owner.as_str()));
        assert_eq!(head.claim_token.as_deref(), Some(held.claim_token.as_str()));
        fs::write(root.join("independent/release"), "release")?;
        scheduler
            .synchronize(scheduler_work::object_key("project-1", "independent"))
            .await
            .map_err(anyhow::Error::msg)?;
        scheduler.shutdown().await.map_err(anyhow::Error::msg)?;
        rpc::assert_closed(&root.join("independent"))?;
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(legacy.load(Ordering::SeqCst), 0);
        assert_eq!(fixture.count("task")?, 0);
        assert_eq!(fixture.count("object_version")?, 0);
        assert!(!fixture.temp.path().join("result.txt").exists());
    }
    Ok(())
}

#[tokio::test]
async fn interrupt_stops_writers_and_cannot_rewrite_a_frozen_attempt() -> Result<()> {
    let fixture = fixture()?;
    queue_fixture::enqueue(&fixture, &["head", "next", "independent"])?;
    let root = fixture.temp.path().join("rpc");
    let calls = Arc::new(AtomicUsize::new(0));
    let scheduler = scheduler(&fixture, factory(&root, calls.clone()), 2, Arc::default());
    rpc::wait_until(|| root.join("head/turn-ready").exists()).await;
    rpc::wait_until(|| root.join("independent/turn-ready").exists()).await;
    let request = interrupt_request(&fixture, "head")?;
    let query = scheduler
        .object_attempts(fixture.runtime.clone(), request.target.run_id.clone())
        .await
        .map_err(anyhow::Error::msg)?;
    assert_eq!(query[0].availability, ObjectAvailability::Active);
    let mut stale = request.clone();
    stale.target.turn_id = Some("wrong-turn".into());
    assert!(scheduler
        .interrupt_attempt(fixture.runtime.clone(), stale)
        .await
        .unwrap_err()
        .contains("STALE_TARGET"));
    assert_eq!(attempts(&fixture, "head")?[0].state, State::Running);
    assert!(scheduler
        .steer(
            scheduler_work::object_key("project-1", "head"),
            "replace task".into()
        )
        .await
        .unwrap_err()
        .contains("DEFINITION_FROZEN"));
    let receipt = scheduler
        .interrupt_attempt(fixture.runtime.clone(), request.clone())
        .await
        .map_err(anyhow::Error::msg)?;
    rpc::assert_closed(&root.join("head"))?;
    let frozen = attempts(&fixture, "head")?.remove(0);
    assert_eq!(frozen.state, State::Interrupted);
    assert_eq!(receipt.result.state, State::Interrupted);
    assert_eq!(attempts(&fixture, "independent")?[0].state, State::Running);
    assert!(frozen
        .output
        .as_ref()
        .unwrap()
        .contains_key("child-output.txt"));
    let replay = scheduler
        .interrupt_attempt(fixture.runtime.clone(), request.clone())
        .await
        .map_err(anyhow::Error::msg)?;
    assert_eq!(replay, receipt);
    scheduler.shutdown().await.map_err(anyhow::Error::msg)?;
    rpc::assert_closed(&root.join("independent"))?;
    assert_eq!(
        scheduler
            .interrupt_attempt(fixture.runtime.clone(), request.clone())
            .await
            .map_err(anyhow::Error::msg)?,
        receipt
    );
    let query = scheduler
        .object_attempts(fixture.runtime.clone(), request.target.run_id.clone())
        .await
        .map_err(anyhow::Error::msg)?;
    assert_eq!(query[0].availability, ObjectAvailability::Finished);
    assert_eq!(attempts(&fixture, "head")?, vec![frozen]);
    assert_eq!(task_record(&fixture, "next")?.status, "planned");
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    Ok(())
}

#[tokio::test]
async fn a_new_scheduler_does_not_replay_ready_or_running_records() -> Result<()> {
    for running in [false, true] {
        let fixture = fixture()?;
        queue_fixture::enqueue(&fixture, &["head", "next"])?;
        let claim =
            object_run_preparation::claim_next(&fixture.runtime, "project-1", "old")?.unwrap();
        let run_id = claim.record().run.id.clone();
        if running {
            drop(object_attempt::start(&fixture.runtime, claim)?);
        } else {
            object_run_preparation::prepare(&fixture.runtime, claim)?;
        }
        let preparation = object_run_preparation::get(&fixture.runtime, "project-1", &run_id)?;
        let before = attempts(&fixture, "head")?;
        if running {
            mutate(
                &fixture,
                "object_task",
                "fine-a",
                "/prompt",
                serde_json::json!("later live prompt"),
            )?;
        }
        let queue = object_task_queue::list(&fixture.runtime, "project-1")?;
        let Fixture { runtime, temp } = fixture;
        drop(runtime);
        let fixture = Fixture {
            runtime: ProjectStore::open(temp.path(), "project-1")?.into_runtime(),
            temp,
        };
        let calls = Arc::new(AtomicUsize::new(0));
        let counted = calls.clone();
        let factory: Factory = Arc::new(move |_, _| {
            counted.fetch_add(1, Ordering::SeqCst);
            Err("unexpected attempt replay".into())
        });
        let legacy = Arc::new(AtomicUsize::new(0));
        let scheduler = scheduler(&fixture, factory, 6, legacy.clone());
        scheduler.wake().map_err(anyhow::Error::msg)?;
        scheduler
            .synchronize("barrier".into())
            .await
            .map_err(anyhow::Error::msg)?;
        let views = scheduler
            .object_attempts(fixture.runtime.clone(), run_id.clone())
            .await
            .map_err(anyhow::Error::msg)?;
        if running {
            assert_eq!(views.len(), 1);
            assert_eq!(views[0].availability, ObjectAvailability::RecoveryRequired);
            assert_eq!(views[0].attempt.state, State::Running);
            assert_eq!(views[0].definition.prompt, before[0].fine.prompt);
            assert_eq!(views[0].definition.title, before[0].fine.title);
            assert_eq!(views[0].definition.acceptance, before[0].fine.acceptance);
            assert_eq!(views[0].definition.revision, before[0].fine.revision);
            assert_eq!(views[0].checkpoints.input, before[0].input);
            assert_eq!(views[0].checkpoints.output, None);
            assert_ne!(
                views[0].definition.prompt,
                task_record(&fixture, "fine-a")?.prompt
            );
            let request = interrupt_request(&fixture, "head")?;
            assert!(scheduler
                .interrupt_attempt(fixture.runtime.clone(), request)
                .await
                .unwrap_err()
                .contains("RECOVERY_REQUIRED"));
        } else {
            assert!(views.is_empty());
        }
        scheduler.shutdown().await.map_err(anyhow::Error::msg)?;
        assert_eq!(
            scheduler
                .object_attempts(fixture.runtime.clone(), run_id.clone())
                .await
                .map_err(anyhow::Error::msg)?,
            views
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(legacy.load(Ordering::SeqCst), 0);
        assert_eq!(attempts(&fixture, "head")?, before);
        assert_eq!(
            object_run_preparation::get(&fixture.runtime, "project-1", &run_id)?,
            preparation
        );
        assert_eq!(
            object_task_queue::list(&fixture.runtime, "project-1")?,
            queue
        );
        assert_eq!(task_record(&fixture, "next")?.status, "planned");
    }
    Ok(())
}
