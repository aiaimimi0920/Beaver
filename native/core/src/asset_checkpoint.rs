use crate::{asset_preview::Client, asset_reference, asset_task, files::Files, store::Store};
use anyhow::{Context, Result};
use std::{
    collections::HashSet,
    fs,
    path::Path,
    sync::{Arc, Mutex},
};

/// Never remove recovery files referenced by any task, including a late follow-up.
pub fn prune(store: &Store, files: &Files, task_id: &str, directory: &Path) -> Result<()> {
    files.check_checkpoint_directory(task_id, directory)?;
    if !directory.is_dir() {
        return Ok(());
    }
    let directory = fs::canonicalize(directory)?;
    let mut pinned = HashSet::new();
    for state in store.list::<asset_task::State>("asset-task")? {
        for path in state
            .checkpoint
            .iter()
            .chain(state.feedback.iter().filter_map(|f| f.checkpoint.as_ref()))
            .chain(
                state
                    .work
                    .attempts
                    .iter()
                    .flat_map(|a| a.checkpoint.iter().chain(a.end_checkpoint.iter())),
            )
        {
            if let Ok(path) = files
                .resolve_checkpoint(path)
                .and_then(|p| Ok(fs::canonicalize(p)?))
            {
                pinned.insert(path);
            }
        }
    }
    // Follow-ups can be queued before their asset state has been initialized.
    for task in store.list::<serde_json::Value>("task")? {
        for recorded in [
            task["assetRestore"].as_str(),
            task["assetFeedbackSeed"]["checkpoint"].as_str(),
        ]
        .into_iter()
        .flatten()
        {
            if let Ok(path) = files
                .resolve_checkpoint(recorded)
                .and_then(|p| Ok(fs::canonicalize(p)?))
            {
                pinned.insert(path);
            }
        }
    }
    let mut unused = Vec::new();
    for entry in fs::read_dir(&directory)? {
        let entry = entry?;
        if entry.file_type()?.is_file() && entry.path().extension().is_some_and(|e| e == "blend") {
            let path = fs::canonicalize(entry.path())?;
            if path.parent() == Some(directory.as_path()) && !pinned.contains(&path) {
                unused.push((entry.metadata()?.modified()?, path));
            }
        }
    }
    unused.sort_by_key(|item| item.0);
    for (_, path) in unused.iter().take(unused.len().saturating_sub(2)) {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub async fn save(store: &Arc<Mutex<Store>>, files: &Files, client: &Client) -> Result<String> {
    {
        let db = store
            .lock()
            .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
        anyhow::ensure!(
            asset_task::get(&db, &client.task_id)?.session_id.as_deref()
                == Some(&client.session_id),
            "Blender session changed before saving"
        );
        prune(&db, files, &client.task_id, &client.checkpoints)?;
    }
    let path = client.checkpoint().await?;
    let path = files.checkpoint_location(&client.task_id, Path::new(&path))?;
    let db = store
        .lock()
        .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
    let mut state = asset_task::get(&db, &client.task_id)?;
    anyhow::ensure!(
        state.session_id.as_deref() == Some(&client.session_id),
        "Blender session changed while saving"
    );
    state.checkpoint = Some(path.clone());
    asset_task::save(&db, &state)?;
    Ok(path)
}

pub async fn final_frame(store: &Arc<Mutex<Store>>, files: &Files, client: &Client) -> Result<()> {
    let (frame, bytes) = client
        .latest()
        .await
        .context("Final observer frame unavailable")?;
    let db = store
        .lock()
        .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
    let mut reference = asset_reference::capture(&db, files, &client.task_id, frame, &bytes)?;
    reference.used = true;
    db.put("asset-reference", &reference.id, &reference)?;
    let mut state = asset_task::get(&db, &client.task_id)?;
    state.last_frame = Some(reference);
    asset_task::save(&db, &state)
}
