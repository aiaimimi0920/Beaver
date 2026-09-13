mod language;
mod lexer;
mod policy;
mod scan;
mod strings;
mod task;
mod vendor;

pub mod cli;
pub mod tool;
pub use policy::{check, Baseline, Entry, Exception, Exceptions, Report, Stamp};
pub use scan::{read, scan, Scope};
pub use task::{check_changes, snapshot_baseline};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

pub const INSTRUCTIONS: &str = include_str!("../../../../resources/instructions/code-structure.md");
pub const EXCEPTIONS_FILE: &str = ".beaver-code-structure.json";
pub const BASELINE_ENV: &str = "BEAVER_CODE_BASELINE";
pub const MAX_SOURCE_BYTES: u64 = 8 * 1024 * 1024;

pub fn is_source(path: &str) -> bool {
    language::Language::of(path).is_some()
}

pub fn measure(path: &str, bytes: &[u8]) -> Result<Stamp> {
    let language = language::Language::of(path).context("Unsupported source extension")?;
    let source = std::str::from_utf8(bytes).with_context(|| format!("Invalid UTF-8: {path}"))?;
    let canonical = source.replace("\r\n", "\n").replace('\r', "\n");
    Ok(Stamp {
        effective_lines: lexer::count(&canonical, language),
        sha256: format!("{:x}", Sha256::digest(canonical.as_bytes())),
    })
}

#[cfg(test)]
mod tests;
