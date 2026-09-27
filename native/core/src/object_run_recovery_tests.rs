use super::run_fixture::{claim, fixture};
use crate::{
    object_catalog_test_fixture::Fixture,
    object_framework::Baseline,
    object_run_preparation::{self as preparation, PreparationState},
    object_run_recovery::{ContentStatus, Report, WorkspaceStatus, WriterStatus},
    object_task_queue as queue, object_tasks,
    project_storage::ProjectStore,
};
use anyhow::Result;
use rusqlite::params;
use std::fs;

#[test]
fn disposition_eligibility_requires_complete_clean_recovery_evidence() {
    let report = Report {
        records_current: true,
        writer_status: WriterStatus::StopRecorded,
        content_status: ContentStatus::Verified,
        workspace_status: WorkspaceStatus::MatchesCheckpoint,
        paused: false,
        issues: vec![],
    };
    assert!(report.can_dispose());

    let mutators: [fn(&mut Report); 6] = [
        |report: &mut Report| report.records_current = false,
        |report: &mut Report| report.writer_status = WriterStatus::Unconfirmed,
        |report: &mut Report| report.content_status = ContentStatus::Invalid,
        |report: &mut Report| report.workspace_status = WorkspaceStatus::Drifted,
        |report: &mut Report| report.paused = true,
        |report: &mut Report| report.issues.push("manual review".into()),
    ];
    for mutate in mutators {
        let mut candidate = report.clone();
        mutate(&mut candidate);
        assert!(!candidate.can_dispose());
    }
}

#[test]
fn interrupted_preparation_survives_restart_without_replay_or_a_second_writer() -> Result<()> {
    for state in [
        PreparationState::Pending,
        PreparationState::Preparing,
        PreparationState::Ready,
        PreparationState::Failed,
    ] {
        let fixture = fixture()?;
        let claimed = claim(
            &fixture,
            (state != PreparationState::Failed).then_some(Baseline::Empty {}),
        )?;
        let run_id = claimed.record().id.clone();
        let workspace = fixture.runtime.files().workspace(&run_id)?;
        match state {
            PreparationState::Preparing => {
                let error = preparation::prepare_with(&fixture.runtime, claimed, || {
                    fs::write(workspace.join("retained.txt"), "interrupted")?;
                    anyhow::bail!("injected interruption")
                })
                .unwrap_err();
                assert!(error.to_string().contains("injected interruption"));
            }
            PreparationState::Ready => {
                preparation::prepare(&fixture.runtime, claimed)?;
            }
            _ => drop(claimed),
        }
        let before = preparation::get(&fixture.runtime, "project-1", &run_id)?.unwrap();
        assert_eq!(before.state, state);
        super::queue_fixture::enqueue(&fixture, &["head", "independent"])?;
        let Fixture { runtime, temp } = fixture;
        drop(runtime);
        let reopened = ProjectStore::open(temp.path(), "project-1")?.into_runtime();
        assert_eq!(
            preparation::get(&reopened, "project-1", &run_id)?,
            Some(before)
        );
        assert_eq!(
            workspace.exists(),
            matches!(state, PreparationState::Preparing | PreparationState::Ready)
        );
        if state == PreparationState::Preparing {
            assert_eq!(
                fs::read_to_string(workspace.join("retained.txt"))?,
                "interrupted"
            );
        }
        let independent = preparation::claim_next(&reopened, "project-1", "restarted")?.unwrap();
        assert_eq!(independent.record().medium.id, "independent");
        assert!(preparation::claim_next(&reopened, "project-1", "second")?.is_none());
        let task = object_tasks::get_task(&reopened, "project-1", "work")?.unwrap();
        assert_eq!(
            task.status,
            if state == PreparationState::Failed {
                "failed"
            } else {
                "queued"
            }
        );
    }
    Ok(())
}

#[test]
fn changed_claim_or_revisions_reject_the_file_completion_and_retain_evidence() -> Result<()> {
    for (kind, path, expected) in [
        (
            "object_task",
            "$.revision",
            "OBJECT_RUN_PREPARATION_REVISION_CONFLICT",
        ),
        (
            "object_run",
            "$.revision",
            "OBJECT_RUN_PREPARATION_REVISION_CONFLICT",
        ),
        (
            "object_task_queue",
            "$.generation",
            "OBJECT_TASK_STALE_CLAIM",
        ),
    ] {
        let fixture = fixture()?;
        let claimed = claim(&fixture, Some(Baseline::Empty {}))?;
        let run_id = claimed.record().id.clone();
        let workspace = fixture.runtime.files().workspace(&run_id)?;
        let id = match kind {
            "object_run" => run_id.as_str(),
            "object_task_queue" => "project-1:work",
            _ => "work",
        };
        let error = preparation::prepare_with(&fixture.runtime, claimed, || {
            assert!(workspace.is_dir());
            let handle = fixture.runtime.store();
            let store = handle.try_lock().expect("file completion must run outside the Store lock");
            store.connection.execute(
                "UPDATE entities SET value=json_set(value,?,json_extract(value,?)+1) WHERE kind=? AND id=?",
                params![path, path, kind, id],
            )?;
            fs::write(workspace.join("retained.txt"), "prepared")?;
            Ok(())
        }).unwrap_err();
        assert!(format!("{error:#}").contains(expected), "{kind}: {error:#}");
        assert_eq!(
            preparation::get(&fixture.runtime, "project-1", &run_id)?
                .unwrap()
                .state,
            PreparationState::Preparing
        );
        assert_eq!(
            fs::read_to_string(workspace.join("retained.txt"))?,
            "prepared"
        );
        super::queue_fixture::enqueue(&fixture, &["head"])?;
        assert!(preparation::claim_next(&fixture.runtime, "project-1", "second")?.is_none());
    }
    Ok(())
}

#[test]
fn metadata_completion_and_cancellation_cannot_discard_prepared_work() -> Result<()> {
    for ready in [false, true] {
        let fixture = fixture()?;
        let claimed = claim(&fixture, Some(Baseline::Empty {}))?;
        let record = if ready {
            preparation::prepare(&fixture.runtime, claimed)?
        } else {
            claimed.record().clone()
        };
        let before = object_tasks::snapshot(&fixture.runtime, "project-1")?;
        for outcome in [false, true] {
            let error = queue::finish_claim(
                &fixture.runtime,
                "project-1",
                "work",
                "worker",
                &record.claim_token,
                record.generation,
                outcome,
            )
            .unwrap_err();
            assert!(error
                .to_string()
                .contains("OBJECT_RUN_DISPOSITION_REQUIRED"));
        }
        let error = queue::cancel_claim(
            &fixture.runtime,
            "project-1",
            "work",
            "worker",
            &record.claim_token,
            record.generation,
        )
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("OBJECT_RUN_DISPOSITION_REQUIRED"));
        assert_eq!(
            object_tasks::snapshot(&fixture.runtime, "project-1")?,
            before
        );
        assert_eq!(
            preparation::get(&fixture.runtime, "project-1", &record.id)?,
            Some(record)
        );
    }
    Ok(())
}
