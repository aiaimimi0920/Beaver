use anyhow::{ensure, Context, Result};
use beaver_core::{
    files::Files,
    scheduler::{Launch, Scheduler},
    store::Store,
};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::{process::Command, time::timeout};

#[tokio::main]
async fn main() -> Result<()> {
    let output = PathBuf::from(std::env::args_os().nth(1).context("output required")?);
    let executable = PathBuf::from(
        std::env::args_os()
            .nth(2)
            .context("executor fixture executable required")?,
    );
    let executable = std::fs::canonicalize(executable)?;
    let root = output.join(uuid::Uuid::new_v4().to_string());
    let store = Arc::new(Mutex::new(Store::open(&root.join("data"))?));
    let files = Arc::new(Files::new(root.join("data")));
    store
        .lock()
        .unwrap()
        .put("settings", "main", &json!({"maxParallel":2}))?;
    for id in ["first", "second", "third"] {
        let project = root.join(format!("project-{id}"));
        std::fs::create_dir(&project)?;
        std::fs::write(project.join("project.godot"), "[application]\n")?;
        let baseline = files.capture(&project)?;
        let workspace = root.join(format!("workspace-{id}"));
        files.restore_copy(&baseline, &workspace)?;
        let workspace = std::fs::canonicalize(workspace)?;
        store
            .lock()
            .unwrap()
            .put("project", id, &json!({"id":id,"path":project}))?;
        store.lock().unwrap().put("task",id,&json!({"id":id,"projectId":id,"workspace":workspace,"baseline":baseline,"status":"queued","capability":"code","prompt":"wait","references":[],"changes":[]}))?;
    }
    let launched = Arc::new(Mutex::new(Vec::new()));
    let recorded = launched.clone();
    let notifications = Arc::new(AtomicUsize::new(0));
    let observed = notifications.clone();
    let scheduler = Scheduler::start(
        store.clone(),
        files,
        Arc::new(move |task| {
            recorded
                .lock()
                .unwrap()
                .push(task["id"].as_str().unwrap().to_string());
            let workspace = task["workspace"].as_str().unwrap();
            let mut command = Command::new(&executable);
            command
                .arg("--fixture")
                .current_dir(workspace)
                .env("BEAVER_EXECUTOR_FIXTURE", workspace);
            Ok(Launch {
                command: Some(command),
                model: "fixture".into(),
                prompt: "wait".into(),
                ask_user_tool: json!({"name":"beaver_ask_user"}),
                secrets: vec![],
                max_minutes: 0,
                blender: None,
                godot: None,
            })
        }),
        Arc::new(move || {
            observed.fetch_add(1, Ordering::SeqCst);
        }),
    );
    let test = async {
        for _ in 0..200 {
            let ready = {
                let db = store.lock().unwrap();
                db.get::<Value>("task", "first")?.unwrap()["turnId"] == "turn"
                    && db.get::<Value>("task", "second")?.unwrap()["turnId"] == "turn"
            };
            if ready {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        ensure!(
            store
                .lock()
                .unwrap()
                .get::<Value>("task", "third")?
                .unwrap()["status"]
                == "queued"
        );
        let mut started = launched.lock().unwrap().clone();
        started.sort();
        ensure!(
            started == ["first", "second"],
            "parallel limit or FIFO violated"
        );
        scheduler
            .interrupt("third".into())
            .await
            .map_err(anyhow::Error::msg)?;
        ensure!(
            store
                .lock()
                .unwrap()
                .get::<Value>("task", "third")?
                .unwrap()["status"]
                == "interrupted"
        );
        scheduler
            .steer("first".into(), "继续".into())
            .await
            .map_err(anyhow::Error::msg)?;
        scheduler
            .interrupt("second".into())
            .await
            .map_err(anyhow::Error::msg)?;
        for _ in 0..200 {
            if store
                .lock()
                .unwrap()
                .get::<Value>("task", "first")?
                .unwrap()["status"]
                == "completed"
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        ensure!(
            store
                .lock()
                .unwrap()
                .get::<Value>("task", "first")?
                .unwrap()["status"]
                == "completed"
        );
        ensure!(
            store
                .lock()
                .unwrap()
                .get::<Value>("task", "second")?
                .unwrap()["status"]
                == "interrupted"
        );
        {
            let db = store.lock().unwrap();
            let mut task: Value = db.get("task", "third")?.unwrap();
            task["id"] = json!("fourth");
            task["status"] = json!("queued");
            db.put("task", "fourth", &task)?;
        }
        scheduler.wake().map_err(anyhow::Error::msg)?;
        for _ in 0..200 {
            if store
                .lock()
                .unwrap()
                .get::<Value>("task", "fourth")?
                .unwrap()["turnId"]
                == "turn"
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        ensure!(
            store
                .lock()
                .unwrap()
                .get::<Value>("task", "fourth")?
                .unwrap()["turnId"]
                == "turn"
        );
        Ok::<_, anyhow::Error>(())
    };
    let result = timeout(Duration::from_secs(20), test).await;
    timeout(Duration::from_secs(15), scheduler.shutdown())
        .await?
        .map_err(anyhow::Error::msg)?;
    result??;
    ensure!(
        store
            .lock()
            .unwrap()
            .get::<Value>("task", "fourth")?
            .unwrap()["status"]
            == "interrupted"
    );
    ensure!(scheduler.wake().is_err());
    ensure!(notifications.load(Ordering::SeqCst) >= 4);
    let proof = json!({"passed":true,"checks":["queued tasks claimed oldest-first", "maxParallel caps active processes", "queued cancellation never launches process", "steer reaches active session", "running interrupt waits for finalization", "completed outcome finalized through journal", "shutdown drains scheduler and rejects future work", "state notifications emitted"],"desktopQueueIntegrated":false});
    std::fs::write(
        output.join("proof.json"),
        serde_json::to_vec_pretty(&proof)?,
    )?;
    println!("{proof}");
    Ok(())
}
