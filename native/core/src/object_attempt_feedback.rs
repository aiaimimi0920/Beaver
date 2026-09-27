//! Model image transport shared by publication follow-ups and candidate rework.
use crate::object_run_recovery::{candidate::publication::followup::image, resume::rework::frame};
use crate::{object_attempt::Attempt, object_attempt_file, project_runtime::ProjectRuntime};
use anyhow::Result;
use serde::Serialize;

#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum Binding {
    Rework(frame::Binding),
    Publication(image::Binding),
}

impl Binding {
    pub fn historical_data_url(&self) -> Option<&str> {
        let relocation = match self {
            Self::Rework(value) => value.relocation.as_ref(),
            Self::Publication(value) => value.relocation.as_ref(),
        };
        relocation.map(|value| value.data_url.as_str())
    }
    pub fn file(&self) -> Option<&object_attempt_file::Request> {
        match self {
            Self::Rework(_) => None,
            Self::Publication(value) => value.file.as_ref(),
        }
    }
    pub fn data_url(&self) -> Option<&str> {
        match self {
            Self::Rework(value) => Some(&value.data_url),
            Self::Publication(value) => value.data_url.as_deref(),
        }
    }
}

pub(crate) fn resolve(runtime: &ProjectRuntime, attempt: &Attempt) -> Result<Option<Binding>> {
    if let Some(binding) = frame::resolve(runtime, attempt)? {
        return Ok(Some(Binding::Rework(binding)));
    }
    Ok(image::resolve(runtime, attempt)?.map(Binding::Publication))
}
