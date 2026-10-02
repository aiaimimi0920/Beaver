use super::*;
use std::path::PathBuf;

fn fixture() -> Result<(tempfile::TempDir, Value, PathBuf)> {
    let directory = tempfile::tempdir()?;
    let run = uuid::Uuid::new_v4().to_string();
    let request = uuid::Uuid::new_v4().to_string();
    let path = directory
        .path()
        .join(format!(".beaver-context/external/{run}/{request}/job.json"));
    fs::create_dir_all(path.parent().unwrap())?;
    let start = json!({
        "jobId":uuid::Uuid::new_v4().to_string(),"runId":run,"requestId":request,"status":"running",
        "process":{"pid":u32::MAX,"jobDirectory":"/recorded/path/not-used-for-reading",
            "command":{"executable":"/must-not-execute"},"scriptSha256":"a".repeat(64),
            "executableSha256":"b".repeat(64)}
    });
    let mut report = start.clone();
    report["status"] = json!("succeeded");
    report["result"] = json!({"success":true,"exitCode":0,"ownedTreeCleanup":"completed",
        "outputs":[{"path":"model.glb","sha256":"c".repeat(64),"bytes":12}]});
    fs::write(&path, serde_json::to_vec(&report)?)?;
    Ok((directory, start, path))
}

#[test]
fn persisted_terminal_job_is_visible_without_poll_or_live_registry() -> Result<()> {
    let (directory, start, path) = fixture()?;
    let before = fs::read(&path)?;
    let evidence = historical_job_evidence(directory.path(), &start)?;
    assert_eq!(evidence["report"]["status"], "succeeded");
    assert_eq!(
        evidence["report"]["result"]["outputs"][0]["sha256"],
        "c".repeat(64)
    );
    assert_eq!(evidence["historicalEvidenceOnly"], true);
    assert_eq!(evidence["outputHashesAreRecorded"], true);
    assert_eq!(evidence["establishesCleanup"], false);
    assert_eq!(evidence["authorizesRecovery"], false);
    assert_eq!(evidence["reportSha256"], blender::digest(&before));
    assert_eq!(fs::read(path)?, before);
    assert!(!directory.path().join("model.glb").exists());
    Ok(())
}

#[test]
fn identity_mismatch_and_caller_path_substitution_are_rejected() -> Result<()> {
    let (directory, start, path) = fixture()?;
    for field in ["runId", "requestId", "jobId"] {
        let mut report: Value = serde_json::from_slice(&fs::read(&path)?)?;
        report[field] = json!(uuid::Uuid::new_v4().to_string());
        fs::write(&path, serde_json::to_vec(&report)?)?;
        assert!(historical_job_evidence(directory.path(), &start).is_err());
        report[field] = start[field].clone();
        fs::write(&path, serde_json::to_vec(&report)?)?;
    }
    let mut bad = start.clone();
    bad["runId"] = json!("../outside");
    assert!(historical_job_evidence(directory.path(), &bad).is_err());
    bad = start.clone();
    bad["process"]["scriptSha256"] = json!("d".repeat(64));
    assert!(historical_job_evidence(directory.path(), &bad).is_err());
    bad = start.clone();
    bad["process"]["jobDirectory"] = json!(path.parent().unwrap().join("another"));
    assert!(historical_job_evidence(directory.path(), &bad).is_err());
    Ok(())
}

#[test]
fn malformed_and_oversized_reports_are_rejected_read_only() -> Result<()> {
    let (directory, start, path) = fixture()?;
    fs::write(&path, vec![b' '; REPORT_LIMIT as usize + 1])?;
    assert!(historical_job_evidence(directory.path(), &start).is_err());
    assert_eq!(fs::metadata(&path)?.len(), REPORT_LIMIT + 1);
    fs::write(&path, b"[]")?;
    assert!(historical_job_evidence(directory.path(), &start).is_err());
    fs::remove_file(&path)?;
    fs::create_dir(&path)?;
    assert!(historical_job_evidence(directory.path(), &start).is_err());
    Ok(())
}

#[cfg(unix)]
#[test]
fn linked_reports_and_linked_job_directories_are_rejected() -> Result<()> {
    let (directory, start, path) = fixture()?;
    let outside = tempfile::tempdir()?;
    let other = outside.path().join("report.json");
    fs::rename(&path, &other)?;
    std::os::unix::fs::symlink(&other, &path)?;
    assert!(historical_job_evidence(directory.path(), &start).is_err());
    fs::remove_file(&path)?;
    fs::hard_link(&other, &path)?;
    assert!(historical_job_evidence(directory.path(), &start).is_err());
    fs::remove_file(&path)?;
    let job_directory = path.parent().unwrap();
    fs::remove_dir(job_directory)?;
    std::os::unix::fs::symlink(outside.path(), job_directory)?;
    assert!(historical_job_evidence(directory.path(), &start).is_err());
    Ok(())
}
