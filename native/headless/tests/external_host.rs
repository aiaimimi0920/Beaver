use anyhow::{Context, Result};
use beaver_headless::Host;
use serde_json::{json, Value};
use std::{path::Path, sync::Arc, time::Duration};

async fn ready(host: &Arc<Host>, task: &Value) -> Result<Value> {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Ok(context) = host
                .call("external.context", json!({"taskId":task["id"]}))
                .await
            {
                return Ok(context);
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn external_claim_uses_real_run_without_provider_and_scopes_receipts_and_logs() -> Result<()>
{
    let temp = tempfile::tempdir()?;
    let data = temp.path().join("host");
    let host = Host::open(&data)?;
    let project = host
        .call(
            "project.create",
            json!({
                "parent":temp.path(),"name":"External","template":"blank"
            }),
        )
        .await?;
    for fields in [
        json!({"executionMode":"external-agent","assetTask":true}),
        json!({"executionMode":"made-up"}),
    ] {
        let mut input = json!({"projectId":project["id"],"prompt":"Reject invalid mode"});
        input
            .as_object_mut()
            .unwrap()
            .extend(fields.as_object().unwrap().clone());
        assert!(host.call("task.create", input).await.is_err());
    }
    let task = host
        .call(
            "task.create",
            json!({
                "projectId":project["id"],"prompt":"Write a task-local note", "decompose":false,
                "executionMode":"external-agent"
            }),
        )
        .await?;
    let context = ready(&host, &task).await?;
    assert_eq!(context["task"]["status"], "running");
    assert!(context["task"]["threadId"].is_null());
    assert!(context["task"]["turnId"].is_null());
    assert_ne!(context["runId"], task["id"]);
    assert!(!context["runId"]
        .as_str()
        .context("Missing real run ID")?
        .is_empty());
    assert_eq!(context["revision"], 0);
    let pending = host.call("external.pending", json!({})).await?;
    assert!(pending
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["runId"] == context["runId"]));
    let input = json!({"taskId":task["id"],"runId":context["runId"],"revision":0,
    "requestId":"write-note","arguments":{"tool":"file.write","arguments":{
        "path":"note.txt","content":"Written through Beaver","expectedSha256":null
    }}});
    for field in ["owner", "threadId", "turnId"] {
        let mut forged = input.clone();
        forged[field] = json!("caller-minted");
        assert!(host.call("external.tool", forged).await.is_err());
    }
    let receipt = host.call("external.tool", input.clone()).await?;
    assert_eq!(receipt["status"], "succeeded");
    assert_eq!(receipt["revision"], 1);
    assert_eq!(host.call("external.tool", input).await?, receipt);
    assert_eq!(
        host.call(
            "external.receipt",
            json!({
                "taskId":task["id"],"runId":context["runId"],"requestId":"write-note"
            })
        )
        .await?,
        receipt
    );
    assert_eq!(
        std::fs::read_to_string(
            Path::new(context["workspace"].as_str().unwrap()).join("note.txt")
        )?,
        "Written through Beaver"
    );
    assert!(!Path::new(project["path"].as_str().unwrap())
        .join("note.txt")
        .exists());
    let logs = host
        .call(
            "logs.query",
            json!({"taskId":task["id"],"method":"external.tool"}),
        )
        .await?;
    assert!(logs["records"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["taskId"] == task["id"] && row["projectId"] == project["id"]));
    host.shutdown().await?;
    drop(host);
    let reopened = Host::open(&data)?;
    assert_eq!(
        reopened.call("external.pending", json!({})).await?,
        json!([])
    );
    let history = reopened
        .call("external.history", json!({"taskId":task["id"]}))
        .await?;
    assert!(history["runs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|run| run["runId"] == context["runId"]));
    assert_eq!(
        reopened
            .call(
                "external.receipt",
                json!({
                    "taskId":task["id"],"runId":context["runId"],"requestId":"write-note"
                })
            )
            .await?,
        receipt
    );
    assert!(reopened.call("external.tool", json!({"taskId":task["id"],"runId":context["runId"],
        "requestId":"stale-write","revision":1,"arguments":{"tool":"file.read","arguments":{"path":"note.txt"}}
    })).await.is_err());
    let state = reopened.call("state", json!({})).await?;
    assert_eq!(state["tasks"][0]["status"], "interrupted");
    reopened.shutdown().await?;
    Ok(())
}
