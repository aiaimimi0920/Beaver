use crate::asset_task::{self, Feedback, State};
use base64::Engine;
use serde_json::{json, Value};

pub const INSTRUCTIONS: &str = include_str!("../../../resources/instructions/asset-task.md");

pub fn definition() -> Value {
    let text = json!({"type":"string","maxLength":12000});
    json!({
        "type":"function",
        "name":"beaver_asset_task",
        "description":"Manage this asset's durable production stages, safe checkpoints and serialized visual feedback. Poll at safe Blender boundaries. finishRound closes an asset round, not a Codex turn. Inspect the actual returned image before acknowledge; existing beaver_ask_user handles creative decisions.",
        "inputSchema":{
            "type":"object","additionalProperties":false,"required":["operation"],
            "properties":{
                "operation":{"type":"string","enum":["state","stages","poll","checkpoint","finishRound","acknowledge","deciding","execute","check","complete","verifyApplied","verifyNotApplied","fail","retryFailed"]},
                "revision":{"type":"integer","minimum":0},
                "stages":{"type":"array","maxItems":100,"items":{
                    "type":"object","additionalProperties":false,
                    "required":["id","name","status","dependencies","objects","evidence","round"],
                    "properties":{
                        "id":{"type":"string","maxLength":100},"name":{"type":"string","maxLength":300},
                        "status":{"type":"string","enum":["pending","running","completed","suspended","deciding","adjusting","checking","failed"]},
                        "dependencies":{"type":"array","items":{"type":"string"}},
                        "objects":{"type":"array","items":{"type":"string"}},
                        "evidence":text,"round":{"type":"integer","minimum":1}
                    }
                }},
                "feedbackId":{"type":"string","maxLength":100},"frameId":{"type":"string","maxLength":100},
                "imageObservation":text,"impact":text,"evidence":text,
                "affectedStages":{"type":"array","maxItems":100,"items":{"type":"string","maxLength":100}}
            }
        }
    })
}

/// Deferred text/images never enter a model turn before their production round.
pub fn projection(state: &State) -> Value {
    json!({
        "taskId":state.task_id,"round":state.round,"phase":state.phase,"revision":state.revision,
        "stages":state.stages,"checkpoint":state.checkpoint,"recovery":state.recovery,
        "currentFeedback":asset_task::eligible(state),
        "deferredCount":state.feedback.iter().filter(|f| !f.terminal() && f.round > state.round).count(),
        "completedFeedback":state.feedback.iter().filter(|f| f.terminal()).map(|f| json!({"id":f.id,"status":f.status})).collect::<Vec<_>>()
    })
}

pub fn text_result(value: Value) -> Value {
    json!({"success":true,"contentItems":[{"type":"inputText","text":value.to_string()}]})
}

pub fn image_result(feedback: &Feedback, png: &[u8]) -> Value {
    let encoded = base64::engine::general_purpose::STANDARD.encode(png);
    json!({"success":true,"contentItems":[
        {"type":"inputText","text":json!({"feedback":feedback,"instruction":"Inspect the actual image. The geometric hit is historical and not a semantic label. Recheck the current scene before mutation, then acknowledge with frameId, visible imageObservation and actual dependency impact."}).to_string()},
        {"type":"inputImage","imageUrl":format!("data:image/png;base64,{encoded}")}
    ]})
}
