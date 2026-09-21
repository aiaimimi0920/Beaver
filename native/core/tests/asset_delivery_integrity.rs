#[path = "support/asset_delivery.rs"]
mod support;
use anyhow::Result;
use beaver_core::{
    asset_delivery_files as artifacts, asset_delivery_review as review, asset_stages, asset_task,
    executor::Outcome, store::Store, task_callback, task_finish,
};
use serde_json::{json, Value};
use std::{fs, sync::atomic::AtomicBool};
use support::Fixture;

#[tokio::test]
async fn invalid_files_do_not_consume_request_and_export_uses_frozen_bytes() -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    fs::write(f.workspace.join("empty.txt"), "")?;
    fs::create_dir(f.workspace.join(".GODOT"))?;
    fs::write(f.workspace.join(".GODOT/private"), "internal")?;
    fs::write(f.workspace.join("safe.txt"), "frozen")?;
    fs::File::create(f.workspace.join("huge.bin"))?.set_len(artifacts::MAX_FILE_BYTES + 1)?;
    for paths in [
        vec![],
        vec!["missing"],
        vec!["empty.txt"],
        vec!["../outside"],
        vec!["safe.txt", "safe.txt"],
        vec![".GODOT/private"],
        vec!["huge.bin"],
    ] {
        assert!(
            f.call(f.request("files", &paths)?).await.is_err(),
            "{paths:?}"
        );
        assert!(f.state()?.delivery.unwrap().pending.is_none());
        assert!(
            task_callback::inspect(&f.store.lock().unwrap(), "task", Some("files"))?["receipt"]
                .is_null()
        );
    }
    let submitted = f.call(f.request("files", &["safe.txt"])?).await?;
    let candidate = artifacts::get(
        &f.store.lock().unwrap(),
        "task",
        submitted["candidateId"].as_str().unwrap(),
    )?;
    fs::write(f.workspace.join("safe.txt"), "live")?;
    let export = artifacts::export(&f.files(), &candidate)?;
    assert!(export.starts_with(f.temp.path().join("delivery-exports").canonicalize()?));
    assert_eq!(fs::read(export.join("safe.txt"))?, b"frozen");
    assert!(artifacts::get(&f.store.lock().unwrap(), "another-task", &candidate.id).is_err());
    assert!(artifacts::read(&f.files(), &candidate, "../outside").is_err());
    fs::write(f.files().blob(&candidate.files["safe.txt"])?, "corrupt")?;
    assert!(artifacts::read(&f.files(), &candidate, "safe.txt").is_err());
    assert!(artifacts::export(&f.files(), &candidate).is_err());
    assert!(f.decide(f.decision(&candidate.id, "approve")?).is_err());
    // A broken artifact must still be rejectable, so recovery cannot deadlock.
    f.decide(f.decision(&candidate.id, "reject")?)?;
    Ok(())
}

#[tokio::test]
async fn failed_commits_leave_no_partial_candidate_or_owner_decision() -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    fs::write(f.workspace.join("design.md"), "frozen")?;
    let db = rusqlite::Connection::open(f.temp.path().join("beaver.sqlite"))?;
    db.execute_batch("CREATE TRIGGER reject_submission BEFORE INSERT ON events WHEN NEW.kind='taskCallback' BEGIN SELECT RAISE(ABORT, 'disk failure'); END;")?;
    let request = f.request("submit", &["design.md"])?;
    assert!(f.call(request.clone()).await.is_err());
    assert!(f
        .store
        .lock()
        .unwrap()
        .list::<Value>(&artifacts::kind("task"))?
        .is_empty());
    assert_eq!(f.task()?["status"], "running");
    assert_eq!(f.state()?.revision, 1);
    assert!(
        task_callback::inspect(&f.store.lock().unwrap(), "task", Some("submit"))?["receipt"]
            .is_null()
    );
    db.execute_batch("DROP TRIGGER reject_submission")?;
    let candidate = f.call(request).await?["candidateId"]
        .as_str()
        .unwrap()
        .to_owned();
    let decision = f.decision(&candidate, "approve")?;
    db.execute_batch("CREATE TRIGGER reject_decision BEFORE INSERT ON events WHEN NEW.kind='assetDeliveryDecision' BEGIN SELECT RAISE(ABORT, 'disk failure'); END;")?;
    assert!(f.decide(decision.clone()).is_err());
    assert!(review::duplicate(&f.store.lock().unwrap(), &decision)?.is_none());
    assert_eq!(f.task()?["waitingDelivery"], candidate);
    assert_eq!(f.state()?.revision, decision.expected_revision);
    db.execute_batch("DROP TRIGGER reject_decision")?;
    f.decide(decision)?;
    Ok(())
}

#[tokio::test]
async fn stale_prepared_decisions_and_future_stages_cannot_advance_workflow() -> Result<()> {
    let f = Fixture::new()?;
    f.plan().await?;
    fs::write(f.workspace.join("design.md"), "frozen")?;
    let mut wrong = f.request("submit", &["design.md"])?;
    wrong["stageId"] = json!("model");
    assert!(f.call(wrong).await.is_err());
    let request = f.request("submit", &["design.md"])?;
    let (one, two) = tokio::join!(f.call(request.clone()), f.call(request));
    let submitted = one?;
    assert_eq!(submitted, two?);
    assert_eq!(f.state()?.delivery.unwrap().history.len(), 1);
    let candidate = submitted["candidateId"].as_str().unwrap();
    let first = f.decision(candidate, "approve")?;
    let second = f.decision(candidate, "reject")?;
    let prepared = review::prepare(&f.store.lock().unwrap(), second)?;
    let verified = prepared.verify(&f.files())?;
    f.decide(first)?;
    assert!(review::decide(&mut f.store.lock().unwrap(), verified).is_err());
    assert_eq!(
        f.state()?.delivery.unwrap().approved,
        vec![candidate.to_owned()]
    );
    Ok(())
}

#[tokio::test]
async fn finalization_merges_only_current_approved_bytes_and_blocks_drift() -> Result<()> {
    for mode in ["approved", "drift", "extra", "deleted"] {
        let f = Fixture::new()?;
        f.plan().await?;
        for (index, path) in ["design.md", "model.txt"].iter().enumerate() {
            fs::write(f.workspace.join(path), "approved bytes")?;
            let submitted = f
                .call(f.request(&format!("stage-{index}"), &[path])?)
                .await?;
            f.decide(f.decision(submitted["candidateId"].as_str().unwrap(), "approve")?)?;
            f.resume(&format!("turn-{index}"))?;
        }
        let mut state = f.state()?;
        asset_stages::complete_round(&mut state)?;
        asset_task::save(&f.store.lock().unwrap(), &state)?;
        match mode {
            "drift" => fs::write(f.workspace.join("model.txt"), "unapproved")?,
            "extra" => fs::write(f.workspace.join("extra.txt"), "unreviewed")?,
            "deleted" => fs::remove_file(f.workspace.join("design.md"))?,
            _ => {}
        }
        let finished = task_finish::finish(
            &mut f.store.lock().unwrap(),
            &f.files(),
            "task",
            Outcome::Completed,
            &AtomicBool::new(false),
        )?;
        if mode == "approved" {
            assert_eq!(finished["status"], "completed", "{finished}");
            assert_eq!(
                fs::read(f.temp.path().join("project/model.txt"))?,
                b"approved bytes"
            );
        } else {
            assert_eq!(finished["status"], "failed", "{mode}: {finished}");
            assert!(!f.temp.path().join("project/model.txt").exists());
            assert!(!f.temp.path().join("project/design.md").exists());
            assert!(Store::open(f.temp.path())?
                .list::<Value>("operation")?
                .is_empty());
        }
    }
    Ok(())
}
