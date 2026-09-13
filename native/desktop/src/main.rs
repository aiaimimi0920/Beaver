#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod asset_protocol;
mod asset_runtime;
mod backend;
mod business_api;
mod business_catalog;
mod business_mcp;
mod business_routing;
mod data_dispatch;
mod game_runtime;
mod instance;
mod setup_runtime;
mod shell;
mod task_control;
mod task_runtime;
mod template_runtime;
mod workflow_runtime;

use serde_json::Value;
use std::sync::Arc;

use backend::{business_call, Backend};

#[tauri::command]
async fn beaver_call(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<Backend>>,
    method: String,
    input: Option<Value>,
) -> Result<Value, String> {
    business_call(app, state.inner().clone(), method, input, "ui").await
}

// Keep command failures separate from IPC transport failures.
mod anyhow_result {
    pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
}

fn main() {
    let mut arguments = std::env::args_os().skip(1);
    let mode = arguments.next();
    if mode.as_deref() == Some(std::ffi::OsStr::new("--check-code-structure")) {
        match beaver_core::code_structure::cli::run(arguments.collect()) {
            Ok(true) => {}
            Ok(false) => std::process::exit(1),
            Err(error) => {
                eprintln!("Code structure: {error}");
                std::process::exit(2);
            }
        }
        return;
    }
    if mode.as_deref() == Some(std::ffi::OsStr::new("--business-mcp")) {
        if arguments.next().is_some() {
            std::process::exit(2);
        }
        if business_mcp::run().is_err() {
            eprintln!("Beaver business MCP stopped; check the local API configuration");
            std::process::exit(1);
        }
        return;
    }
    if mode.as_deref()
        == Some(std::ffi::OsStr::new(
            beaver_core::media_server::MODE_ARGUMENT,
        ))
    {
        if arguments.next().is_some() {
            std::process::exit(2);
        }
        if beaver_core::media_server::run_stdio().is_err() {
            eprintln!("Beaver media MCP stopped unexpectedly");
            std::process::exit(1);
        }
        return;
    }
    if mode.as_deref()
        == Some(std::ffi::OsStr::new(
            beaver_core::migration_bundle::MODE_ARGUMENT,
        ))
    {
        match beaver_core::migration_bundle::command(arguments.collect()) {
            Ok(report) => println!("{report}"),
            Err(error) => {
                eprintln!("Beaver migration bundle failed: {error}");
                std::process::exit(1);
            }
        }
        return;
    }
    shell::run();
}
