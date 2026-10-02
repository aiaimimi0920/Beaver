use super::*;
use crate::object_task_planning_fixture::{command, question};
use crate::project_runtime::ProjectRuntime;

pub(super) struct Fixture {
    pub base: objects::Fixture,
    pub adopted: planning::Session,
    pub answer: planning::AnswerRequest,
}

pub(super) fn start(runtime: &ProjectRuntime, draft: &str) -> Result<planning::Session> {
    let revision =
        object_tasks::get_draft(runtime, runtime.project_id(), draft)?.map_or(0, |d| d.revision);
    let input = planning::StartRequest {
        project_id: runtime.project_id().into(),
        request_id: format!("start-{draft}"),
        draft_id: draft.into(),
        expected_draft_revision: revision,
        expected_plan_revision: object_tasks::snapshot(runtime, runtime.project_id())?
            .plan_revision,
        goal: "保留 original / start-adopted / planning:start-adopted 叙述原文".into(),
        acceptance: "A usable independent plan".into(),
        ask_ratio: 30,
    };
    Ok(planning::start(runtime, &input)?.session)
}

pub(super) fn current(runtime: &ProjectRuntime, draft: &str) -> Result<planning::Session> {
    Ok(planning::get(runtime, runtime.project_id(), draft)?.unwrap())
}

pub(super) fn save(runtime: &ProjectRuntime, draft: &str, plan: Value) -> Result<()> {
    object_tasks::save_draft(
        runtime,
        &serde_json::from_value(json!({
            "projectId":runtime.project_id(),"draftId":draft,"expectedRevision":0,
            "expectedPlanRevision":object_tasks::snapshot(runtime,runtime.project_id())?.plan_revision,
            "plan":plan
        }))?,
    )?;
    Ok(())
}

pub(super) fn commit(runtime: &ProjectRuntime, draft: &str, request: &str) -> Result<()> {
    let draft = object_tasks::get_draft(runtime, runtime.project_id(), draft)?.unwrap();
    object_tasks::commit(
        runtime,
        &serde_json::from_value(json!({
            "projectId":runtime.project_id(),"requestId":request,"draftId":draft.id,
            "expectedDraftRevision":draft.revision,"expectedPlanRevision":draft.plan_revision
        }))?,
    )?;
    Ok(())
}

pub(super) fn proposal(task: &str) -> object_tasks::PlanProposal {
    serde_json::from_value(json!({
        "tasks":[{"id":task,"granularity":"coarse","title":"AI candidate",
            "prompt":"Do not replace original start-adopted in this text","acceptance":"Review"}],
        "assumptions":[{"id":format!("assumption-{task}"),"statement":"original assumption",
            "basis":"Reviewable reasoning"}]
    }))
    .unwrap()
}

impl Fixture {
    pub fn new() -> Result<Self> {
        let base = objects::Fixture::new()?;
        let runtime = ProjectStore::open(&base.source, "original")?.into_runtime();
        let manual = json!({"tasks":[
            {"id":"root","granularity":"coarse","title":"Root","prompt":"Old root","acceptance":""},
            {"id":"build","granularity":"medium","title":"Build","prompt":"Old build","acceptance":"",
                "objectId":base.parent.id,"parentTaskId":"root",
                "baseline":{"basePolicy":"pinnedVersion","selectedVersionId":base.first.version_id}}
        ],"assumptions":[{"id":"shared","statement":"First choice","basis":"Manual"}]});
        save(&runtime, "manual", manual.clone())?;
        commit(&runtime, "manual", "commit-manual")?;
        let started = start(&runtime, "adopted")?;
        let key = planning::RoundKey::from(&started);
        round::bind(&runtime, &key, "old-thread", Some("old-turn"))?;
        round::questions(
            &runtime,
            &key,
            &json!({"questions":[question("auto",10),question("user",90)]}),
        )?;
        let waiting = current(&runtime, "adopted")?;
        let answer = planning::AnswerRequest {
            project_id: "original".into(),
            session_id: waiting.id.clone(),
            request_id: "answer-adopted".into(),
            expected_revision: waiting.revision,
            answers: [("user".into(), "  original user answer  ".into())].into(),
        };
        let answered = planning::answer(&runtime, &answer)?.session;
        round::bind(
            &runtime,
            &planning::RoundKey::from(&answered),
            "history-thread",
            Some("history-turn"),
        )?;
        round::propose(
            &runtime,
            &planning::RoundKey::from(&answered),
            proposal("ai-task"),
        )?;
        let adopted = planning::adopt(
            &runtime,
            &command(&current(&runtime, "adopted")?, "adopt-history"),
        )?;
        commit(&runtime, "adopted", "commit-ai")?;
        let mut changed = manual;
        changed["assumptions"][0]["statement"] = json!("Later choice with same ID");
        save(&runtime, "assumption-history", changed)?;
        commit(&runtime, "assumption-history", "commit-assumptions")?;
        for index in 0..2 {
            object_tasks::revise_planned(
                &runtime,
                &serde_json::from_value(json!({
                    "projectId":"original","requestId":format!("revise-{index}"),"taskId":"root",
                    "expectedTaskRevision":index,"expectedPlanRevision":3+index,
                    "definition":{"title":format!("Root {index}"),"prompt":"New root","acceptance":"","dependsOn":[]},
                    "reason":"Later edit"
                }))?,
            )?;
        }
        object_tasks::cancel_planned(
            &runtime,
            &serde_json::from_value(json!({
                "projectId":"original","requestId":"cancel-build","taskId":"build",
                "expectedTaskRevision":0,"expectedPlanRevision":5
            }))?,
        )?;
        save(
            &runtime,
            "object-task-plan",
            json!({"tasks":[{
                "id":"baseline-task","granularity":"coarse","title":"Baseline","prompt":"original baseline","acceptance":""
            }]}),
        )?;
        let proposed = start(&runtime, "object-task-plan")?;
        round::propose(
            &runtime,
            &planning::RoundKey::from(&proposed),
            proposal("future-task"),
        )?;
        let waiting = start(&runtime, "waiting")?;
        round::bind(
            &runtime,
            &planning::RoundKey::from(&waiting),
            "waiting-thread",
            Some("waiting-turn"),
        )?;
        round::questions(
            &runtime,
            &planning::RoundKey::from(&waiting),
            &json!({"questions":[question("next",90)]}),
        )?;
        let cancelled = start(&runtime, "cancelled")?;
        let cancelled = planning::cancel(&runtime, &command(&cancelled, "cancel-once"))?;
        planning::cancel(&runtime, &command(&cancelled, "cancel-twice"))?;
        for (draft, status) in [
            ("failed", planning::Status::Failed),
            ("interrupted", planning::Status::Interrupted),
        ] {
            let session = start(&runtime, draft)?;
            round::finish(
                &runtime,
                &planning::RoundKey::from(&session),
                status,
                "historical error original",
            )?;
        }
        let mut child = base.child.clone();
        child.category = "Changed after historical context".into();
        objects::update(&runtime, &child, "metadata-after-planning")?;
        drop(runtime);
        Ok(Self {
            base,
            adopted,
            answer,
        })
    }
}
