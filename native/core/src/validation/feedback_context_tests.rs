use super::{feedback_context, repository, task_completion, test_support::Fixture};
use anyhow::Result;
use serde_json::{json, Value};
use std::fs;

#[test]
fn repair_relaunch_preserves_original_feedback_selection_and_sources() -> Result<()> {
    let mut fixture = Fixture::new()?;
    let flow = fixture.flow()?;
    let original = fixture.visual(&flow)?;
    let workspace = fixture.project.join(".beaver-context/test-workspace");
    fs::create_dir_all(&workspace)?;
    let feedback = json!({"text":"Keep the selected button visible","runId":original.id,
        "snapshotId":original.snapshot_id,
        "selection":{"evidenceId":original.evidence[0].id,"region":[0.1,0.2,0.3,0.4]}});
    feedback_context::freeze_feedback(&fixture.files, &workspace, &original, &feedback)?;
    let root = workspace.join(".beaver-context/validation");
    let original_bytes = fs::read(root.join("feedback.json"))?;
    let original_run = fs::read(root.join("run.json"))?;
    let original_source = fs::read(root.join("source/game.gd"))?;
    let original_media = fs::read(root.join(format!("media/{}.png", original.evidence[0].id)))?;
    let mut repair_paths = vec![];
    for value in [2, 3] {
        fs::write(
            fixture.project.join("game.gd"),
            format!("extends Node\nvar value = {value}\n"),
        )?;
        let mut repair = fixture.run(None)?;
        repair.status = "failed".into();
        repair.log = format!("value {value} assertion failed");
        fixture.save(&repair)?;
        let task =
            json!({"projectId":"p","validationRepair":{"runId":repair.id,"error":"GUT failed"}});
        // Initial launch and repeated launch of the same repair must both preserve feedback.
        for _ in 0..2 {
            task_completion::freeze_repair(
                &fixture.store,
                fixture.files.root(),
                &workspace,
                &task,
            )?;
        }
        let path = root.join("repairs").join(&repair.id);
        let captured: Value = serde_json::from_slice(&fs::read(path.join("run.json"))?)?;
        assert_eq!(captured["id"], repair.id);
        assert_eq!(
            fs::read(path.join("source/game.gd"))?,
            fs::read(fixture.project.join("game.gd"))?
        );
        assert!(path.join("repair.json").is_file());
        let prompt = crate::task_brief::prompt(&task, &Value::Null);
        assert!(prompt.contains(&format!(".beaver-context/validation/repairs/{}", repair.id)));
        repair_paths.push(path);
    }
    assert_ne!(repair_paths[0], repair_paths[1]);
    assert_eq!(fs::read(root.join("feedback.json"))?, original_bytes);
    assert_eq!(fs::read(root.join("run.json"))?, original_run);
    assert_eq!(fs::read(root.join("source/game.gd"))?, original_source);
    assert_eq!(
        fs::read(root.join(format!("media/{}.png", original.evidence[0].id)))?,
        original_media
    );
    assert!(fs::read_to_string(repair_paths[0].join("source/game.gd"))?.contains("value = 2"));
    let mut invalid = original.clone();
    invalid.id = "../../escape".into();
    assert!(
        feedback_context::freeze_repair(&fixture.files, &workspace, &invalid, &json!({})).is_err()
    );
    assert_eq!(
        repository::digest(&original.snapshot)?,
        original.snapshot_id
    );
    Ok(())
}
