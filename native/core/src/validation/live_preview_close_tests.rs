use super::*;
use std::{sync::mpsc, thread, time::Duration};

fn session(id: &str, worker: JoinHandle<()>) -> Session {
    Session {
        id: id.into(),
        project: "p".into(),
        run: "run".into(),
        request: "request".into(),
        snapshot: "snapshot".into(),
        stop: Arc::new(AtomicBool::new(false)),
        view: Arc::new(Mutex::new(View {
            touched: Instant::now(),
            revision: 0,
            frozen: false,
            camera: Camera::default(),
            width: 960,
            height: 540,
            status: "ready".into(),
            error: None,
            frame: Value::Null,
            pick_request: Value::Null,
        })),
        worker: Some(worker),
        close_error: None,
    }
}

fn wait_finished(sessions: &Sessions) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if sessions
            .0
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .worker
            .as_ref()
            .unwrap()
            .is_finished()
        {
            return;
        }
        assert!(Instant::now() < deadline, "test worker did not finish");
        thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn close_acknowledges_only_after_worker_finishes_and_joins() -> Result<()> {
    let (finish, finishing) = mpsc::channel();
    let sessions = Sessions(Mutex::new(Some(session(
        "s",
        thread::spawn(move || {
            finishing.recv().unwrap();
        }),
    ))));
    let stop = sessions.0.lock().unwrap().as_ref().unwrap().stop.clone();
    let pending = sessions.close("p", "s")?;
    assert_eq!(
        pending,
        json!({"projectId":"p","sessionId":"s","closed":false})
    );
    assert!(stop.load(Ordering::SeqCst));
    finish.send(())?;
    wait_finished(&sessions);
    let closed = sessions.close("p", "s")?;
    assert_eq!(
        closed,
        json!({"projectId":"p","sessionId":"s","closed":true})
    );
    assert!(sessions
        .0
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .worker
        .is_none());
    assert_eq!(sessions.close("p", "s")?, closed);
    Ok(())
}

#[test]
fn wrong_project_does_not_stop_the_worker() -> Result<()> {
    let (finish, finishing) = mpsc::channel();
    let sessions = Sessions(Mutex::new(Some(session(
        "s",
        thread::spawn(move || {
            let _ = finishing.recv();
        }),
    ))));
    let stop = sessions.0.lock().unwrap().as_ref().unwrap().stop.clone();
    assert_eq!(
        sessions.close("other", "s").unwrap_err().to_string(),
        "PREVIEW_SESSION_MISMATCH"
    );
    assert!(!stop.load(Ordering::SeqCst));
    finish.send(())?;
    sessions.shutdown()
}

#[test]
fn stale_retry_cannot_stop_a_replacement_session() -> Result<()> {
    let (finish, finishing) = mpsc::channel();
    let sessions = Sessions(Mutex::new(Some(session(
        "replacement",
        thread::spawn(move || {
            let _ = finishing.recv();
        }),
    ))));
    let stop = sessions.0.lock().unwrap().as_ref().unwrap().stop.clone();
    assert_eq!(
        sessions.close("p", "old")?,
        json!({"projectId":"p","sessionId":"old","closed":true})
    );
    assert!(!stop.load(Ordering::SeqCst));
    assert_eq!(
        sessions.0.lock().unwrap().as_ref().unwrap().id,
        "replacement"
    );
    finish.send(())?;
    sessions.shutdown()
}

#[test]
fn missing_session_close_is_idempotent() -> Result<()> {
    let sessions = Sessions::default();
    let expected = json!({"projectId":"p","sessionId":"absent","closed":true});
    assert_eq!(sessions.close("p", "absent")?, expected);
    assert_eq!(sessions.close("p", "absent")?, expected);
    assert!(sessions.0.lock().unwrap().is_none());
    Ok(())
}

#[test]
fn worker_panic_remains_a_failure_on_retry_and_shutdown() {
    let sessions = Sessions(Mutex::new(Some(session(
        "s",
        thread::spawn(|| {
            panic!("simulated preview worker failure");
        }),
    ))));
    wait_finished(&sessions);
    for _ in 0..2 {
        assert_eq!(
            sessions.close("p", "s").unwrap_err().to_string(),
            "PREVIEW_WORKER_FAILED"
        );
    }
    assert!(sessions
        .0
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .worker
        .is_none());
    assert_eq!(
        sessions.shutdown().unwrap_err().to_string(),
        "PREVIEW_WORKER_FAILED"
    );
}
