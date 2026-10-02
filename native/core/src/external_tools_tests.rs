use super::*;
use anyhow::Result;
use serde_json::{json, Value};
use std::{
    fs,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

fn context(root: &std::path::Path) -> Context {
    Context {
        workspace: root.to_owned(),
        run_id: uuid::Uuid::new_v4().to_string(),
        request_id: uuid::Uuid::new_v4().to_string(),
        blender_path: None,
    }
}
fn call(context: &Context, tool: &str, args: Value) -> Result<Value> {
    execute(context, tool, &args, &AtomicBool::new(false), &mut |_| {
        Ok(())
    })
}
#[test]
fn scoped_files_require_explicit_hash_and_reject_hidden_paths() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let context = context(temp.path());
    assert!(call(
        &context,
        "file.write",
        json!({"path":"x.txt","content":"x"})
    )
    .is_err());
    let created = call(
        &context,
        "file.write",
        json!({"path":"dir/x.txt","content":"first","expectedSha256":null}),
    )?;
    assert_eq!(created["sha256"], blender::digest(b"first"));
    assert!(call(
        &context,
        "file.write",
        json!({"path":"dir/x.txt","content":"clobber","expectedSha256":null})
    )
    .is_err());
    fs::write(temp.path().join("dir/x.txt"), "human change")?;
    assert!(call(
        &context,
        "file.write",
        json!({"path":"dir/x.txt","content":"clobber","expectedSha256":created["sha256"]})
    )
    .is_err());
    let read = call(&context, "file.read", json!({"path":"dir/x.txt"}))?;
    call(
        &context,
        "file.write",
        json!({"path":"dir/x.txt","content":"approved","expectedSha256":read["sha256"]}),
    )?;
    for path in [
        "../escape",
        "/tmp/escape",
        ".git/config",
        ".beaver-context/job",
        "sub/.env",
        "a\\b",
        "C:/x",
        "a/./b",
    ] {
        assert!(
            call(
                &context,
                "file.write",
                json!({"path":path,"content":"bad","expectedSha256":null})
            )
            .is_err(),
            "{path}"
        );
    }
    assert_eq!(
        fs::read_to_string(temp.path().join("dir/x.txt"))?,
        "approved"
    );
    Ok(())
}
#[cfg(unix)]
#[test]
fn links_cancellation_and_undeclared_arguments_are_rejected() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let outside = tempfile::tempdir()?;
    let context = context(temp.path());
    fs::write(outside.path().join("secret"), "outside")?;
    std::os::unix::fs::symlink(outside.path(), temp.path().join("linked"))?;
    assert!(call(&context, "file.read", json!({"path":"linked/secret"})).is_err());
    fs::hard_link(outside.path().join("secret"), temp.path().join("hard"))?;
    assert!(call(&context, "file.read", json!({"path":"hard"})).is_err());
    assert!(execute(
        &context,
        "file.write",
        &json!({"path":"x","content":"no","expectedSha256":null}),
        &AtomicBool::new(true),
        &mut |_| Ok(())
    )
    .is_err());
    assert!(!temp.path().join("x").exists());
    assert!(call(
        &context,
        "file.read",
        json!({"path":"hard","workspace":"/tmp"})
    )
    .is_err());
    assert!(call(&context, "shell", json!({"command":"echo no"})).is_err());
    Ok(())
}
#[test]
fn failed_validation_does_not_poison_run_and_fake_assets_fail() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let context = context(temp.path());
    assert!(call(
        &context,
        "blender.start",
        json!({"code":"print('no')","outputs":[{"path":"x.blend"}]})
    )
    .is_err());
    ensure_idle(&context.run_id)?;
    fs::write(temp.path().join("x.blend"), "not a blend")?;
    assert!(outputs::validate(&temp.path().join("x.blend")).is_err());
    fs::write(temp.path().join("x.png"), [])?;
    assert!(outputs::validate(&temp.path().join("x.png")).is_err());
    Ok(())
}
fn wait(context: &Context, job: &Value) -> Result<Value> {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let state = call(context, "blender.poll", json!({"jobId":job["jobId"]}))?;
        if !["preparing", "running"].contains(&state["status"].as_str().unwrap_or("")) {
            return Ok(state);
        }
        anyhow::ensure!(
            Instant::now() < deadline,
            "Real Blender test exceeded deadline"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}
#[test]
#[ignore = "requires an installed native Blender; run with BEAVER_TEST_BLENDER and --ignored"]
fn real_blender_owned_job_publishes_blend_glb_and_texture() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let mut context = context(temp.path());
    context.blender_path = Some(
        std::env::var_os("BEAVER_TEST_BLENDER")
            .ok_or_else(|| anyhow::anyhow!("BEAVER_TEST_BLENDER is required"))?
            .into(),
    );
    std::env::set_var("BEAVER_EXTERNAL_TEST_SECRET", "must-not-be-inherited");
    let script = r#"import os
assert 'BEAVER_EXTERNAL_TEST_SECRET' not in os.environ
assert os.environ['HOME'].startswith(BEAVER_WORKSPACE)
bpy.ops.mesh.primitive_uv_sphere_add()
bpy.ops.wm.save_as_mainfile(filepath=beaver_output('model.blend'), compress=False)
bpy.ops.export_scene.gltf(filepath=beaver_output('model.glb'), export_format='GLB')
image=bpy.data.images.new('probe', width=4, height=4)
image.filepath_raw=beaver_output('texture.png')
image.file_format='PNG'
image.save()
"#;
    let job = call(
        &context,
        "blender.start",
        json!({"code":script,"outputs":[{"path":"model.blend","expectedSha256":null},{"path":"model.glb","expectedSha256":null},{"path":"texture.png","expectedSha256":null}],"timeoutSeconds":30}),
    )?;
    assert!(job["process"]["pid"].as_u64().unwrap() > 0);
    let other = super::tests::context(temp.path());
    assert!(call(&other, "blender.poll", json!({"jobId":job["jobId"]})).is_err());
    assert!(job["process"]["command"]["arguments"]
        .as_array()
        .unwrap()
        .contains(&json!("--python-exit-code")));
    let final_state = wait(&context, &job)?;
    assert_eq!(final_state["status"], "succeeded", "{final_state}");
    assert_eq!(final_state["result"]["exitCode"], 0);
    assert_eq!(
        final_state["result"]["outputs"].as_array().unwrap().len(),
        3
    );
    for output in final_state["result"]["outputs"].as_array().unwrap() {
        assert_eq!(
            crate::files::file_hash(&temp.path().join(output["path"].as_str().unwrap()))?,
            output["sha256"].as_str().map(str::to_owned)
        );
    }
    ensure_idle(&context.run_id)?;
    cleanup(&context.run_id)?;
    std::env::remove_var("BEAVER_EXTERNAL_TEST_SECRET");
    Ok(())
}
#[test]
#[ignore = "requires an installed native Blender; run with BEAVER_TEST_BLENDER and --ignored"]
fn real_blender_cancel_timeout_and_python_error_never_publish() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let mut context = context(temp.path());
    context.blender_path = Some(
        std::env::var_os("BEAVER_TEST_BLENDER")
            .ok_or_else(|| anyhow::anyhow!("BEAVER_TEST_BLENDER is required"))?
            .into(),
    );
    for (code, timeout, cancel, expected) in [
        ("import time; time.sleep(20)", 30, true, "cancelled"),
        ("import time; time.sleep(20)", 1, false, "timed_out"),
        (
            "raise RuntimeError('deliberate probe')",
            30,
            false,
            "failed",
        ),
    ] {
        context.request_id = uuid::Uuid::new_v4().to_string();
        let job = call(
            &context,
            "blender.start",
            json!({"code":code,"outputs":[{"path":"not-published.txt","expectedSha256":null}],"timeoutSeconds":timeout}),
        )?;
        if cancel {
            call(&context, "blender.cancel", json!({"jobId":job["jobId"]}))?;
        }
        let result = wait(&context, &job)?;
        assert_eq!(result["status"], expected, "{result}");
        if expected == "failed" {
            assert_ne!(result["result"]["exitCode"], 0);
        }
        assert!(!temp.path().join("not-published.txt").exists());
        ensure_idle(&context.run_id)?;
    }
    cleanup(&context.run_id)?;
    Ok(())
}
