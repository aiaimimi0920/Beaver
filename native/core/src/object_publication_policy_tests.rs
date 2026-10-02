use super::{object_candidate_review::final_output_with, publication_fixture::*, run_fixture};
use crate::{
    object_attempt::{self, State as AttemptState},
    object_attempt_checks as checks,
    object_attempt_view::{self, Target},
    object_catalog::ObjectReference,
    object_run_recovery::{self as recovery, candidate, resume},
};
use anyhow::Result;
use serde_json::json;
use std::{fs, sync::atomic::AtomicBool};

#[test]
fn publication_compares_historical_output_with_current_head_and_requires_replacement_consent(
) -> Result<()> {
    let f = fixture()?;
    let old = run_fixture::capture(&f, "hero", "old", "old", vec![])?;
    run_fixture::accept(&f, "hero", &old)?;
    let current = run_fixture::capture(&f, "hero", "current", "current", vec![])?;
    run_fixture::accept(&f, "hero", &current)?;
    mutate(
        &f,
        "object_task",
        "head",
        "/identity/baseline",
        json!({"basePolicy":"pinnedVersion", "selectedVersionId":old}),
    )?;
    let review_request = final_output_with(&f, |root| {
        fs::write(root.join("hero.tscn"), "candidate from historical baseline")?;
        Ok(())
    })?;
    let mut req = request(&f, &review_request)?;
    let preview = publication::preview(&f.runtime, &review(&req))?;
    assert_eq!(preview.baseline_version_id, Some(old));
    assert_eq!(preview.accepted_version_id, Some(current));
    assert!(preview.replacement_required);
    let path = preview
        .paths
        .iter()
        .find(|p| p.path == "hero.tscn")
        .unwrap();
    assert_eq!(
        path.before,
        crate::files::file_hash(&f.temp.path().join("hero.tscn"))?
    );
    assert!(publication::publish(&f.runtime, &req)
        .unwrap_err()
        .to_string()
        .contains("CONFIRMATION_REQUIRED"));
    assert_eq!(f.count("object_publication")?, 0);
    assert_eq!(
        fs::read_to_string(f.temp.path().join("hero.tscn"))?,
        "current"
    );
    req.confirm_replacement = true;
    let published = publication::publish(&f.runtime, &req)?;
    assert_eq!(published.state, State::Published);
    assert_eq!(
        fs::read_to_string(f.temp.path().join("hero.tscn"))?,
        "candidate from historical baseline"
    );
    assert_eq!(f.object("hero")?.versions.len(), 3);
    Ok(())
}

#[test]
fn publication_rejects_foreign_outputs_and_changes_to_fixed_references() -> Result<()> {
    for mode in ["foreign", "reference-change", "reference-preserved"] {
        let f = fixture()?;
        let reference = run_fixture::capture(&f, "rival", "rival", "fixed reference", vec![])?;
        run_fixture::accept(&f, "rival", &reference)?;
        if mode != "foreign" {
            let own = run_fixture::capture(
                &f,
                "hero",
                "hero",
                "own",
                vec![ObjectReference {
                    project_id: "project-1".into(),
                    object_id: "rival".into(),
                    version_id: Some(reference),
                }],
            )?;
            run_fixture::accept(&f, "hero", &own)?;
            mutate(
                &f,
                "object_task",
                "head",
                "/identity/baseline",
                json!({"basePolicy":"latestAccepted"}),
            )?;
        }
        let candidate = final_output_with(&f, |root| {
            if mode != "reference-preserved" {
                fs::write(root.join("rival.tscn"), "unauthorized")?;
            }
            Ok(())
        })?;
        candidate::prepare(&f.runtime, &candidate)?;
        let target = publication::ReviewTarget {
            project_id: candidate.project_id.clone(),
            target: candidate.target.clone(),
            review_request_id: candidate.request_id.clone(),
        };
        if mode == "reference-preserved" {
            let preview = publication::preview(&f.runtime, &target)?;
            assert!(preview.files.iter().all(|f| f.path != "rival.tscn"));
            assert!(preview.paths.iter().all(|p| p.path != "rival.tscn"));
            assert_eq!(
                publication::publish(&f.runtime, &request(&f, &candidate)?)?.state,
                State::Published
            );
            assert_eq!(f.object("hero")?.references[0].object_id, "rival");
        } else {
            let error = publication::preview(&f.runtime, &target)
                .unwrap_err()
                .to_string();
            assert!(
                error.contains(if mode == "foreign" {
                    "FOREIGN_FILE"
                } else {
                    "FIXED_REFERENCE_CHANGED"
                }),
                "{error}"
            );
            assert_eq!(f.count("object_publication")?, 0);
        }
        assert_eq!(
            fs::read_to_string(f.temp.path().join("rival.tscn"))?,
            "fixed reference"
        );
    }
    Ok(())
}

#[test]
fn publication_requires_recorded_disposition_of_rework_feedback() -> Result<()> {
    let f = fixture()?;
    let original = ready(&f)?;
    let verification = super::object_recovery_verification::request(&f, "verify-rework")?;
    recovery::verify(&f.runtime, &verification, false)?;
    let rework = resume::Request {
        project_id: "project-1".into(),
        request_id: "rework".into(),
        target: recovery::get(&f.runtime, "project-1", "head")?
            .unwrap()
            .target,
        verification_request_id: verification.request_id,
        advance: None,
        rework: Some(resume::rework::Approval {
            review_request_id: original.review_request_id,
            attempt_id: original.target.attempt_id,
            fine_task_id: "fine-b".into(),
            feedback: "Reduce movement speed".into(),
            image: None,
            preview_frame: None,
            relocation: None,
        }),
    };
    let (_, lease) = resume::execute(&f.runtime, &rework, &AtomicBool::new(false))?;
    let lease = lease.unwrap();
    fs::write(
        workspace(&f, lease.record())?.join("hero.gd"),
        "extends Node\nvar speed = 1\n",
    )?;
    let target = Target::from_record(lease.record());
    object_attempt::finish(
        &f.runtime,
        lease,
        AttemptState::AwaitingGate,
        None,
        &AtomicBool::new(false),
    )?;
    checks::run(
        &f.runtime,
        &checks::Request {
            project_id: "project-1".into(),
            request_id: "check-rework".into(),
            target: target.clone(),
        },
    )?;
    let mut req = request(
        &f,
        &candidate::Request {
            project_id: "project-1".into(),
            request_id: "review-rework".into(),
            target,
            check_request_id: "check-rework".into(),
        },
    )?;
    let preview = publication::preview(&f.runtime, &review(&req))?;
    assert_eq!(preview.feedback.len(), 1);
    assert_eq!(preview.feedback[0].feedback, "Reduce movement speed");
    assert!(publication::publish(&f.runtime, &req)
        .unwrap_err()
        .to_string()
        .contains("FEEDBACK_UNRESOLVED"));
    assert!(!f.temp.path().join("hero.gd").exists());
    req.feedback.push(publication::FeedbackDecision {
        request_id: rework.request_id,
        resolution: publication::Resolution::Resolved,
        note: "Confirmed speed reduced to 1".into(),
        final_relocation: None,
    });
    let published = publication::publish(&f.runtime, &req)?;
    assert_eq!(published.state, State::Published);
    assert_eq!(published.request.feedback, req.feedback);
    assert_eq!(
        object_attempt_view::list_with_details(&f.runtime, &req.target.run_id)?.len(),
        3
    );
    Ok(())
}

#[test]
fn publication_detects_catalog_and_live_file_drift_before_preparing() -> Result<()> {
    for mode in ["catalog", "files", "blob"] {
        let f = fixture()?;
        let req = ready(&f)?;
        if mode == "catalog" {
            mutate(&f, "object", "hero", "/revision", json!(99))?;
        } else if mode == "files" {
            fs::write(
                f.temp.path().join("hero.gd"),
                "extends Node\nvar speed = 3\n",
            )?;
        } else {
            let preview = publication::preview(&f.runtime, &review(&req))?;
            fs::write(
                f.runtime
                    .files()
                    .blob(preview.paths[0].after.as_ref().unwrap())?,
                "changed blob",
            )?;
        }
        assert!(publication::publish(&f.runtime, &req).is_err());
        assert_eq!(f.count("object_publication")?, 0);
        assert_eq!(task_record(&f, "head")?.status, "awaitingAcceptance");
        assert_eq!(f.object("hero")?.versions.len(), 0);
    }
    Ok(())
}
