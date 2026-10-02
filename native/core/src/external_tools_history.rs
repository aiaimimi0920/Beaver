//! Supplemental file-backed history. Never use this module to prove process
//! termination, authorize recovery, replace a durable admission or replay work.
use super::{blender, files};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::{fs, io::Read, path::Path};

const REPORT_LIMIT: u64 = 512 * 1024;

/// Both arguments must come from the host's scope-checked task/SQLite records.
/// `durable_start` is the persisted Blender start progress, not request arguments.
/// Recorded paths and PIDs are display-only; the only file opened is derived here.
pub fn historical_job_evidence(workspace: &Path, durable_start: &Value) -> Result<Value> {
    let root = files::workspace(workspace)?;
    let run = durable_start["runId"]
        .as_str()
        .context("Durable job run missing")?;
    let request = durable_start["requestId"]
        .as_str()
        .context("Durable job request missing")?;
    let job = durable_start["jobId"]
        .as_str()
        .context("Durable job identity missing")?;
    blender::identifier(run)?;
    blender::identifier(request)?;
    uuid::Uuid::parse_str(job).context("Invalid durable job identity")?;
    ensure!(
        durable_start["process"].is_object(),
        "Durable job process identity missing"
    );
    for field in ["scriptSha256", "executableSha256"] {
        ensure!(
            files::expected(&durable_start["process"][field])?.is_some(),
            "Durable job hash missing"
        );
    }
    let relative = format!(".beaver-context/external/{run}/{request}/job.json");
    let path = crate::files::safe_path(&root, &relative)?;
    require_file(&fs::symlink_metadata(&path)?)?;
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x00200000); // FILE_FLAG_OPEN_REPARSE_POINT
    }
    let file = options.open(&path)?;
    require_file(&file.metadata()?)?;
    let mut bytes = Vec::new();
    file.take(REPORT_LIMIT + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= REPORT_LIMIT,
        "Historical job report exceeds 512 KiB"
    );
    let report: Value =
        serde_json::from_slice(&bytes).context("Invalid historical job report JSON")?;
    ensure!(
        report.is_object(),
        "Historical job report must be an object"
    );
    for field in ["runId", "requestId", "jobId", "process"] {
        ensure!(
            report[field] == durable_start[field],
            "Historical job report identity mismatch: {field}"
        );
    }
    ensure!(
        matches!(
            report["status"].as_str(),
            Some("running" | "succeeded" | "failed" | "cancelled" | "timed_out" | "unknown")
        ),
        "Invalid historical job status"
    );
    if let Some(outputs) = report["result"].get("outputs") {
        let outputs = outputs
            .as_array()
            .context("Invalid historical output list")?;
        ensure!(outputs.len() <= 64, "Historical output list exceeds limit");
        for output in outputs {
            let relative = output["path"]
                .as_str()
                .context("Historical output path missing")?;
            files::path(&root, relative)?;
            ensure!(
                files::expected(&output["sha256"])?.is_some(),
                "Historical output hash missing"
            );
        }
    }
    Ok(json!({
        "source":"task-job-receipt", "historicalEvidenceOnly":true,
        "establishesCleanup":false, "authorizesRecovery":false,
        "outputHashesAreRecorded":true, "reportSha256":blender::digest(&bytes),
        "report":report
    }))
}

fn require_file(metadata: &fs::Metadata) -> Result<()> {
    ensure!(
        metadata.is_file() && !crate::files::linked(metadata),
        "Historical job report must be a regular, unlinked file"
    );
    ensure!(
        metadata.len() > 0 && metadata.len() <= REPORT_LIMIT,
        "Historical job report is empty or oversized"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        ensure!(
            metadata.nlink() == 1,
            "Hard-linked historical job report rejected"
        );
    }
    Ok(())
}

#[cfg(test)]
#[path = "external_tools_history_tests.rs"]
mod tests;
