use serde_json::{json, Value};
use std::{collections::HashSet, time::Duration};
use tokio::time::Instant;

#[derive(Clone, Copy)]
pub struct IdleLimits {
    pub warning_after: Duration,
    pub pause_after: Duration,
}

impl Default for IdleLimits {
    fn default() -> Self {
        Self {
            warning_after: Duration::from_secs(180),
            pause_after: Duration::from_secs(300),
        }
    }
}

pub(crate) struct Health {
    limits: IdleLimits,
    last_progress: Instant,
    last_progress_at: String,
    tools: HashSet<String>,
    compacting: bool,
    warned: bool,
    retries: u64,
}

impl Health {
    pub fn new(limits: IdleLimits) -> Self {
        Self {
            limits,
            last_progress: Instant::now(),
            last_progress_at: now(),
            tools: HashSet::new(),
            compacting: false,
            warned: false,
            retries: 0,
        }
    }

    fn progress(&mut self) {
        self.last_progress = Instant::now();
        self.last_progress_at = now();
        self.warned = false;
    }

    pub fn observe(&mut self, method: &str, params: &Value) {
        match method {
            "item/started" | "item/completed" => {
                let item = &params["item"];
                let kind = item["type"].as_str().unwrap_or("");
                let complete = method == "item/completed";
                if is_tool(kind) {
                    if let Some(id) = item["id"].as_str() {
                        let changed = if complete {
                            self.tools.remove(id)
                        } else {
                            self.tools.insert(id.to_owned())
                        };
                        if changed || complete {
                            self.progress();
                        }
                    }
                } else if kind == "contextCompaction" {
                    if self.compacting == complete {
                        self.compacting = !complete;
                        self.progress();
                    }
                } else if complete && matches!(kind, "agentMessage" | "reasoning") {
                    self.progress();
                }
            }
            "item/agentMessage/delta"
            | "item/reasoning/summaryTextDelta"
            | "item/reasoning/textDelta"
            | "item/commandExecution/outputDelta" => {
                if params["delta"].as_str().is_some_and(|s| !s.is_empty()) {
                    self.progress();
                }
            }
            "turn/plan/updated" => self.progress(),
            _ => {}
        }
    }

    pub fn retry(&mut self) {
        // A retry notice is not useful progress and must not extend the deadline.
        self.retries = self.retries.saturating_add(1);
    }

    pub fn next_turn(&mut self) {
        self.tools.clear();
        self.compacting = false;
    }

    pub fn warning(&mut self) -> bool {
        if self.tools.is_empty()
            && !self.warned
            && self.last_progress.elapsed() >= self.limits.warning_after
        {
            self.warned = true;
            true
        } else {
            false
        }
    }

    pub fn stalled(&self) -> bool {
        self.tools.is_empty() && self.last_progress.elapsed() >= self.limits.pause_after
    }

    pub fn snapshot(&self) -> Value {
        json!({
            "phase": if !self.tools.is_empty() { "tool" } else if self.compacting { "compacting" } else { "model" },
            "lastProgressAt": self.last_progress_at,
            "idleSeconds": self.last_progress.elapsed().as_secs(),
            "activeTools": self.tools.len(),
            "retries": self.retries,
            "warning": self.warned,
            "warningAfterSeconds": self.limits.warning_after.as_secs(),
            "pauseAfterSeconds": self.limits.pause_after.as_secs()
        })
    }
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn is_tool(kind: &str) -> bool {
    matches!(
        kind,
        "mcpToolCall"
            | "commandExecution"
            | "dynamicToolCall"
            | "fileChange"
            | "webSearch"
            | "imageGeneration"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retries_and_usage_do_not_hide_a_stall_but_tools_and_output_do() {
        let mut health = Health::new(IdleLimits::default());
        health.last_progress = Instant::now() - Duration::from_secs(301);
        health.retry();
        health.observe("thread/tokenUsage/updated", &json!({"totalTokens": 100}));
        assert!(health.warning());
        assert!(!health.warning());
        assert!(health.stalled());
        health.observe(
            "item/started",
            &json!({"item":{"id":"a","type":"commandExecution"}}),
        );
        health.observe(
            "item/started",
            &json!({"item":{"id":"b","type":"mcpToolCall"}}),
        );
        health.last_progress = Instant::now() - Duration::from_secs(900);
        assert!(!health.stalled());
        assert!(!health.warning());
        health.observe(
            "item/completed",
            &json!({"item":{"id":"a","type":"commandExecution"}}),
        );
        assert_eq!(health.snapshot()["activeTools"], 1);
        health.observe(
            "item/completed",
            &json!({"item":{"id":"b","type":"mcpToolCall"}}),
        );
        assert_eq!(health.snapshot()["activeTools"], 0);
        assert!(!health.stalled());
        health.last_progress = Instant::now() - Duration::from_secs(301);
        health.observe(
            "item/reasoning/textDelta",
            &json!({"delta":"still working"}),
        );
        assert!(!health.stalled());
        assert!(!health.snapshot().to_string().contains("still working"));
    }
}
