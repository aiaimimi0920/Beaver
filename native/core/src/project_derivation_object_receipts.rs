//! Reconstruct only known typed commands and verify their original input before remapping.
use crate::{
    object_command_receipt::{digest, CommandResult, Receipt, MAX_REVISION},
    object_registration::{RegisterRequest, UpdateRequest},
    object_version_acceptance::AcceptanceRequest,
    object_version_capture::CaptureRequest,
    object_version_manifest::{self, VersionStatus},
    project_derivation_copy::Request,
    project_derivation_identity::generated,
};
use anyhow::{bail, ensure, Context, Result};
use serde_json::Value;

pub(crate) fn key(result: &CommandResult) -> Result<String> {
    digest(&(&result.project_id, &result.request_id))
}

pub(crate) fn target_key(value: &Value, request: &Request) -> Result<String> {
    let receipt: Receipt = serde_json::from_value(value.clone())?;
    digest(&(
        &request.target_project_id,
        generated(
            request,
            "object_command_request",
            &receipt.result.request_id,
        )?,
    ))
}

// The real request structs fix serialization order/defaults. Hashing a Value is not equivalent.
pub(crate) fn input_digest(result: &CommandResult) -> Result<String> {
    let object = &result.object;
    ensure!(object.revision <= MAX_REVISION, "INVALID_OBJECT_REVISION");
    if let Some(version_id) = &result.version_id {
        let version = object
            .versions
            .iter()
            .find(|v| &v.version_id == version_id)
            .context("DERIVATION_OBJECT_RECEIPT_VERSION_MISSING")?;
        let manifest = object_version_manifest::read_version(object, version)?;
        let expected_revision = object
            .revision
            .checked_sub(1)
            .context("DERIVATION_OBJECT_RECEIPT_REVISION_INVALID")?;
        return match manifest.status {
            VersionStatus::Captured => {
                ensure!(
                    object.versions.last() == Some(version),
                    "DERIVATION_OBJECT_CAPTURE_ORDER"
                );
                digest(&(
                    "capture",
                    CaptureRequest {
                        project_id: result.project_id.clone(),
                        request_id: result.request_id.clone(),
                        object_id: object.id.clone(),
                        expected_revision,
                    },
                ))
            }
            VersionStatus::Accepted => digest(&(
                "accept",
                AcceptanceRequest {
                    project_id: result.project_id.clone(),
                    request_id: result.request_id.clone(),
                    object_id: object.id.clone(),
                    version_id: version_id.clone(),
                    expected_revision,
                },
            )),
            _ => bail!("DERIVATION_OBJECT_COMMAND_UNSUPPORTED"),
        };
    }
    if object.revision == 0 {
        ensure!(
            object.versions.is_empty(),
            "DERIVATION_OBJECT_REGISTRATION_VERSIONS"
        );
        digest(&(
            "register",
            RegisterRequest {
                project_id: result.project_id.clone(),
                request_id: result.request_id.clone(),
                name: object.name.clone(),
                category: object.category.clone(),
                tags: object.tags.clone(),
                thumbnail_path: object.thumbnail_path.clone(),
                parent_object_id: object.parent_object_id.clone(),
                components: object.components.clone(),
                files: object.files.clone(),
                references: object.references.clone(),
            },
        ))
    } else {
        digest(&(
            "update",
            UpdateRequest {
                project_id: result.project_id.clone(),
                request_id: result.request_id.clone(),
                object_id: object.id.clone(),
                expected_revision: object.revision - 1,
                name: object.name.clone(),
                category: object.category.clone(),
                tags: object.tags.clone(),
                thumbnail_path: object.thumbnail_path.clone(),
                parent_object_id: object.parent_object_id.clone(),
                components: object.components.clone(),
                files: object.files.clone(),
                references: object.references.clone(),
            },
        ))
    }
}

pub(crate) fn read(id: &str, value: &Value, project: &str) -> Result<Receipt> {
    let receipt: Receipt = serde_json::from_value(value.clone())?;
    let result = &receipt.result;
    ensure!(
        result.project_id == project
            && result.object.project_id == project
            && object_version_manifest::valid_id(&result.request_id)
            && key(result)? == id,
        "DERIVATION_OBJECT_RECEIPT_IDENTITY_MISMATCH"
    );
    ensure!(
        receipt.input_digest == input_digest(result)?,
        "DERIVATION_OBJECT_RECEIPT_DIGEST_MISMATCH"
    );
    Ok(receipt)
}
