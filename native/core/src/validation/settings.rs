use super::{repository, requests::Request};
use crate::store::Store;
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub revision: u32,
    pub visual_required: bool,
    pub ffmpeg: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            revision: 0,
            visual_required: true,
            ffmpeg: String::new(),
        }
    }
}

pub fn read(store: &Store, project: &str) -> Result<Settings> {
    repository::project(store, project)?;
    Ok(store
        .get("validationSettings", project)?
        .unwrap_or_default())
}

pub fn save(store: &mut Store, input: &Value) -> Result<Value> {
    let request = Request::new("validation.settings.save", input)?;
    if let Some(result) = request.replay(store)? {
        return Ok(result);
    }
    let project = super::operations::string(input, "projectId")?;
    let current = read(store, project)?;
    let mut next: Settings = serde_json::from_value(input["settings"].clone())?;
    ensure!(
        input["expectedRevision"].as_u64() == Some(u64::from(current.revision)),
        "Validation settings changed; refresh before saving"
    );
    ensure!(
        next.ffmpeg.len() <= 2000 && !next.ffmpeg.contains('\0'),
        "Invalid FFmpeg path"
    );
    next.revision = current.revision + 1;
    let result = serde_json::to_value(next)?;
    request.finish(
        store,
        result.clone(),
        vec![("validationSettings", project.into(), result)],
    )
}
