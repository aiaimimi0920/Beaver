use crate::{framework_command, framework_contract::Plugin};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::{path::Path, sync::atomic::AtomicBool};

pub fn execute(
    plugin: &Plugin,
    action: &str,
    workspace: &Path,
    context: &Value,
    cancelled: &AtomicBool,
) -> Result<Value> {
    ensure!(
        ["probe", "install", "enable", "reload"].contains(&action),
        "Unknown plugin action"
    );
    let mut context = context.clone();
    context["plugin"] = json!(plugin);
    context["action"] = json!(action);
    let command = match action {
        "install" => plugin.install.as_ref(),
        "enable" => plugin.enable.as_ref(),
        "reload" => plugin.reload.as_ref(),
        _ => Some(&plugin.probe),
    }
    .context("Plugin adapter does not support this action")?;
    let mut evidence = vec![framework_command::run(
        command, workspace, &context, cancelled,
    )?];
    if action != "probe" {
        context["action"] = json!("probe");
        evidence.push(framework_command::run(
            &plugin.probe,
            workspace,
            &context,
            cancelled,
        )?);
    }
    let report = &evidence.last().unwrap()["report"];
    ensure!(
        report["pluginId"] == plugin.id && report["host"] == plugin.host,
        "Plugin probe identity mismatch"
    );
    for field in [
        "installed",
        "compatible",
        "enabled",
        "callable",
        "restartRequired",
    ] {
        ensure!(
            report[field].is_boolean(),
            "Plugin probe missing boolean {field}"
        );
    }
    let versions =
        report["pluginVersion"] == plugin.version && report["hostVersion"] == plugin.host_version;
    let ready = versions
        && ["installed", "compatible", "enabled", "callable"]
            .iter()
            .all(|k| report[*k] == true)
        && report["restartRequired"] == false;
    Ok(
        json!({"pluginId":plugin.id,"host":plugin.host,"version":plugin.version,"hostVersion":plugin.host_version,"ready":ready,"status":if ready {"ready"} else {"notReady"},"evidence":evidence,"scope":"task-workspace-and-context; adapters must not modify global installations"}),
    )
}
