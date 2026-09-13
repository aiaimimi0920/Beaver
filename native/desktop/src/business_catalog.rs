use serde_json::{json, Value};

/// Public names match the desktop contract. Nested domain validation remains in core.
pub fn tools() -> Vec<Value> {
    let domain: Value =
        serde_json::from_str(include_str!("../../../dist-native/business-schemas.json"))
            .expect("embedded business schemas");
    let definitions = [
        ("state", "Read projects, tasks, redacted settings and feature packages.", "", "", true),
        ("project.create", "Create a Godot project. Optional npr={godot: absolute editor path or directory} installs/enables the NPR package.", "parent:s,name:s,template:s", "design:o,blueprint:o,npr:o", false),
        ("project.npr.install", "Install/repair NPR on a project with no active tasks; never overwrites customized addon files.", "id:s,godot:s", "", false),
        ("workflow.list", "Discover standard workflows and their project enablement.", "id:s", "", true),
        ("workflow.run", "Use a standard workflow in the project: inspect contract, validate actual assets or render previews. Preview supports fixed camera and grayscale for matched close-ups.", "id:s,workflow:s,action:s", "definition:s,camera:o,grayscale:b", false),
        ("logs.query", "Read persisted business/Codex/tool call metadata by cursor. Payloads are summarized without raw credentials or assets.", "", "after:i,limit:i,taskId:s,projectId:s,method:s", true),
        ("project.import", "Register an existing Godot project by absolute directory.", "path:s", "", false),
        ("project.blueprint.save", "Save planning with optimistic revision checking.", "id:s,blueprint:o,expectedRevision:i", "", false),
        ("project.overview.save", "Change basic project planning with explicit risk acceptance.", "id:s,overview:o,expectedRevision:i,allowRiskyChanges:b", "", false),
        ("task.create", "Queue a creation or review task. assetTask enables the managed Blender production workflow; do not combine with review or decomposition. Returns a task; poll state and task.events.", "projectId:s,prompt:s", "title:s,direction:s,stopConditions:s,references:a,maxMinutes:i,capability:s,askRatio:r,decompose:b,autoAccept:b,assetTask:b", false),
        ("task.autonomy", "Set task ask ratio (0,10,30,70,100) or null to follow global settings. Pending answers are not silently submitted.", "id:s,askRatio:r", "", false),
        ("task.followup", "Create a followup task using the original task context.", "id:s,text:s", "", false),
        ("task.delegate", "Create a delegated task using the original task context.", "id:s,text:s", "", false),
        ("task.approval", "Set automatic or manual completion approval for a task and its unfinished planned children. Completed children still require explicit task.accept.", "id:s,autoAccept:b", "", false),
        ("task.dialogueRollback", "Create a task to revise or undo prior work through dialogue.", "id:s,text:s", "", false),
        ("task.direction", "Set task direction.", "id:s,direction:s", "", false),
        ("task.continue", "Continue a stopped task or steer running work. freshContext starts a new AI session for failed/interrupted tasks while retaining workspace, history and rollback baseline. Poll state/events.", "id:s,text:s", "freshContext:b", false),
        ("task.interrupt", "Interrupt task execution, preserving history and working files.", "id:s", "", false),
        ("task.answer", "Answer pending questions. Optional automatic lists IDs explicitly filled using the current policy and exact recommended labels.", "id:s,questionId:s,answers:o", "automatic:a", false),
        ("task.accept", "Accept a completed task and its feature baseline.", "id:s", "", false),
        ("task.retryMerge", "Retry a file-conflicted task's recorded output without running AI. Matching project content is preserved; genuine conflicts still block the entire merge.", "id:s", "", false),
        ("task.rollback", "Revert task changes with conflict checks; keep lists relative paths to preserve.", "id:s,keep:a", "", false),
        ("task.events", "Read persisted execution events for a task.", "id:s", "", true),
        ("task.resources", "List task references and changes.", "id:s", "", true),
        ("task.resourceText", "Read a task resource by its returned path.", "id:s,path:s", "", true),
        ("task.resourceBytes", "Read original resource bytes as base64, up to 500 KB; preserves unknown encodings.", "id:s,path:s", "", true),
        ("assets", "List assets in a registered project or task workspace.", "id:s", "", true),
        ("asset.text", "Read a text asset using a project-relative path.", "id:s,path:s", "", true),
        ("asset.import", "Import local files without a file picker. Absolute paths required.", "id:s,paths:a", "", false),
        ("document.read", "Read a Markdown document and its revision for subsequent saving.", "id:s,path:s", "", true),
        ("document.save", "Save Markdown with revision conflict protection; null revision creates a new document.", "id:s,path:s,text:s,revision:n", "", false),
        ("feature.add", "Queue integration of a real bundled feature package.", "projectId:s,featureId:s", "", false),
        ("settings.save", "Save settings and supplied credential slots. Credentials are sensitive input.", "settings:o,keys:o", "", false),
        ("settings.importLocalCodex", "Explicitly import local Codex provider configuration.", "settings:o,keys:o", "", false),
        ("settings.clearKey", "Delete a credential slot.", "slot:s", "", false),
        ("tools.detect", "Detect configured executables and versions.", "", "tools:o", true),
        ("tools.setupStatus", "Read tool preparation status.", "", "", true),
        ("tools.setup", "Prepare configured tools; may install software and take several minutes.", "", "tools:o", false),
        ("tools.install", "Install a named tool: codex, godot, blender or node.", "name:s", "", false),
        ("tools.cancelSetup", "Cancel tool preparation.", "", "", false),
        ("game.play", "Launch a registered project in Godot on the host desktop.", "id:s", "", false),
        ("game.presets", "List export presets for a project.", "id:s", "", true),
        ("game.export", "Export a project; formal exports require an exact green release candidate. Internal exports remain available for development.", "id:s,destination:s,preset:s", "purpose:s,releaseCheckId:s,snapshotId:s,scopeId:s", false),
        ("game.verifyExport", "Verify an exported bundle and record the matching project's delivery status.", "path:s", "", false),
        ("game.prepareTemplates", "Download official Godot export templates.", "", "", false),
        ("game.importTemplates", "Import a trusted local Godot template archive without a file picker.", "path:s", "", false),
        ("game.cancelTemplates", "Cancel export template preparation.", "", "", false),
        ("screenshots", "List capturable windows on the host desktop.", "", "", true),
        ("screenshot.capture", "Capture the specified host window into project assets.", "id:s,source:s", "", false),
        ("project.reveal", "Reveal project files in the host file manager.", "id:s", "", false),
        ("task.reveal", "Reveal task workspace in the host file manager.", "id:s", "", false),
        ("asset.reveal", "Reveal a project-relative asset in the host file manager.", "id:s,path:s", "", false),
    ];
    definitions.into_iter().chain(crate::asset_task_catalog::definitions()).map(|(name, description, required, optional, read)| {
        let mut properties = serde_json::Map::new();
        for field in required.split(',').chain(optional.split(',')).filter(|v| !v.is_empty()) {
            let (key, kind) = field.split_once(':').expect("static schema field");
            let mut schema = match kind {
                "s" => json!({"type":"string"}),
                "i" => json!({"type":"integer","minimum":0}),
                "b" => json!({"type":"boolean"}),
                "a" => json!({"type":"array"}),
                "n" => json!({"type":["string","null"]}),
                "r" => json!({"type":["integer","null"],"enum":[0,10,30,70,100,null]}),
                _ => json!({"type":"object"}),
            };
            if let Some(nested) = domain.get(key) {
                schema = nested.clone();
            }
            match key {
                "npr" => schema=json!({"type":"object","properties":{"godot":{"type":"string"}},"required":["godot"],"additionalProperties":false}),
                "camera" => schema = beaver_core::workflows::camera_schema(),
                "template" => schema["enum"] = json!(["blank", "nightbar"]),
                "capability" => schema["enum"] = json!(["code", "review"]),
                "timing" => schema["enum"] = json!(["now", "afterRound"]),
                "purpose" => schema["enum"] = json!(["internal", "formal"]),
                "name" if name == "tools.install" => schema["enum"] = json!(["codex", "godot", "blender", "node"]),
                "keep" | "paths" | "automatic" => schema["items"] = json!({"type":"string"}),
                "answers" | "keys" => schema["additionalProperties"] = json!({"type":"string"}),
                "settings" => schema["description"] = json!("Use settings returned by state, modify desired fields and submit the full object."),
                "tools" => {
                    schema["properties"] = json!({"codex":{"type":"string"},"godot":{"type":"string"},"blender":{"type":"string"},"node":{"type":"string"}});
                    schema["required"] = json!(["codex","godot","blender","node"]);
                    schema["additionalProperties"] = json!(false);
                }
                "references" => schema["items"] = json!({"type":"object","properties":{"path":{"type":"string"},"note":{"type":"string"},"region":{"type":"object","properties":{"x":{"type":"number","minimum":0,"maximum":1},"y":{"type":"number","minimum":0,"maximum":1},"w":{"type":"number","minimum":0,"maximum":1},"h":{"type":"number","minimum":0,"maximum":1}},"required":["x","y","w","h"]}},"required":["path","note"]}),
                _ => {}
            }
            properties.insert(key.to_owned(), schema);
        }
        let required: Vec<_> = required.split(',').filter(|v| !v.is_empty()).map(|v| v.split(':').next().unwrap()).collect();
        json!({"name":name,"description":description,"inputSchema":{"type":"object","properties":properties,"required":required,"additionalProperties":false},"annotations":{"readOnlyHint":read,"destructiveHint":!read,"openWorldHint":true}})
    }).chain(crate::validation_catalog::tools()).collect()
}

pub fn validate(method: &str, input: &Value) -> Result<(), (&'static str, String)> {
    let tool = tools()
        .into_iter()
        .find(|tool| tool["name"] == method)
        .ok_or((
            "METHOD_NOT_FOUND",
            format!("Unknown business method: {method}"),
        ))?;
    let fail = |message: String| ("INVALID_INPUT", message);
    let fields = input
        .as_object()
        .ok_or_else(|| fail("Input must be an object".into()))?;
    let schema = &tool["inputSchema"];
    for key in schema["required"].as_array().unwrap() {
        let key = key.as_str().unwrap();
        if !fields.contains_key(key) {
            return Err(fail(format!("Missing field: {key}")));
        }
    }
    for (key, value) in fields {
        let property = &schema["properties"][key];
        let valid = match property["type"].as_str() {
            Some("string") => value.is_string(),
            Some("integer") => value.as_u64().is_some(),
            Some("boolean") => value.is_boolean(),
            Some("array") => value.is_array(),
            Some("object") => value.is_object(),
            _ => {
                property["type"].is_array()
                    && (value.is_null()
                        || value.is_string()
                        || (key == "askRatio" && value.as_u64().is_some()))
            }
        };
        if !valid {
            return Err(fail(format!("Invalid or unknown field: {key}")));
        }
        if property["enum"]
            .as_array()
            .is_some_and(|values| !values.contains(value))
        {
            return Err(fail(format!("Invalid choice: {key}")));
        }
    }
    if method == "asset.import"
        && !input["paths"].as_array().is_some_and(|paths| {
            !paths.is_empty()
                && paths.len() <= 100
                && paths.iter().all(|p| {
                    p.as_str()
                        .is_some_and(|p| std::path::Path::new(p).is_absolute())
                })
        })
    {
        return Err(fail("paths must contain 1..100 absolute file paths".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn external_imports_cannot_fall_back_to_dialogs() {
        assert!(validate("asset.import", &json!({"id":"x"})).is_err());
        assert!(validate("asset.import", &json!({"id":"x","paths":[]})).is_err());
        assert!(validate("game.importTemplates", &json!({})).is_err());
        assert!(validate("chooseDirectory", &json!({})).is_err());
        assert!(validate("state", &json!({"typo":true})).is_err());
        assert!(validate(
            "document.save",
            &json!({"id":"x","path":"a.md","text":"a","revision":null})
        )
        .is_ok());
    }
    #[test]
    fn merge_retry_has_no_force_override() {
        assert!(validate("task.retryMerge", &json!({"id":"t"})).is_ok());
        assert!(validate("task.retryMerge", &json!({})).is_err());
        assert!(validate("task.retryMerge", &json!({"id":"t","force":true})).is_err());
    }
    #[test]
    fn fresh_context_is_an_explicit_boolean_option() {
        assert!(validate("task.continue", &json!({"id":"t","text":""})).is_ok());
        assert!(validate(
            "task.continue",
            &json!({"id":"t","text":"","freshContext":true})
        )
        .is_ok());
        assert!(validate(
            "task.continue",
            &json!({"id":"t","text":"","freshContext":"true"})
        )
        .is_err());
    }
}
