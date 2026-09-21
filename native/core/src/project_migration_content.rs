//! Content-addressed history only; workspace, checkpoint and evidence paths are separate.
use crate::{
    data_backup::Entry,
    files::Files,
    project_migration_ownership::{Entity, Ownership},
    project_migration_references::{Checks, Issue},
};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Default, Serialize)]
pub struct Content {
    #[serde(flatten)]
    pub checks: Checks,
    /// A verified blob can be needed in more than one project partition.
    pub blobs: BTreeMap<String, BTreeSet<String>>,
}

struct Inspector<'a> {
    files: BTreeMap<&'a str, Option<&'a str>>,
    ownership: &'a Ownership,
    report: Content,
}

impl Inspector<'_> {
    fn invalid(&mut self, source: &Entity, field: &str, reason: &'static str) {
        self.report.checks.checked += 1;
        self.report
            .checks
            .issues
            .push(Issue::new(source, field, reason));
    }

    fn hash(&mut self, source: &Entity, field: &str, value: &Value) {
        let Some(hash) = value.as_str() else {
            return self.invalid(source, field, "INVALID_CONTENT_HASH");
        };
        if Files::new(Default::default()).blob(hash).is_err() {
            return self.invalid(source, field, "INVALID_CONTENT_HASH");
        }
        self.report.checks.checked += 1;
        let owner = self.ownership.entity(source);
        let reason = match self.files.get(format!("blobs/{hash}").as_str()) {
            None | Some(None) => Some("CONTENT_NOT_FOUND"),
            Some(Some(actual)) if *actual != hash => Some("CONTENT_HASH_MISMATCH"),
            _ if !matches!(owner, Ok(Some(_))) => Some("CONTENT_OWNER_UNRESOLVED"),
            _ => None,
        };
        if let Some(reason) = reason {
            let mut issue = Issue::new(source, field, reason);
            issue.target_kind = Some("blob".into());
            issue.target_id = Some(hash.into());
            self.report.checks.issues.push(issue);
        } else if let Ok(Some(project)) = owner {
            self.report
                .blobs
                .entry(hash.into())
                .or_default()
                .insert(project);
        }
    }

    fn snapshot(&mut self, source: &Entity, field: &str, value: Option<&Value>) {
        let Some(value) = value else { return };
        let Some(snapshot) = value.as_object() else {
            return self.invalid(source, field, "INVALID_SNAPSHOT");
        };
        for (path, hash) in snapshot {
            let pointer = path.replace('~', "~0").replace('/', "~1");
            self.hash(source, &format!("{field}/{pointer}"), hash);
        }
    }

    fn changes(&mut self, source: &Entity, field: &str, value: Option<&Value>) {
        let Some(value) = value else { return };
        let Some(changes) = value.as_array() else {
            return self.invalid(source, field, "INVALID_CHANGES");
        };
        for (index, change) in changes.iter().enumerate() {
            if !change.is_object() {
                self.invalid(source, &format!("{field}/{index}"), "INVALID_CHANGE");
                continue;
            }
            for side in ["before", "after"] {
                if let Some(hash) = change.get(side).filter(|hash| !hash.is_null()) {
                    self.hash(source, &format!("{field}/{index}/{side}"), hash);
                }
            }
        }
    }

    fn task(&mut self, source: &Entity, value: &Value, prefix: &str) {
        for field in [
            "/baseline",
            "/feature/snapshot",
            "/feature/previous/snapshot",
        ] {
            self.snapshot(source, &format!("{prefix}{field}"), value.pointer(field));
        }
        self.changes(source, &format!("{prefix}/changes"), value.get("changes"));
    }

    fn source_references(
        &mut self,
        source: &Entity,
        run: &Value,
        field: &str,
        items: Option<&Value>,
    ) {
        let Some(items) = items else { return };
        let Some(items) = items.as_array() else {
            return self.invalid(source, field, "INVALID_SOURCE_REFERENCE");
        };
        for (index, item) in items.iter().enumerate() {
            let field = format!("{field}/{index}/path");
            let path = item["path"]
                .as_str()
                .and_then(|path| crate::validation::flow::relative(path).ok());
            let Some(path) = path else {
                self.invalid(source, &field, "INVALID_SOURCE_REFERENCE");
                continue;
            };
            let Some(snapshot) = run.get("snapshot").and_then(Value::as_object) else {
                self.invalid(source, &field, "SOURCE_SNAPSHOT_UNAVAILABLE");
                continue;
            };
            // The snapshot pass verifies every hash; never fall back to live/project/workspace bytes.
            if snapshot.contains_key(path) {
                self.report.checks.checked += 1;
            } else {
                // Authored references may be absent; retain the declaration for explicit disposition.
                self.invalid(source, &field, "SOURCE_REFERENCE_UNAVAILABLE");
            }
        }
    }

    fn source_groups(&mut self, source: &Entity, run: &Value, field: &str, groups: Option<&Value>) {
        let Some(groups) = groups else { return };
        let Some(groups) = groups.as_array() else {
            return self.invalid(source, field, "INVALID_SOURCE_REFERENCE");
        };
        for (index, group) in groups.iter().enumerate() {
            let field = format!("{field}/{index}");
            if !group.is_object() {
                self.invalid(source, &field, "INVALID_SOURCE_REFERENCE");
                continue;
            }
            self.source_references(
                source,
                run,
                &format!("{field}/references"),
                group.get("references"),
            );
        }
    }

    fn validation_sources(&mut self, source: &Entity, run: &Value) {
        if let Some(flow) = run.get("flow").filter(|flow| !flow.is_null()) {
            if let Some(definition) = flow.get("definition").filter(|value| value.is_object()) {
                self.source_references(
                    source,
                    run,
                    "/flow/definition/references",
                    definition.get("references"),
                );
                self.source_groups(
                    source,
                    run,
                    "/flow/definition/steps",
                    definition.get("steps"),
                );
            } else {
                self.invalid(source, "/flow/definition", "INVALID_SOURCE_REFERENCE");
            }
        }
        self.source_groups(source, run, "/evidence", run.get("evidence"));
    }

    fn attempts(&mut self, source: &Entity, value: &Value) {
        let Some(work) = value.get("work") else {
            return;
        };
        if !work.is_object() {
            return self.invalid(source, "/work", "INVALID_ASSET_WORK");
        }
        let Some(attempts) = work.get("attempts").and_then(Value::as_array) else {
            return self.invalid(source, "/work/attempts", "INVALID_ATTEMPTS");
        };
        for (index, attempt) in attempts.iter().enumerate() {
            let prefix = format!("/work/attempts/{index}");
            if !attempt.is_object() {
                self.invalid(source, &prefix, "INVALID_ATTEMPT");
                continue;
            }
            let Some(inputs) = attempt.get("inputFiles").filter(|value| !value.is_null()) else {
                continue;
            };
            let Some(inputs) = inputs.as_array() else {
                self.invalid(
                    source,
                    &format!("{prefix}/inputFiles"),
                    "INVALID_INPUT_FILES",
                );
                continue;
            };
            for (index, input) in inputs.iter().enumerate() {
                let field = format!("{prefix}/inputFiles/{index}");
                if !input.is_object() {
                    self.invalid(source, &field, "INVALID_INPUT_FILE");
                    continue;
                }
                // Historical inputs refer to frozen blobs, not current workspace bytes.
                self.hash(source, &format!("{field}/sha256"), &input["sha256"]);
            }
        }
    }
}

pub(crate) fn inspect(entities: &[Entity], ownership: &Ownership, files: &[Entry]) -> Content {
    let mut inspector = Inspector {
        files: files
            .iter()
            .map(|entry| (entry.path.as_str(), entry.sha256.as_deref()))
            .collect(),
        ownership,
        report: Content::default(),
    };
    for source in entities {
        let Some(value) = &source.value else { continue };
        match source.kind.as_str() {
            "task" => inspector.task(source, value, ""),
            "asset-task" => inspector.attempts(source, value),
            "operation" => {
                inspector.changes(source, "/changes", value.get("changes"));
                if let Some(after) = value.get("taskAfter") {
                    inspector.task(source, after, "/taskAfter");
                }
            }
            "feature" | "validationRun" | "validationRelease" => {
                inspector.snapshot(source, "/snapshot", value.get("snapshot"));
                if source.kind == "validationRun" {
                    inspector.validation_sources(source, value);
                }
            }
            kind if kind.starts_with("asset-delivery/") => {
                inspector.snapshot(source, "/files", value.get("files"));
            }
            _ => {}
        }
    }
    inspector.report
}
