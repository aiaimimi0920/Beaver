//! Checks declared database references without guessing IDs in narrative or unknown fields.
use crate::project_migration_ownership::{Entity, Owner, Ownership};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub source_kind: String,
    pub source_id: String,
    pub field: String,
    pub reason: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_id: Option<String>,
}

impl Issue {
    pub(crate) fn new(source: &Entity, field: &str, reason: &'static str) -> Self {
        Self {
            source_kind: source.kind.clone(),
            source_id: source.id.clone(),
            field: field.into(),
            reason,
            target_kind: None,
            target_id: None,
        }
    }
}

#[derive(Debug, Default, Serialize)]
pub struct Checks {
    pub checked: usize,
    pub issues: Vec<Issue>,
}

pub(crate) struct References<'a> {
    entities: BTreeMap<(&'a str, &'a str), (&'a Entity, Owner)>,
    pub checks: Checks,
}

impl<'a> References<'a> {
    pub fn new(entities: &'a [Entity], ownership: &Ownership) -> Self {
        Self {
            entities: entities
                .iter()
                .map(|entity| {
                    (
                        (entity.kind.as_str(), entity.id.as_str()),
                        (entity, ownership.entity(entity)),
                    )
                })
                .collect(),
            checks: Checks::default(),
        }
    }

    pub fn invalid(&mut self, source: &Entity, field: &str) {
        self.checks.checked += 1;
        self.checks
            .issues
            .push(Issue::new(source, field, "INVALID_REFERENCE_FIELD"));
    }

    pub fn one(&mut self, source: &Entity, field: &str, kind: &str, value: Option<&Value>) {
        let Some(value) = value.filter(|value| !value.is_null()) else {
            return;
        };
        let Some(id) = value.as_str().filter(|id| !id.is_empty()) else {
            return self.invalid(source, field);
        };
        self.target(source, field, kind, id, None);
    }

    pub fn many(&mut self, source: &Entity, field: &str, kind: &str, value: Option<&Value>) {
        let Some(value) = value else { return };
        let Some(ids) = value.as_array() else {
            return self.invalid(source, field);
        };
        for (index, id) in ids.iter().enumerate() {
            let field = format!("{field}/{index}");
            if id.is_null() {
                self.invalid(source, &field);
            } else {
                self.one(source, &field, kind, Some(id));
            }
        }
    }

    pub fn frozen_flow(&mut self, source: &Entity, field: &str, value: &Value) {
        let Some(id) = value["id"].as_str().filter(|id| !id.is_empty()) else {
            return self.invalid(source, field);
        };
        let Some(revision) = value["revision"].as_u64() else {
            return self.invalid(source, field);
        };
        self.target(
            source,
            field,
            "validationFlowRevision",
            &format!("{id}:{revision}"),
            Some(value),
        );
    }

    fn target(
        &mut self,
        source: &Entity,
        field: &str,
        kind: &str,
        id: &str,
        frozen: Option<&Value>,
    ) {
        self.checks.checked += 1;
        let reason = (|| {
            let (_, source_owner) = self
                .entities
                .get(&(source.kind.as_str(), source.id.as_str()))?;
            let Ok(Some(project)) = source_owner else {
                return Some("SOURCE_OWNER_UNRESOLVED");
            };
            let Some((entity, target_owner)) = self.entities.get(&(kind, id)) else {
                return Some("REFERENCE_NOT_FOUND");
            };
            let Ok(Some(target_project)) = target_owner else {
                return Some("TARGET_OWNER_UNRESOLVED");
            };
            if project != target_project {
                return Some("CROSS_PROJECT_REFERENCE");
            }
            if let Some(frozen) = frozen {
                let stored = entity.value.as_ref()?;
                if ["id", "projectId", "revision", "definition"]
                    .iter()
                    .any(|key| stored[key] != frozen[key])
                {
                    return Some("FROZEN_FLOW_MISMATCH");
                }
            }
            None
        })();
        if let Some(reason) = reason {
            let mut issue = Issue::new(source, field, reason);
            issue.target_kind = Some(kind.into());
            issue.target_id = Some(id.into());
            self.checks.issues.push(issue);
        }
    }
}

pub(crate) fn inspect(entities: &[Entity], ownership: &Ownership) -> Checks {
    let mut references = References::new(entities, ownership);
    for source in entities {
        if let Some(value) = &source.value {
            crate::project_migration_reference_fields::inspect(&mut references, source, value);
        }
    }
    references.checks
}
