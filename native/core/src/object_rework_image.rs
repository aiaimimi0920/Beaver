//! Regions refer to one immutable PNG in the reviewed attempt's output.
use crate::{
    object_attempt::Attempt, object_attempt_file as file, project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Context, Result};
use base64::Engine;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Region {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub prompt: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Feedback {
    pub path: String,
    pub sha256: String,
    pub width: u32,
    pub height: u32,
    pub regions: Vec<Region>,
}

impl Feedback {
    pub(super) fn validate(&self) -> Result<()> {
        ensure!(
            self.path.len() <= 1024
                && self.path.to_ascii_lowercase().ends_with(".png")
                && self.sha256.len() == 64
                && self
                    .sha256
                    .bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
                && self.width > 0
                && self.height > 0
                && self.width <= 16384
                && self.height <= 16384
                && u64::from(self.width) * u64::from(self.height) <= 16_777_216
                && !self.regions.is_empty()
                && self.regions.len() <= 8,
            "INVALID_OBJECT_REWORK_IMAGE"
        );
        for region in &self.regions {
            ensure!(
                [region.x, region.y, region.width, region.height]
                    .iter()
                    .all(|v| v.is_finite())
                    && region.x >= 0.0
                    && region.y >= 0.0
                    && region.width > 0.0
                    && region.height > 0.0
                    && region.x + region.width <= 1.0
                    && region.y + region.height <= 1.0
                    && region.prompt.len() <= 1000,
                "INVALID_OBJECT_REWORK_IMAGE_REGION"
            );
        }
        Ok(())
    }

    pub(super) fn require_source(&self, attempt: &Attempt) -> Result<()> {
        self.validate()?;
        ensure!(
            attempt
                .output
                .as_ref()
                .and_then(|files| files.get(&self.path))
                == Some(&self.sha256),
            "OBJECT_REWORK_IMAGE_SOURCE_MISMATCH"
        );
        Ok(())
    }

    pub(crate) fn verify(&self, runtime: &ProjectRuntime, attempt: &Attempt) -> Result<()> {
        self.require_source(attempt)?;
        let response = file::read(
            runtime,
            &file::Request {
                project_id: runtime.project_id().into(),
                run_id: attempt.preparation.run.id.clone(),
                attempt_id: attempt.id.clone(),
                checkpoint: file::Checkpoint::Output,
                path: self.path.clone(),
                sha256: self.sha256.clone(),
            },
        )?;
        let file::Content::Image { mime, base64 } = response.content else {
            anyhow::bail!("OBJECT_REWORK_IMAGE_UNAVAILABLE");
        };
        ensure!(mime == "image/png", "OBJECT_REWORK_IMAGE_FORMAT");
        let bytes = base64::engine::general_purpose::STANDARD.decode(base64)?;
        let mut reader =
            image::ImageReader::with_format(std::io::Cursor::new(bytes), image::ImageFormat::Png);
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(16384);
        limits.max_image_height = Some(16384);
        limits.max_alloc = Some(128 * 1024 * 1024);
        reader.limits(limits);
        let decoded = reader
            .decode()
            .context("OBJECT_REWORK_IMAGE_DECODE_FAILED")?;
        ensure!(
            (decoded.width(), decoded.height()) == (self.width, self.height),
            "OBJECT_REWORK_IMAGE_DIMENSIONS_MISMATCH"
        );
        Ok(())
    }
}
