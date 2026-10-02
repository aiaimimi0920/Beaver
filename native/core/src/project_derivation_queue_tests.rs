use super::*;
use crate::{
    object_run_preparation as preparation, object_task_coarse_dispatch as coarse,
    object_task_dispatch as dispatch, object_task_queue_reorder as reorder,
    object_task_queue_view as view,
};
#[path = "project_derivation_queue_fixture.rs"]
mod fixture;
#[path = "project_derivation_queue_order_tests.rs"]
mod order;
#[path = "project_derivation_queue_rejection_tests.rs"]
mod rejection;
use fixture::{pause, QueueFixture};

#[test]
fn project_derivation_queue_preserves_history_but_requires_new_resume_after_reopen() -> Result<()> {
    let f = QueueFixture::new()?;
    let source_before = data_backup::inventory(&f.base.source)?;
    let prepared_path = f.base.temp.path().join("queue-prepared");
    let prepared = copy::prepare(f.base.request(), &prepared_path)?;
    let map = |kind: &str, id: &str| -> Result<String> {
        Ok(Rewrite(&prepared.identities).key(kind, id)?.id)
    };
    let build = map("object_task", "build")?;
    let free = map("object_task", "free")?;
    let second = map("object_task", "second")?;
    let discard = map("object_task", "discard")?;
    let prepared_before = data_backup::inventory(&prepared_path)?;
    let target = f.base.temp.path().join("queue-target");
    assembly::create(&prepared_path, &target)?;
    assembly::activate(&prepared_path, &target)?;
    let data = f.base.temp.path().join("queue-host");
    let router = ProjectStorageRouter::new(Arc::new(Mutex::new(Store::open(&data)?)));
    router.register_assembly(&prepared_path, &target, &data)?;
    let runtime = router.open_registered("derived")?;
    let before = object_tasks::snapshot(&runtime, "derived")?;
    assert_eq!(before.plan_revision, 3);
    assert!(before
        .runs
        .iter()
        .all(|run| run.baseline_version_id.is_none()));
    let current = view::get(&runtime, "derived")?;
    assert_eq!(
        current.items.iter().map(|i| &i.task_id).collect::<Vec<_>>(),
        [&free, &build, &second, &discard]
    );
    assert_eq!(current.items[1].title, "Revised build");
    assert_eq!(current.items[3].state, "cancelled");
    for item in &current.items[..3] {
        assert!(item.blockers.contains(&"paused".into()));
    }
    assert!(current.items[2].blockers.contains(&"earlierQueued".into()));
    assert_ne!(current.version, f.reordered.result.version);
    for (kind, source) in [
        (
            "object_task_queue_reorder_receipt",
            serde_json::to_value(&f.reordered)?,
        ),
        (
            "object_task_dispatch_receipt",
            serde_json::to_value(&f.unpaused)?,
        ),
        (
            "object_task_dispatch_receipt",
            serde_json::to_value(&f.free_paused)?,
        ),
        (
            "object_task_coarse_dispatch_receipt",
            serde_json::to_value(&f.coarse_unpaused)?,
        ),
    ] {
        let key = crate::project_derivation_queue_records::receipt_key(
            "original",
            source["request"]["requestId"].as_str().unwrap(),
        )?;
        let receipt: Value = runtime
            .store()
            .lock()
            .unwrap()
            .get(kind, &map(kind, &key)?)?
            .unwrap();
        assert_ne!(
            receipt["request"]["requestId"],
            source["request"]["requestId"]
        );
        let replay = match kind {
            "object_task_queue_reorder_receipt" => {
                assert_ne!(receipt["result"]["version"], source["result"]["version"]);
                assert_ne!(receipt["result"]["version"], current.version);
                assert_eq!(receipt["result"]["items"][1]["title"], "Build");
                serde_json::to_value(reorder::reorder(
                    &runtime,
                    &serde_json::from_value(receipt["request"].clone())?,
                )?)?
            }
            "object_task_coarse_dispatch_receipt" => serde_json::to_value(coarse::set_paused(
                &runtime,
                &serde_json::from_value(receipt["request"].clone())?,
            )?)?,
            _ => serde_json::to_value(dispatch::set_paused(
                &runtime,
                &serde_json::from_value(receipt["request"].clone())?,
            )?)?,
        };
        assert_eq!(replay, receipt);
        let mut conflict = receipt["request"].clone();
        let field = if kind == "object_task_queue_reorder_receipt" {
            "expectedVersion"
        } else {
            "paused"
        };
        conflict[field] = if field == "paused" {
            json!(!conflict[field].as_bool().unwrap())
        } else {
            json!(current.version)
        };
        let error = match kind {
            "object_task_queue_reorder_receipt" => {
                reorder::reorder(&runtime, &serde_json::from_value(conflict)?).unwrap_err()
            }
            "object_task_coarse_dispatch_receipt" => {
                coarse::set_paused(&runtime, &serde_json::from_value(conflict)?).unwrap_err()
            }
            _ => dispatch::set_paused(&runtime, &serde_json::from_value(conflict)?).unwrap_err(),
        };
        assert!(format!("{error:#}").contains("REQUEST_CONFLICT"));
    }
    assert_eq!(view::get(&runtime, "derived")?, current);
    assert_eq!(object_tasks::snapshot(&runtime, "derived")?, before);
    assert!(object_tasks::claim_next(&runtime, "derived", "direct-test")?.is_none());
    assert!(preparation::claim_next(&runtime, "derived", "scheduler-test")?.is_none());
    for run in &before.runs {
        assert!(preparation::get(&runtime, "derived", &run.id)?.is_none());
    }
    drop(runtime);
    router.close("derived")?;
    router.register_assembly(&prepared_path, &target, &data)?;
    let runtime = router.open_registered("derived")?;
    assert_eq!(view::get(&runtime, "derived")?, current);
    assert!(preparation::claim_next(&runtime, "derived", "reopened-test")?.is_none());
    // A new explicit resume is the only operation that makes this copied queue claimable.
    let resumed = pause(&runtime, &build, "owner-resume-copy", 3, false)?;
    assert_eq!(resumed.result.revision, 4);
    let claimed = preparation::claim_next(&runtime, "derived", "authorized-test")?.unwrap();
    assert_eq!(claimed.record().medium.id, build);
    assert_eq!(claimed.record().generation, 1);
    assert_eq!(data_backup::inventory(&f.base.source)?, source_before);
    assert_eq!(data_backup::inventory(&prepared_path)?, prepared_before);
    Ok(())
}

#[test]
fn project_derivation_queue_second_derivation_does_not_stack_pauses_on_paused_work() -> Result<()> {
    let f = QueueFixture::new()?;
    let prepared_path = f.base.temp.path().join("first-prepared");
    copy::prepare(f.base.request(), &prepared_path)?;
    let target = f.base.temp.path().join("first-target");
    assembly::create(&prepared_path, &target)?;
    assembly::activate(&prepared_path, &target)?;
    let first = ProjectStore::open(&target.join("project"), "derived")?.into_runtime();
    let snapshot = object_tasks::snapshot(&first, "derived")?;
    let controls = serde_json::to_value(&snapshot)?["dispatchControls"].clone();
    let first_source = first.project_root().to_path_buf();
    drop(first);
    let second_path = f.base.temp.path().join("second-prepared");
    copy::prepare(
        copy::Request {
            request_id: "derive-again".into(),
            source: first_source,
            source_project_id: "derived".into(),
            target_project_id: "again".into(),
        },
        &second_path,
    )?;
    let second_target = f.base.temp.path().join("second-target");
    assembly::create(&second_path, &second_target)?;
    assembly::activate(&second_path, &second_target)?;
    let second = ProjectStore::open(&second_target.join("project"), "again")?.into_runtime();
    let second_controls = serde_json::to_value(object_tasks::snapshot(&second, "again")?)?
        ["dispatchControls"]
        .clone();
    let revisions = |v: &Value| {
        let mut values: Vec<_> = v
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["revision"].as_u64().unwrap())
            .collect();
        values.sort();
        values
    };
    assert_eq!(revisions(&controls), revisions(&second_controls));
    assert!(preparation::claim_next(&second, "again", "second-test")?.is_none());
    Ok(())
}
