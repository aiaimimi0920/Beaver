use anyhow::{bail, Context, Result};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

pub struct Plan {
    pub executable: PathBuf,
    pub args: Vec<String>,
    pub timeout: Duration,
}

pub fn plan(name: &str, data: &Path, configured_node: &str) -> Result<Plan> {
    if !cfg!(windows) {
        bail!("当前自动安装仅验证 Windows。请安装工具后在设置中选择其可执行文件。");
    }
    if name == "codex" {
        let node = crate::tools::find("node", configured_node)?;
        let npm = node
            .parent()
            .context("Node 目录无效")?
            .join("node_modules/npm/bin/npm-cli.js");
        if !npm.is_file() {
            bail!("未找到 Node 配套 npm，请安装完整 Node.js 后重试");
        }
        return Ok(Plan {
            executable: node,
            args: vec![
                crate::reveal::windows_path(&npm)?,
                "install".into(),
                "--prefix".into(),
                crate::reveal::windows_path(&data.join("tools"))?,
                "@openai/codex".into(),
            ],
            timeout: Duration::from_secs(300),
        });
    }
    let id = match name {
        "node" => "OpenJS.NodeJS.LTS",
        "godot" => "GodotEngine.GodotEngine",
        "blender" => "BlenderFoundation.Blender",
        _ => bail!("未知工具类型"),
    };
    Ok(Plan {
        executable: PathBuf::from("winget.exe"),
        args: [
            "install",
            "--id",
            id,
            "--exact",
            "--accept-package-agreements",
            "--accept-source-agreements",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        timeout: Duration::from_secs(600),
    })
}

pub fn install(
    name: &str,
    data: &Path,
    configured_node: &str,
    cancelled: &AtomicBool,
) -> Result<()> {
    if cancelled.load(Ordering::SeqCst) {
        bail!("工具操作已取消");
    }
    let plan = plan(name, data, configured_node)?;
    let args: Vec<_> = plan.args.iter().map(String::as_str).collect();
    let output =
        crate::process::run_cancellable(&plan.executable, &args, None, plan.timeout, cancelled)?;
    if output.code != 0 {
        bail!("{}", output.text);
    }
    if name == "codex" && crate::tools::native_codex(&data.join("tools")).is_none() {
        bail!("Codex 安装包中未找到当前架构的程序，未写入无效配置");
    }
    // Caller must re-detect and validate before persisting a tool path.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancellation_happens_before_discovery_or_process_launch() {
        assert!(install(
            "codex",
            Path::new("missing-data"),
            "missing-node",
            &AtomicBool::new(true)
        )
        .is_err());
        assert!(crate::tools::detect_one("node", "", &[], &AtomicBool::new(true)).is_err());
        assert!(crate::tools::detect_one("invalid", "", &[], &AtomicBool::new(false)).is_err());
    }
    #[test]
    #[cfg(windows)]
    fn installer_arguments_are_fixed_and_npm_paths_remain_separate() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("space & quote' directory");
        std::fs::create_dir_all(root.join("node_modules/npm/bin"))?;
        std::fs::write(root.join("node.exe"), b"fixture")?;
        std::fs::write(root.join("node_modules/npm/bin/npm-cli.js"), b"fixture")?;
        let node = root.join("node.exe");
        let p = plan("codex", &root, node.to_str().unwrap())?;
        assert_eq!(p.executable, std::fs::canonicalize(node)?);
        assert_eq!(p.args.len(), 5);
        assert!(!p.args[0].starts_with(r"\\?\"));
        assert_eq!(
            Path::new(&p.args[0]),
            root.join("node_modules/npm/bin/npm-cli.js")
        );
        assert_eq!(p.args[3], root.join("tools").to_string_lossy());
        assert_eq!(p.args[4], "@openai/codex");
        assert_eq!(p.timeout, Duration::from_secs(300));
        for (name, id) in [
            ("node", "OpenJS.NodeJS.LTS"),
            ("godot", "GodotEngine.GodotEngine"),
            ("blender", "BlenderFoundation.Blender"),
        ] {
            let p = plan(name, &root, "")?;
            assert_eq!(p.args[2], id);
            assert_eq!(p.timeout, Duration::from_secs(600));
        }
        assert!(plan("unknown", &root, "").is_err());
        Ok(())
    }
}
