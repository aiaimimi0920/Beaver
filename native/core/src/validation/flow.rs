use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Reference {
    pub path: String,
    #[serde(default)]
    pub node: String,
    #[serde(default)]
    pub symbol: String,
    #[serde(default = "authored")]
    pub source: String,
}
fn authored() -> String {
    "author".into()
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub seed: u32,
    pub locale: String,
    #[serde(default)]
    pub saves: Vec<String>,
    #[serde(default = "threshold")]
    pub threshold: f64,
    #[serde(default)]
    pub masks: Vec<[u32; 4]>,
}
fn threshold() -> f64 {
    0.98
}
impl Default for Config {
    fn default() -> Self {
        Self {
            width: 960,
            height: 540,
            fps: 30,
            seed: 1,
            locale: "en".into(),
            saves: vec![],
            threshold: threshold(),
            masks: vec![],
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Action {
    Wait {
        frames: u32,
    },
    Action {
        name: String,
        pressed: bool,
    },
    Key {
        code: u32,
        pressed: bool,
    },
    Click {
        node: String,
    },
    WaitFor {
        node: String,
        property: String,
        equals: Value,
        timeout: u32,
    },
    Capture,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub id: String,
    #[serde(flatten)]
    pub action: Action,
    #[serde(default)]
    pub references: Vec<Reference>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Definition {
    pub key: String,
    pub name: String,
    pub category: String,
    pub purpose: String,
    #[serde(default)]
    pub entry: String,
    #[serde(default)]
    pub task_ids: Vec<String>,
    #[serde(default)]
    pub config: Config,
    pub steps: Vec<Step>,
    #[serde(default)]
    pub video: bool,
    #[serde(default)]
    pub references: Vec<Reference>,
    #[serde(default)]
    pub retired_reason: String,
    #[serde(default)]
    pub roaming: Option<super::roaming::Roaming>,
}

pub fn relative(path: &str) -> Result<&str> {
    let path = path.strip_prefix("res://").unwrap_or(path);
    if path.is_empty()
        || path.contains(['\\', ':', '\0'])
        || path
            .split('/')
            .any(|s| s.is_empty() || s == "." || s == "..")
    {
        bail!("Invalid project relative path: {path}");
    }
    Ok(path)
}

impl Definition {
    pub fn validate(&self) -> Result<()> {
        if let Some(roaming) = &self.roaming {
            roaming.validate()?;
        }
        if self.key.is_empty()
            || self.key.len() > 100
            || !self
                .key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
        {
            bail!("Flow key must contain 1-100 letters, digits, '-' or '_'");
        }
        if !["roaming", "feature", "ui", "task"].contains(&self.category.as_str())
            || self.name.trim().is_empty()
            || self.purpose.trim().is_empty()
        {
            bail!("Flow requires a category, name and purpose");
        }
        if !self.entry.is_empty() {
            relative(&self.entry)?;
        }
        let c = &self.config;
        if !(64..=3840).contains(&c.width)
            || !(64..=2160).contains(&c.height)
            || !(1..=60).contains(&c.fps)
            || !(0.8..=1.0).contains(&c.threshold)
            || c.locale.is_empty()
            || c.locale.len() > 40
        {
            bail!("Invalid capture configuration");
        }
        for path in &c.saves {
            relative(path)?;
        }
        let mut masked = 0u64;
        for [x, y, w, h] in &c.masks {
            if *w == 0
                || *h == 0
                || x.saturating_add(*w) > c.width
                || y.saturating_add(*h) > c.height
            {
                bail!("Invalid comparison mask");
            }
            masked += u64::from(*w) * u64::from(*h);
        }
        if masked * 2 >= u64::from(c.width) * u64::from(c.height) {
            bail!("Comparison masks must leave at least half of the image visible");
        }
        if self.steps.is_empty() || self.steps.len() > 500 {
            bail!("Expected 1-500 steps");
        }
        let mut ids = BTreeSet::new();
        let mut frames = 0;
        let mut captures = 0;
        for step in &self.steps {
            if step.id.is_empty() || step.id.len() > 100 || !ids.insert(&step.id) {
                bail!("Step IDs must be nonempty and unique");
            }
            match &step.action {
                Action::Wait { frames: n } if (1..=18000).contains(n) => frames += n,
                Action::WaitFor {
                    node,
                    property,
                    timeout,
                    ..
                } if !node.is_empty() && !property.is_empty() && (1..=18000).contains(timeout) => {
                    frames += timeout
                }
                Action::Action { name, .. } if !name.is_empty() => {}
                Action::Key { code, .. } if *code > 0 => {}
                Action::Click { node } if !node.is_empty() => {}
                Action::Capture => captures += 1,
                _ => bail!("Invalid step {}", step.id),
            }
            for reference in &step.references {
                validate_reference(reference)?;
            }
        }
        if captures == 0 || captures > 100 || frames > c.fps * 600 {
            bail!("Flow requires 1-100 capture points and a duration at most 10 minutes");
        }
        for reference in &self.references {
            validate_reference(reference)?;
        }
        Ok(())
    }

    pub fn signature(&self) -> Result<String> {
        super::repository::digest(&serde_json::json!({"entry":self.entry,"config":self.config,
            "steps":self.steps,"video":self.video,"runner":super::RUNNER_VERSION}))
    }
}

fn validate_reference(reference: &Reference) -> Result<()> {
    relative(&reference.path)?;
    if !["author", "runtime", "ai"].contains(&reference.source.as_str()) {
        bail!("Unknown reference source");
    }
    Ok(())
}
