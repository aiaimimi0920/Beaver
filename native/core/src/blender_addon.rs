//! Locate an already-installed addon without changing the user's Blender setup.
use anyhow::{Context, Result};
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

fn configuration_root(
    platform: &str,
    variable: impl Fn(&str) -> Option<OsString>,
) -> Result<PathBuf> {
    let nonempty = |name: &str| variable(name).filter(|value| !value.is_empty());
    match platform {
        "windows" => Ok(PathBuf::from(
            nonempty("APPDATA").context("Blender addon discovery requires APPDATA")?,
        )
        .join("Blender Foundation/Blender")),
        "macos" => Ok(PathBuf::from(
            nonempty("HOME").context("Blender addon discovery requires HOME")?,
        )
        .join("Library/Application Support/Blender")),
        _ => match nonempty("XDG_CONFIG_HOME") {
            Some(root) => {
                let root = PathBuf::from(root);
                anyhow::ensure!(root.is_absolute(), "XDG_CONFIG_HOME must be absolute");
                Ok(root.join("blender"))
            }
            None => Ok(PathBuf::from(
                nonempty("HOME").context("Blender addon discovery requires HOME")?,
            )
            .join(".config/blender")),
        },
    }
}

fn discover(root: &Path) -> Result<PathBuf> {
    let mut candidates = Vec::new();
    if root.is_dir() {
        for entry in fs::read_dir(root)? {
            let entry = entry?;
            let version = entry
                .file_name()
                .to_string_lossy()
                .split('.')
                .map(str::parse::<u32>)
                .collect::<std::result::Result<Vec<_>, _>>();
            let path = entry.path().join("scripts/addons/blender_mcp.py");
            if let Ok(version) = version {
                if path.is_file() {
                    candidates.push((version, path));
                }
            }
        }
    }
    candidates.sort_by(|a, b| b.0.cmp(&a.0));
    candidates
        .into_iter()
        .next()
        .map(|(_, path)| path)
        .context("Install the Blender MCP addon before starting NPR production")
}

pub(super) fn installed() -> Result<PathBuf> {
    discover(&configuration_root(std::env::consts::OS, |key| {
        std::env::var_os(key)
    })?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn platform_roots_are_explicit_and_need_no_windows_environment() -> Result<()> {
        let vars = BTreeMap::from([
            ("APPDATA", OsString::from("C:/Users/test/AppData/Roaming")),
            ("HOME", OsString::from("/home/test")),
            ("XDG_CONFIG_HOME", OsString::from("/isolated/config")),
        ]);
        let lookup = |key: &str| vars.get(key).cloned();
        assert_eq!(
            configuration_root("linux", lookup)?,
            PathBuf::from("/isolated/config/blender")
        );
        assert_eq!(
            configuration_root("macos", lookup)?,
            PathBuf::from("/home/test/Library/Application Support/Blender")
        );
        assert_eq!(
            configuration_root("windows", lookup)?,
            PathBuf::from("C:/Users/test/AppData/Roaming/Blender Foundation/Blender")
        );
        assert_eq!(
            configuration_root("linux", |key| (key == "HOME").then(|| "/home/test".into()))?,
            PathBuf::from("/home/test/.config/blender")
        );
        assert!(configuration_root("windows", |_| None).is_err());
        assert!(configuration_root("linux", |key| (key == "XDG_CONFIG_HOME")
            .then(|| "relative".into()))
        .is_err());
        Ok(())
    }

    #[test]
    fn discovery_requires_real_addon_and_sorts_versions_numerically() -> Result<()> {
        let root = tempfile::tempdir()?;
        assert!(discover(root.path()).is_err());
        for version in ["4.9", "4.10", "not-a-version", "5.0"] {
            let folder = root.path().join(version).join("scripts/addons");
            fs::create_dir_all(&folder)?;
            if version != "5.0" {
                fs::write(folder.join("blender_mcp.py"), "# installed addon fixture")?;
            }
        }
        assert_eq!(
            discover(root.path())?,
            root.path().join("4.10/scripts/addons/blender_mcp.py")
        );
        Ok(())
    }
}
