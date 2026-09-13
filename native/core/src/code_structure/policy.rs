use anyhow::{bail, ensure, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Stamp {
    pub effective_lines: usize,
    pub sha256: String,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Baseline {
    pub schema_version: u32,
    pub files: BTreeMap<String, Stamp>,
}

impl Baseline {
    pub fn empty() -> Self {
        Self {
            schema_version: 1,
            files: BTreeMap::new(),
        }
    }

    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let value: Self = serde_json::from_slice(bytes)?;
        ensure!(
            value.schema_version == 1,
            "Unsupported code-structure baseline version"
        );
        Ok(value)
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Exception {
    pub effective_lines: usize,
    pub sha256: String,
    pub responsibility: String,
    pub reason: String,
    pub tests: Vec<String>,
}

impl Exception {
    pub fn matches(&self, stamp: &Stamp) -> bool {
        (501..=700).contains(&stamp.effective_lines)
            && self.effective_lines == stamp.effective_lines
            && self.sha256 == stamp.sha256
            && !self.responsibility.trim().is_empty()
            && !self.reason.trim().is_empty()
            && !self.tests.is_empty()
            && self.tests.iter().all(|test| !test.trim().is_empty())
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Exceptions {
    pub schema_version: u32,
    pub files: BTreeMap<String, Exception>,
}

impl Default for Exceptions {
    fn default() -> Self {
        Self {
            schema_version: 1,
            files: BTreeMap::new(),
        }
    }
}

impl Exceptions {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let value: Self = serde_json::from_slice(bytes)?;
        ensure!(
            value.schema_version == 1,
            "Unsupported code-structure exception version"
        );
        ensure!(
            value.files.len() <= 100,
            "Too many code-structure exceptions"
        );
        Ok(value)
    }

    pub fn validate(&self, stamps: &BTreeMap<String, Stamp>) -> Vec<String> {
        self.files.iter().filter_map(|(path, exception)| {
            if stamps.get(path).is_some_and(|stamp| exception.matches(stamp)) {
                None
            } else {
                Some(format!("{path}: stale or invalid 501-700 line exception; split or update its evidence"))
            }
        }).collect()
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub path: String,
    #[serde(flatten)]
    pub stamp: Stamp,
    pub status: &'static str,
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub ok: bool,
    pub files: Vec<Entry>,
    pub violations: Vec<String>,
}

impl Report {
    pub fn ensure_ok(&self) -> Result<()> {
        if !self.ok {
            bail!("Code structure check failed; continue the task and split the affected sources. Output remains in the task workspace. {}",
                self.violations.iter().take(20).cloned().collect::<Vec<_>>().join("; "));
        }
        Ok(())
    }
}

pub fn check(
    sources: BTreeMap<String, (Stamp, bool)>,
    baseline: &Baseline,
    exceptions: &Exceptions,
    strict: bool,
) -> Report {
    let stamps = sources
        .iter()
        .map(|(path, (stamp, _))| (path.clone(), stamp.clone()))
        .collect();
    let mut report = Report {
        violations: exceptions.validate(&stamps),
        ..Report::default()
    };
    for (path, (stamp, immutable)) in sources {
        let count = stamp.effective_lines;
        let status = if immutable {
            "immutable"
        } else if count <= 250 {
            "ok"
        } else if count <= 500 {
            "cohesion"
        } else if !strict && baseline.files.get(&path) == Some(&stamp) {
            "legacy"
        } else if count > 1500 {
            "hardLimit"
        } else if count > 700 {
            "splitRequired"
        } else if exceptions
            .files
            .get(&path)
            .is_some_and(|exception| exception.matches(&stamp))
        {
            "exception"
        } else {
            "exceptionRequired"
        };
        if matches!(status, "hardLimit" | "splitRequired" | "exceptionRequired") {
            let action = if count > 700 {
                "split into responsibility-owned modules (no exception above 700)"
            } else {
                "split, or document a current 501-700 line exception with protective tests"
            };
            report
                .violations
                .push(format!("{path}: {count} effective lines; {action}"));
        }
        report.files.push(Entry {
            path,
            stamp,
            status,
        });
    }
    report.ok = report.violations.is_empty();
    report
}
