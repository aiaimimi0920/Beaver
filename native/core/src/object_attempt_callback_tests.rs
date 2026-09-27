use super::{attempt_fixture::*, run_fixture};
use crate::{object_attempt, object_attempt_callback as callback, object_attempt_control};
use anyhow::Result;
use serde_json::{json, Value};

fn params(arguments: Value) -> Value {
    json!({"tool":callback::TOOL,"threadId":"thread","turnId":"turn","arguments":arguments})
}

fn call(
    runtime: &crate::project_runtime::ProjectRuntime,
    lease: &object_attempt::Lease,
    params: &Value,
) -> Result<Value> {
    let reply = callback::call(runtime, lease, params)?;
    assert_eq!(reply["success"], true);
    Ok(serde_json::from_str(
        reply["contentItems"][0]["text"].as_str().unwrap(),
    )?)
}

#[test]
fn object_attempt_callback_scopes_frozen_input_and_rejects_invalid_requests() -> Result<()> {
    let f = fixture()?;
    let version = run_fixture::capture(&f, "hero", "input", "frozen input", vec![])?;
    run_fixture::accept(&f, "hero", &version)?;
    mutate(
        &f,
        "object_task",
        "head",
        "/identity/baseline",
        json!({"basePolicy":"latestAccepted"}),
    )?;
    let mut lease = start(&f)?;
    object_attempt::bind(&f.runtime, &mut lease, "thread", Some("turn"))?;
    let context = call(&f.runtime, &lease, &params(json!({"operation":"context"})))?;
    let token = &lease.record().preparation.claim_token;
    assert!(!context.to_string().contains(token));
    assert_eq!(context["attemptId"], lease.record().id);
    std::fs::write(workspace(&f, lease.record())?.join("hero.tscn"), "changed")?;
    let read =
        json!({"operation":"inputFile","path":"hero.tscn","sha256":context["input"]["hero.tscn"]});
    assert_eq!(
        call(&f.runtime, &lease, &params(read.clone()))?["content"]["text"],
        "frozen input"
    );
    for input in [
        json!({"operation":"context","attemptId":"foreign"}),
        json!({"operation":"publish"}),
        json!({"operation":"inputFile","path":"../hero.tscn","sha256":context["input"]["hero.tscn"]}),
        json!({"operation":"inputFile","path":"hero.tscn","sha256":"a".repeat(64)}),
        json!({"operation":"inputFile"}),
    ] {
        assert!(callback::call(&f.runtime, &lease, &params(input)).is_err());
    }
    for field in ["threadId", "turnId", "tool"] {
        let mut foreign = params(read.clone());
        foreign[field] = json!("foreign");
        assert!(callback::call(&f.runtime, &lease, &foreign).is_err());
    }
    let request = interrupt_request(&f, "head")?;
    object_attempt_control::request(&f.runtime, &request, true)?;
    assert!(callback::call(&f.runtime, &lease, &params(read))
        .unwrap_err()
        .to_string()
        .contains("INTERRUPTED"));
    Ok(())
}

#[test]
fn object_attempt_callback_rejects_unbound_and_stale_lease() -> Result<()> {
    let f = fixture()?;
    let mut lease = start(&f)?;
    let request = params(json!({"operation":"capabilities"}));
    assert!(callback::call(&f.runtime, &lease, &request).is_err());
    object_attempt::bind(&f.runtime, &mut lease, "thread", Some("turn"))?;
    assert_eq!(
        call(&f.runtime, &lease, &request)?["tools"][0],
        callback::definition()
    );
    mutate(
        &f,
        "object_attempt",
        &lease.record().id,
        "/state",
        json!("interrupted"),
    )?;
    assert!(callback::call(&f.runtime, &lease, &request).is_err());
    Ok(())
}
