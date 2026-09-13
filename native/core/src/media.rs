use crate::{files::safe_path, preferences::valid_base};
use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::{
    multipart::{Form, Part},
    Client, Response,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::PathBuf,
    time::Duration,
};

const MEDIA_LIMIT: usize = 100 * 1024 * 1024;
const JSON_LIMIT: usize = 140 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Provider {
    base_url: String,
    model: String,
    route: String,
    key: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    prompt: String,
    output: Option<String>,
    #[serde(default)]
    references: Vec<String>,
    voice: Option<String>,
}

pub struct Media {
    root: PathBuf,
    providers: BTreeMap<String, Option<Provider>>,
    client: Client,
    downloads: Client,
}

async fn bounded(mut response: Response, limit: usize) -> Result<Vec<u8>> {
    if response.content_length().is_some_and(|n| n > limit as u64) {
        bail!("AI response exceeds size limit");
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| anyhow::anyhow!("Unable to read AI response"))?
    {
        if chunk.len() > limit.saturating_sub(bytes.len()) {
            bail!("AI response exceeds size limit");
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

impl Media {
    pub fn new(root: PathBuf, configuration: &str) -> Result<Self> {
        let providers = serde_json::from_str(configuration)
            .map_err(|_| anyhow::anyhow!("Invalid media provider configuration"))?;
        let client = Client::builder()
            .timeout(Duration::from_secs(180))
            .connect_timeout(Duration::from_secs(20))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .context("Unable to initialize media client")?;
        let downloads = Client::builder()
            .https_only(true)
            .timeout(Duration::from_secs(120))
            .connect_timeout(Duration::from_secs(20))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .context("Unable to initialize media download client")?;
        Ok(Self {
            root,
            providers,
            client,
            downloads,
        })
    }

    pub async fn call(&self, name: &str, raw: Value) -> Result<String> {
        if name == crate::code_structure::tool::NAME {
            return crate::code_structure::tool::call(self.root.clone(), raw).await;
        }
        if matches!(name, "beaver_workflow_list" | "beaver_workflow_run") {
            return crate::workflows::call(self.root.clone(), name, raw).await;
        }
        let capability = match name {
            "generate_image" => "image",
            "generate_speech" => "speech",
            "generate_music" => "music",
            "translate_text" => "translation",
            _ => bail!("Unknown tool"),
        };
        let input: Input = serde_json::from_value(raw)
            .map_err(|_| anyhow::anyhow!("Invalid media tool arguments"))?;
        if !(1..=30000).contains(&input.prompt.encode_utf16().count()) || input.references.len() > 5
        {
            bail!("Media tool arguments exceed limits");
        }
        let provider = self
            .providers
            .get(capability)
            .and_then(Option::as_ref)
            .with_context(|| {
                format!(
                    "{capability} service is not configured. Do not claim generation succeeded."
                )
            })?;
        if !self.root.is_dir() {
            bail!("Project root missing");
        }
        let output = if capability == "translation" {
            None
        } else {
            let relative = input
                .output
                .as_deref()
                .filter(|s| !s.is_empty())
                .context("Output path required")?;
            let path = safe_path(&self.root, relative)?;
            if path.try_exists()? {
                bail!("Output already exists; choose a new asset path");
            }
            Some(path)
        };
        let mut route = provider.route.clone();
        if route.is_empty() && capability == "translation" {
            route = "/responses".into();
        }
        if !route.starts_with('/')
            || route.starts_with("//")
            || route.contains(['\\', '#', '\r', '\n'])
        {
            bail!("Configure an explicit API route for this capability");
        }
        let base = valid_base(&provider.base_url)?;
        let form = if capability == "image" && !input.references.is_empty() {
            if route == "/images/generations" {
                route = "/images/edits".into();
            }
            let mut form = Form::new()
                .text("model", provider.model.clone())
                .text("prompt", input.prompt.clone());
            for reference in &input.references {
                let path = safe_path(&self.root, reference)?;
                let mut bytes = Vec::new();
                fs::File::open(&path)?
                    .take(20 * 1024 * 1024 + 1)
                    .read_to_end(&mut bytes)?;
                if bytes.len() > 20 * 1024 * 1024 {
                    bail!("Reference image exceeds 20 MB");
                }
                let extension = path
                    .extension()
                    .and_then(|v| v.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                let mime = match extension.as_str() {
                    "jpg" | "jpeg" => "image/jpeg",
                    "webp" => "image/webp",
                    _ => "image/png",
                };
                let filename = path
                    .file_name()
                    .and_then(|v| v.to_str())
                    .context("Invalid reference filename")?
                    .to_owned();
                form = form.part(
                    "image[]",
                    Part::bytes(bytes).file_name(filename).mime_str(mime)?,
                );
            }
            Some(form)
        } else {
            None
        };
        let mut request = self.client.post(format!("{base}{route}"));
        if !provider.key.is_empty() {
            request = request.bearer_auth(&provider.key);
        }
        request = if let Some(form) = form {
            request.multipart(form)
        } else {
            request.json(&match capability {
                "speech" => json!({"model":provider.model,"input":input.prompt,"voice":input.voice.filter(|s|!s.is_empty()).unwrap_or_else(||"alloy".into()),"response_format":"wav"}),
                "translation" => json!({"model":provider.model,"input":input.prompt}),
                _ => json!({"model":provider.model,"prompt":input.prompt}),
            })
        };
        // Never return reqwest errors: they can contain credential-bearing URLs.
        let response = request
            .send()
            .await
            .map_err(|_| anyhow::anyhow!("AI request could not be completed"))?;
        if !response.status().is_success() {
            bail!("AI request failed: HTTP {}", response.status().as_u16());
        }
        if capability == "translation" {
            let result: Value = serde_json::from_slice(&bounded(response, 4 * 1024 * 1024).await?)
                .map_err(|_| anyhow::anyhow!("Invalid translation JSON response"))?;
            return Ok(result.to_string());
        }
        let is_json = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.contains("json"));
        let bytes = if is_json {
            let result: Value = serde_json::from_slice(&bounded(response, JSON_LIMIT).await?)
                .map_err(|_| anyhow::anyhow!("Invalid media JSON response"))?;
            let item = &result["data"][0];
            if let Some(encoded) = item["b64_json"].as_str().filter(|s| !s.is_empty()) {
                STANDARD
                    .decode(encoded)
                    .map_err(|_| anyhow::anyhow!("Invalid media base64 response"))?
            } else if let Some(url) = item["url"].as_str().filter(|s| !s.is_empty()) {
                let url = url::Url::parse(url)
                    .map_err(|_| anyhow::anyhow!("Invalid media download URL"))?;
                if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some()
                {
                    bail!("Media download requires HTTPS without credentials");
                }
                let media = self
                    .downloads
                    .get(url)
                    .send()
                    .await
                    .map_err(|_| anyhow::anyhow!("Media download failed"))?;
                if !media.status().is_success() {
                    bail!("Media download failed");
                }
                bounded(media, MEDIA_LIMIT).await?
            } else {
                bail!("Provider returned no media");
            }
        } else {
            bounded(response, MEDIA_LIMIT).await?
        };
        if bytes.is_empty() || bytes.len() > MEDIA_LIMIT {
            bail!("Invalid media size (maximum 100 MB)");
        }
        let output = output.context("Output path required")?;
        let parent = output.parent().context("Invalid output directory")?;
        fs::create_dir_all(parent)?;
        let checked = safe_path(&self.root, input.output.as_deref().unwrap_or(""))?;
        let mut temp = tempfile::NamedTempFile::new_in(parent)?;
        temp.write_all(&bytes)?;
        temp.as_file().sync_all()?;
        temp.persist_noclobber(checked).map_err(|_| {
            anyhow::anyhow!("Cannot create media output; existing assets are never overwritten")
        })?;
        Ok(json!({"path":input.output,"bytes":bytes.len()}).to_string())
    }
}

pub fn tools() -> Value {
    let entries = [
        (
            "generate_image",
            "Generate or edit a project image using configured image service.",
            true,
        ),
        (
            "generate_speech",
            "Generate speech through configured speech service.",
            true,
        ),
        (
            "generate_music",
            "Generate music using the configured Beaver media response contract.",
            true,
        ),
        (
            "translate_text",
            "Translate text through configured Responses API service.",
            false,
        ),
    ];
    Value::Array(entries.into_iter().map(|(name,description,output)| {
        let mut properties = json!({"prompt":{"type":"string"}});
        let mut required = vec!["prompt"];
        if output { properties["output"]=json!({"type":"string"}); required.push("output"); }
        if name == "generate_image" { properties["references"]=json!({"type":"array","items":{"type":"string"}}); }
        if name == "generate_speech" { properties["voice"]=json!({"type":"string"}); }
        json!({"name":name,"description":description,"inputSchema":{"type":"object","properties":properties,"required":required,"additionalProperties":false}})
    }).collect())
}
