use anyhow::{ensure, Context, Result};
use std::{io::Write, path::PathBuf, time::Duration};

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    match args.get(1).and_then(|arg| arg.to_str()) {
        Some("--child") => {
            let marker = PathBuf::from(args.get(2).context("marker required")?);
            std::fs::write(
                marker.with_extension("started"),
                std::process::id().to_string(),
            )?;
            std::thread::sleep(Duration::from_secs(3));
            std::fs::write(args.get(2).context("marker required")?, "child survived")?;
            return Ok(());
        }
        Some("--parent") => {
            let mut command = std::process::Command::new(std::env::current_exe()?);
            command
                .arg("--child")
                .arg(args.get(2).context("marker required")?)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                command.creation_flags(0x08000000);
            }
            let mut child = command.spawn()?;
            std::thread::sleep(Duration::from_secs(10));
            child.wait()?;
            return Ok(());
        }
        Some("--large") => {
            std::io::stdout().write_all(&vec![b'x'; 9 * 1024 * 1024])?;
            return Ok(());
        }
        Some("--output") => {
            println!("stdout fixture");
            eprintln!("stderr fixture");
            std::process::exit(7);
        }
        _ => {}
    }
    let output = PathBuf::from(args.get(1).context("proof path required")?);
    let temp = tempfile::tempdir()?;
    let exe = std::env::current_exe()?;
    let result = beaver_core::process::run(&exe, &["--output"], None, Duration::from_secs(5))?;
    ensure!(
        result.code == 7
            && result.text.contains("stdout fixture")
            && result.text.contains("stderr fixture"),
        "stdout/stderr or exit code lost"
    );
    ensure!(
        beaver_core::process::run(&exe, &["--large"], None, Duration::from_secs(5)).is_err(),
        "oversized output accepted"
    );
    let marker = temp.path().join("orphan-marker");
    let result = beaver_core::process::run(
        &exe,
        &["--parent", marker.to_str().context("marker encoding")?],
        None,
        Duration::from_millis(600),
    );
    ensure!(result.is_err(), "hanging tool did not time out");
    ensure!(
        marker.with_extension("started").is_file(),
        "descendant never started; timeout check is inconclusive"
    );
    std::thread::sleep(Duration::from_secs(4));
    ensure!(!marker.exists(), "descendant survived timeout cleanup");
    let cancelled = std::sync::atomic::AtomicBool::new(true);
    let cancelled_marker = temp.path().join("cancelled-marker");
    ensure!(
        beaver_core::process::run_cancellable(
            &exe,
            &[
                "--parent",
                cancelled_marker.to_str().context("marker encoding")?
            ],
            None,
            Duration::from_secs(10),
            &cancelled,
        )
        .is_err(),
        "pre-cancelled process was accepted"
    );
    ensure!(
        !cancelled_marker.with_extension("started").exists(),
        "pre-cancelled process started"
    );
    cancelled.store(false, std::sync::atomic::Ordering::SeqCst);
    std::thread::scope(|scope| -> Result<()> {
        let monitor = scope.spawn(|| -> Result<()> {
            let deadline = std::time::Instant::now() + Duration::from_secs(8);
            while !cancelled_marker.with_extension("started").is_file() {
                if std::time::Instant::now() >= deadline {
                    cancelled.store(true, std::sync::atomic::Ordering::SeqCst);
                    anyhow::bail!("cancellation child never started");
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            cancelled.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        });
        let result = beaver_core::process::run_cancellable(
            &exe,
            &[
                "--parent",
                cancelled_marker.to_str().context("marker encoding")?,
            ],
            None,
            Duration::from_secs(10),
            &cancelled,
        );
        monitor
            .join()
            .map_err(|_| anyhow::anyhow!("cancellation monitor panicked"))??;
        ensure!(result.is_err(), "running tool ignored cancellation");
        Ok(())
    })?;
    std::thread::sleep(Duration::from_secs(4));
    ensure!(
        !cancelled_marker.exists(),
        "descendant survived cancellation cleanup"
    );
    std::fs::create_dir_all(output.parent().context("proof parent required")?)?;
    std::fs::write(
        &output,
        serde_json::to_vec_pretty(
            &serde_json::json!({"checks":["stdout and stderr captured","exit code preserved","oversized output rejected","timeout kills descendant before deferred write","pre-cancelled process never starts","cancellation kills already-started descendant before deferred write"],"passed":true}),
        )?,
    )?;
    println!("{}", output.display());
    Ok(())
}
