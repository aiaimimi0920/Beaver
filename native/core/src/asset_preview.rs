use crate::{asset_reference::MAX_IMAGE, asset_task::Frame};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{path::PathBuf, time::Duration};

#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    port: u16,
    token: String,
    pub task_id: String,
    pub project_id: String,
    pub session_id: String,
    pub checkpoints: PathBuf,
}

impl Client {
    pub fn new(
        port: u16,
        token: String,
        task_id: String,
        project_id: String,
        session_id: String,
        checkpoints: PathBuf,
    ) -> Result<Self> {
        Ok(Self {
            http: reqwest::Client::builder()
                .no_proxy()
                .connect_timeout(Duration::from_secs(1))
                .timeout(Duration::from_secs(6))
                .build()?,
            port,
            token,
            task_id,
            project_id,
            session_id,
            checkpoints,
        })
    }
    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        self.http
            .request(method, format!("http://127.0.0.1:{}{path}", self.port))
            .bearer_auth(&self.token)
            .header("X-Beaver-Task", &self.task_id)
            .header("X-Beaver-Project", &self.project_id)
            .header("X-Beaver-Session", &self.session_id)
    }
    async fn read(&self, mut response: reqwest::Response, limit: usize) -> Result<Vec<u8>> {
        if response
            .content_length()
            .is_some_and(|size| size > limit as u64)
        {
            bail!("Observer response exceeds limit");
        }
        let status = response.status();
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            if bytes.len() + chunk.len() > limit {
                bail!("Observer response exceeds limit");
            }
            bytes.extend_from_slice(&chunk);
        }
        if !status.is_success() {
            let error: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
            bail!(
                "{}",
                error["error"]
                    .as_str()
                    .unwrap_or("Observer connection failed")
            );
        }
        Ok(bytes)
    }
    pub async fn call(&self, method: &str, input: Value) -> Result<Value> {
        if ![
            "status",
            "view",
            "pick",
            "validate",
            "checkpoint",
            "frameMeta",
        ]
        .contains(&method)
        {
            bail!("Unknown observer method");
        }
        let response = self
            .request(reqwest::Method::POST, &format!("/{method}"))
            .json(&input)
            .send()
            .await?;
        Ok(serde_json::from_slice(
            &self.read(response, 128 * 1024).await?,
        )?)
    }
    pub async fn frame(&self, id: &str) -> Result<Vec<u8>> {
        crate::asset_task::validate_id(id)?;
        let response = self
            .request(reqwest::Method::GET, &format!("/frame/{id}"))
            .send()
            .await?;
        self.read(response, MAX_IMAGE).await
    }
    pub async fn latest(&self) -> Result<(Frame, Vec<u8>)> {
        let subscriber = format!("capture-{}", uuid::Uuid::new_v4());
        let result = self.latest_subscribed(&subscriber).await;
        let _ = self
            .call("status", json!({"subscriber":subscriber,"release":true}))
            .await;
        result
    }
    async fn latest_subscribed(&self, subscriber: &str) -> Result<(Frame, Vec<u8>)> {
        for _ in 0..10 {
            let status = self
                .call("status", json!({"subscriber":subscriber}))
                .await?;
            if let Ok(frame) = serde_json::from_value::<Frame>(status["frame"].clone()) {
                if frame.session_id != self.session_id {
                    bail!("Preview identity changed");
                }
                let age = (chrono::Utc::now().timestamp_millis().max(0) as u64)
                    .saturating_sub(frame.captured_at);
                if status["sceneRevision"] == frame.scene_revision
                    && status["generation"] == frame.generation
                    && status["busy"].is_null()
                    && age < 2000
                {
                    if let Ok(bytes) = self.frame(&frame.id).await {
                        return Ok((frame, bytes));
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(120)).await;
        }
        bail!("Frame changed during capture; retry")
    }
    pub async fn capture(&self, id: &str) -> Result<(Frame, Vec<u8>)> {
        crate::asset_task::validate_id(id)?;
        let frame: Frame =
            serde_json::from_value(self.call("frameMeta", json!({"frameId":id})).await?)?;
        if frame.id != id || frame.session_id != self.session_id {
            bail!("Preview identity changed");
        }
        let bytes = self.frame(id).await?;
        Ok((frame, bytes))
    }
    pub async fn checkpoint(&self) -> Result<String> {
        let result = self.call("checkpoint", json!({})).await?;
        let path = PathBuf::from(
            result["path"]
                .as_str()
                .context("Blender did not report a recovery scene")?,
        );
        let canonical = std::fs::canonicalize(&path)?;
        if !canonical.starts_with(std::fs::canonicalize(&self.checkpoints)?)
            || std::fs::metadata(&canonical)?.len() < 12
        {
            bail!("Invalid recovery scene location");
        }
        Ok(canonical.to_string_lossy().into_owned())
    }
}
