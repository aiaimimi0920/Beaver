use super::*;
use crate::object_tasks::{CancelPlannedReceipt, CommitReceipt, Draft, TaskDefinitionRevision};

pub(super) struct Fixture {
    pub base: objects::Fixture,
    pub commit: CommitReceipt,
    pub revision: TaskDefinitionRevision,
    pub cancel: CancelPlannedReceipt,
    pub unlocked: Draft,
}

pub(super) fn save(
    runtime: &crate::project_runtime::ProjectRuntime,
    id: &str,
    revision: u64,
    plan: Value,
) -> Result<Draft> {
    object_tasks::save_draft(
        runtime,
        &serde_json::from_value(json!({
            "projectId":runtime.project_id(), "draftId":id, "expectedRevision":revision,
            "expectedPlanRevision":object_tasks::snapshot(runtime, runtime.project_id())?.plan_revision,
            "plan":plan
        }))?,
    )
}

impl Fixture {
    pub fn new() -> Result<Self> {
        Self::create(false)
    }

    pub fn with_declarations() -> Result<Self> {
        Self::create(true)
    }

    fn create(declarations: bool) -> Result<Self> {
        let base = objects::Fixture::new()?;
        let runtime = ProjectStore::open(&base.source, "original")?.into_runtime();
        save(
            &runtime,
            "historical",
            0,
            json!({
                "objects":[{"id":"new-object","name":"New object"}],
                "tasks":[
                    {"id":"root","granularity":"coarse","title":"Root","prompt":"root original","acceptance":""},
                    {"id":"build","granularity":"medium","title":"Build","prompt":"build original","acceptance":"",
                        "objectId":base.parent.id,"parentTaskId":"root",
                        "baseline":{"basePolicy":"pinnedVersion","selectedVersionId":base.first.version_id}},
                    {"id":"shape","granularity":"fine","title":"Shape","prompt":"shape original","acceptance":"",
                        "objectId":base.parent.id,"parentTaskId":"build","stageId":"model"},
                    {"id":"discard","granularity":"medium","title":"Discard","prompt":"discard original","acceptance":"",
                        "objectId":"new-object","parentTaskId":"root","dependsOn":["build"]},
                    {"id":"discard-fine","granularity":"fine","title":"Discard fine","prompt":"discard child","acceptance":"",
                        "objectId":"new-object","parentTaskId":"discard","stageId":"model"}
                ],
                "assumptions":[{"id":"local-assumption","statement":"Keep original text root","basis":"Manual choice"}]
            }),
        )?;
        let commit = object_tasks::commit(
            &runtime,
            &serde_json::from_value(json!({
                "projectId":"original","requestId":"commit","draftId":"historical",
                "expectedDraftRevision":1,"expectedPlanRevision":0
            }))?,
        )?;
        if declarations {
            for (task, request) in [
                ("root", "early-root"),
                ("root", "early-root-again"),
                ("build", "build-confirmed"),
                ("discard", "discard-confirmed"),
            ] {
                declare(&runtime, task, request)?;
            }
        }
        save(
            &runtime,
            "stale",
            0,
            json!({"tasks":[{
                "id":"stale-task","granularity":"coarse","title":"Stale","prompt":"still editable","acceptance":"", "dependsOn":["root"]
            }]}),
        )?;
        let revision = object_tasks::revise_planned(
            &runtime,
            &serde_json::from_value(json!({
                "projectId":"original","requestId":"revise","taskId":"root","expectedTaskRevision":0,"expectedPlanRevision":1,
                "definition":{"title":"Revised root","prompt":"root original revised","acceptance":"","dependsOn":[]},
                "reason":"Manual correction"
            }))?,
        )?;
        if declarations {
            declare(&runtime, "root", "revised-root")?;
        }
        let cancel = object_tasks::cancel_planned(
            &runtime,
            &serde_json::from_value(json!({
                "projectId":"original","requestId":"cancel","taskId":"discard","expectedTaskRevision":0,"expectedPlanRevision":2
            }))?,
        )?;
        let unlocked = object_tasks::unlock_draft(
            &runtime,
            &serde_json::from_value(json!({
                "projectId":"original","requestId":"unlock","draftId":"historical","expectedRevision":2,"expectedPlanRevision":3
            }))?,
        )?;
        save(
            &runtime,
            "locked",
            0,
            json!({"tasks":[{
                "id":"locked-task","granularity":"coarse","title":"Locked","prompt":"locked original","acceptance":""
            }]}),
        )?;
        object_tasks::commit(
            &runtime,
            &serde_json::from_value(json!({
                "projectId":"original","requestId":"commit-locked","draftId":"locked","expectedDraftRevision":1,"expectedPlanRevision":3
            }))?,
        )?;
        save(
            &runtime,
            "object-task-plan",
            0,
            json!({
                "objects":[{"id":"future-object","name":"Future object"}],
                "tasks":[{"id":"future-task","granularity":"medium","title":"Future","prompt":"future original root","acceptance":"",
                    "objectId":"future-object","dependsOn":["build"]}]
            }),
        )?;
        // A legitimate metadata edit must not invalidate the historical pinned version.
        let mut parent = base.parent.clone();
        parent.category = "Changed after planning".into();
        objects::update(&runtime, &parent, "rename-after-plan")?;
        drop(runtime);
        Ok(Self {
            base,
            commit,
            revision,
            cancel,
            unlocked,
        })
    }
}

pub(super) fn declare(
    runtime: &crate::project_runtime::ProjectRuntime,
    task: &str,
    request: &str,
) -> Result<crate::object_task_planning_declaration::Declaration> {
    let snapshot = object_tasks::snapshot(runtime, runtime.project_id())?;
    let state = snapshot
        .planning_states
        .iter()
        .find(|s| s.task_id == task)
        .unwrap();
    crate::object_task_planning_declaration::declare(
        runtime,
        &serde_json::from_value(json!({
            "projectId":runtime.project_id(),"taskId":task,"requestId":request,
            "expectedPlanRevision":snapshot.plan_revision,"expectedScopeHash":state.scope_hash,
            "reason":format!("Owner reviewed original {task} scope")
        }))?,
    )
}
