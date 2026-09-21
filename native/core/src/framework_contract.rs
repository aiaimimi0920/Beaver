use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Command {
    pub executable: String,
    pub sha256: String,
    pub args: Vec<String>,
    pub timeout_seconds: u64,
    #[serde(default)]
    pub files: std::collections::BTreeMap<String, String>,
}

impl Command {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            std::path::Path::new(&self.executable).is_absolute(),
            "Adapter executable must be absolute"
        );
        ensure!(
            self.sha256.len() == 64
                && self
                    .sha256
                    .bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)),
            "Pin adapter executable SHA-256"
        );
        ensure!(
            (1..=600).contains(&self.timeout_seconds)
                && self.args.len() <= 64
                && self.args.iter().all(|a| a.len() <= 4000),
            "Invalid adapter limits"
        );
        ensure!(
            self.files.len() <= 32,
            "Pin at most 32 adapter scripts/resources"
        );
        for (path, hash) in &self.files {
            ensure!(
                std::path::Path::new(path).is_absolute()
                    && hash.len() == 64
                    && hash
                        .bytes()
                        .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)),
                "Adapter resource requires absolute path and SHA-256"
            );
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Plugin {
    pub id: String,
    pub host: String,
    pub version: String,
    pub host_version: String,
    pub probe: Command,
    pub install: Option<Command>,
    pub enable: Option<Command>,
    pub reload: Option<Command>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Rule {
    pub id: String,
    pub version: u32,
    pub checker: Command,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Configuration {
    pub revision: u64,
    pub plugins: Vec<Plugin>,
    pub rules: Vec<Rule>,
    pub semantic_required: bool,
}

impl Configuration {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.plugins.len() <= 32 && self.rules.len() <= 32,
            "At most 32 plugins and rules"
        );
        let mut ids = std::collections::HashSet::new();
        for p in &self.plugins {
            crate::asset_task::validate_id(&p.id)?;
            ensure!(
                ids.insert(&p.id) && ["godot", "blender", "codex"].contains(&p.host.as_str()),
                "Duplicate plugin or unsupported host"
            );
            ensure!(
                !p.version.is_empty()
                    && p.version.len() <= 100
                    && !p.host_version.is_empty()
                    && p.host_version.len() <= 100,
                "Pin plugin and host versions"
            );
            for command in [
                Some(&p.probe),
                p.install.as_ref(),
                p.enable.as_ref(),
                p.reload.as_ref(),
            ]
            .into_iter()
            .flatten()
            {
                command.validate()?;
            }
        }
        ids.clear();
        for rule in &self.rules {
            crate::asset_task::validate_id(&rule.id)?;
            ensure!(
                ids.insert(&rule.id) && rule.version > 0,
                "Duplicate or unversioned rule"
            );
            rule.checker.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Job {
    Callback {
        request: Value,
    },
    Plugin {
        plugin: String,
        action: String,
    },
    Check {
        #[serde(rename = "candidateId")]
        candidate_id: String,
    },
    InputExport {
        #[serde(rename = "attemptId")]
        attempt_id: String,
    },
    Dependencies {
        command: Command,
        paths: Vec<String>,
    },
}

#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "camelCase", deny_unknown_fields)]
pub enum Request {
    State,
    Inspect {
        #[serde(rename = "operationId")]
        operation_id: String,
    },
    Cancel {
        #[serde(rename = "operationId")]
        operation_id: String,
    },
    Start {
        #[serde(rename = "requestId")]
        request_id: String,
        job: Job,
    },
    Configure {
        configuration: Configuration,
    },
    Judge {
        #[serde(rename = "candidateId")]
        candidate_id: String,
        #[serde(rename = "configurationRevision")]
        configuration_revision: u64,
        verdict: String,
        note: String,
        paths: Vec<String>,
    },
}

pub fn definition() -> Value {
    json!({"type":"function","name":"beaver_workflow","description":"Generic framework: state lists configured plugin adapters, immutable checks, observed traces, recovery and durable operations. start queues one job per task; poll inspect until terminal; cancel never replays side effects. Jobs: callback {request} runs the existing beaver_task file callback asynchronously; plugin {plugin,action:probe|install|enable|reload}; check {candidateId}; inputExport {attemptId}; dependencies {command,paths}. Commands use absolute executable, sha256, args, timeoutSeconds. Configure is owner-only. judge records MODEL opinion over candidate image paths, never owner approval. A succeeded callback operation may return paused:true: end this turn immediately.","inputSchema":{
    "type":"object","additionalProperties":false,"required":["operation"],"properties":{
        "operation":{"type":"string","enum":["state","inspect","cancel","start","judge"]},
        "operationId":{"type":"string"},"requestId":{"type":"string"},"job":{"type":"object"},
        "candidateId":{"type":"string"},"configurationRevision":{"type":"integer"},"verdict":{"type":"string","enum":["pass","fail"]},"note":{"type":"string"},"paths":{"type":"array","items":{"type":"string"}}
    }}})
}

pub fn business_tools() -> Vec<Value> {
    vec![
        json!({"name":"task.framework","description":"Owner access to generic framework state, durable start/inspect/cancel, configure adapters and versioned rules, and candidate semantic judgments. Configure requires the current revision; Beaver increments it. No automatic owner approval.","inputSchema":{"type":"object","additionalProperties":false,"required":["id","request"],"properties":{"id":{"type":"string"},"request":{"type":"object"}}},"annotations":{"readOnlyHint":false,"destructiveHint":false,"openWorldHint":true}}),
    ]
}
