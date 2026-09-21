use anyhow::{Context, Result};
use beaver_core::project_storage::ProjectStore;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

pub fn project(parent: &Path, name: &str, id: &str) -> Result<PathBuf> {
    let root = parent.join(name);
    fs::create_dir(&root)?;
    fs::write(root.join("project.godot"), "config_version=5\n")?;
    drop(ProjectStore::initialize(&root, id)?);
    Ok(root)
}

pub fn tree(root: &Path) -> Result<BTreeMap<PathBuf, Option<Vec<u8>>>> {
    fn visit(
        root: &Path,
        directory: &Path,
        result: &mut BTreeMap<PathBuf, Option<Vec<u8>>>,
    ) -> Result<()> {
        for entry in fs::read_dir(directory)? {
            let path = entry?.path();
            let relative = path.strip_prefix(root)?.to_owned();
            if path.is_dir() {
                result.insert(relative, None);
                visit(root, &path, result)?;
            } else {
                // Windows rejects reads even beyond EOF while the empty lock file is owned.
                let bytes = if path.file_name().is_some_and(|name| name == ".project.lock")
                    && fs::metadata(&path)?.len() == 0
                {
                    Vec::new()
                } else {
                    fs::read(&path)
                        .with_context(|| format!("read fixture file {}", path.display()))?
                };
                result.insert(relative, Some(bytes));
            }
        }
        Ok(())
    }
    let mut result = BTreeMap::new();
    visit(root, root, &mut result)?;
    Ok(result)
}

pub fn copy(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir(destination)?;
    for (relative, bytes) in tree(source)? {
        match bytes {
            Some(bytes) => fs::write(destination.join(relative), bytes)?,
            None => fs::create_dir_all(destination.join(relative))?,
        }
    }
    Ok(())
}
