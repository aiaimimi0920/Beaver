//! Typed object identity conversion. Asset paths, blob hashes and content are never rewritten.
use crate::{
    object_catalog::{ObjectComponent, ObjectRecord, ObjectReference, ObjectVersion},
    object_version_manifest::VersionManifest,
    project_derivation_copy::Request,
    project_derivation_identity::{generated, IdentityMap, Key},
    project_derivation_object_receipts as receipts,
    project_derivation_validation_records::Rewrite,
};
use anyhow::{bail, Result};
use serde_json::Value;

pub(crate) fn supports(kind: &str) -> bool {
    matches!(kind, "object" | "object_version" | "object_command_receipt")
}

struct Objects<'a> {
    ids: Rewrite<'a>,
    request: &'a Request,
}

impl Objects<'_> {
    fn optional(&self, value: &mut Option<String>, kind: &str) -> Result<()> {
        if let Some(id) = value {
            *id = self.ids.key(kind, id)?.id;
        }
        Ok(())
    }

    fn components(&self, values: &mut [ObjectComponent]) -> Result<()> {
        for component in values {
            component.id = generated(self.request, "object_component", &component.id)?;
        }
        Ok(())
    }

    fn references(&self, values: &mut [ObjectReference]) -> Result<()> {
        for reference in values {
            reference.project_id = self.ids.key("project", &reference.project_id)?.id;
            reference.object_id = self.ids.key("object", &reference.object_id)?.id;
            self.optional(&mut reference.version_id, "object_version")?;
        }
        Ok(())
    }

    fn manifest(&self, manifest: &mut VersionManifest) -> Result<()> {
        manifest.project_id = self.ids.key("project", &manifest.project_id)?.id;
        manifest.object_id = self.ids.key("object", &manifest.object_id)?.id;
        manifest.version_id = self.ids.key("object_version", &manifest.version_id)?.id;
        self.optional(&mut manifest.parent_object_id, "object")?;
        self.components(&mut manifest.components)?;
        self.references(&mut manifest.references)?;
        Ok(())
    }

    fn version(&self, version: &mut ObjectVersion) -> Result<()> {
        let mut manifest: VersionManifest = serde_json::from_value(version.manifest.clone())?;
        self.manifest(&mut manifest)?;
        version.version_id = self.ids.key("object_version", &version.version_id)?.id;
        version.manifest = serde_json::to_value(manifest)?;
        Ok(())
    }

    fn object(&self, object: &mut ObjectRecord) -> Result<()> {
        object.id = self.ids.key("object", &object.id)?.id;
        object.project_id = self.ids.key("project", &object.project_id)?.id;
        self.optional(&mut object.parent_object_id, "object")?;
        self.components(&mut object.components)?;
        self.references(&mut object.references)?;
        for version in &mut object.versions {
            self.version(version)?;
        }
        Ok(())
    }
}

pub(crate) fn rewrite(
    map: &IdentityMap,
    request: &Request,
    kind: &str,
    id: &str,
    value: &Value,
) -> Result<(Key, Value)> {
    let objects = Objects {
        ids: Rewrite(map),
        request,
    };
    let key = objects.ids.key(kind, id)?;
    let value = match kind {
        "object" => {
            let mut object: ObjectRecord = serde_json::from_value(value.clone())?;
            objects.object(&mut object)?;
            serde_json::to_value(object)?
        }
        "object_version" => {
            let (owner, mut version): (String, ObjectVersion) =
                serde_json::from_value(value.clone())?;
            objects.version(&mut version)?;
            serde_json::to_value((objects.ids.key("object", &owner)?.id, version))?
        }
        "object_command_receipt" => {
            let mut receipt = receipts::read(id, value, &request.source_project_id)?;
            objects.object(&mut receipt.result.object)?;
            receipt.result.project_id = request.target_project_id.clone();
            receipt.result.request_id = generated(
                request,
                "object_command_request",
                &receipt.result.request_id,
            )?;
            objects.optional(&mut receipt.result.version_id, "object_version")?;
            receipt.input_digest = receipts::input_digest(&receipt.result)?;
            serde_json::to_value(receipt)?
        }
        _ => bail!("unsupported derivation object kind: {kind}"),
    };
    Ok((key, value))
}

pub(crate) fn rewrite_snapshot(
    map: &IdentityMap,
    request: &Request,
    object: &mut ObjectRecord,
) -> Result<()> {
    Objects {
        ids: Rewrite(map),
        request,
    }
    .object(object)
}

pub(crate) fn rewrite_manifest(
    map: &IdentityMap,
    request: &Request,
    manifest: &mut VersionManifest,
) -> Result<()> {
    Objects {
        ids: Rewrite(map),
        request,
    }
    .manifest(manifest)
}

#[cfg(all(test, windows))]
#[path = "project_derivation_object_tests.rs"]
mod tests;
