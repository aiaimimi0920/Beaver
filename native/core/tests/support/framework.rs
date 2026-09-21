#![allow(dead_code)]
use crate::delivery_support::Fixture;
use anyhow::{bail, Result};
use beaver_core::{framework, framework_contract::Command, framework_operations};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, time::Duration};

impl Fixture {
    pub async fn framework(&self, model: bool, request: Value) -> Result<Value> {
        framework::call(
            self.store.clone(),
            self.files(),
            "task".into(),
            model.then(|| {
                (
                    "thread".into(),
                    self.task().unwrap()["turnId"].as_str().unwrap().into(),
                )
            }),
            request,
        )
        .await
    }

    pub async fn start(&self, model: bool, job: Value) -> Result<Value> {
        self.framework(
            model,
            json!({"operation":"start","requestId":uuid::Uuid::new_v4().to_string(),"job":job}),
        )
        .await
    }

    pub async fn terminal(&self, id: &str) -> Result<Value> {
        for _ in 0..400 {
            let op = self
                .framework(false, json!({"operation":"inspect","operationId":id}))
                .await?;
            if framework_operations::terminal(op["status"].as_str().unwrap()) {
                return Ok(op);
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        bail!("Operation did not finish: {id}")
    }

    pub async fn configure(&self, plugins: Value, rules: Value, semantic: bool) -> Result<Value> {
        let revision = framework::configuration(&self.store.lock().unwrap(), "task")?.revision;
        self.framework(false,json!({"operation":"configure","configuration":{"revision":revision,"plugins":plugins,"rules":rules,"semanticRequired":semantic}})).await
    }

    pub async fn check(&self, candidate: &str) -> Result<Value> {
        let op = self
            .start(false, json!({"kind":"check","candidateId":candidate}))
            .await?;
        self.terminal(op["id"].as_str().unwrap()).await
    }
}

pub fn command() -> Result<Command> {
    let path = std::env::current_exe()?;
    Ok(Command {
        executable: path.to_string_lossy().into_owned(),
        sha256: hash(&path)?,
        args: vec![
            "--ignored".into(),
            "--exact".into(),
            "framework_support::adapter_process".into(),
            "--nocapture".into(),
        ],
        timeout_seconds: 5,
        files: BTreeMap::new(),
    })
}

pub fn hash(path: &std::path::Path) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(std::fs::read(path)?)))
}

pub fn plugin(id: &str) -> Result<Value> {
    let command = command()?;
    Ok(
        json!({"id":id,"host":"blender","version":"1.0","hostVersion":"4.5","probe":command,"install":command,"enable":command,"reload":command}),
    )
}

pub fn rule(id: &str) -> Result<Value> {
    Ok(json!({"id":id,"version":1,"checker":command()?}))
}

pub async fn started(path: &std::path::Path) -> Result<()> {
    for _ in 0..200 {
        if path.exists() {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    bail!("Fixture child did not start")
}

// Spawn this test executable as a real cancellable, pinned adapter process.
#[test]
#[ignore]
fn adapter_process() -> Result<()> {
    let path = std::env::var("BEAVER_CONTEXT")?;
    let context: Value = serde_json::from_slice(&std::fs::read(path)?)?;
    let workspace = std::path::Path::new(context["workspace"].as_str().unwrap());
    let id = context["plugin"]["id"]
        .as_str()
        .or_else(|| context["configuration"]["rules"][0]["id"].as_str())
        .unwrap();
    if id == "slow" {
        std::fs::write(workspace.join("adapter-started"), "started")?;
        std::thread::sleep(Duration::from_secs(3));
    }
    if id == "crash" {
        bail!("fixture failure: private-output");
    }
    if id == "malformed" {
        println!("no structured result");
        return Ok(());
    }
    let report = if context["plugin"].is_object() {
        let marker = workspace.join("installed");
        if context["action"] == "install" {
            use std::io::Write;
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&marker)?
                .write_all(b"install\n")?;
        }
        json!({"pluginId":id,"host":"blender","pluginVersion":if id=="wrong-version" {"0.9"} else {"1.0"},"hostVersion":"4.5","installed":marker.exists(),"compatible":true,"enabled":true,"callable":true,"restartRequired":id=="restart"})
    } else {
        if id == "tamper" {
            std::fs::write(workspace.join("docs/design.md"), "tampered")?;
        }
        json!({"ruleId":id,"ruleVersion":if id=="wrong-rule-version" {2} else {1},"verdict":if id=="fail" {"fail"} else {"pass"}})
    };
    println!("private-output\nBEAVER_RESULT={report}");
    Ok(())
}
