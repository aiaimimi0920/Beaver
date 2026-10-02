use super::*;

#[test]
fn publication_png_pending_source_damage_reopens_retries_original_request_or_aborts() -> Result<()>
{
    for checkpoint in ["prepared", "beforeCommit"] {
        for abort in [false, true] {
            let (f, publish, source) = ready_png()?;
            let before = object_tasks::snapshot(&f.runtime, "project-1")?;
            let blob = f
                .runtime
                .files()
                .blob(&source.image.as_ref().unwrap().sha256)?;
            let bytes = std::fs::read(&blob)?;
            let op = publication::execute_with(&f.runtime, &publish, |step| {
                if step == checkpoint {
                    std::fs::write(&blob, b"damaged after preparation")?;
                }
                Ok(())
            })?;
            assert_eq!(op.state, State::Applying);
            assert!(op
                .error
                .as_ref()
                .unwrap()
                .contains("OBJECT_PUBLICATION_FEEDBACK_EVIDENCE"));
            assert_eq!(op.request, publish);
            assert_eq!(object_tasks::snapshot(&f.runtime, "project-1")?, before);
            assert!(f.object("hero")?.versions.is_empty());
            let f = reopen(f)?;
            let pending = publication::list(&f.runtime, "project-1", &publish.target.task_id)?;
            assert_eq!(pending, vec![op]);
            assert_eq!(
                publication::publish(&f.runtime, &publish)?.state,
                State::Applying
            );
            let mut changed = publish.clone();
            mapping(&mut changed).regions[1] = relocation::Region::Absent {
                source_region: 1,
                note: "Changed while retrying".into(),
            };
            assert!(publication::publish(&f.runtime, &changed)
                .unwrap_err()
                .to_string()
                .contains("OBJECT_PUBLICATION_REQUEST_CONFLICT"));
            if abort {
                assert_eq!(
                    publication::abort(&f.runtime, "project-1", &publish.request_id)?.state,
                    State::Aborted
                );
                assert_eq!(object_tasks::snapshot(&f.runtime, "project-1")?, before);
                assert!(!f.temp.path().join("preview.png").exists());
            } else {
                std::fs::write(&blob, bytes)?;
                let completed = publication::publish(&f.runtime, &publish)?;
                assert_eq!(completed.state, State::Published);
                assert_eq!(completed.request, publish);
                std::fs::write(&blob, b"damaged after publication")?;
                assert_eq!(publication::publish(&f.runtime, &publish)?, completed);
                assert_eq!(
                    publication::list(&f.runtime, "project-1", &publish.target.task_id)?,
                    vec![completed]
                );
            }
        }
    }
    Ok(())
}
