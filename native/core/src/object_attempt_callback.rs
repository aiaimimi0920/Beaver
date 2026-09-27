//! Read-only model tools scoped by the host's live attempt lease.
#[path = "object_attempt_feedback.rs"]
mod feedback_image;
use crate::{
    object_attempt::{self, Lease},
    object_attempt_file,
    project_runtime::ProjectRuntime,
};
use anyhow::{ensure, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

pub(crate) const TOOL: &str = "beaver_object_attempt";

#[path = "object_attempt_image.rs"]
mod image;

#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "camelCase", deny_unknown_fields)]
enum Request {
    Capabilities {},
    Context {},
    FeedbackImage {},
    InputFile { path: String, sha256: String },
}

pub(crate) fn definition() -> Value {
    json!({"type":"function","name":TOOL,
    "description":"Inspect capabilities, frozen task context, or a frozen input file for this attempt only. Read-only; never accepts, publishes, starts tools or advances stages. Use path and sha256 from context.input for inputFile. If context.feedbackImage is present, call feedbackImage without file arguments to inspect the historical PNG bound to this follow-up. Valid PNG files return an actual image plus source metadata; inspect that image when applying image-region feedback.",
    "inputSchema":{"type":"object","additionalProperties":false,"required":["operation"],
        "properties":{
            "operation":{"type":"string","enum":["capabilities","context","inputFile","feedbackImage"]},
            "path":{"type":"string","maxLength":4096},
            "sha256":{"type":"string","minLength":64,"maxLength":64}
        }}})
}

pub(crate) fn context(runtime: &ProjectRuntime, lease: &Lease) -> Result<Value> {
    let record = lease.record();
    Ok(
        json!({"projectId":record.preparation.project_id,"objectId":record.preparation.run.object_id,
        "runId":record.preparation.run.id,"attemptId":record.id,"fine":record.fine,
        "medium":record.preparation.medium,"baseline":record.preparation.baseline,"input":record.input,
        "feedbackImage":feedback_image::resolve(runtime, record)?}),
    )
}

pub(crate) fn call(runtime: &ProjectRuntime, lease: &Lease, params: &Value) -> Result<Value> {
    let record = lease.record();
    ensure!(params["tool"] == TOOL, "OBJECT_ATTEMPT_TOOL_UNAVAILABLE");
    ensure!(
        record.thread_id.is_some()
            && record.turn_id.is_some()
            && params["threadId"].as_str() == record.thread_id.as_deref()
            && params["turnId"].as_str() == record.turn_id.as_deref(),
        "OBJECT_ATTEMPT_RPC_IDENTITY_MISMATCH"
    );
    object_attempt::validate_callback(runtime, lease)?;
    let arguments = params
        .get("arguments")
        .context("OBJECT_ATTEMPT_TOOL_ARGUMENTS_MISSING")?;
    ensure!(
        serde_json::to_vec(arguments)?.len() <= 8192,
        "OBJECT_ATTEMPT_TOOL_ARGUMENTS_TOO_LARGE"
    );
    let request: Request = serde_json::from_value(arguments.clone())
        .context("OBJECT_ATTEMPT_TOOL_ARGUMENTS_INVALID")?;
    let value = match request {
        Request::Capabilities {} => {
            crate::asset_tool::text_result(json!({"schemaVersion":1,"tools":[definition()],
            "scope":{"projectId":record.preparation.project_id,"objectId":record.preparation.run.object_id,
                "runId":record.preparation.run.id,"attemptId":record.id},
            "readOnly":true,"inputFileInlineLimit":object_attempt_file::INLINE_LIMIT,
            "inputImageFormats":["image/png"],
            "unavailable":["managedTools","accept","publish","advance","outputCheckpoint"]}))
        }
        Request::Context {} => crate::asset_tool::text_result(context(runtime, lease)?),
        Request::FeedbackImage {} => {
            let binding = feedback_image::resolve(runtime, record)?
                .context("OBJECT_FEEDBACK_IMAGE_UNAVAILABLE")?;
            let mut result = if let Some(file) = binding.file() {
                image::result(object_attempt_file::read(runtime, file)?)?
            } else {
                image::archived(
                    binding
                        .data_url()
                        .context("OBJECT_FEEDBACK_IMAGE_UNAVAILABLE")?,
                )?
            };
            result["contentItems"].as_array_mut().unwrap().push(json!({
                "type":"inputText", "text":serde_json::to_string(&binding)?
            }));
            if let Some(historical) = binding.historical_data_url() {
                let items = result["contentItems"].as_array_mut().unwrap();
                items.insert(0, json!({"type":"inputText", "text":"Relocation target frame follows. Its numbered regions belong to the reviewed feedback checkpoint. The historical source frame follows the mapping metadata; region indices in the confirmation are zero-based."}));
                items.push(json!({"type":"inputText", "text":"Historical source frame follows. Use only the explicit matched/absent correspondence; old coordinates and topology remain historical."}));
                items.extend(
                    image::archived(historical)?["contentItems"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .cloned(),
                );
            }
            result
        }
        Request::InputFile { path, sha256 } => {
            ensure!(
                !path.is_empty()
                    && path.len() <= 4096
                    && sha256.len() == 64
                    && sha256.bytes().all(|byte| byte.is_ascii_hexdigit()),
                "OBJECT_ATTEMPT_TOOL_FILE_INVALID"
            );
            image::result(object_attempt_file::read(
                runtime,
                &object_attempt_file::Request {
                    project_id: record.preparation.project_id.clone(),
                    run_id: record.preparation.run.id.clone(),
                    attempt_id: record.id.clone(),
                    checkpoint: object_attempt_file::Checkpoint::Input,
                    path,
                    sha256,
                },
            )?)?
        }
    };
    // Blob IO runs outside the store lock. Do not deliver a result after cancellation.
    object_attempt::validate_callback(runtime, lease)?;
    Ok(value)
}
