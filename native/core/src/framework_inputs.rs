use crate::{
    asset_delivery_files as artifacts,
    asset_task::State,
    asset_work_inputs::{self, Role},
    files::{safe_path, Files, Snapshot},
    framework_contract::Command,
    store::Store,
};
use anyhow::{ensure, Result};
use serde_json::{json, Value};
use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

pub fn verify_closure(store: &Store, files: &Files, workspace: &Path, state: &State) -> Result<()> {
    let Some(flow) = &state.delivery else {
        return Ok(());
    };
    for rule in &crate::framework::configuration(store, &state.task_id)?.rules {
        crate::framework_command::verify(&rule.checker)?;
    }
    for (index, id) in flow.approved.iter().enumerate() {
        let candidate = artifacts::get(store, &state.task_id, id)?;
        crate::framework_checks::gate(store, &state.task_id, &candidate)?;
        let later = artifacts::approved_files(store, &state.task_id, &flow.approved[index + 1..])?;
        for input in asset_work_inputs::for_attempts(state, &candidate.attempt_ids)? {
            artifacts::verify(
                files,
                &Snapshot::from([(input.path.clone(), input.sha256.clone())]),
            )?;
            if input.role == Role::Dependency {
                let hash = later.get(&input.path).unwrap_or(&input.sha256);
                artifacts::verify_workspace(
                    files,
                    workspace,
                    &Snapshot::from([(input.path, hash.clone())]),
                )?;
            }
        }
    }
    Ok(())
}

pub fn scan(
    files: &Files,
    workspace: &Path,
    command: &Command,
    paths: &[String],
    context: &Value,
    cancelled: &AtomicBool,
) -> Result<Value> {
    ensure!(
        command.args.is_empty(),
        "Blender dependency scanner supplies its own safe startup arguments"
    );
    ensure!(
        paths
            .iter()
            .all(|p| p.to_ascii_lowercase().ends_with(".blend")),
        "Blender dependency scan requires .blend sources"
    );
    let sources = artifacts::capture(files, workspace, paths)?;
    let directory = tempfile::tempdir()?;
    let script = directory.path().join("dependencies.py");
    std::fs::write(
        &script,
        include_str!("../../../resources/workflows/blender_dependencies.py"),
    )?;
    let mut scanner = command.clone();
    scanner.files.insert(
        script.to_string_lossy().into_owned(),
        crate::files::file_hash_limited(&script, Some(1024 * 1024))?
            .ok_or_else(|| anyhow::anyhow!("Missing scanner script"))?,
    );
    scanner.args = vec![
        "--background".into(),
        "--factory-startup".into(),
        "--disable-autoexec".into(),
        "--python".into(),
        script.to_string_lossy().into_owned(),
    ];
    let mut context = context.clone();
    context["paths"] = json!(paths);
    let evidence = crate::framework_command::run(&scanner, workspace, &context, cancelled)?;
    let report = &evidence["report"];
    ensure!(
        report["scannerVersion"] == 1,
        "Unsupported dependency scanner report"
    );
    let entries = report["paths"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing dependency paths"))?;
    ensure!(
        entries.len() <= 256,
        "Dependency scan exceeds 256 paths; split the input"
    );
    let root = std::fs::canonicalize(workspace)?;
    let mut found = Snapshot::new();
    let mut unresolved = Vec::new();
    let mut total = 0_u64;
    let mut seen = std::collections::HashSet::new();
    for entry in entries {
        ensure!(!cancelled.load(Ordering::SeqCst), "Operation cancelled");
        let path = entry
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid dependency path"))?;
        let absolute = Path::new(path);
        let canonical = std::fs::canonicalize(absolute).ok();
        let relative = canonical
            .as_ref()
            .and_then(|p| p.strip_prefix(&root).ok())
            .map(|p| p.to_string_lossy().replace('\\', "/"));
        if let Some(relative) = &relative {
            let key = if cfg!(windows) {
                relative.to_lowercase()
            } else {
                relative.clone()
            };
            if !seen.insert(key) {
                continue;
            }
            if let Ok(metadata) = std::fs::metadata(absolute) {
                total = total.saturating_add(metadata.len());
                ensure!(
                    total <= artifacts::MAX_TOTAL_BYTES,
                    "Dependency scan exceeds 512 MiB; split the input"
                );
            }
        }
        let captured = relative
            .as_ref()
            .map(|p| artifacts::capture(files, workspace, std::slice::from_ref(p)))
            .transpose();
        match captured {
            Ok(Some(snapshot)) if !std::fs::symlink_metadata(absolute).map(|m|crate::files::linked(&m)).unwrap_or(true) => found.extend(snapshot),
            _ => unresolved.push(json!({"path":path,"reason":"External, missing, linked, oversized or unsupported dependency; copy/import explicitly and rescan"})),
        }
    }
    for path in paths {
        safe_path(workspace, path)?;
    }
    artifacts::verify_workspace(files, workspace, &sources)?;
    Ok(
        json!({"sources":sources,"dependencies":found,"unresolved":unresolved,"complete":unresolved.is_empty(),"coverage":"Blender stored file references, excluding packed assets; runtime-generated dependencies require explicit declarations","evidence":evidence}),
    )
}
