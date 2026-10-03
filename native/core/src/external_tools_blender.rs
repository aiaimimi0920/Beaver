use super::{
    files::{self, Output},
    process, Context,
};
use anyhow::{ensure, Context as _, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
    sync::atomic::AtomicBool,
    time::Duration,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Arguments {
    code: String,
    outputs: Vec<Output>,
    #[serde(default = "default_timeout")]
    timeout_seconds: u64,
}
fn default_timeout() -> u64 {
    300
}

pub(super) struct Prepared {
    pub directory: PathBuf,
    workspace: PathBuf,
    outputs: Vec<Output>,
    executable: PathBuf,
    executable_sha256: String,
    arguments: Vec<String>,
    timeout: Duration,
    script_sha256: String,
}

pub(super) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(super) fn identifier(value: &str) -> Result<()> {
    ensure!(
        !value.is_empty()
            && value.len() <= 128
            && value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
        "Invalid external run/request identifier"
    );
    Ok(())
}

pub(super) fn prepare(context: &Context, arguments: &Value) -> Result<Prepared> {
    identifier(&context.run_id)?;
    identifier(&context.request_id)?;
    let args: Arguments = serde_json::from_value(arguments.clone())?;
    ensure!(
        !args.code.is_empty() && args.code.len() as u64 <= files::TEXT_LIMIT,
        "Blender code must contain 1 byte to 1 MiB"
    );
    ensure!(
        (1..=3600).contains(&args.timeout_seconds),
        "Blender timeout must be 1 to 3600 seconds"
    );
    ensure!(
        !args.outputs.is_empty() && args.outputs.len() <= 64,
        "Declare 1 to 64 output files"
    );
    let workspace = files::workspace(&context.workspace)?;
    let mut seen = BTreeSet::new();
    for (index, output) in args.outputs.iter().enumerate() {
        ensure!(
            arguments["outputs"][index].get("expectedSha256").is_some(),
            "Each output requires expectedSha256 (null for a new file)"
        );
        ensure!(
            seen.insert(output.path.to_ascii_lowercase()),
            "Duplicate or case-aliased output"
        );
        files::verify(&workspace, output)?;
    }
    let executable = match &context.blender_path {
        Some(path) => crate::tools::find(
            "blender",
            path.to_str().context("Blender path is not UTF-8")?,
        )?,
        None => crate::tools::find("blender", "")?,
    };
    let metadata = fs::symlink_metadata(&executable)?;
    ensure!(
        metadata.is_file() && !crate::files::linked(&metadata),
        "Blender must be a regular native executable"
    );
    let executable_sha256 = crate::files::file_hash_limited(&executable, Some(files::ASSET_LIMIT))?
        .context("Blender executable missing")?;
    let relative = format!(
        ".beaver-context/external/{}/{}",
        context.run_id, context.request_id
    );
    let directory = crate::files::safe_path(&workspace, &relative)?;
    let parent = directory.parent().context("Missing job parent")?;
    fs::create_dir_all(parent)?;
    crate::files::safe_path(&workspace, &relative)?;
    // Receipt directories are single-use. A crashed/uncertain request is never replayed.
    fs::create_dir(&directory)
        .context("External request directory already exists or is unavailable")?;
    let staging = directory.join("output");
    fs::create_dir(&staging)?;
    let mut outputs = serde_json::Map::new();
    for output in &args.outputs {
        let target = crate::files::safe_path(&staging, &output.path)?;
        fs::create_dir_all(target.parent().unwrap())?;
        outputs.insert(output.path.clone(), json!(target));
    }
    fs::write(directory.join("script.py"), &args.code)?;
    fs::write(
        directory.join("runner.py"),
        include_bytes!("../../../resources/workflows/external_blender.py"),
    )?;
    fs::write(
        directory.join("context.json"),
        serde_json::to_vec(&json!({"workspace":workspace,"outputs":outputs}))?,
    )?;
    let command = vec![
        "--background".into(),
        "--factory-startup".into(),
        "--disable-autoexec".into(),
        "--python-exit-code".into(),
        "1".into(),
        "--python".into(),
        directory.join("runner.py").to_string_lossy().into_owned(),
        "--".into(),
        directory.to_string_lossy().into_owned(),
    ];
    Ok(Prepared {
        directory,
        workspace,
        outputs: args.outputs,
        executable,
        executable_sha256,
        arguments: command,
        timeout: Duration::from_secs(args.timeout_seconds),
        script_sha256: digest(args.code.as_bytes()),
    })
}

impl Prepared {
    pub(super) fn spawn(&self) -> Result<process::Owned> {
        ensure!(
            crate::files::file_hash_limited(&self.executable, Some(files::ASSET_LIMIT))?.as_ref()
                == Some(&self.executable_sha256),
            "Pinned Blender executable changed before spawn"
        );
        process::Owned::spawn(
            &self.executable,
            &self.arguments,
            &self.directory,
            &self.directory,
        )
    }
    pub(super) fn receipt(&self, pid: u32) -> Value {
        json!({"pid":pid,"command":{"executable":self.executable,"arguments":self.arguments},"executableSha256":self.executable_sha256,"scriptSha256":self.script_sha256,"workspace":self.workspace,"jobDirectory":self.directory,"logPath":self.directory.join("process.log"),"timeoutSeconds":self.timeout.as_secs(),"executionTrust":"trusted_python_not_os_sandbox","outputs":self.outputs})
    }
    pub(super) fn complete(
        self,
        mut child: process::Owned,
        cancelled: &AtomicBool,
    ) -> Result<Value> {
        let mut result = child.wait(&self.directory, self.timeout, cancelled)?;
        result["outputs"] = json!([]);
        result["log"] = log(&self.directory)?;
        if result["success"] != true {
            return Ok(result);
        }
        let finalize = (|| -> Result<()> {
            ensure!(
                !cancelled.load(std::sync::atomic::Ordering::SeqCst),
                "Job cancelled before publishing"
            );
            ensure!(
                self.directory.join("completed.json").is_file(),
                "Blender exited without completing the managed script"
            );
            ensure!(
                crate::files::file_hash_limited(&self.executable, Some(files::ASSET_LIMIT))?
                    .as_ref()
                    == Some(&self.executable_sha256),
                "Pinned Blender changed during execution"
            );
            let staging = self.directory.join("output");
            let actual = crate::files::list_files(&staging)?;
            let expected: BTreeSet<_> = self
                .outputs
                .iter()
                .map(|output| output.path.clone())
                .collect();
            ensure!(
                actual.into_iter().collect::<BTreeSet<_>>() == expected,
                "Staged output files do not match the declaration"
            );
            let mut total = 0_u64;
            for output in &self.outputs {
                files::verify(&self.workspace, output)?;
                let staged = files::path(&staging, &output.path)?;
                super::outputs::validate(&staged)?;
                total += fs::metadata(staged)?.len();
                ensure!(
                    total <= files::ASSET_LIMIT,
                    "Combined staged outputs exceed 512 MiB"
                );
            }
            for output in &self.outputs {
                ensure!(
                    !cancelled.load(std::sync::atomic::Ordering::SeqCst),
                    "Job cancelled during publication"
                );
                let staged = files::path(&staging, &output.path)?;
                let published = files::publish(&self.workspace, output, &staged)?;
                result["outputs"].as_array_mut().unwrap().push(published);
            }
            Ok(())
        })();
        if let Err(error) = finalize {
            result["success"] = json!(false);
            result["publicationError"] = json!(error.to_string());
            result["partialPublication"] = json!(!result["outputs"].as_array().unwrap().is_empty());
        }
        Ok(result)
    }
}

fn log(directory: &std::path::Path) -> Result<Value> {
    let path = directory.join("process.log");
    let mut file = fs::File::open(&path)?;
    let bytes = file.metadata()?.len();
    file.seek(SeekFrom::Start(bytes.saturating_sub(64 * 1024)))?;
    let mut tail = Vec::new();
    file.take(64 * 1024).read_to_end(&mut tail)?;
    Ok(
        json!({"path":path,"bytes":bytes,"tail":String::from_utf8_lossy(&tail),"tailTruncated":bytes>64*1024,"sha256":crate::files::file_hash_limited(&path, Some(process::LOG_LIMIT + 1024 * 1024)).ok().flatten()}),
    )
}
