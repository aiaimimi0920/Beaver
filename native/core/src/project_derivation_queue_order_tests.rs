use super::*;

#[test]
fn project_derivation_queue_preserves_tied_position_heads_despite_remapped_id_order() -> Result<()>
{
    let base = objects::Fixture::new()?;
    let runtime = ProjectStore::open(&base.source, "original")?.into_runtime();
    let mut request = base.request();
    // Select provenance that reverses the ID tie-breaker, so the regression cannot pass by chance.
    for index in 0..100 {
        request.request_id = format!("tied-order-{index}");
        if generated(&request, "object_task", "first")?
            > generated(&request, "object_task", "second")?
        {
            break;
        }
    }
    let first = generated(&request, "object_task", "first")?;
    let second = generated(&request, "object_task", "second")?;
    assert!(first > second);
    save(
        &runtime,
        "tied-plan",
        0,
        json!({"tasks":[
            {"id":"root","granularity":"coarse","title":"Root","prompt":"Root","acceptance":""},
            {"id":"first","granularity":"medium","title":"First","prompt":"First","acceptance":"",
                "objectId":base.parent.id,"parentTaskId":"root"},
            {"id":"second","granularity":"medium","title":"Second","prompt":"Second","acceptance":"",
                "objectId":base.parent.id,"parentTaskId":"root"}
        ]}),
    )?;
    object_tasks::commit(
        &runtime,
        &serde_json::from_value(json!({
            "projectId":"original","requestId":"tie-commit","draftId":"tied-plan",
            "expectedDraftRevision":1,"expectedPlanRevision":0
        }))?,
    )?;
    let entries = object_tasks::enqueue(&runtime, "original", &["second".into(), "first".into()])?;
    assert_eq!(entries[0].position, entries[1].position);
    assert_eq!(entries[0].task_id, "first");
    drop(runtime);
    let before = data_backup::inventory(&base.source)?;
    let prepared = base.temp.path().join("tie-prepared");
    copy::prepare(request, &prepared)?;
    let target = base.temp.path().join("tie-target");
    assembly::create(&prepared, &target)?;
    assembly::activate(&prepared, &target)?;
    let copied = ProjectStore::open(&target.join("project"), "derived")?.into_runtime();
    let current = view::get(&copied, "derived")?;
    assert_eq!(
        current.items.iter().map(|i| &i.task_id).collect::<Vec<_>>(),
        [&first, &second]
    );
    assert_eq!(current.items[0].blockers, ["paused"]);
    assert_eq!(current.items[1].blockers, ["earlierQueued", "paused"]);
    pause(&copied, &second, "resume-later", 1, false)?;
    assert!(preparation::claim_next(&copied, "derived", "blocked-later")?.is_none());
    pause(&copied, &first, "resume-head", 1, false)?;
    assert_eq!(
        preparation::claim_next(&copied, "derived", "authorized-head")?
            .unwrap()
            .record()
            .medium
            .id,
        first
    );
    assert_eq!(data_backup::inventory(&base.source)?, before);
    Ok(())
}

#[test]
fn project_derivation_queue_historical_tokens_preserve_linked_cas_aliases() -> Result<()> {
    let f = QueueFixture::new()?;
    let runtime = ProjectStore::open(&f.base.source, "original")?.into_runtime();
    let mut receipts = Vec::new();
    for id in ["linked-first", "linked-second"] {
        let receipt = reorder::reorder(
            &runtime,
            &reorder::Request {
                project_id: "original".into(),
                request_id: id.into(),
                expected_version: view::get(&runtime, "original")?.version,
                task_id: "free".into(),
                previous_task_id: None,
                next_task_id: Some("build".into()),
            },
        )?;
        receipts.push(receipt);
    }
    assert_eq!(
        receipts[0].result.version,
        receipts[1].request.expected_version
    );
    drop(runtime);
    let prepared_path = f.base.temp.path().join("linked-prepared");
    let prepared = copy::prepare(f.base.request(), &prepared_path)?;
    let target = f.base.temp.path().join("linked-target");
    assembly::create(&prepared_path, &target)?;
    assembly::activate(&prepared_path, &target)?;
    let runtime = ProjectStore::open(&target.join("project"), "derived")?.into_runtime();
    let mut aliases = Vec::new();
    for source in receipts {
        let key = crate::project_derivation_queue_records::receipt_key(
            "original",
            &source.request.request_id,
        )?;
        let target_key = Rewrite(&prepared.identities)
            .key("object_task_queue_reorder_receipt", &key)?
            .id;
        let receipt: reorder::Receipt = runtime
            .store()
            .lock()
            .unwrap()
            .get("object_task_queue_reorder_receipt", &target_key)?
            .unwrap();
        assert_ne!(receipt.result.version, source.result.version);
        assert_eq!(reorder::reorder(&runtime, &receipt.request)?, receipt);
        aliases.push(receipt);
    }
    assert_eq!(
        aliases[0].result.version,
        aliases[1].request.expected_version
    );
    Ok(())
}
