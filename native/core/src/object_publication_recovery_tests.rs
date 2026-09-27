use super::{publication_fixture::*, run_fixture};
use crate::{
    object_catalog_test_fixture::Fixture, object_run_preparation, object_tasks,
    project_storage::ProjectStore,
};
use anyhow::Result;
use std::fs;

#[test]
fn publication_reopens_each_file_boundary_and_commits_once() -> Result<()> {
    for point in ["prepared", "applied:a.txt", "beforeCommit"] {
        let f = fixture()?;
        let request = multiple_files(&f)?;
        let object = f.object("hero")?;
        let snapshot = object_tasks::snapshot(&f.runtime, "project-1")?;
        let queue = object_tasks::queue(&f.runtime, "project-1")?;
        let pending = fail_at(&f, &request, point)?;
        assert_eq!(pending.state, State::Applying);
        assert!(pending
            .error
            .as_deref()
            .unwrap()
            .contains("simulated host loss"));
        assert_eq!(f.object("hero")?, object);
        assert_eq!(object_tasks::snapshot(&f.runtime, "project-1")?, snapshot);
        assert_eq!(object_tasks::queue(&f.runtime, "project-1")?, queue);
        let Fixture { runtime, temp } = f;
        drop(runtime);
        let runtime = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
        assert_eq!(
            publication::list(&runtime, "project-1", "head")?,
            vec![pending]
        );
        assert_eq!(object_tasks::snapshot(&runtime, "project-1")?, snapshot);
        assert!(object_run_preparation::claim_next(&runtime, "project-1", "restart")?.is_none());
        let published = publication::publish(&runtime, &request)?;
        assert_eq!(published.state, State::Published);
        assert_eq!(publication::publish(&runtime, &request)?, published);
        assert_eq!(
            runtime
                .store()
                .lock()
                .unwrap()
                .list::<serde_json::Value>("object_version")?
                .len(),
            1
        );
        assert_eq!(fs::read_to_string(temp.path().join("a.txt"))?, "first");
        assert_eq!(fs::read_to_string(temp.path().join("z.txt"))?, "last");
    }
    Ok(())
}

#[test]
fn publication_abort_preserves_external_edits_and_never_owns_untouched_matching_files() -> Result<()>
{
    let f = fixture()?;
    let request = multiple_files(&f)?;
    fail_at(&f, &request, "applied:a.txt")?;
    fs::write(f.temp.path().join("a.txt"), "external")?;
    fs::write(f.temp.path().join("z.txt"), "last")?;
    let retry = publication::publish(&f.runtime, &request)?;
    assert_eq!(retry.state, State::Applying);
    assert!(retry.error.unwrap().contains("FILE_DRIFT"));
    let aborted = publication::abort(&f.runtime, "project-1", "publish")?;
    assert_eq!(aborted.state, State::Aborted);
    assert!(aborted
        .error
        .as_deref()
        .unwrap()
        .contains("EXTERNAL_CHANGES_RETAINED: a.txt"));
    assert_eq!(fs::read_to_string(f.temp.path().join("a.txt"))?, "external");
    assert_eq!(fs::read_to_string(f.temp.path().join("z.txt"))?, "last");
    assert!(!f.temp.path().join("hero.gd").exists());
    assert_eq!(
        publication::abort(&f.runtime, "project-1", "publish")?,
        aborted
    );
    assert_eq!(publication::publish(&f.runtime, &request)?, aborted);
    assert_eq!(task_record(&f, "head")?.status, "awaitingAcceptance");
    assert!(object_run_preparation::claim_next(&f.runtime, "project-1", "next")?.is_none());
    assert!(attempts(&f, "head")?.iter().all(|a| a.output.is_some()));
    Ok(())
}

#[test]
fn publication_abort_restores_the_accepted_head_and_keeps_frozen_candidate() -> Result<()> {
    let f = fixture()?;
    let version = run_fixture::capture(&f, "hero", "old", "old accepted content", vec![])?;
    run_fixture::accept(&f, "hero", &version)?;
    let object = f.object("hero")?;
    let mut request = ready(&f)?;
    request.confirm_replacement = true;
    let pending = fail_at(&f, &request, "beforeCommit")?;
    assert_eq!(pending.state, State::Applying);
    assert!(!f.temp.path().join("hero.tscn").exists());
    assert!(f.temp.path().join("hero.gd").exists());
    assert_eq!(f.object("hero")?, object);
    let Fixture { runtime, temp } = f;
    drop(runtime);
    let runtime = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
    let aborted = publication::abort(&runtime, "project-1", "publish")?;
    assert_eq!(aborted.state, State::Aborted);
    assert!(aborted.error.is_none());
    assert_eq!(
        fs::read_to_string(temp.path().join("hero.tscn"))?,
        "old accepted content"
    );
    assert!(!temp.path().join("hero.gd").exists());
    assert_eq!(
        crate::object_run_recovery::candidate::list(
            &runtime,
            "project-1",
            &request.target.attempt_id
        )?
        .len(),
        1
    );
    assert_eq!(
        publication::publish(
            &runtime,
            &Request {
                request_id: "publish-again".into(),
                ..request
            }
        )?
        .state,
        State::Published
    );
    Ok(())
}
