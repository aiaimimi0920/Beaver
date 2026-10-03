use crate::{object_task_planning_fixture as fixture, object_task_planning_launch::Launch};
use anyhow::Result;
use fs2::FileExt;
use serde_json::{json, Value};
use std::{
    fs::{File, OpenOptions},
    io::{BufRead, Write},
    path::Path,
    time::Duration,
};
use tokio::process::Command;

pub(crate) fn notify() -> crate::object_task_planning_executor::Notify {
    std::sync::Arc::new(|_| {})
}

pub(crate) fn launch(root: &Path, mode: &str, context: Value) -> Result<Launch> {
    let scratch = tempfile::tempdir()?;
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args([
            "--exact",
            "object_task_planning_rpc_fixture::fake_server",
            "--nocapture",
        ])
        .env("BEAVER_PLANNING_FAKE_ROOT", root)
        .env("BEAVER_PLANNING_FAKE_MODE", mode)
        .current_dir(scratch.path());
    Ok(Launch {
        command,
        cwd: scratch.path().into(),
        model: "test-model".into(),
        context,
        ask_tool: json!({"name":"beaver_ask_user"}),
        proposal_tool: json!({"name":"beaver_propose_object_tasks"}),
        secrets: vec!["test-provider-secret".into()],
        timeout: Duration::from_secs(10),
        scratch: Some(scratch),
    })
}

fn call(id: usize, tool: &str, arguments: Value) -> Value {
    json!({"id":id,"method":"item/tool/call","params":{
        "threadId":"thread","turnId":"turn","tool":tool,"arguments":arguments}})
}

fn script(mode: &str) -> Vec<Value> {
    let proposal = serde_json::to_value(fixture::proposal()).unwrap();
    match mode {
        "title" => vec![call(
            100,
            "beaver_suggest_task_title",
            json!({"title":"Add buffered dash"}),
        )],
        "title-correction" => {
            let mut wrong = call(
                100,
                "beaver_suggest_task_title",
                json!({"title":"Wrong turn"}),
            );
            wrong["params"]["turnId"] = json!("other-turn");
            vec![
                wrong,
                call(101, "exec_command", json!({"cmd":"forbidden"})),
                call(
                    102,
                    "beaver_suggest_task_title",
                    json!({"title":"Title","prompt":"rewritten"}),
                ),
                call(
                    103,
                    "beaver_suggest_task_title",
                    json!({"title":"x".repeat(49)}),
                ),
                call(
                    104,
                    "beaver_suggest_task_title",
                    json!({"title":"Add buffered dash"}),
                ),
            ]
        }
        "questions" => vec![call(
            100,
            "beaver_ask_user",
            json!({"questions":[fixture::question("style", 90)]}),
        )],
        "automatic" => vec![
            call(
                100,
                "beaver_ask_user",
                json!({"questions":[fixture::question("style", 10)]}),
            ),
            call(101, "beaver_propose_object_tasks", proposal),
        ],
        "correction" => {
            let mut wrong = call(100, "beaver_propose_object_tasks", proposal.clone());
            wrong["params"]["turnId"] = json!("other-turn");
            let mut namespaced = call(101, "beaver_propose_object_tasks", proposal.clone());
            namespaced["params"]["namespace"] = json!("execution");
            vec![
                wrong,
                namespaced,
                call(102, "exec_command", json!({"cmd":"forbidden"})),
                call(103, "beaver_propose_object_tasks", json!({"tasks":[{}]})),
                call(104, "beaver_propose_object_tasks", proposal),
            ]
        }
        "empty" => vec![json!({"method":"turn/completed","params":{
            "threadId":"thread","turn":{"id":"turn","status":"completed"}}})],
        "error" => vec![
            json!({"method":"error","params":{"threadId":"thread","turnId":"turn",
            "willRetry":false,"error":{"message":"Provider rejected test-provider-secret"}}}),
        ],
        "hang" | "initialize-hang" => vec![],
        _ => vec![call(100, "beaver_propose_object_tasks", proposal)],
    }
}

#[test]
fn fake_server() -> Result<()> {
    let Some(root) = std::env::var_os("BEAVER_PLANNING_FAKE_ROOT") else {
        return Ok(());
    };
    let root = Path::new(&root);
    let lease = File::create(root.join("lease"))?;
    lease.lock_exclusive()?;
    let mut transcript = Vec::new();
    publish_transcript(root, &transcript)?;
    let mode = std::env::var("BEAVER_PLANNING_FAKE_MODE")?;
    let mut messages = script(&mode).into_iter();
    let mut stdout = std::io::stdout().lock();
    for line in std::io::stdin().lock().lines() {
        let input: Value = serde_json::from_str(&line?)?;
        let mut frame = serde_json::to_vec(&input)?;
        frame.push(b'\n');
        transcript.extend_from_slice(&frame);
        publish_transcript(root, &transcript)?;
        let method = input["method"].as_str().unwrap_or("");
        let result = match method {
            "initialize" if mode != "initialize-hang" => Some(json!({})),
            "thread/start" => Some(json!({"thread":{"id":"thread"}})),
            "turn/start" => Some(json!({"turn":{"id":"turn"}})),
            _ => None,
        };
        if let Some(result) = result {
            writeln!(stdout, "{}", json!({"id":input["id"],"result":result}))?;
        }
        if method == "turn/start"
            || (method.is_empty() && input["id"].as_u64().is_some_and(|id| id >= 100))
        {
            if let Some(message) = messages.next() {
                writeln!(stdout, "{message}")?;
            }
        }
        stdout.flush()?;
    }
    Ok(())
}

// The host deliberately kills the fixture immediately after its final reply.
// Commit a same-directory snapshot so a kill during serialization or writing
// cannot leave a torn JSONL tail in the transcript inspected by the parent.
fn publish_transcript(root: &Path, bytes: &[u8]) -> Result<()> {
    let mut staged = tempfile::NamedTempFile::new_in(root)?;
    staged.write_all(bytes)?;
    staged.flush()?;
    staged
        .persist(root.join("rpc.jsonl"))
        .map_err(|error| error.error)?;
    Ok(())
}

#[test]
fn transcript_snapshots_survive_interrupted_staging_and_keep_strict_parsing() -> Result<()> {
    let root = tempfile::tempdir()?;
    let start = json!({"method":"thread/start","params":{"sandbox":"read-only"}});
    let reply = json!({"id":100,"result":{"success":true}});
    let mut committed = serde_json::to_vec(&start)?;
    committed.push(b'\n');
    publish_transcript(root.path(), &committed)?;
    // Simulate termination between writing the staging file and atomic commit.
    let mut interrupted = tempfile::NamedTempFile::new_in(root.path())?;
    interrupted.write_all(b"{\"id\":100,\"result\":{\"content")?;
    interrupted.flush()?;
    assert_eq!(transcript(root.path())?, vec![start.clone()]);
    committed.extend_from_slice(&serde_json::to_vec(&reply)?);
    committed.push(b'\n');
    publish_transcript(root.path(), &committed)?;
    assert_eq!(transcript(root.path())?, vec![start, reply]);
    // Never hide corruption in the authoritative, published transcript.
    std::fs::write(root.path().join("rpc.jsonl"), b"{}\n{\"broken\":\"")?;
    assert!(transcript(root.path()).is_err());
    Ok(())
}

pub(crate) fn transcript(root: &Path) -> Result<Vec<Value>> {
    Ok(std::fs::read_to_string(root.join("rpc.jsonl"))?
        .lines()
        .map(serde_json::from_str)
        .collect::<std::result::Result<_, _>>()?)
}

pub(crate) fn assert_closed(root: &Path) -> Result<()> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(root.join("lease"))?;
    file.try_lock_exclusive()?;
    Ok(())
}

pub(crate) async fn wait_until(mut ready: impl FnMut() -> bool) {
    tokio::time::timeout(Duration::from_secs(10), async {
        while !ready() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("planning fixture timed out");
}
