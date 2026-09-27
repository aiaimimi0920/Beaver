use super::{
    cancel,
    definition_fixture::{fixture, rejected, request},
};
use crate::{
    object_framework::Identity,
    object_tasks::{self, TaskDefinition},
};
use anyhow::Result;

#[test]
fn rejects_stale_versions_noops_invalid_text_and_bad_dependency_graphs() -> Result<()> {
    let f = fixture()?;
    let original = request(&f, "fine", "revise")?;
    let mut cases = vec![];
    let mut add = |code: &str, edit: fn(&mut object_tasks::RevisePlannedRequest)| {
        let mut request = original.clone();
        edit(&mut request);
        cases.push((code.to_owned(), request));
    };
    add("OBJECT_TASK_PLAN_REVISION_CONFLICT", |r| {
        r.expected_plan_revision = 0
    });
    add("OBJECT_TASK_REVISION_CONFLICT", |r| {
        r.expected_task_revision = 1
    });
    add("INVALID_OBJECT_TASK: title", |r| {
        r.definition.title = "  ".into()
    });
    add("INVALID_OBJECT_TASK: prompt", |r| {
        r.definition.prompt = "\n".into()
    });
    add("INVALID_OBJECT_TASK: acceptance", |r| {
        r.definition.acceptance = "a".repeat(10_001)
    });
    add("INVALID_OBJECT_TASK_REVISION_REASON", |r| {
        r.reason = " ".into()
    });
    add("INVALID_OBJECT_TASK_REVISION_REASON", |r| {
        r.reason = "a".repeat(2_001)
    });
    add("OBJECT_TASK_DEPENDENCY_NOT_FOUND", |r| {
        r.definition.depends_on = vec!["missing".into()]
    });
    add("OBJECT_TASK_SELF_DEPENDENCY", |r| {
        r.definition.depends_on = vec!["fine".into()]
    });
    add("DUPLICATE_OBJECT_TASK_DEPENDENCY", |r| {
        r.definition.depends_on = vec!["other".into(); 2]
    });
    add("OBJECT_TASK_DEPENDENCY_CYCLE", |r| {
        r.definition.depends_on = vec!["transitive".into(), "dependent".into()]
    });
    for (code, request) in cases {
        rejected(&f, &request, &code)?;
    }
    let task = object_tasks::get_task(&f.runtime, "project-1", "fine")?.unwrap();
    let mut noop = original;
    noop.definition = TaskDefinition::from_task(&task);
    rejected(&f, &noop, "OBJECT_TASK_DEFINITION_UNCHANGED")?;
    object_tasks::cancel_planned(&f.runtime, &cancel::request("other", "cancel", 0, 1))?;
    rejected(
        &f,
        &request(&f, "fine", "revise")?,
        "OBJECT_TASK_DEPENDENCY_CANCELLED",
    )?;
    Ok(())
}

#[test]
fn rejects_started_target_owned_descendants_and_started_or_resolved_runs() -> Result<()> {
    for (root, changed, status) in [("fine", "fine", "running"), ("coarse", "fine", "completed")] {
        let f = fixture()?;
        let request = request(&f, root, "revise")?;
        let mut task = object_tasks::get_task(&f.runtime, "project-1", changed)?.unwrap();
        task.status = status.into();
        f.runtime
            .store()
            .lock()
            .unwrap()
            .put("object_task", changed, &task)?;
        rejected(&f, &request, "OBJECT_TASK_NOT_PLANNED")?;
    }
    for resolved in [false, true] {
        let f = fixture()?;
        let request = request(&f, "coarse", "revise")?;
        let mut run = object_tasks::snapshot(&f.runtime, "project-1")?
            .runs
            .into_iter()
            .find(|r| r.medium_task_id == "medium")
            .unwrap();
        if resolved {
            run.baseline_version_id = Some("accepted-version".into());
        } else {
            run.status = "running".into();
        }
        f.runtime
            .store()
            .lock()
            .unwrap()
            .put("object_run", &run.id, &run)?;
        rejected(&f, &request, "OBJECT_TASK_RUN_NOT_PLANNED")?;
    }
    let f = fixture()?;
    object_tasks::cancel_planned(&f.runtime, &cancel::request("fine", "cancel", 0, 1))?;
    rejected(
        &f,
        &request(&f, "fine", "revise")?,
        "OBJECT_TASK_NOT_PLANNED",
    )?;
    Ok(())
}

#[test]
fn rejects_project_identity_and_fine_run_mismatches() -> Result<()> {
    let f = fixture()?;
    let mut wrong_project = request(&f, "fine", "revise")?;
    wrong_project.project_id = "other-project".into();
    rejected(&f, &wrong_project, "PROJECT_RUNTIME_MISMATCH")?;
    assert!(object_tasks::revisions(&f.runtime, "other-project", "fine").is_err());
    assert!(object_tasks::revisions(&f.runtime, "project-1", "missing").is_err());
    for changed in ["other", "dependent-child"] {
        let f = fixture()?;
        let request = request(&f, "fine", "revise")?;
        let mut task = object_tasks::get_task(&f.runtime, "project-1", changed)?.unwrap();
        task.project_id = "other-project".into();
        f.runtime
            .store()
            .lock()
            .unwrap()
            .put("object_task", changed, &task)?;
        rejected(&f, &request, "OBJECT_TASK_PROJECT_MISMATCH")?;
    }
    let request = request(&f, "fine", "revise")?;
    let mut fine = object_tasks::get_task(&f.runtime, "project-1", "fine")?.unwrap();
    let run_id = fine.run_id.clone().unwrap();
    let mut duplicate = object_tasks::get_run(&f.runtime, "project-1", &run_id)?.unwrap();
    duplicate.id = "different-run".into();
    fine.run_id = Some(duplicate.id.clone());
    if let Identity::Fine { run_id, .. } = &mut fine.identity {
        *run_id = duplicate.id.clone();
    }
    let handle = f.runtime.store();
    handle
        .lock()
        .unwrap()
        .put("object_run", &duplicate.id, &duplicate)?;
    handle.lock().unwrap().put("object_task", &fine.id, &fine)?;
    rejected(&f, &request, "OBJECT_TASK_FINE_PARENT_MISMATCH")?;
    Ok(())
}
