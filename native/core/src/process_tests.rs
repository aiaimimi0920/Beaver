use super::*;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};

const CHILD: &str = r#"
printf '%s' "$$" > parent.pid
sh -c 'while :; do printf . >> heartbeat; sleep 0.02; done' &
descendant=$!
printf '%s' "$descendant" > descendant.pid
while [ ! -f heartbeat ]; do sleep 0.01; done
printf 'fixture stdout\n'
printf 'fixture stderr\n' >&2
printf ready > ready
if [ "$1" = overflow ]; then yes overflow-evidence; else wait "$descendant"; fi
"#;

struct Fixture {
    temp: tempfile::TempDir,
    log: PathBuf,
}

impl Fixture {
    fn new() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let log = temp.path().join("process.log");
        Ok(Self { temp, log })
    }

    fn run(&self, mode: &str, timeout: Duration, cancelled: &AtomicBool) -> Result<Output> {
        run_cancellable_env_logged(
            Path::new("/bin/sh"),
            &["-c", CHILD, "fixture", mode],
            Some(self.temp.path()),
            timeout,
            cancelled,
            &Default::default(),
            &self.log,
        )
    }

    fn assert_stopped(&self) -> Result<()> {
        for file in ["parent.pid", "descendant.pid"] {
            let pid: i32 = fs::read_to_string(self.temp.path().join(file))?.parse()?;
            let deadline = Instant::now() + Duration::from_secs(2);
            while running(pid) && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(10));
            }
            assert!(!running(pid), "Owned fixture process still running: {file}");
        }
        let heartbeat = self.temp.path().join("heartbeat");
        let length = fs::metadata(&heartbeat)?.len();
        let log_length = fs::metadata(&self.log)?.len();
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(fs::metadata(heartbeat)?.len(), length);
        assert_eq!(fs::metadata(&self.log)?.len(), log_length);
        Ok(())
    }

    fn assert_failure(&self, result: Result<Output>, expected: &str) -> Result<()> {
        let error = result.err().context("Expected process failure")?;
        assert!(error.downcast_ref::<ProcessLogError>().is_some());
        let error = error.to_string();
        assert!(error.contains(expected), "{error}");
        assert!(error.contains(self.log.to_str().unwrap()), "{error}");
        assert!(!error.contains("fixture stdout"));
        assert!(!error.contains("fixture stderr"));
        assert!(fs::metadata(&self.log)?.len() <= OUTPUT_LIMIT);
        self.assert_stopped()
    }
}

// A killed descendant can briefly remain a zombie under the test host's PID 1.
// Check this fixture's recorded PID without ever sending it another signal.
fn running(pid: i32) -> bool {
    if unsafe { libc::kill(pid, 0) } != 0 {
        return std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH);
    }
    #[cfg(target_os = "linux")]
    if let Ok(stat) = fs::read_to_string(format!("/proc/{pid}/stat")) {
        if stat
            .rsplit_once(") ")
            .is_some_and(|(_, rest)| rest.starts_with('Z'))
        {
            return false;
        }
    }
    true
}

#[test]
fn timeout_preserves_both_streams_and_stops_owned_descendant() -> Result<()> {
    let f = Fixture::new()?;
    let result = f.run("wait", Duration::from_millis(500), &AtomicBool::new(false));
    f.assert_failure(result, "工具执行超时")?;
    let log = fs::read_to_string(&f.log)?;
    assert!(log.contains("fixture stdout"));
    assert!(log.contains("fixture stderr"));
    Ok(())
}

#[test]
fn cancellation_preserves_log_and_stops_owned_descendant() -> Result<()> {
    let f = Fixture::new()?;
    let cancelled = AtomicBool::new(false);
    let (result, ready) = std::thread::scope(|scope| {
        let waiter = scope.spawn(|| {
            let deadline = Instant::now() + Duration::from_secs(5);
            let ready = f.temp.path().join("ready");
            while !ready.exists() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(10));
            }
            cancelled.store(true, Ordering::SeqCst);
            ready.exists()
        });
        let result = f.run("wait", Duration::from_secs(10), &cancelled);
        (result, waiter.join().unwrap())
    });
    assert!(ready, "Fixture must write both streams before cancellation");
    f.assert_failure(result, "工具操作已取消")?;
    let log = fs::read_to_string(&f.log)?;
    assert!(log.contains("fixture stdout"));
    assert!(log.contains("fixture stderr"));
    Ok(())
}

#[test]
fn output_limit_retains_bounded_tail_and_stops_owned_descendant() -> Result<()> {
    let f = Fixture::new()?;
    let result = f.run("overflow", Duration::from_secs(10), &AtomicBool::new(false));
    f.assert_failure(result, "工具输出超过限制")?;
    assert_eq!(fs::metadata(&f.log)?.len(), OUTPUT_LIMIT);
    let log = fs::read_to_string(&f.log)?;
    assert!(log.contains("overflow-evidence"));
    assert!(
        !log.contains("fixture stdout"),
        "Keep the tail, not the prefix"
    );
    Ok(())
}

#[test]
fn nonzero_exit_retains_output_without_changing_exit_contract() -> Result<()> {
    let f = Fixture::new()?;
    let output = run_cancellable_env_logged(
        Path::new("/bin/sh"),
        &[
            "-c",
            "printf '\\357\\273\\277stdout\\n'; printf 'stderr\\n' >&2; exit 7",
        ],
        None,
        Duration::from_secs(5),
        &AtomicBool::new(false),
        &Default::default(),
        &f.log,
    )?;
    assert_eq!(output.code, 7);
    assert_eq!(output.text, "stdout\nstderr\n");
    assert_eq!(
        fs::read_to_string(&f.log)?.trim_start_matches('\u{feff}'),
        output.text
    );
    let old = run(
        Path::new("/bin/sh"),
        &["-c", "printf old; exit 3"],
        None,
        Duration::from_secs(5),
    )?;
    assert_eq!(old.code, 3);
    assert_eq!(old.text, "old");
    Ok(())
}

#[test]
fn existing_evidence_is_never_overwritten() -> Result<()> {
    let f = Fixture::new()?;
    fs::write(&f.log, "existing evidence")?;
    let result = f.run("wait", Duration::from_secs(1), &AtomicBool::new(false));
    assert!(result.is_err());
    assert_eq!(fs::read_to_string(&f.log)?, "existing evidence");
    assert!(!f.temp.path().join("parent.pid").exists());
    Ok(())
}

#[test]
fn published_log_cannot_grow_through_descendant_capture_handle() -> Result<()> {
    let f = Fixture::new()?;
    let output = run_cancellable_env_logged(
        Path::new("/bin/sh"),
        &[
            "-c",
            r#"
printf 'root output\n'
sh -c 'printf "%s" "$$" > descendant.pid
remaining=300
while [ ! -f release ] && [ "$remaining" -gt 0 ]; do
    sleep 0.01
    remaining=$((remaining - 1))
done
[ -f release ] || exit 0
head -c 9437184 /dev/zero
printf "late stderr\n" >&2
printf done > done' &
exit 0
"#,
        ],
        Some(f.temp.path()),
        Duration::from_secs(5),
        &AtomicBool::new(false),
        &Default::default(),
        &f.log,
    )?;
    assert_eq!(output.code, 0);
    let published = fs::read(&f.log)?;
    assert_eq!(published, b"root output\n");
    // The short-lived descendant is released only after the root's result/log
    // were published. It exits by itself; never signal an already reaped PGID.
    fs::write(f.temp.path().join("release"), "")?;
    let deadline = Instant::now() + Duration::from_secs(5);
    while !f.temp.path().join("done").exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        f.temp.path().join("done").exists(),
        "Descendant must perform its late write"
    );
    assert_eq!(fs::read(&f.log)?, published);
    assert!(fs::metadata(&f.log)?.len() <= OUTPUT_LIMIT);
    let pid: i32 = fs::read_to_string(f.temp.path().join("descendant.pid"))?.parse()?;
    while running(pid) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(!running(pid), "Fixture descendant must exit by itself");
    Ok(())
}
