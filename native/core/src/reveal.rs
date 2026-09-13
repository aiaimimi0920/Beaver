use crate::{files::safe_path, store::Store};
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};

pub struct Target {
    pub path: PathBuf,
    pub select: bool,
}

pub fn resolve(store: &Store, method: &str, input: &Value) -> Result<Target> {
    let id = input["id"].as_str().context("缺少项目或任务标识")?;
    uuid::Uuid::parse_str(id).context("项目或任务标识无效")?;
    let (kind, field, select) = match method {
        "project.reveal" => ("project", "path", false),
        "task.reveal" => ("task", "workspace", false),
        "asset.reveal" => ("project", "path", true),
        _ => bail!("未知文件显示操作"),
    };
    let record: Value = store.get(kind, id)?.context("项目或任务不存在")?;
    let root = Path::new(record[field].as_str().context("目录记录无效")?);
    let path = if select {
        let relative = input["path"]
            .as_str()
            .filter(|path| !path.is_empty() && path.encode_utf16().count() <= 2000)
            .context("素材路径无效")?;
        safe_path(root, relative)?
    } else {
        std::fs::canonicalize(root)?
    };
    let metadata = std::fs::metadata(&path).context("目标文件或目录已不存在")?;
    if !select && !metadata.is_dir() {
        bail!("目标不是目录");
    }
    Ok(Target { path, select })
}

// Explorer accepts normal drive/UNC paths, not Rust's extended-length prefix.
pub fn windows_path(path: &Path) -> Result<String> {
    let path = path.to_str().context("路径不是有效 UTF-8")?;
    Ok(if let Some(unc) = path.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{unc}")
    } else {
        path.strip_prefix(r"\\?\").unwrap_or(path).to_owned()
    })
}

pub fn open(target: &Target) -> Result<()> {
    #[cfg(windows)]
    let mut command = {
        use std::os::windows::process::CommandExt;
        let windows = std::env::var_os("SystemRoot").context("无法定位系统目录")?;
        let mut command = std::process::Command::new(PathBuf::from(windows).join("explorer.exe"));
        let path = windows_path(&target.path)?;
        command.arg(if target.select {
            format!("/select,{path}")
        } else {
            path
        });
        command.creation_flags(0x08000000);
        command
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = std::process::Command::new("/usr/bin/open");
        if target.select {
            command.arg("-R");
        }
        command.arg(&target.path);
        command
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        if target.select {
            let uri = url::Url::from_file_path(&target.path)
                .map_err(|_| anyhow::anyhow!("素材 URI 无效"))?;
            let mut command = std::process::Command::new("dbus-send");
            command
                .args([
                    "--session",
                    "--type=method_call",
                    "--dest=org.freedesktop.FileManager1",
                    "/org/freedesktop/FileManager1",
                    "org.freedesktop.FileManager1.ShowItems",
                ])
                .arg(format!("array:string:{uri}"))
                .arg("string:");
            command
        } else {
            let mut command = std::process::Command::new("xdg-open");
            command.arg(&target.path);
            command
        }
    };
    // Launch only the system file manager; never execute the selected asset.
    let mut child = command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("无法打开系统文件管理器")?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn only_registered_project_and_task_targets_are_revealed() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let store = Store::open(&temp.path().join("data"))?;
        let project = temp.path().join("project");
        let workspace = temp.path().join("workspace");
        std::fs::create_dir(&project)?;
        std::fs::create_dir(&workspace)?;
        std::fs::write(project.join("角色.txt"), b"fixture")?;
        let id = uuid::Uuid::new_v4().to_string();
        store.put("project", &id, &json!({"path":project}))?;
        store.put("task", &id, &json!({"workspace":workspace}))?;
        assert_eq!(
            resolve(&store, "project.reveal", &json!({"id":id}))?.path,
            std::fs::canonicalize(&project)?
        );
        assert_eq!(
            resolve(&store, "task.reveal", &json!({"id":id}))?.path,
            std::fs::canonicalize(&workspace)?
        );
        let asset = resolve(&store, "asset.reveal", &json!({"id":id,"path":"角色.txt"}))?;
        assert!(asset.select);
        assert_eq!(
            asset.path,
            std::fs::canonicalize(&project)?.join("角色.txt")
        );
        for relative in ["../outside", "C:/Windows", "a\\b", "missing", ""] {
            assert!(resolve(&store, "asset.reveal", &json!({"id":id,"path":relative})).is_err());
        }
        assert!(resolve(&store, "project.reveal", &json!({"id":"invalid"})).is_err());
        assert_eq!(
            windows_path(Path::new(r"\\?\C:\game\角色.txt"))?,
            r"C:\game\角色.txt"
        );
        assert_eq!(
            windows_path(Path::new(r"\\?\UNC\server\share\game"))?,
            r"\\server\share\game"
        );
        Ok(())
    }
}
