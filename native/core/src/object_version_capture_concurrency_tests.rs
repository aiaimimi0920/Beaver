use super::*;
use crate::{
    object_catalog::ObjectFile,
    object_catalog_test_fixture::{capture_request, update_request, Fixture},
};
use std::{sync::mpsc, time::Duration};

fn begin(f: &Fixture, request: &CaptureRequest) -> Result<(Command, ObjectRecord)> {
    let command = Command::new(
        &f.runtime,
        &request.project_id,
        &request.request_id,
        "capture",
        request,
    )?;
    let Start::Capture(object) = start(&f.runtime, request, &command)? else {
        panic!("unexpected replay")
    };
    Ok((command, object))
}

#[test]
fn same_capture_race_replays_but_different_request_loses_compare_and_swap() -> Result<()> {
    let f = Fixture::new()?;
    let object = object_registration::register(&f.runtime, &f.request("register"))?.object;
    let request = capture_request(&object, "capture");
    let other_request = capture_request(&object, "other-capture");
    let (command, baseline) = begin(&f, &request)?;
    let (retry, retry_baseline) = begin(&f, &request)?;
    let (other, other_baseline) = begin(&f, &other_request)?;
    let version = freeze(&f.runtime, &baseline)?;
    let retry_version = freeze(&f.runtime, &retry_baseline)?;
    let other_version = freeze(&f.runtime, &other_baseline)?;
    let result = finish(&f.runtime, &request, &command, &baseline, version)?;
    assert_eq!(
        finish(&f.runtime, &request, &retry, &retry_baseline, retry_version)?,
        result
    );
    assert!(finish(
        &f.runtime,
        &other_request,
        &other,
        &other_baseline,
        other_version
    )
    .unwrap_err()
    .to_string()
    .contains("OBJECT_REVISION_CONFLICT"));
    assert_eq!(f.count("object_version")?, 1);
    assert_eq!(f.count("object_command_receipt")?, 2);
    Ok(())
}

#[test]
fn metadata_change_during_file_capture_rolls_back_the_version_commit() -> Result<()> {
    let f = Fixture::new()?;
    let mut request = f.request("register");
    request.files.push(ObjectFile {
        path: "hero.gd".into(),
        role: "script".into(),
    });
    fs::write(f.temp.path().join("hero.gd"), "extends Node\n")?;
    let object = object_registration::register(&f.runtime, &request)?.object;
    let request = capture_request(&object, "capture");
    let (command, baseline) = begin(&f, &request)?;
    let store = f.runtime.store();
    let guard = store.lock().unwrap();
    let runtime = f.runtime.clone();
    let copy = baseline.clone();
    let (sender, receiver) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        sender.send(freeze(&runtime, &copy)).unwrap();
    });
    let frozen = receiver.recv_timeout(Duration::from_secs(10));
    drop(guard);
    worker.join().unwrap();
    let frozen = frozen.context("capture held a Store lock during file IO")??;
    let mut edit = update_request(&object, "edit");
    edit.name = "Concurrent edit".into();
    let changed = object_registration::update(&f.runtime, &edit)?.object;
    assert!(finish(&f.runtime, &request, &command, &baseline, frozen)
        .unwrap_err()
        .to_string()
        .contains("OBJECT_REVISION_CONFLICT"));
    assert_eq!(f.object(&object.id)?, changed);
    assert_eq!(f.count("object_version")?, 0);
    assert_eq!(f.count("object_command_receipt")?, 2);
    Ok(())
}
