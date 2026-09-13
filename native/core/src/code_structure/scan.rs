use super::{
    check, is_source, measure, vendor, Baseline, Exceptions, Report, EXCEPTIONS_FILE,
    MAX_SOURCE_BYTES,
};
use crate::files::safe_path;
use anyhow::{ensure, Context, Result};
use std::{collections::BTreeMap, fs, io::Read, path::Path};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Repository,
    Game,
}

pub fn read(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= limit,
        "Code-structure input too large: {}",
        path.display()
    );
    Ok(bytes)
}

fn excluded(name: &str, scope: Scope, depth: usize) -> bool {
    [
        ".git",
        ".godot",
        ".beaver",
        ".beaver-context",
        "node_modules",
        "target",
        "release",
        "exports",
    ]
    .contains(&name)
        || (scope == Scope::Repository
            && depth == 0
            && ["output", "dist", "dist-native", ".codex", ".playwright-cli"].contains(&name))
}

fn visit(
    root: &Path,
    relative: &str,
    scope: Scope,
    paths: &mut Vec<String>,
    depth: usize,
) -> Result<()> {
    ensure!(depth < 64, "Source directory nesting exceeds 64");
    let directory = if relative.is_empty() {
        root.to_path_buf()
    } else {
        safe_path(root, relative)?
    };
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| anyhow::anyhow!("Non-UTF-8 source filename"))?;
        if excluded(&name, scope, depth) || name.starts_with(".beaver-write-") {
            continue;
        }
        let path = if relative.is_empty() {
            name
        } else {
            format!("{relative}/{name}")
        };
        let checked = safe_path(root, &path)?;
        let metadata = fs::symlink_metadata(&checked)?;
        if metadata.is_dir() {
            visit(root, &path, scope, paths, depth + 1)?;
        } else if metadata.is_file() && is_source(&path) {
            paths.push(path);
            ensure!(paths.len() <= 100_000, "Too many source files");
        }
    }
    Ok(())
}

pub fn scan(root: &Path, scope: Scope, baseline: &Baseline, strict: bool) -> Result<Report> {
    ensure!(root.is_dir(), "Source root is not a directory");
    let mut paths = Vec::new();
    visit(root, "", scope, &mut paths, 0)?;
    paths.sort();
    let mut sources = BTreeMap::new();
    for path in paths {
        let bytes = read(&safe_path(root, &path)?, MAX_SOURCE_BYTES)?;
        let stamp = measure(&path, &bytes)?;
        sources.insert(path.clone(), (stamp, vendor::immutable(&path, &bytes)));
    }
    let configuration = safe_path(root, EXCEPTIONS_FILE)?;
    let exceptions = if configuration.try_exists()? {
        Exceptions::parse(&read(&configuration, 64 * 1024)?)
            .with_context(|| format!("Invalid {EXCEPTIONS_FILE}"))?
    } else {
        Exceptions::default()
    };
    Ok(check(sources, baseline, &exceptions, strict))
}
