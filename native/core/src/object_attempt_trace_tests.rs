use super::attempt_fixture::{fixture, start};
use crate::{
    object_attempt::{self, State},
    object_attempt_trace::{self as trace, Request},
};
use anyhow::Result;
use serde_json::json;
use std::sync::atomic::AtomicBool;

#[test]
fn object_attempt_trace_is_bounded_private_and_retained_after_finish() -> Result<()> {
    let f = fixture()?;
    let mut lease = start(&f)?;
    let request = Request {
        project_id: "project-1".into(),
        run_id: lease.record().preparation.run.id.clone(),
        attempt_id: lease.record().id.clone(),
    };
    assert!(trace::read(&f.runtime, &request)?.entries.is_empty());
    object_attempt::bind(&f.runtime, &mut lease, "thread", Some("turn"))?;
    let mut params = json!({"threadId":"thread", "turnId":"turn", "item":{"type":"commandExecution", "status":"completed", "command":"secret-command", "aggregatedOutput":"secret-output"}});
    for key in ["threadId", "turnId"] {
        let mut wrong = params.clone();
        wrong[key] = json!("foreign");
        assert!(trace::notification(&lease, "item/completed", &wrong).is_none());
    }
    assert!(trace::notification(&lease, "item/commandExecution/outputDelta", &params).is_none());
    for _ in 0..258 {
        object_attempt::record_event(
            &f.runtime,
            &lease,
            trace::notification(&lease, "item/completed", &params).unwrap(),
        )?;
    }
    let saved = trace::read(&f.runtime, &request)?;
    assert_eq!(saved.entries.len(), 256);
    assert!(saved.truncated);
    assert!(!serde_json::to_string(&saved)?.contains("secret"));
    params["item"]["type"] = json!("agentMessage");
    assert!(trace::notification(&lease, "item/completed", &params).is_none());
    object_attempt::finish(
        &f.runtime,
        lease,
        State::AwaitingGate,
        None,
        &AtomicBool::new(false),
    )?;
    assert_eq!(trace::read(&f.runtime, &request)?.entries.len(), 256);
    for invalid in [
        Request {
            project_id: "foreign".into(),
            ..request.clone()
        },
        Request {
            run_id: "foreign".into(),
            ..request.clone()
        },
        Request {
            attempt_id: "foreign".into(),
            ..request.clone()
        },
    ] {
        assert!(trace::read(&f.runtime, &invalid).is_err());
    }
    drop(f.runtime);
    let reopened =
        crate::project_storage::ProjectStore::open(f.temp.path(), "project-1")?.into_runtime();
    assert_eq!(
        serde_json::to_value(trace::read(&reopened, &request)?)?,
        serde_json::to_value(&saved)?
    );
    Ok(())
}

#[test]
fn object_attempt_trace_rejects_stale_writer_without_changing_evidence() -> Result<()> {
    let f = fixture()?;
    let mut lease = start(&f)?;
    object_attempt::bind(&f.runtime, &mut lease, "thread", Some("turn"))?;
    let entry = trace::notification(
        &lease,
        "turn/completed",
        &json!({"threadId":"thread", "turn":{"id":"turn", "status":"completed"}}),
    )
    .unwrap();
    let mut changed = lease.record().clone();
    changed.state = State::Interrupted;
    f.runtime
        .store()
        .lock()
        .unwrap()
        .put("object_attempt", &changed.id, &changed)?;
    assert!(object_attempt::record_event(&f.runtime, &lease, entry).is_err());
    let request = Request {
        project_id: "project-1".into(),
        run_id: changed.preparation.run.id,
        attempt_id: changed.id,
    };
    assert!(trace::read(&f.runtime, &request)?.entries.is_empty());
    Ok(())
}
