use crate::{asset_preview::Client, asset_reference, asset_task, store::Store};
use anyhow::{Context, Result};
use std::{
    collections::HashSet,
    fs,
    path::Path,
    sync::{Arc, Mutex},
};

/// Never remove recovery files referenced by any task, including a late follow-up.
pub fn prune(store: &Store, directory: &Path) -> Result<()> {
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
        {
            if let Ok(path) = fs::canonicalize(path) {
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

pub async fn save(store: &Arc<Mutex<Store>>, client: &Client) -> Result<String> {
    {
        let db = store
            .lock()
            .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
        prune(&db, &client.checkpoints)?;
    }
    let path = client.checkpoint().await?;
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

pub async fn final_frame(store: &Arc<Mutex<Store>>, root: &Path, client: &Client) -> Result<()> {
    let (frame, bytes) = client
        .latest()
        .await
        .context("Final observer frame unavailable")?;
    let db = store
        .lock()
        .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
    let mut reference = asset_reference::capture(&db, root, &client.task_id, frame, &bytes)?;
    reference.used = true;
    db.put("asset-reference", &reference.id, &reference)?;
    let mut state = asset_task::get(&db, &client.task_id)?;
    state.last_frame = Some(reference);
    asset_task::save(&db, &state)
}
