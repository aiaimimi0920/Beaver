//! Managed tools for authenticated external task writers. Blender Python is trusted
//! code with the host user's OS access. Staging and path checks are not a sandbox.
use anyhow::{bail, Result};
use serde_json::Value;
use std::{path::PathBuf, sync::atomic::AtomicBool};

#[path = "external_tools_blender.rs"]
mod blender;
#[path = "external_tools_contract.rs"]
mod contract;
pub use contract::contract;
#[path = "external_tools_files.rs"]
mod files;
#[path = "external_tools_history.rs"]
mod history;
pub use history::historical_job_evidence;
#[path = "external_tools_jobs.rs"]
mod jobs;
#[path = "external_tools_outputs.rs"]
mod outputs;
#[path = "external_tools_process.rs"]
mod process;
#[cfg(test)]
#[path = "external_tools_tests.rs"]
mod tests;

#[derive(Clone)]
pub struct Context {
    /// Host-resolved task workspace, never a path supplied by the external caller.
    pub workspace: PathBuf,
    pub run_id: String,
    pub request_id: String,
    /// Host-selected configured executable. None uses Beaver's existing discovery.
    pub blender_path: Option<PathBuf>,
}

pub fn execute(
    context: &Context,
    tool: &str,
    arguments: &Value,
    cancelled: &AtomicBool,
    progress: &mut dyn FnMut(&Value) -> Result<()>,
) -> Result<Value> {
    anyhow::ensure!(
        !cancelled.load(std::sync::atomic::Ordering::SeqCst),
        "External tool request cancelled"
    );
    files::workspace(&context.workspace)?;
    match tool {
        "file.read" => files::read(context, arguments),
        "file.write" => {
            ensure_idle(&context.run_id)?;
            files::write(context, arguments, cancelled)
        }
        "blender.start" | "blender.python" => jobs::start(context, arguments, cancelled, progress),
        "blender.poll" => jobs::poll(context, arguments, false),
        "blender.cancel" => jobs::poll(context, arguments, true),
        "workflow.list" => {
            anyhow::ensure!(
                arguments.as_object().is_some_and(|v| v.is_empty()),
                "workflow.list takes an empty object"
            );
            crate::workflows::list(&context.workspace)
        }
        "workflow.run" => {
            ensure_idle(&context.run_id)?;
            crate::workflows::run(&context.workspace, arguments.clone(), cancelled)
        }
        _ => bail!("Unknown managed external tool"),
    }
}

/// Stops only this run's in-process owned jobs; never kills by executable name or
/// reuses persisted PIDs after a host restart. An unknown outcome is not retried.
pub fn cleanup(run_id: &str) -> Result<()> {
    jobs::cleanup(run_id)
}

pub fn ensure_idle(run_id: &str) -> Result<()> {
    jobs::ensure_idle(run_id)
}
