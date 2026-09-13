use super::flow::{Action, Definition, Step};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Roaming {
    pub actions: Vec<String>,
    pub rounds: u32,
    pub min_frames: u32,
    pub max_frames: u32,
    #[serde(default)]
    pub setup: Vec<Step>,
}

impl Roaming {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            !self.actions.is_empty()
                && self.actions.len() <= 30
                && self.actions.iter().all(|s| !s.is_empty() && s.len() <= 100),
            "Choose valid game input actions for roaming"
        );
        ensure!(
            (1..=50).contains(&self.rounds)
                && self.min_frames > 0
                && self.min_frames <= self.max_frames
                && self.max_frames <= 1800,
            "Invalid roaming duration"
        );
        ensure!(
            self.setup.len() <= 100,
            "Too many roaming preparation steps"
        );
        Ok(())
    }
}

/// Store the generated actions, not just a seed: subsequent runs replay these exact steps.
pub fn generate(definition: &mut Definition, seed: u32) -> Result<()> {
    let config = definition
        .roaming
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Roaming configuration missing"))?;
    config.validate()?;
    ensure!(
        definition.category == "roaming",
        "Only roaming flows can explore a new route"
    );
    let mut state = u64::from(seed) + 1;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut steps = config.setup.clone();
    for index in 0..config.rounds {
        let name = config.actions[(next() % config.actions.len() as u64) as usize].clone();
        let frames = config.min_frames
            + (next() % u64::from(config.max_frames - config.min_frames + 1)) as u32;
        for (suffix, action) in [
            (
                "press",
                Action::Action {
                    name: name.clone(),
                    pressed: true,
                },
            ),
            ("move", Action::Wait { frames }),
            ("capture", Action::Capture),
            (
                "release",
                Action::Action {
                    name,
                    pressed: false,
                },
            ),
        ] {
            steps.push(Step {
                id: format!("roam-{index}-{suffix}"),
                action,
                references: vec![],
            });
        }
    }
    definition.config.seed = seed;
    definition.steps = steps;
    definition.validate()
}
