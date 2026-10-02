use serde_json::{json, Value};

/// Public names match the desktop contract. Nested domain validation remains in core.
pub fn tools() -> Vec<Value> {
    let domain: Value =
        serde_json::from_str(include_str!("../../../dist-native/business-schemas.json"))
            .expect("embedded business schemas");
    let definitions = [
        ("state", "Read projects, tasks, redacted settings and feature packages.", "", "", true),
        ("migration.inspect", "Inspect a complete offline migration archive without modifying it. Does not grant activation readiness.", "backup:s", "", true),
        ("migration.prepareProjects", "Restore an offline archive to a NEW directory and partition project storage. Failures preserve the partial copy; retry with a new destination. Remains pending explicit activation.", "backup:s,destination:s", "", false),
        ("migration.activate", "Explicitly validate and activate a partitioned recovery copy, probing configured tools. Does not change the current/default data directory or start tasks. Optional toolPaths is an absolute JSON file path.", "backup:s,prepared:s", "toolPaths:s", false),
        ("migration.activateAssembly", "Explicitly publish a prepared project derivation assembly. The durable assembly receipt is written before its pending marker is removed; host registration is unchanged.", "preparation:s,destination:s", "", false),
        ("migration.registerAssembly", "Register an explicitly activated derivation assembly, then recover its runtime. Returns registrationCommitted and runtimeReady separately. A recovery failure preserves registration and is retryable with the same paths; never overwrites another project or activates a pending assembly.", "preparation:s,destination:s", "", false),
        ("objectFramework.status", "Read project storage readiness and object framework blockers without opening or migrating project storage.", "projectId:s", "", true),
        ("object.list", "List project-owned registered objects, optionally filtered by object name, component kind or file path.", "projectId:s", "query:s", true),
        ("object.get", "Read one project-owned object registration and its immutable version manifest.", "projectId:s,objectId:s", "", true),
        ("object.attemptScenePreview.get", "Read the durable frozen attempt checkpoint capture.", "projectId:s,runId:s,attemptId:s,checkpoint:s,path:s,sha256:s", "", true),
        ("object.attemptScenePreview.run", "Render a frozen attempt checkpoint. Optional resolution: 540p, 720p, 1080p. Reuse requestId after response loss. Not acceptance evidence.", "projectId:s,requestId:s,target:o", "resolution:s", false),
        ("object.scenePreview.get", "Read the latest durable frozen Godot scene or Blender Workbench preview capture for an exact version member.", "projectId:s,objectId:s,versionId:s,path:s,sha256:s", "", true),
        ("object.scenePreview.run", "Render a frozen Godot scene or Blender Workbench preview and pinned dependencies using the managed visual queue. Optional resolution: 540p, 720p, 1080p. Reuse requestId after response loss. Not acceptance evidence.", "projectId:s,requestId:s,target:o", "resolution:s", false),
        ("object.versionFile", "Read a frozen version member from verified project-local content. Supports text, image and audio up to 8 MiB; never reads live project files. Larger files return metadata without content verification.", "projectId:s,objectId:s,versionId:s,path:s,sha256:s", "", true),
        ("object.register", "Register an object in an open project. Empty objects are allowed. Reuse requestId to retry; no tasks or accepted versions are created.", "projectId:s,requestId:s,name:s", "components:a,files:a,references:a,category:s,tags:a,thumbnailPath:n,parentObjectId:n", false),
        ("object.updateRegistration", "Replace object metadata and writable ownership with expectedRevision checking. References must pin accepted versions in this project. Preserves version history; retries reuse requestId.", "projectId:s,requestId:s,objectId:s,expectedRevision:i,name:s,components:a,files:a,references:a", "category:s,tags:a,thumbnailPath:n,parentObjectId:n", false),
        ("object.captureVersion", "Freeze registered files and metadata into an immutable, unaccepted version. Does not accept, publish or start a task. Reuse requestId after a lost response.", "projectId:s,requestId:s,objectId:s,expectedRevision:i", "", false),
        ("object.acceptVersion", "Accept one captured project-owned version without copying content, publishing the object or starting a task. Reuse requestId for retries; expectedRevision protects concurrent changes.", "projectId:s,requestId:s,objectId:s,versionId:s,expectedRevision:i", "", false),
        ("object.inspectExternal", "Discover Beaver project identity from its directory and read accepted-version digests or blockers. Optional projectId must match the source identity. Reuse an open runtime snapshot or inspect a closed project without registration, migration or activation. Blob verification occurs during preparation.", "path:s", "projectId:s,query:s", true),
        ("object.inspectFiles", "Read ordinary files and folders as a read-only import snapshot. Does not copy, register or modify the selected source.", "paths:a", "", true),
        ("object.prepareImport", "Verify a frozen accepted version closure against sourceDigest from inspection and save a requestId-scoped retry receipt with new identities. Does not modify the target object catalog or commit an import.", "requestId:s,targetProjectId:s,source:o,objectId:s,baseline:o,sourceDigest:s", "", false),
        ("object.getImportPreparation", "Read a previously prepared object import receipt.", "projectId:s,preparationId:s", "", true),
        ("object.commitImport", "Import a prepared Beaver object and its frozen dependency closure. Preserve asset paths; never overwrite target files. Retry the same preparationId; versions remain pending validation.", "projectId:s,preparationId:s", "", false),
        ("object.importOperation", "Read project-source import progress without source access or automatic recovery.", "projectId:s,preparationId:s", "", true),
        ("object.abortImport", "Abort an uncommitted project-source import; only remove matching journaled writes and preserve external edits.", "projectId:s,preparationId:s", "", false),
        ("object.importPreparations", "List target-owned durable import preparations without accessing any source. Cursor pagination; preparation does not mean import committed.", "projectId:s", "after:s", true),
        ("object.prepareFileImport", "Verify an unchanged ordinary-file snapshot and save a request-scoped preparation receipt. Does not copy, register or commit files.", "requestId:s,targetProjectId:s,snapshot:o,groups:a", "", false),
        ("object.getFileImportPreparation", "Read a previously prepared ordinary-file import receipt.", "targetProjectId:s,preparationId:s", "", true),
        ("object.commitFileImport", "Explicitly commit an ordinary-file preparation. Reuse preparationId after failure or response loss; frozen content and durable write intent support recovery. Imported versions remain pending validation.", "projectId:s,preparationId:s", "", false),
        ("object.fileImportOperation", "Read ordinary-file import progress without source access or automatic recovery.", "projectId:s,preparationId:s", "", true),
        ("object.abortFileImport", "Abort an uncommitted ordinary-file import, removing only matching journaled writes. External changes block cleanup and remain intact.", "projectId:s,preparationId:s", "", false),
        ("project.storage.status", "Inspect the registered project location and manifest without opening or modifying project storage; detected does not imply database readiness.", "id:s", "", true),
        ("project.create", "Create a Godot project. Optional npr={godot: absolute editor path or directory} installs/enables the NPR package.", "parent:s,name:s,template:s", "design:o,blueprint:o,npr:o", false),
        ("project.npr.install", "Install/repair NPR on a project with no active tasks; never overwrites customized addon files.", "id:s,godot:s", "", false),
        ("workflow.list", "Discover standard workflows and their project enablement.", "id:s", "", true),
        ("workflow.run", "Use a standard workflow in the project: inspect contract, validate actual assets or render previews. Preview supports fixed camera and grayscale for matched close-ups.", "id:s,workflow:s,action:s", "definition:s,camera:o,grayscale:b", false),
        ("logs.query", "Read persisted business/Codex/tool call metadata by cursor. Payloads are summarized without raw credentials or assets.", "", "after:i,limit:i,taskId:s,projectId:s,method:s", true),
        ("project.import", "Register an existing Godot project by absolute directory.", "path:s", "", false),
        ("project.unregister", "Remove host registration only, preserving all project files and history. Requires the current registered path. Admitted work drains on its original storage; returns id, path and draining.", "id:s,expectedPath:s", "", false),
        ("project.reassociate", "Explicitly bind a registered project ID to a different local .beaver directory. Requires the current registered path and no active runtime references; preserves all project files.", "id:s,expectedPath:s,path:s", "", false),
        ("project.blueprint.save", "Save planning with optimistic revision checking.", "id:s,blueprint:o,expectedRevision:i", "", false),
        ("project.overview.save", "Change basic project planning with explicit risk acceptance.", "id:s,overview:o,expectedRevision:i,allowRiskyChanges:b", "", false),
        ("task.create", "Queue a legacy creation or review task. assetTask enables the managed Blender workflow. objectFramework identities are reserved and rejected until object scheduling is enabled. Returns a task; poll state and task.events.", "projectId:s,prompt:s", "title:s,direction:s,stopConditions:s,references:a,maxMinutes:i,capability:s,askRatio:r,decompose:b,autoAccept:b,assetTask:b,objectFramework:o", false),
        ("task.autonomy", "Set task ask ratio (0,10,30,70,100) or null to follow global settings. Pending answers are not silently submitted.", "id:s,askRatio:r", "", false),
        ("task.followup", "Create a followup task using the original task context.", "id:s,text:s", "", false),
        ("task.delegate", "Create a delegated task using the original task context.", "id:s,text:s", "", false),
        ("task.approval", "Set automatic or manual completion approval for a task and its unfinished planned children. Completed children still require explicit task.accept.", "id:s,autoAccept:b", "", false),
        ("task.dialogueRollback", "Create a task to revise or undo prior work through dialogue.", "id:s,text:s", "", false),
        ("task.direction", "Set task direction.", "id:s,direction:s", "", false),
        ("task.continue", "Continue a stopped task or steer running work. freshContext starts a new AI session for failed/interrupted tasks while retaining workspace, history and rollback baseline. Poll state/events.", "id:s,text:s", "freshContext:b", false),
        ("task.interrupt", "Interrupt task execution, preserving history and working files.", "id:s", "", false),
        ("task.answer", "Answer pending questions. Optional automatic lists IDs explicitly filled using the current policy and exact recommended labels.", "id:s,questionId:s,answers:o", "automatic:a", false),
        ("objectTaskPlanning.start", "Start a read-only Codex planning round for the current object-task draft. It may ask questions or submit a proposal; it never executes tasks.", "projectId:s,requestId:s,draftId:s,expectedDraftRevision:i,expectedPlanRevision:i,goal:s,acceptance:s,askRatio:i", "", false),
        ("objectTaskPlanning.get", "Read the current Codex planning session for a project draft.", "projectId:s,draftId:s", "", true),
        ("objectTask.suggestTitle", "Request a read-only Codex title suggestion from supplied draft text. Returns a candidate only; explicit user acceptance and draft saving remain separate.", "projectId:s,prompt:s,acceptance:s", "", true),
        ("objectTaskPlanning.answer", "Answer the current Codex planning questions and resume the round.", "projectId:s,sessionId:s,requestId:s,expectedRevision:i,answers:o", "", false),
        ("objectTaskPlanning.cancel", "Cancel a running or awaiting Codex planning round.", "projectId:s,sessionId:s,requestId:s,expectedRevision:i", "", false),
        ("objectTaskPlanning.adopt", "Explicitly adopt a reviewed Codex proposal into the object-task draft; execution remains a separate commit.", "projectId:s,sessionId:s,requestId:s,expectedRevision:i", "", false),
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
        ("game.prepareTemplates", "Prepare matching editor-sibling templates, or official stable templates for an official editor.", "", "", false),
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
                "components" if name.starts_with("object.") => schema = json!({"type":"array","maxItems":256,"items":{"type":"object","properties":{"id":{"type":"string"},"kind":{"type":"string"},"name":{"type":"string"}},"required":["id","kind","name"],"additionalProperties":false}}),
                "files" if name.starts_with("object.") => schema = json!({"type":"array","maxItems":1024,"items":{"type":"object","properties":{"path":{"type":"string"},"role":{"type":"string"}},"required":["path","role"],"additionalProperties":false}}),
                "references" if name.starts_with("object.") => schema = json!({"type":"array","maxItems":256,"items":{"type":"object","properties":{"projectId":{"type":"string"},"objectId":{"type":"string"},"versionId":{"type":"string"}},"required":["projectId","objectId","versionId"],"additionalProperties":false}}),
                "tags" if name.starts_with("object.") => schema = json!({"type":"array","maxItems":64,"items":{"type":"string"}}),
                "category" if name.starts_with("object.") => schema = json!({"type":"string","minLength":1,"maxLength":128}),
                "references" => schema["items"] = json!({"type":"object","properties":{"path":{"type":"string"},"note":{"type":"string"},"region":{"type":"object","properties":{"x":{"type":"number","minimum":0,"maximum":1},"y":{"type":"number","minimum":0,"maximum":1},"w":{"type":"number","minimum":0,"maximum":1},"h":{"type":"number","minimum":0,"maximum":1}},"required":["x","y","w","h"]}},"required":["path","note"]}),
                _ => {}
            }
            properties.insert(key.to_owned(), schema);
        }
        let required: Vec<_> = required.split(',').filter(|v| !v.is_empty()).map(|v| v.split(':').next().unwrap()).collect();
        json!({"name":name,"description":description,"inputSchema":{"type":"object","properties":properties,"required":required,"additionalProperties":false},"annotations":{"readOnlyHint":read,"destructiveHint":!read,"openWorldHint":true}})
    }).chain(crate::object_task_catalog::tools()).chain(crate::validation_catalog::tools()).chain(beaver_core::task_callback_contract::business_tools()).chain(beaver_core::framework_contract::business_tools()).collect()
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
    if method == "object.inspectFiles"
        && !input["paths"].as_array().is_some_and(|paths| {
            !paths.is_empty()
                && paths.len() <= 100
                && paths.iter().all(|path| {
                    path.as_str().is_some_and(|path| {
                        !path.is_empty()
                            && path.len() <= 2_000
                            && std::path::Path::new(path).is_absolute()
                    })
                })
        })
    {
        return Err(fail("paths must contain 1..100 absolute paths".into()));
    }
    Ok(())
}

#[cfg(test)]
#[path = "object_catalog_contract_tests.rs"]
mod object_catalog_tests;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn object_import_discovery_accepts_path_only_and_optional_expected_identity() {
        let path = std::env::temp_dir().join("beaver-external-project");
        assert!(validate("object.inspectExternal", &json!({"path":path})).is_ok());
        assert!(validate(
            "object.inspectExternal",
            &json!({
                "path":path, "projectId":"source-1", "query":"hero"
            })
        )
        .is_ok());
        for invalid in [
            json!({}),
            json!({"projectId":"source-1"}),
            json!({"path":path, "projectId":null}),
            json!({"path":path, "projectId":1}),
            json!({"path":path, "register":true}),
        ] {
            assert!(validate("object.inspectExternal", &invalid).is_err());
        }
        let tool = tools()
            .into_iter()
            .find(|tool| tool["name"] == "object.inspectExternal")
            .unwrap();
        assert_eq!(tool["annotations"]["readOnlyHint"], true);
        assert_eq!(tool["inputSchema"]["required"], json!(["path"]));
    }

    #[test]
    fn object_import_requires_request_identity_and_inspected_digest() {
        let input = json!({"requestId":"r1","targetProjectId":"target","source":{},
            "objectId":"object","baseline":{"kind":"pinnedVersion","versionId":"v1"},
            "sourceDigest":"a".repeat(64)});
        assert!(validate("object.prepareImport", &input).is_ok());
        for field in ["requestId", "sourceDigest", "targetProjectId"] {
            let mut invalid = input.clone();
            invalid.as_object_mut().unwrap().remove(field);
            assert!(validate("object.prepareImport", &invalid).is_err());
        }
        let mut invalid = input;
        invalid["force"] = json!(true);
        assert!(validate("object.prepareImport", &invalid).is_err());
    }
    #[test]
    fn delivery_approval_requires_integer_cas_and_has_no_force_override() {
        let input = json!({"id":"task","requestId":"decision","expectedRevision":2,"candidateId":"candidate","decision":"approve","note":""});
        assert!(validate("assetTask.deliveryDecide", &input).is_ok());
        let tool = tools()
            .into_iter()
            .find(|tool| tool["name"] == "assetTask.deliveryDecide")
            .unwrap();
        assert_eq!(
            tool["inputSchema"]["properties"]["expectedRevision"]["type"],
            "integer"
        );
        for invalid in [json!("2"), Value::Null, json!(-1), json!(2.5)] {
            let mut wrong = input.clone();
            wrong["expectedRevision"] = invalid;
            assert!(validate("assetTask.deliveryDecide", &wrong).is_err());
        }
        let mut wrong = input;
        wrong["force"] = json!(true);
        assert!(validate("assetTask.deliveryDecide", &wrong).is_err());
    }
    #[test]
    fn external_imports_cannot_fall_back_to_dialogs() {
        assert!(validate("asset.import", &json!({"id":"x"})).is_err());
        assert!(validate("asset.import", &json!({"id":"x","paths":[]})).is_err());
        assert!(validate("game.importTemplates", &json!({})).is_err());
        assert!(validate("chooseDirectory", &json!({})).is_err());
        assert!(validate("object.inspectFiles", &json!({})).is_err());
        assert!(validate("object.inspectFiles", &json!({"paths":[]})).is_err());
        assert!(validate("state", &json!({"typo":true})).is_err());
        assert!(validate(
            "document.save",
            &json!({"id":"x","path":"a.md","text":"a","revision":null})
        )
        .is_ok());
    }

    #[test]
    fn ordinary_file_inspection_is_read_only_and_strict() {
        let path = std::env::temp_dir().join("beaver-object-import-file");
        assert!(validate("object.inspectFiles", &json!({"paths":[path]})).is_ok());
        let tool = tools()
            .into_iter()
            .find(|tool| tool["name"] == "object.inspectFiles")
            .unwrap();
        assert_eq!(tool["annotations"]["readOnlyHint"], true);
        for invalid in [
            json!({"paths":["relative"]}),
            json!({"paths":[1]}),
            json!({"paths":[path,"relative"]}),
            json!({"paths":[path],"extra":true}),
        ] {
            assert!(validate("object.inspectFiles", &invalid).is_err());
        }
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

    #[test]
    fn object_framework_exposes_readiness_and_transports_reserved_identity() {
        assert!(validate("objectFramework.status", &json!({"projectId":"project"})).is_ok());
        assert!(validate("objectFramework.status", &json!({})).is_err());
        assert!(validate(
            "objectFramework.status",
            &json!({"projectId":"project","initialize":true})
        )
        .is_err());
        let tools = tools();
        let status = tools
            .iter()
            .find(|tool| tool["name"] == "objectFramework.status")
            .unwrap();
        assert_eq!(status["annotations"]["readOnlyHint"], true);
        let create = tools
            .iter()
            .find(|tool| tool["name"] == "task.create")
            .unwrap();
        let identity = &create["inputSchema"]["properties"]["objectFramework"];
        assert_eq!(identity["type"], "object");
        let layers: Vec<_> = identity["oneOf"]
            .as_array()
            .expect("discriminated task identity variants")
            .iter()
            .map(|variant| variant["properties"]["layer"]["const"].as_str().unwrap())
            .collect();
        assert_eq!(layers, ["coarse", "medium", "fine"]);
        let request = json!({"projectId":"project","prompt":"Create","objectFramework":{
            "schemaVersion":1,"layer":"medium","objectId":"object-1","baseline":{"basePolicy":"empty"}}});
        assert!(validate("task.create", &request).is_ok());
        assert!(beaver_core::object_framework::require_legacy(&request).is_err());
        assert!(validate(
            "task.create",
            &json!({"projectId":"project","prompt":"Legacy","assetTask":true})
        )
        .is_ok());
    }
}

#[cfg(test)]
mod planning_contract_tests {
    use super::*;
    #[test]
    fn object_planning_contract_requires_explicit_identity_and_strict_revisions() {
        let start = json!({"projectId":"p","requestId":"r","draftId":"d","expectedDraftRevision":0,"expectedPlanRevision":0,"goal":"g","acceptance":"a","askRatio":30});
        assert!(validate("objectTaskPlanning.start", &start).is_ok());
        let mut invalid = start.clone();
        invalid["askRatio"] = json!(31);
        assert!(validate("objectTaskPlanning.start", &invalid).is_ok());
        invalid["expectedDraftRevision"] = json!("0");
        assert!(validate("objectTaskPlanning.start", &invalid).is_err());
        assert!(validate("objectTaskPlanning.get", &json!({"projectId":"p"})).is_err());
    }
}
