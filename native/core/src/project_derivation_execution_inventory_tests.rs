use super::*;
use std::fs;

#[test]
fn project_derivation_execution_rejects_missing_or_changed_output_and_blobs() -> Result<()> {
    for change in [
        "missing-workspace",
        "missing-file",
        "changed-file",
        "extra-file",
        "missing-blob",
        "changed-blob",
    ] {
        let f = ExecutionFixture::new(State::Failed, "empty")?;
        let workspace = f.workspace();
        let hash = f.attempt.output.as_ref().unwrap()["partial.txt"].clone();
        let blob = f.base.source.join(".beaver/content/blobs").join(hash);
        let expected = match change {
            "missing-workspace" => {
                fs::remove_dir_all(&workspace)?;
                "WORKSPACE_MISSING"
            }
            "missing-file" => {
                fs::remove_file(workspace.join("partial.txt"))?;
                "WORKSPACE_DRIFT"
            }
            "changed-file" => {
                fs::write(workspace.join("partial.txt"), "changed")?;
                "WORKSPACE_DRIFT"
            }
            "extra-file" => {
                fs::write(workspace.join("extra.txt"), "extra")?;
                "WORKSPACE_DRIFT"
            }
            "missing-blob" => {
                fs::remove_file(blob)?;
                "BLOB_MISSING"
            }
            _ => {
                fs::write(blob, "bad bytes")?;
                "BLOB_MISMATCH"
            }
        };
        f.rejected(change, expected)?;
    }
    Ok(())
}

#[test]
fn project_derivation_execution_rejects_case_alias_and_unknown_workspace_namespaces() -> Result<()>
{
    let f = ExecutionFixture::new(State::Failed, "empty")?;
    let before = data_backup::inventory(&f.base.source)?;
    let mut entries = before.clone();
    let mut alias = entries
        .iter()
        .find(|e| e.path.ends_with("partial.txt"))
        .unwrap()
        .clone();
    alias.path = alias.path.to_uppercase();
    entries.push(alias);
    let storage = ProjectStore::open(&f.base.source, "original")?;
    let error = crate::project_derivation_execution_inventory::validate(
        &storage.store().connection,
        &entries,
    )
    .unwrap_err();
    assert!(error.to_string().contains("INVENTORY_ALIAS"));
    drop(storage);
    assert_eq!(data_backup::inventory(&f.base.source)?, before);
    fs::create_dir(f.base.source.join(".beaver/workspaces/unknown"))?;
    f.rejected(
        "unknown-workspace",
        "ambiguous or unknown workspace identity",
    )?;
    Ok(())
}

#[test]
fn project_derivation_execution_rejects_ambiguous_legacy_and_object_workspace_identity(
) -> Result<()> {
    let f = ExecutionFixture::new(State::Failed, "empty")?;
    let storage = ProjectStore::open(&f.base.source, "original")?;
    storage.store().put(
        "task",
        &f.attempt.preparation.run.id,
        &json!({"id":f.attempt.preparation.run.id,"projectId":"original","status":"interrupted"}),
    )?;
    drop(storage);
    f.rejected(
        "ambiguous-workspace",
        "ambiguous or unknown workspace identity",
    )
}
