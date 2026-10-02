use super::*;

#[test]
fn publication_final_relocation_pending_target_damage_reopens_retries_or_aborts() -> Result<()> {
    for step in ["prepared", "beforeCommit"] {
        for abort in [false, true] {
            let (f, publish, target) = ready_final()?;
            let before = object_tasks::snapshot(&f.runtime, "project-1")?;
            let pending = publication::execute_with(&f.runtime, &publish, |name| {
                if name == step {
                    let mut broken = target.clone();
                    broken["frame"]["dataUrl"] = json!("data:image/png;base64,broken");
                    f.runtime.store().lock().unwrap().put(
                        "objectPreviewFrames",
                        "final-frame",
                        &json!([broken]),
                    )?;
                }
                Ok(())
            })?;
            assert_eq!(pending.state, State::Applying);
            assert!(pending
                .error
                .as_ref()
                .unwrap()
                .contains("RELOCATION_TARGET_CHANGED"));
            assert_eq!(object_tasks::snapshot(&f.runtime, "project-1")?, before);
            let f = reopen(f)?;
            f.runtime.store().lock().unwrap().put(
                "objectPreviewFrames",
                "final-frame",
                &json!([]),
            )?;
            assert_eq!(
                publication::publish(&f.runtime, &publish)?.state,
                State::Applying
            );
            assert_eq!(
                publication::list(&f.runtime, "project-1", &publish.target.task_id)?.len(),
                1
            );
            if abort {
                assert_eq!(
                    publication::abort(&f.runtime, "project-1", &publish.request_id)?.state,
                    State::Aborted
                );
                assert_eq!(object_tasks::snapshot(&f.runtime, "project-1")?, before);
                assert!(!f.temp.path().join("preview.png").exists());
            } else {
                f.runtime.store().lock().unwrap().put(
                    "objectPreviewFrames",
                    "final-frame",
                    &json!([target]),
                )?;
                let op = publication::publish(&f.runtime, &publish)?;
                assert_eq!(op.state, State::Published);
                f.runtime.store().lock().unwrap().put(
                    "objectPreviewFrames",
                    "final-frame",
                    &json!([]),
                )?;
                assert_eq!(publication::publish(&f.runtime, &publish)?, op);
            }
        }
    }
    Ok(())
}

fn legacy_journal(f: &Fixture, request: &Request) -> Result<Request> {
    let store = f.runtime.store();
    let store = store.lock().unwrap();
    let mut saved = store.list::<Value>("object_publication")?.remove(0);
    for item in saved["operation"]["preview"]["feedback"]
        .as_array_mut()
        .unwrap()
    {
        item.as_object_mut()
            .unwrap()
            .remove("relocationRequirement");
        item.as_object_mut().unwrap().remove("origin");
    }
    for decision in saved["operation"]["request"]["feedback"]
        .as_array_mut()
        .unwrap()
    {
        decision.as_object_mut().unwrap().remove("finalRelocation");
    }
    let mut preview: publication::Preview =
        serde_json::from_value(saved["operation"]["preview"].clone())?;
    preview.digest.clear();
    let digest = crate::framework_checks::digest(&(
        preview,
        saved["review"]["sourceDigest"].as_str().unwrap(),
    ))?;
    saved["operation"]["preview"]["digest"] = json!(digest);
    saved["operation"]["request"]["previewDigest"] = json!(digest);
    let key = crate::framework_checks::digest(&(&request.project_id, &request.request_id))?;
    store.put("object_publication", &key, &saved)?;
    Ok(serde_json::from_value(
        saved["operation"]["request"].clone(),
    )?)
}

#[test]
fn publication_final_relocation_schema_one_terminal_replay_and_legacy_pending_remain_safe(
) -> Result<()> {
    for terminal in [false, true] {
        let (f, publish, _) = ready_final()?;
        if terminal {
            publication::publish(&f.runtime, &publish)?;
        } else {
            fail_at(&f, &publish, "prepared")?;
        }
        let legacy = legacy_journal(&f, &publish)?;
        let f = reopen(f)?;
        let op = publication::publish(&f.runtime, &legacy)?;
        assert_eq!(op.schema_version, 1);
        if terminal {
            assert_eq!(op.state, State::Published);
            assert!(op
                .request
                .feedback
                .iter()
                .all(|d| d.final_relocation.is_none()));
        } else {
            assert_eq!(op.state, State::Applying);
            assert!(op
                .error
                .unwrap()
                .contains("OBJECT_PUBLICATION_SOURCE_CHANGED"));
            assert!(f.object("hero")?.versions.is_empty());
            assert_eq!(
                publication::abort(&f.runtime, "project-1", &legacy.request_id)?.state,
                State::Aborted
            );
        }
    }
    Ok(())
}
