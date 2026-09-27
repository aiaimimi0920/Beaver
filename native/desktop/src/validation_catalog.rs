use serde_json::{json, Value};

pub fn tools() -> Vec<Value> {
    let definitions = [
        ("validation.preview.capture", "Save the acknowledged frame with image regions and optional engine-issued frozen static mesh pick receipts. Immutable; up to eight frames and 16 MiB per run. Not acceptance evidence. Retry with the same requestId.", "runId:s,sessionId:s,revision:i,requestId:s", "selection:o", false),
        ("validation.preview.pick", "Query frozen Godot or Blender static meshes by ray or optional normalized rectangle (x,y,width,height; point inside). Box selection clips triangles to the camera frustum and includes occluded meshes; up to 32 nodePaths with explicit truncation. Excludes unsupported geometry. Poll the same request until ready. Exact frame identity required; at most 32 queries per frame.", "sessionId:s,revision:i,sequence:i,sha256:s,requestId:s,point:o", "rectangle:o", false),
        ("validation.preview.saved", "Read and verify saved interactive frames without starting an engine.", "runId:s", "", true),
        ("validation.preview.open", "Open an ephemeral Godot or Blender camera session from a completed frozen object preview. One session globally; same request reuses it. Not acceptance evidence.", "runId:s,requestId:s", "", false),
        ("validation.preview.read", "Read the latest engine frame and renew the visible viewer lease. Stops automatically after 15 seconds without reads.", "sessionId:s", "", false),
        ("validation.preview.view", "Set camera, resolution and optional frozen state atomically. Frozen retains one acknowledged frame; Godot pauses its SceneTree. Blender renders a static scene on camera changes, without animation; static mesh picking excludes modifiers, shape keys and instances. False resumes camera interaction. Wait for a frame with matching revision and frozen before saving. Sizes: 960x540, 1280x720, 1920x1080. Revision must increase.", "sessionId:s,revision:i,camera:o,width:i,height:i", "frozen:b", false),
        ("validation.preview.close", "Close only the specified preview session; never cancels production tasks.", "sessionId:s", "", false),
        ("validation.list", "List project code results, four visual categories, feedback, coverage and release checks.", "", "", true),
        ("validation.flow.list", "List registered replayable flows and their current results.", "", "", true),
        ("validation.flow.save", "Create or revise a replayable flow; expectedRevision=0 creates it. Edits require a reason. See beaver.validation.json format.", "definition:o,expectedRevision:i,requestId:s", "reason:s", false),
        ("validation.flow.explore", "Save a new seeded roaming route without changing the original route or baseline.", "flowId:s,expectedRevision:i,seed:i,requestId:s", "", false),
        ("validation.code.run", "Queue real GUT tests on a frozen current project snapshot.", "requestId:s", "taskId:s", false),
        ("validation.flow.run", "Queue a rendered replay with real screenshots and optional video.", "flowId:s,expectedRevision:i,requestId:s", "taskId:s", false),
        ("validation.run.all", "Queue code and all active visual flows; this is not a formal release check.", "requestId:s", "", false),
        ("validation.run.rerun", "Replay a historical recipe against current game source; preserve old evidence.", "runId:s,requestId:s", "", false),
        ("validation.run.get", "Read progress, evidence, logs, source paths and the exact comparison baseline.", "runId:s", "", true),
        ("validation.objectReport.get", "Read an exact persisted object attempt report with its frozen stage and candidate review references; never rerun or approve.", "attemptId:s,requestId:s", "", true),
        ("validation.run.cancel", "Cancel only the owned run. Task code gates use task.interrupt.", "runId:s,requestId:s", "", false),
        ("validation.source", "Read historical source and current text for a recorded project-relative path.", "runId:s,path:s", "", true),
        ("validation.evidence.confirm", "Human-only UI confirmation of exact evidence. API/MCP cannot claim human approval.", "runId:s,snapshotId:s,evidenceIds:a,requestId:s", "", false),
        ("validation.feedback.create", "Create a repair or read-only AI review task with frozen media/source context; repair reruns the original recipe.", "runId:s,snapshotId:s,text:s,requestId:s", "evidenceId:s,range:a,region:a,mode:s,taskId:s", false),
        ("validation.settings.save", "Save project visual release policy and optional FFmpeg path with revision checking.", "settings:o,expectedRevision:i,requestId:s", "", false),
        ("validation.release.start", "Freeze a formal candidate, preset, coverage and policy; queue fresh GUT and required visuals.", "preset:s,requestId:s", "", false),
        ("validation.release.get", "Read exact release scope and green requirements. Missing evidence never passes.", "releaseCheckId:s", "", true),
    ];
    definitions.into_iter().map(|(name, description, required, optional, read)| {
        let required = format!("projectId:s,{required}");
        let mut properties = serde_json::Map::new();
        for field in required.split(',').chain(optional.split(',')).filter(|v| !v.is_empty()) {
            let (key, kind) = field.split_once(':').expect("static validation field");
            let mut schema = match kind {
                "i" => json!({"type":"integer","minimum":0}),
                "b" => json!({"type":"boolean"}),
                "a" => json!({"type":"array"}),
                "o" => json!({"type":"object"}),
                _ => json!({"type":"string"}),
            };
            match key {
                "mode" => schema["enum"] = json!(["new","child","followup","review"]),
                "evidenceIds" => schema["items"] = json!({"type":"string"}),
                "range" | "region" => schema["items"] = json!({"type":"number"}),
                "settings" => schema = json!({"type":"object","properties":{"visualRequired":{"type":"boolean"},"ffmpeg":{"type":"string"},"revision":{"type":"integer"}},"required":["visualRequired","ffmpeg"],"additionalProperties":false}),
                _ => {}
            }
            properties.insert(key.into(), schema);
        }
        let required: Vec<_> = required.split(',').filter(|s| !s.is_empty()).map(|s| s.split(':').next().unwrap()).collect();
        json!({"name":name,"description":description,"inputSchema":{"type":"object","properties":properties,"required":required,"additionalProperties":false},"annotations":{"readOnlyHint":read,"destructiveHint":false,"openWorldHint":true}})
    }).collect()
}
