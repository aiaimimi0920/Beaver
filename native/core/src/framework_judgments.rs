use crate::{
    asset_delivery_files as artifacts,
    files::{Files, Snapshot},
    framework_checks::digest,
    store::Store,
};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

#[allow(clippy::too_many_arguments)]
pub fn judge(
    store: Arc<Mutex<Store>>,
    files: &Files,
    task: &str,
    candidate_id: &str,
    revision: u64,
    verdict: &str,
    note: &str,
    paths: &[String],
    identity: Option<&(String, String)>,
) -> Result<Value> {
    ensure!(
        ["pass", "fail"].contains(&verdict) && !note.trim().is_empty() && note.len() <= 12000,
        "A semantic judgment requires verdict and a reason of at most 12000 bytes"
    );
    ensure!(
        !paths.is_empty() && paths.len() <= 16,
        "Select 1 to 16 frozen candidate images as visual evidence"
    );
    let (candidate, configuration, captured) = {
        let db = store
            .lock()
            .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
        let config = crate::framework::configuration(&db, task)?;
        ensure!(
            config.revision == revision,
            "Framework configuration changed"
        );
        (
            artifacts::get(&db, task, candidate_id)?,
            config,
            crate::framework_evidence::context(&db, task)?,
        )
    };
    let mut visual = Snapshot::new();
    for path in paths {
        let bytes = artifacts::read(files, &candidate, path)?;
        let mut reader =
            image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format()?;
        let mut limits = image::Limits::default();
        limits.max_alloc = Some(64 * 1024 * 1024);
        reader.limits(limits);
        reader
            .decode()
            .context("Semantic evidence must be a decodable candidate image")?;
        visual.insert(path.clone(), candidate.files[path].clone());
    }
    let source = if identity.is_some() { "model" } else { "owner" };
    let judgment = json!({"candidateId":candidate_id,"candidateSha256":digest(&candidate)?,"configurationRevision":revision,"configurationSha256":digest(&configuration)?,"source":source,"verdict":verdict,"note":note,"visual":visual,"time":crate::asset_task::now()});
    let mut db = store
        .lock()
        .map_err(|_| anyhow::anyhow!("Database lock unavailable"))?;
    if let Some((thread, turn)) = identity {
        crate::task_callback::active(&db.connection, task, thread, turn)?;
    }
    ensure!(
        crate::framework_evidence::context(&db, task)? == captured
            && crate::framework::configuration(&db, task)? == configuration
            && digest(&artifacts::get(&db, task, candidate_id)?)? == digest(&candidate)?,
        "Judgment inputs changed during image verification"
    );
    db.transaction(|db| {
        crate::task_callback::put(
            db,
            &format!("framework-judgment/{task}"),
            &format!("{source}-{candidate_id}"),
            &judgment,
        )?;
        crate::task_callback::put(
            db,
            &format!("framework-judgment-history/{task}"),
            &uuid::Uuid::new_v4().to_string(),
            &judgment,
        )
    })?;
    Ok(judgment)
}
