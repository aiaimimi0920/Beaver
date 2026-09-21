//! Conservative ownership rules for the legacy host database, not a data converter.
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) type Owner = Result<Option<String>, &'static str>;

pub(crate) struct Entity {
    pub kind: String,
    pub id: String,
    pub value: Option<Value>,
}

pub(crate) struct Ownership {
    projects: BTreeSet<String>,
    tasks: BTreeMap<String, Result<String, &'static str>>,
}

fn field<'a>(value: &'a Value, path: &str) -> Result<Option<&'a str>, &'static str> {
    match value.pointer(path) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(id)) if !id.is_empty() => Ok(Some(id)),
        _ => Err("INVALID_OWNER_FIELD"),
    }
}

fn required<'a>(value: &'a Value, path: &str) -> Result<&'a str, &'static str> {
    field(value, path)?.ok_or("MISSING_OWNER_FIELD")
}

impl Ownership {
    pub fn new(projects: BTreeSet<String>, entities: &[Entity]) -> Self {
        let mut ownership = Self {
            projects,
            tasks: BTreeMap::new(),
        };
        for entity in entities.iter().filter(|entity| entity.kind == "task") {
            let owner = (|| {
                let value = entity.value.as_ref().ok_or("INVALID_JSON")?;
                if required(value, "/id")? != entity.id {
                    return Err("ENTITY_ID_MISMATCH");
                }
                ownership.project(required(value, "/projectId")?)
            })();
            ownership.tasks.insert(entity.id.clone(), owner);
        }
        ownership
    }

    fn project(&self, id: &str) -> Result<String, &'static str> {
        self.projects
            .contains(id)
            .then(|| id.to_owned())
            .ok_or("PROJECT_NOT_REGISTERED")
    }

    pub fn task(&self, id: &str) -> Owner {
        self.tasks
            .get(id)
            .ok_or("TASK_NOT_FOUND")?
            .clone()
            .map(Some)
    }

    fn claims(&self, value: &Value, task: Option<&str>, project: Option<&str>) -> Owner {
        let mut task = task;
        let mut owner = project.map(|id| self.project(id)).transpose()?;
        for path in ["/taskId", "/context/taskId"] {
            if let Some(id) = field(value, path)? {
                if task.is_some_and(|previous| previous != id) {
                    return Err("TASK_OWNER_MISMATCH");
                }
                task = Some(id);
            }
        }
        if let Some(task) = task {
            let task_owner = self.task(task)?;
            if owner.is_some() && owner != task_owner {
                return Err("PROJECT_OWNER_MISMATCH");
            }
            owner = task_owner;
        }
        for path in ["/projectId", "/context/projectId", "/taskAfter/projectId"] {
            if let Some(id) = field(value, path)? {
                let project = self.project(id)?;
                if owner.as_ref().is_some_and(|previous| previous != &project) {
                    return Err("PROJECT_OWNER_MISMATCH");
                }
                owner = Some(project);
            }
        }
        owner.map(Some).ok_or("MISSING_OWNER_FIELD")
    }

    pub fn entity(&self, entity: &Entity) -> Owner {
        let value = entity.value.as_ref().ok_or("INVALID_JSON")?;
        let kind = entity.kind.as_str();
        let id = entity.id.as_str();
        match kind {
            "secret" | "secret_backup" => return Ok(None),
            "settings" | "toolSetup" if id == "main" => return Ok(None),
            "task-callback-revision" => return self.task(id),
            _ => {}
        }
        if !value.is_object() {
            return Err("INVALID_ENTITY_SHAPE");
        }
        match kind {
            "project" | "task" => {
                if required(value, "/id")? != id {
                    return Err("ENTITY_ID_MISMATCH");
                }
                if kind == "project" {
                    self.claims(value, None, Some(id))
                } else {
                    self.claims(value, Some(id), None)
                }
            }
            "asset-task" | "framework-configuration" | "validationCoverage" => {
                self.claims(value, Some(id), None)
            }
            "validationSettings" | "validationManifest" => self.claims(value, None, Some(id)),
            "framework-operation" | "asset-reference" | "operation" => {
                let task = required(value, "/taskId")?;
                if kind == "operation" {
                    required(value, "/projectId")?;
                    if field(value, "/taskAfter/id")?.is_some_and(|after| after != task) {
                        return Err("TASK_OWNER_MISMATCH");
                    }
                }
                self.claims(value, Some(task), None)
            }
            "feature" => {
                let project = self
                    .projects
                    .iter()
                    .find(|project| {
                        id.strip_prefix(project.as_str())
                            .is_some_and(|suffix| suffix.starts_with(':') && suffix.len() > 1)
                    })
                    .ok_or("PROJECT_NOT_REGISTERED")?;
                self.claims(value, Some(required(value, "/taskId")?), Some(project))
            }
            "validationFlowRevision" => {
                let flow = required(value, "/id")?;
                let revision = value
                    .get("revision")
                    .and_then(Value::as_u64)
                    .ok_or("INVALID_REVISION")?;
                if id != format!("{flow}:{revision}") {
                    return Err("ENTITY_ID_MISMATCH");
                }
                self.claims(value, None, Some(required(value, "/projectId")?))
            }
            "validationRun"
            | "validationFlow"
            | "validationBaseline"
            | "validationFeedback"
            | "validationRelease"
            | "validationRepairDecision" => {
                self.claims(value, None, Some(required(value, "/projectId")?))
            }
            // Historical receipt keys are hashes; their ownership cannot be reversed.
            "validationRequest" => {
                let project =
                    field(value, "/projectId")?.ok_or("VALIDATION_REQUEST_OWNER_UNKNOWN")?;
                self.claims(value, None, Some(project))
            }
            _ => {
                let (prefix, task) = kind.split_once('/').ok_or("UNKNOWN_ENTITY_KIND")?;
                if ![
                    "asset-delivery",
                    "asset-delivery-decisions",
                    "task-callback",
                    "framework-trace",
                    "framework-observation",
                    "framework-recovery",
                    "framework-check",
                    "framework-judgment",
                    "framework-judgment-history",
                ]
                .contains(&prefix)
                {
                    return Err("UNKNOWN_ENTITY_KIND");
                }
                self.claims(value, Some(task), None)
            }
        }
    }

    pub fn call(&self, id: &str, task: Option<&str>, project: Option<&str>, raw: &str) -> Owner {
        let value: Value = serde_json::from_str(raw).map_err(|_| "INVALID_JSON")?;
        if required(&value, "/id")? != id {
            return Err("CALL_ID_MISMATCH");
        }
        if field(&value, "/taskId")? != task || field(&value, "/projectId")? != project {
            return Err("CALL_COLUMN_MISMATCH");
        }
        if task.is_none() && project.is_none() {
            Ok(None)
        } else {
            self.claims(&value, task, project)
        }
    }
}
