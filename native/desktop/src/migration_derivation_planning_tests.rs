use super::*;
use beaver_core::{
    object_task_planning as planning, object_task_planning_round as round,
    object_task_planning_service::Service, object_tasks, project_runtime::ProjectRuntime,
};
use std::sync::atomic::{AtomicUsize, Ordering};

fn start(runtime: &ProjectRuntime, draft: &str) -> Result<planning::Session> {
    Ok(planning::start(
        runtime,
        &serde_json::from_value(json!({
            "projectId":"original","requestId":format!("start-{draft}"),"draftId":draft,
            "expectedDraftRevision":0,"expectedPlanRevision":0,"goal":"Preserve original narrative",
            "acceptance":"Reviewable plan","askRatio":30
        }))?,
    )?
    .session)
}

fn command(session: &planning::Session, request: &str) -> planning::SessionRequest {
    planning::SessionRequest {
        project_id: session.project_id.clone(),
        session_id: session.id.clone(),
        request_id: request.into(),
        expected_revision: session.revision,
    }
}

fn proposal(task: &str) -> object_tasks::PlanProposal {
    serde_json::from_value(json!({"tasks":[{
        "id":task,"granularity":"coarse","title":"AI candidate","prompt":"Continue explicitly","acceptance":"Review"
    }]})).unwrap()
}

fn park(runtime: &ProjectRuntime, session: &planning::Session) -> Result<planning::Session> {
    round::bind(
        runtime,
        &planning::RoundKey::from(session),
        "historical-thread",
        Some("historical-turn"),
    )?;
    round::questions(
        runtime,
        &planning::RoundKey::from(session),
        &json!({"questions":[{
            "id":"style","question":"Choose a style?","importance":90,
            "recommended":"Stylized","reason":"Fits the brief",
            "options":[{"label":"Stylized"},{"label":"Realistic"}]
        }]}),
    )?;
    Ok(planning::get(runtime, "original", &session.input.draft_id)?.unwrap())
}

fn counted_service(count: &Arc<AtomicUsize>) -> Service {
    let invoked = count.clone();
    Service::new(
        Arc::new(move |runtime, key| {
            round::context(runtime, key)?;
            invoked.fetch_add(1, Ordering::SeqCst);
            anyhow::bail!("CONTROLLED_PLANNING_PREPARATION_FAILURE")
        }),
        Arc::new(|_| {}),
        2,
    )
}

fn api(f: &Fixture, service: &Service, method: &str, input: Value) -> Result<Value> {
    crate::object_task_planning_runtime::call(service, &f.router, method, input)
}

fn tasks(f: &Fixture, method: &str, input: Value) -> Result<Value> {
    crate::data_dispatch::call_object_tasks_for_test(&f.router, method, input)
        .map_err(anyhow::Error::msg)
}

async fn wait_failed(f: &Fixture, service: &Service, project: &str, draft: &str) -> Result<Value> {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let session = api(
                f,
                service,
                "objectTaskPlanning.get",
                json!({"projectId":project,"draftId":draft}),
            )?;
            if session["status"] == "failed" {
                return Ok(session);
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await?
}

#[tokio::test]
async fn migration_derivation_planning_production_api_only_launches_explicit_new_rounds(
) -> Result<()> {
    let _operation = super::super::super::TEST_OPERATION.lock().unwrap();
    let f = Fixture::new()?;
    let runtime = ProjectStore::open(&f.source, "original")?.into_runtime();
    park(&runtime, &start(&runtime, "waiting")?)?;
    let proposed = start(&runtime, "object-task-plan")?;
    round::propose(
        &runtime,
        &planning::RoundKey::from(&proposed),
        proposal("candidate"),
    )?;
    let history = park(&runtime, &start(&runtime, "history")?)?;
    let history = planning::answer(
        &runtime,
        &planning::AnswerRequest {
            project_id: "original".into(),
            session_id: history.id,
            request_id: "history-answer".into(),
            expected_revision: history.revision,
            answers: [("style".into(), "User style".into())].into(),
        },
    )?
    .session;
    round::propose(
        &runtime,
        &planning::RoundKey::from(&history),
        proposal("historical-task"),
    )?;
    let history = planning::get(&runtime, "original", "history")?.unwrap();
    planning::adopt(&runtime, &command(&history, "history-adopt"))?;
    let cancelled = start(&runtime, "cancelled")?;
    planning::cancel(&runtime, &command(&cancelled, "history-cancel"))?;
    drop(runtime);
    let source_before = data_backup::inventory(&f.source)?;
    let inspection = f.api(
        "migration.inspectDerivationSource",
        json!({"source":f.source}),
    )?;
    let project = inspection["request"]["targetProjectId"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut request = inspection["request"].clone();
    request["preparation"] = json!(f.preparation);
    f.api("migration.prepareDerivation", request)?;
    let preparation_before = data_backup::inventory(&f.preparation)?;
    f.api("migration.assembleDerivation", f.paths())?;
    f.api("migration.activateAssembly", f.paths())?;
    assert_eq!(
        f.api("migration.registerAssembly", f.paths())?["runtimeReady"],
        true
    );
    let count = Arc::new(AtomicUsize::new(0));
    let service = counted_service(&count);
    let result: Result<()> = async {
        let runtime = f.router.runtime_for_project(&project)?;
        let receipts = runtime.store().lock().unwrap().list::<Value>("object_task_planning_receipt")?;
        drop(runtime);
        for receipt in &receipts {
            api(&f,&service,&format!("objectTaskPlanning.{}",receipt["operation"].as_str().unwrap()),receipt["request"].clone())?;
        }
        let waiting = api(&f,&service,"objectTaskPlanning.get",json!({"projectId":project,"draftId":"waiting"}))?;
        assert_eq!(waiting["status"],"awaitingInput");
        assert_eq!(waiting["threadId"],"historical-thread");
        assert_ne!(waiting["id"],"start-waiting");
        f.router.close(&project)?;
        f.router.open_registered(&project)?;
        assert_eq!(api(&f,&service,"objectTaskPlanning.get",json!({"projectId":project,"draftId":"waiting"}))?,waiting);
        assert_eq!(count.load(Ordering::SeqCst),0);
        let answer = json!({"projectId":project,"sessionId":waiting["id"],"requestId":"explicit-answer",
            "expectedRevision":waiting["revision"],"answers":{"style":"Chosen explicitly"}});
        let answered = api(&f,&service,"objectTaskPlanning.answer",answer.clone())?;
        assert_eq!(answered["round"],2);
        assert_ne!(answered["roundId"],waiting["roundId"]);
        assert!(answered["threadId"].is_null() && answered["turnId"].is_null());
        api(&f,&service,"objectTaskPlanning.answer",answer.clone())?;
        let failed = wait_failed(&f,&service,&project,"waiting").await?;
        assert_eq!(count.load(Ordering::SeqCst),1);
        assert_eq!(api(&f,&service,"objectTaskPlanning.answer",answer)?,failed);
        let mut new_start = failed["input"].clone();
        new_start["requestId"] = json!("explicit-retry");
        api(&f,&service,"objectTaskPlanning.start",new_start.clone())?;
        api(&f,&service,"objectTaskPlanning.start",new_start)?;
        wait_failed(&f,&service,&project,"waiting").await?;
        assert_eq!(count.load(Ordering::SeqCst),2);
        let proposed: planning::Session = serde_json::from_value(api(&f,&service,"objectTaskPlanning.get",
            json!({"projectId":project,"draftId":"object-task-plan"}))?)?;
        let adopt = serde_json::to_value(command(&proposed,"adopt-copy"))?;
        let adopted = api(&f,&service,"objectTaskPlanning.adopt",adopt.clone())?;
        assert_eq!(api(&f,&service,"objectTaskPlanning.adopt",adopt)?,adopted);
        let mut draft = tasks(&f,"objectTask.getDraft",json!({"projectId":project,"draftId":"object-task-plan"}))?;
        draft["plan"]["tasks"][0]["prompt"] = json!("Edited via production draft API");
        let saved = tasks(&f,"objectTask.saveDraft",json!({"projectId":project,"draftId":draft["id"],
            "expectedRevision":draft["revision"],"expectedPlanRevision":0,"plan":draft["plan"]}))?;
        tasks(&f,"objectTask.commit",json!({"projectId":project,"requestId":"commit-copy",
            "draftId":saved["id"],"expectedDraftRevision":saved["revision"],"expectedPlanRevision":0}))?;
        assert_eq!(tasks(&f,"objectTask.queue",json!({"projectId":project}))?,json!([]));
        assert_eq!(tasks(&f,"objectTask.claim",json!({"projectId":project,"owner":"planning-test"}))?,Value::Null);
        assert_eq!(count.load(Ordering::SeqCst),2);
        Ok(())
    }.await;
    service.shutdown().await?;
    result?;
    f.router.close(&project)?;
    f.router.open_registered(&project)?;
    let restarted = counted_service(&count);
    let result: Result<()> = (|| {
        let runtime = f.router.runtime_for_project(&project)?;
        let receipts = runtime
            .store()
            .lock()
            .unwrap()
            .list::<Value>("object_task_planning_receipt")?;
        drop(runtime);
        for receipt in receipts {
            api(
                &f,
                &restarted,
                &format!(
                    "objectTaskPlanning.{}",
                    receipt["operation"].as_str().unwrap()
                ),
                receipt["request"].clone(),
            )?;
        }
        assert_eq!(count.load(Ordering::SeqCst), 2);
        assert_eq!(
            tasks(&f, "objectTask.queue", json!({"projectId":project}))?,
            json!([])
        );
        Ok(())
    })();
    restarted.shutdown().await?;
    result?;
    assert_eq!(data_backup::inventory(&f.source)?, source_before);
    assert_eq!(data_backup::inventory(&f.preparation)?, preparation_before);
    Ok(())
}
