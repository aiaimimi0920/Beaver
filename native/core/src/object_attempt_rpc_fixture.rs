use crate::object_attempt_launch::Launch;
use anyhow::Result;
use fs2::FileExt;
use serde_json::{json, Value};
use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, Write},
    path::Path,
    process::Stdio,
    time::Duration,
};
use tokio::process::Command;

const SERVER: &str = "object_tasks::tests::object_attempt_rpc_fixture::fake_server";
const WRITER: &str = "object_tasks::tests::object_attempt_rpc_fixture::fake_writer";

pub(super) fn launch(root: &Path, cwd: &Path, mode: &str) -> Result<Launch> {
    fs::create_dir_all(root)?;
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args(["--exact", SERVER, "--nocapture"])
        .env("BEAVER_ATTEMPT_FAKE_ROOT", root)
        .env("BEAVER_ATTEMPT_FAKE_MODE", mode)
        .current_dir(cwd);
    Ok(Launch {
        command,
        cwd: cwd.into(),
        model: "test-model".into(),
        secrets: vec!["test-provider-secret".into()],
        timeout: Duration::from_secs(10),
        godot: None,
    })
}

fn completed(thread: &str, turn: &str) -> Value {
    json!({"method":"turn/completed","params":{"threadId":thread,
        "turn":{"id":turn,"status":"completed"}}})
}

fn spawn_writer(root: &Path) -> Result<()> {
    let mut child = std::process::Command::new(std::env::current_exe()?)
        .args(["--exact", WRITER, "--nocapture"])
        .env("BEAVER_ATTEMPT_FAKE_ROOT", root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !root.join("child-ready").exists() {
        if child.try_wait()?.is_some() {
            anyhow::bail!("writer exited early");
        }
        anyhow::ensure!(
            std::time::Instant::now() < deadline,
            "writer startup timed out"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    // Intentionally leave this descendant alive, including after root exit.
    Ok(())
}

fn wait_for_release(root: &Path) -> Result<()> {
    fs::write(root.join("turn-ready"), "ready")?;
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    while !root.join("release").exists() {
        anyhow::ensure!(std::time::Instant::now() < deadline, "release timed out");
        std::thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}

#[test]
fn fake_server() -> Result<()> {
    let Some(root) = std::env::var_os("BEAVER_ATTEMPT_FAKE_ROOT") else {
        return Ok(());
    };
    let root = Path::new(&root);
    let lease = File::create(root.join("lease"))?;
    lease.lock_exclusive()?;
    let mode = std::env::var("BEAVER_ATTEMPT_FAKE_MODE")?;
    let mut transcript = OpenOptions::new()
        .create(true)
        .append(true)
        .open(root.join("rpc.jsonl"))?;
    let mut stdout = std::io::stdout().lock();
    for line in std::io::stdin().lock().lines() {
        let input: Value = serde_json::from_str(&line?)?;
        writeln!(transcript, "{input}")?;
        transcript.flush()?;
        let method = input["method"].as_str().unwrap_or("");
        let result = match method {
            "initialize" => Some(json!({})),
            "thread/start" => Some(json!({"thread":{"id":"thread"},"sandbox":{
                "type":match mode.as_str() {
                    "read-only" => "readOnly",
                    "full-access" => "dangerFullAccess",
                    "missing-sandbox" => "",
                    _ => "workspaceWrite",
                }
            }})),
            "turn/start" => Some(json!({"turn":{"id":"turn"}})),
            _ => None,
        };
        if method == "turn/start" {
            fs::write("result.txt", "fine output")?;
            if mode == "godot" {
                fs::write("project.godot", "config_version=5\n")?;
                fs::write(
                    "icon.svg",
                    r#"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"><rect width="8" height="8" fill="red"/></svg>"#,
                )?;
            }
            spawn_writer(root)?;
            if mode == "early" {
                for phase in ["item/started", "item/completed"] {
                    writeln!(
                        stdout,
                        "{}",
                        json!({"method":phase,"params":{"threadId":"thread","turnId":"turn","item":{"type":"commandExecution","status":if phase == "item/started" { "inProgress" } else { "completed" },"command":"test-provider-secret"}}})
                    )?;
                }
                writeln!(stdout, "{}", completed("thread", "turn"))?;
            }
        }
        if let Some(result) = result {
            writeln!(stdout, "{}", json!({"id":input["id"],"result":result}))?;
            stdout.flush()?;
        }
        if method == "turn/start" {
            match mode.as_str() {
                "exit-parent" => std::process::exit(0),
                "hold" => {
                    wait_for_release(root)?;
                    writeln!(stdout, "{}", completed("thread", "turn"))?;
                }
                "identities" => {
                    writeln!(stdout, "{}", completed("other-thread", "turn"))?;
                    writeln!(stdout, "{}", completed("thread", "other-turn"))?;
                    writeln!(
                        stdout,
                        "{}",
                        json!({"id":100,"method":"item/tool/call",
                        "params":{"threadId":"thread","turnId":"other-turn"}})
                    )?;
                }
                "interaction" => writeln!(
                    stdout,
                    "{}",
                    json!({"id":100,"method":"item/tool/call",
                    "params":{"threadId":"thread","turnId":"turn"}})
                )?,
                "callback" => writeln!(
                    stdout,
                    "{}",
                    callback(100, json!({"operation":"capabilities"}))
                )?,
                "image-callback" | "feedback-image-callback" => {
                    writeln!(stdout, "{}", callback(200, json!({"operation":"context"})))?
                }
                "error" => writeln!(
                    stdout,
                    "{}",
                    json!({"method":"error","params":{
                    "threadId":"thread","turnId":"turn","willRetry":false,
                    "error":{"message":"Provider rejected test-provider-secret"}}})
                )?,
                "early" | "hang" => {}
                _ => writeln!(stdout, "{}", completed("thread", "turn"))?,
            }
        }
        if mode == "identities" && input["id"] == 100 {
            assert!(input["error"]["message"]
                .as_str()
                .unwrap()
                .contains("IDENTITY_MISMATCH"));
            wait_for_release(root)?;
            writeln!(stdout, "{}", completed("thread", "turn"))?;
        }
        if matches!(mode.as_str(), "image-callback" | "feedback-image-callback")
            && input.get("method").is_none()
        {
            match input["id"].as_u64() {
                Some(200) => {
                    let context: Value = serde_json::from_str(
                        input["result"]["contentItems"][0]["text"].as_str().unwrap(),
                    )?;
                    // A later workspace write must not replace the frozen feedback PNG.
                    if mode == "feedback-image-callback" {
                        image::RgbaImage::from_pixel(20, 10, image::Rgba([0, 0, 255, 255]))
                            .save("preview.png")?;
                    } else {
                        fs::write("preview.png", "changed workspace image")?;
                    }
                    let arguments = if mode == "feedback-image-callback" {
                        assert!(context["feedbackImage"].is_object());
                        json!({"operation":"feedbackImage"})
                    } else {
                        json!({"operation":"inputFile","path":"preview.png","sha256":context["input"]["preview.png"]})
                    };
                    writeln!(stdout, "{}", callback(201, arguments))?;
                }
                Some(201) => {
                    assert_eq!(input["result"]["success"], true);
                    assert_eq!(input["result"]["contentItems"][1]["type"], "inputImage");
                    fs::write("image-delivered.json", input["result"].to_string())?;
                    writeln!(stdout, "{}", completed("thread", "turn"))?;
                }
                _ => {}
            }
        }
        if mode == "callback" && input.get("method").is_none() {
            let id = input["id"].as_u64().unwrap();
            if id >= 100 {
                assert_eq!(input["result"]["success"], true);
                let value: Value = serde_json::from_str(
                    input["result"]["contentItems"][0]["text"].as_str().unwrap(),
                )?;
                match id {
                    100 => {
                        assert_eq!(value["readOnly"], true);
                        writeln!(stdout, "{}", callback(101, json!({"operation":"context"})))?;
                    }
                    101 => {
                        fs::write("hero.tscn", "live workspace changed")?;
                        writeln!(
                            stdout,
                            "{}",
                            callback(
                                102,
                                json!({"operation":"inputFile","path":"hero.tscn","sha256":value["input"]["hero.tscn"]})
                            )
                        )?;
                    }
                    102 => {
                        assert_eq!(value["content"]["text"], "frozen input");
                        fs::write(
                            "callback-result.txt",
                            value["content"]["text"].as_str().unwrap(),
                        )?;
                        writeln!(stdout, "{}", completed("thread", "turn"))?;
                    }
                    _ => panic!("unexpected callback response"),
                }
            }
        }
        stdout.flush()?;
    }
    Ok(())
}

fn callback(id: u64, arguments: Value) -> Value {
    json!({"id":id,"method":"item/tool/call","params":{"threadId":"thread","turnId":"turn",
        "tool":"beaver_object_attempt","arguments":arguments}})
}

#[test]
fn fake_writer() -> Result<()> {
    let Some(root) = std::env::var_os("BEAVER_ATTEMPT_FAKE_ROOT") else {
        return Ok(());
    };
    let root = Path::new(&root);
    let lease = File::create(root.join("child-lease"))?;
    lease.lock_exclusive()?;
    fs::write("child-output.txt", "descendant output")?;
    fs::write(root.join("child-ready"), "ready")?;
    std::thread::sleep(Duration::from_secs(30));
    Ok(())
}

pub(super) fn transcript(root: &Path) -> Result<Vec<Value>> {
    fs::read_to_string(root.join("rpc.jsonl"))?
        // Killing the server can interrupt its final transcript write.
        .split_inclusive('\n')
        .filter(|line| line.ends_with('\n'))
        .map(|line| Ok(serde_json::from_str(line)?))
        .collect()
}

pub(super) fn assert_closed(root: &Path) -> Result<()> {
    for name in ["lease", "child-lease"] {
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(root.join(name))?
            .try_lock_exclusive()?;
    }
    Ok(())
}

pub(super) async fn wait_until(mut ready: impl FnMut() -> bool) {
    tokio::time::timeout(Duration::from_secs(10), async {
        while !ready() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("attempt fixture timed out");
}
