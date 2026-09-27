use super::{attempt_fixture::*, object_attempt_rpc_fixture as rpc, queue_fixture};
use crate::{
    executor::Control,
    object_attempt::State,
    object_attempt_godot::{self as godot, Import},
    object_attempt_launch::Factory,
    object_attempt_worker, object_run_preparation,
};
use anyhow::{Context, Result};
use fs2::FileExt;
use std::{
    fs::{self, File, OpenOptions},
    path::Path,
    process::Stdio,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};
use tokio::{process::Command, sync::mpsc};

const IMPORTER: &str = "object_tasks::tests::object_attempt_godot::fake_importer";
const WRITER: &str = "object_tasks::tests::object_attempt_godot::fake_import_writer";

#[test]
fn fake_importer() -> Result<()> {
    let Some(root) = std::env::var_os("BEAVER_IMPORT_TEST_ROOT") else {
        return Ok(());
    };
    let root = Path::new(&root);
    fs::write(root.join("root-pid"), std::process::id().to_string())?;
    let lease = File::create(root.join("root-lock"))?;
    lease.lock_exclusive()?;
    let mut child = std::process::Command::new(std::env::current_exe()?)
        .args(["--exact", WRITER, "--nocapture"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while !root.join("writer-ready").exists() {
        anyhow::ensure!(child.try_wait()?.is_none(), "writer exited early");
        anyhow::ensure!(std::time::Instant::now() < deadline, "writer timeout");
        std::thread::sleep(Duration::from_millis(10));
    }
    println!("import diagnostics");
    fs::write(root.join("ready"), "ready")?;
    match std::env::var("BEAVER_IMPORT_TEST_MODE")?.as_str() {
        "exit" => Ok(()),
        "fail" => anyhow::bail!("import failed"),
        _ => {
            std::thread::sleep(Duration::from_secs(30));
            Ok(())
        }
    }
}

#[test]
fn fake_import_writer() -> Result<()> {
    let Some(root) = std::env::var_os("BEAVER_IMPORT_TEST_ROOT") else {
        return Ok(());
    };
    let root = Path::new(&root);
    fs::write(root.join("writer-pid"), std::process::id().to_string())?;
    let lease = File::create(root.join("writer-lock"))?;
    lease.lock_exclusive()?;
    fs::write(root.join("imported.txt"), "imported content")?;
    fs::write(root.join("writer-ready"), "ready")?;
    std::thread::sleep(Duration::from_secs(30));
    Ok(())
}

#[tokio::test]
async fn import_exit_failure_timeout_and_interrupt_close_descendants() -> Result<()> {
    for mode in ["exit", "fail", "timeout", "interrupt"] {
        let temp = tempfile::tempdir()?;
        let root = temp.path().to_owned();
        let mut command = Command::new(std::env::current_exe()?);
        command
            .args(["--exact", IMPORTER, "--nocapture"])
            .env("BEAVER_IMPORT_TEST_ROOT", &root)
            .env("BEAVER_IMPORT_TEST_MODE", mode);
        let (sender, mut receiver) = mpsc::channel(4);
        let timeout = if mode == "timeout" {
            Duration::from_secs(2)
        } else {
            Duration::from_secs(15)
        };
        let worker_root = root.clone();
        let worker = tokio::spawn(async move {
            godot::execute(
                command,
                &worker_root,
                timeout,
                &AtomicBool::new(false),
                &mut receiver,
            )
            .await
        });
        rpc::wait_until(|| root.join("ready").exists()).await;
        if mode == "interrupt" {
            sender.send(Control::Interrupt).await?;
        }
        let result = worker.await?.map_err(anyhow::Error::msg)?;
        let expected = match mode {
            "exit" => State::AwaitingGate,
            "interrupt" => State::Interrupted,
            _ => State::Failed,
        };
        assert_eq!(result.0, expected, "{mode}");
        if mode == "timeout" {
            assert!(result.1.unwrap().contains("GODOT_TIMEOUT"));
        }
        for name in ["root", "writer"] {
            assert_exited(&root, name).with_context(|| format!("{mode}: {name}"))?;
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .open(root.join(format!("{name}-lock")))?;
            // LockFileEx documents deferred OS unlock after process termination.
            // Prove termination above, then wait only for the lock release.
            tokio::time::timeout(Duration::from_secs(5), async {
                loop {
                    match file.try_lock_exclusive() {
                        Ok(()) => return Ok::<_, std::io::Error>(()),
                        Err(error) if error.raw_os_error() == Some(33) => {
                            tokio::time::sleep(Duration::from_millis(10)).await;
                        }
                        Err(error) => return Err(error),
                    }
                }
            })
            .await
            .with_context(|| format!("{mode}: release {name} lock"))??;
        }
        assert_eq!(
            fs::read_to_string(root.join("imported.txt"))?,
            "imported content"
        );
        assert!(fs::read_to_string(root.join("godot-import.log"))?.contains("import diagnostics"));
    }
    Ok(())
}

fn assert_exited(root: &Path, name: &str) -> Result<()> {
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use windows_sys::Win32::{
        Foundation::{ERROR_INVALID_PARAMETER, WAIT_OBJECT_0},
        System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE},
    };
    let pid = fs::read_to_string(root.join(format!("{name}-pid")))?.parse()?;
    unsafe {
        let handle = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        if handle.is_null() {
            let error = std::io::Error::last_os_error();
            anyhow::ensure!(
                error.raw_os_error() == Some(ERROR_INVALID_PARAMETER as i32),
                "{error}"
            );
        } else {
            let process = OwnedHandle::from_raw_handle(handle);
            anyhow::ensure!(
                WaitForSingleObject(process.as_raw_handle(), 0) == WAIT_OBJECT_0,
                "process still running: {pid}"
            );
        }
    }
    Ok(())
}

#[tokio::test]
async fn asset_attempt_skips_import_and_keeps_its_checkpoint() -> Result<()> {
    let fixture = fixture()?;
    queue_fixture::enqueue(&fixture, &["head"])?;
    let claim =
        object_run_preparation::claim_next(&fixture.runtime, "project-1", "worker")?.unwrap();
    let root = fixture.temp.path().join("rpc");
    let launch_factory: Factory = Arc::new(move |attempt, runtime| {
        let cwd = runtime
            .files()
            .resolve_workspace(
                &attempt.preparation.run.id,
                Path::new(&attempt.preparation.workspace),
            )
            .map_err(|error| error.to_string())?;
        let mut launch = rpc::launch(&root, &cwd, "early").map_err(|error| error.to_string())?;
        launch.godot = Some(Import {
            executable: "missing-godot.exe".into(),
            timeout: Duration::from_secs(1),
        });
        Ok(launch)
    });
    let (_sender, receiver) = mpsc::channel(4);
    object_attempt_worker::execute(
        fixture.runtime.clone(),
        claim,
        launch_factory,
        Arc::new(AtomicBool::new(false)),
        receiver,
        Arc::new(|| {}),
    )
    .await
    .map_err(anyhow::Error::msg)?;
    let attempt = attempts(&fixture, "head")?.remove(0);
    assert_eq!(attempt.state, State::AwaitingGate);
    assert!(attempt.output.unwrap().contains_key("result.txt"));
    assert!(!fixture
        .runtime
        .files()
        .codex_home(&attempt.id)?
        .join("godot-import.log")
        .exists());
    Ok(())
}

#[tokio::test]
async fn import_never_searches_parent_projects_or_runs_after_cancellation() -> Result<()> {
    let temp = tempfile::tempdir()?;
    fs::write(temp.path().join("project.godot"), "parent project")?;
    let cwd = temp.path().join("workspace");
    fs::create_dir(&cwd)?;
    let home = temp.path().join("home");
    let (_sender, mut receiver) = mpsc::channel(4);
    for cancelled in [false, true] {
        let result = godot::run(
            Import {
                executable: "missing-godot.exe".into(),
                timeout: Duration::ZERO,
            },
            &cwd,
            &home,
            &AtomicBool::new(cancelled),
            &mut receiver,
        )
        .await
        .map_err(anyhow::Error::msg)?;
        assert_eq!(
            result.0,
            if cancelled {
                State::Interrupted
            } else {
                State::AwaitingGate
            }
        );
        assert!(!home.exists());
    }
    fs::write(cwd.join("project.godot"), "workspace project")?;
    let result = godot::run(
        Import {
            executable: "missing-godot.exe".into(),
            timeout: Duration::ZERO,
        },
        &cwd,
        &home,
        &AtomicBool::new(false),
        &mut receiver,
    )
    .await
    .map_err(anyhow::Error::msg)?;
    assert_eq!(result.0, State::Failed);
    assert!(result.1.unwrap().contains("GODOT_UNAVAILABLE"));
    Ok(())
}

#[tokio::test]
#[ignore = "requires BEAVER_TEST_GODOT_EXE pointing to a native Godot editor"]
async fn worker_imports_real_godot_project_before_freezing_checkpoint() -> Result<()> {
    let executable =
        std::env::var_os("BEAVER_TEST_GODOT_EXE").context("BEAVER_TEST_GODOT_EXE is required")?;
    let fixture = fixture()?;
    queue_fixture::enqueue(&fixture, &["head"])?;
    let claim =
        object_run_preparation::claim_next(&fixture.runtime, "project-1", "worker")?.unwrap();
    let root = fixture.temp.path().join("rpc");
    let rpc_root = root.clone();
    let factory: Factory = Arc::new(move |attempt, runtime| {
        let cwd = runtime
            .files()
            .resolve_workspace(
                &attempt.preparation.run.id,
                Path::new(&attempt.preparation.workspace),
            )
            .map_err(|error| error.to_string())?;
        let mut launch =
            rpc::launch(&rpc_root, &cwd, "godot").map_err(|error| error.to_string())?;
        launch.godot = Some(Import {
            executable: executable.clone().into(),
            timeout: Duration::from_secs(60),
        });
        Ok(launch)
    });
    let (_sender, receiver) = mpsc::channel(4);
    object_attempt_worker::execute(
        fixture.runtime.clone(),
        claim,
        factory,
        Arc::new(AtomicBool::new(false)),
        receiver,
        Arc::new(|| {}),
    )
    .await
    .map_err(anyhow::Error::msg)?;
    rpc::assert_closed(&root)?;
    let attempt = attempts(&fixture, "head")?.remove(0);
    assert_eq!(attempt.state, State::AwaitingGate, "{:?}", attempt.error);
    let output = attempt.output.as_ref().unwrap();
    for name in [
        "result.txt",
        "child-output.txt",
        "project.godot",
        "icon.svg.import",
    ] {
        assert!(output.contains_key(name), "missing checkpoint file: {name}");
    }
    let cwd = workspace(&fixture, &attempt)?;
    assert_eq!(&fixture.runtime.files().capture(&cwd)?, output);
    let log = fs::read_to_string(
        fixture
            .runtime
            .files()
            .codex_home(&attempt.id)?
            .join("godot-import.log"),
    )?;
    assert!(log.contains("Godot Engine"), "{log}");
    Ok(())
}
