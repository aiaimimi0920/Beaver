use super::*;

fn archive(f: &Fixture, id: &str, frames: Value) -> Result<()> {
    f.runtime
        .store()
        .lock()
        .unwrap()
        .put("objectPreviewFrames", id, &frames)?;
    Ok(())
}

#[test]
fn publication_evidence_rechecks_both_frames_before_start_and_recovers() -> Result<()> {
    let f = fixture()?;
    let (mut publish, feedback, source, target) = historical(&f)?;
    deferred::create(&f.runtime, &feedback)?;
    approve(&f, &mut publish)?;
    let before = object_tasks::snapshot(&f.runtime, "project-1")?;
    for (id, original) in [("old-frame", source), ("current-frame", target)] {
        for damage in 0..3 {
            let mut broken = original.clone();
            match damage {
                0 => broken["frame"]["dataUrl"] = json!("data:image/png;base64,broken"),
                1 => broken["source"]["target"]["attemptId"] = json!("foreign"),
                _ => {}
            }
            archive(
                &f,
                id,
                if damage == 2 {
                    json!([])
                } else {
                    json!([broken])
                },
            )?;
            assert!(publication::preview(&f.runtime, &feedback.review).is_err());
            let error = publication::publish(&f.runtime, &publish).unwrap_err();
            assert!(error
                .to_string()
                .contains("OBJECT_PUBLICATION_FEEDBACK_EVIDENCE"));
            assert!(
                publication::list(&f.runtime, "project-1", &publish.target.task_id)?.is_empty()
            );
            assert_eq!(object_tasks::snapshot(&f.runtime, "project-1")?, before);
            archive(&f, id, json!([original.clone()]))?;
        }
    }
    let f = reopen(f)?;
    assert_eq!(
        publication::preview(&f.runtime, &feedback.review)?.digest,
        publish.preview_digest
    );
    assert_eq!(
        publication::publish(&f.runtime, &publish)?.state,
        State::Published
    );
    Ok(())
}

#[test]
fn publication_evidence_pending_retry_and_abort_survive_missing_archive() -> Result<()> {
    for abort in [false, true] {
        let f = fixture()?;
        let (mut publish, feedback, source, _) = historical(&f)?;
        deferred::create(&f.runtime, &feedback)?;
        approve(&f, &mut publish)?;
        let pending = publication::execute_with(&f.runtime, &publish, |step| {
            if step == "beforeCommit" {
                archive(&f, "old-frame", json!([]))?;
            }
            Ok(())
        })?;
        assert_eq!(pending.state, State::Applying);
        assert!(pending
            .error
            .as_ref()
            .unwrap()
            .contains("OBJECT_PUBLICATION_FEEDBACK_EVIDENCE"));
        let f = reopen(f)?;
        let retry = publication::publish(&f.runtime, &publish)?;
        assert_eq!(retry.state, State::Applying);
        assert!(retry
            .error
            .as_ref()
            .unwrap()
            .contains("OBJECT_PUBLICATION_FEEDBACK_EVIDENCE"));
        let pending = publication::list(&f.runtime, "project-1", &publish.target.task_id)?;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].state, State::Applying);
        assert!(pending[0]
            .error
            .as_ref()
            .unwrap()
            .contains("PREVIEW_SAVED_NOT_FOUND"));
        if abort {
            assert_eq!(
                publication::abort(&f.runtime, "project-1", &publish.request_id)?.state,
                State::Aborted
            );
        } else {
            archive(&f, "old-frame", json!([source]))?;
            assert_eq!(
                publication::publish(&f.runtime, &publish)?.state,
                State::Published
            );
            archive(&f, "old-frame", json!([]))?;
            assert_eq!(
                publication::publish(&f.runtime, &publish)?.state,
                State::Published
            );
        }
    }
    Ok(())
}
