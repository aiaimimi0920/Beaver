use crate::{
    files::{list_files, safe_path, Files},
    journal::{Journal, OperationKind},
    store::Store,
};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

pub fn project_path(store: &Store, id: &str) -> Result<PathBuf> {
    let project: Value = store.get("project", id)?.context("项目不存在")?;
    Ok(PathBuf::from(
        project["path"].as_str().context("项目路径无效")?,
    ))
}
fn document_path(relative: &str) -> Result<()> {
    if !relative.ends_with(".md")
        || relative.split('/').any(|part| {
            part.starts_with('.')
                || ["node_modules", "target", "release", "exports"]
                    .contains(&part.to_lowercase().as_str())
        })
    {
        bail!("资料仅支持项目内的 Markdown 文件");
    }
    Ok(())
}
fn read_limited(file: &Path) -> Result<Vec<u8>> {
    let handle = fs::File::open(file)?;
    if handle.metadata()?.len() > 512000 {
        bail!("资料超过 500 KB");
    }
    let mut bytes = Vec::new();
    handle.take(512001).read_to_end(&mut bytes)?;
    if bytes.len() > 512000 {
        bail!("资料超过 500 KB");
    }
    Ok(bytes)
}
pub fn read_document(root: &Path, relative: &str) -> Result<Value> {
    use sha2::{Digest, Sha256};
    document_path(relative)?;
    let bytes = read_limited(&safe_path(root, relative)?)?;
    let text = std::str::from_utf8(&bytes)?
        .strip_prefix('\u{feff}')
        .unwrap_or(std::str::from_utf8(&bytes)?);
    Ok(json!({"path":relative,"text":text,"revision":format!("{:x}",Sha256::digest(&bytes))}))
}
pub fn text(root: &Path, relative: &str) -> Result<String> {
    Ok(String::from_utf8_lossy(&read_limited(&safe_path(root, relative)?)?).into_owned())
}
pub fn assets(root: &Path) -> Result<Vec<Value>> {
    let mut result = Vec::new();
    for relative in list_files(root)? {
        let stat = fs::metadata(safe_path(root, &relative)?)?;
        let extension = Path::new(&relative)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        let kind = match extension.as_str() {
            "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "svg" => "image",
            "wav" | "ogg" | "mp3" | "flac" | "m4a" => "audio",
            "glb" | "gltf" | "obj" | "blend" => "model",
            "tscn" | "scn" => "scene",
            "gd" | "json" | "txt" | "md" | "csv" | "po" | "pot" | "cfg" | "godot" | "tres" => {
                "text"
            }
            _ => "other",
        };
        let modified: chrono::DateTime<chrono::Utc> = stat.modified()?.into();
        result.push(json!({"path":relative,"bytes":stat.len(),"kind":kind,"modifiedAt":modified.to_rfc3339_opts(chrono::SecondsFormat::Millis,true)}));
    }
    Ok(result)
}

/// Caller must hold exclusive project-write ownership for this entire operation.
pub fn save_document(
    store: &mut Store,
    data: &Path,
    project_id: &str,
    relative: &str,
    text: &str,
    revision: Option<&str>,
) -> Result<()> {
    document_path(relative)?;
    if text.len() > 512000 {
        bail!("资料超过 500 KB");
    }
    let files = Files::new(data.to_path_buf());
    if Journal::new(store, &files).blocked(project_id)? {
        bail!("项目有尚未恢复的文件操作，禁止继续写入");
    }
    let root = project_path(store, project_id)?;
    safe_path(&root, relative)?;
    let baseline = files.capture(&root)?;
    if baseline.get(relative).map(String::as_str) != revision {
        bail!("资料已被其他任务或编辑器修改；草稿已保留，请重新读取后整合");
    }
    let id = uuid::Uuid::new_v4().to_string();
    let workspace = data.join("workspaces").join(&id);
    files.restore_copy(&baseline, &workspace)?;
    let file = safe_path(&workspace, relative)?;
    fs::create_dir_all(file.parent().context("没有父目录")?)?;
    fs::write(file, text.strip_prefix('\u{feff}').unwrap_or(text))?;
    let changes = Files::changes(&baseline, &files.capture(&workspace)?);
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let mut task = json!({"id":id,"projectId":project_id,"title":format!("编辑资料 · {relative}"),"prompt":"人工编辑资料","stopConditions":"","status":"interrupted","createdAt":now,"updatedAt":now,"workspace":workspace,"baseline":baseline,"references":[],"conflicts":[],"maxMinutes":0,"capability":"code","changes":changes,"report":"人工修改等待合入。"});
    store.put("task", &id, &task)?;
    task["status"] = json!("completed");
    task["report"] = json!("人工修改已保存。");
    if let Err(error) =
        Journal::new(store, &files).apply(project_id, changes, task.clone(), OperationKind::Merge)
    {
        task["status"] = json!("conflict");
        task["report"] = json!("人工修改未完成合入，工作副本已保留。");
        task["error"] = json!(error.to_string());
        store.put("task", &id, &task)?;
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn edits_are_revision_checked_journaled_and_keep_baseline() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let data = temp.path().join("data");
        let root = temp.path().join("project");
        fs::create_dir(&root)?;
        fs::write(root.join("world.md"), "before")?;
        let mut store = Store::open(&data)?;
        store.put("project", "p", &json!({"path":root}))?;
        let original = read_document(&root, "world.md")?;
        save_document(
            &mut store,
            &data,
            "p",
            "world.md",
            "\u{feff}after",
            original["revision"].as_str(),
        )?;
        assert_eq!(fs::read_to_string(root.join("world.md"))?, "after");
        assert!(save_document(
            &mut store,
            &data,
            "p",
            "world.md",
            "stale",
            original["revision"].as_str()
        )
        .is_err());
        let task = store.list::<Value>("task")?.remove(0);
        assert_eq!(task["status"], "completed");
        assert_eq!(task["baseline"]["world.md"], original["revision"]);
        assert_eq!(store.list::<Value>("operation")?[0]["state"], "complete");
        assert_eq!(assets(&root)?[0]["kind"], "text");
        Ok(())
    }
    #[test]
    fn documents_reject_hidden_paths_invalid_utf8_and_oversize() -> Result<()> {
        let temp = tempfile::tempdir()?;
        for invalid in ["../a.md", ".git/a.md", "exports/a.md", "script.gd"] {
            assert!(read_document(temp.path(), invalid).is_err());
        }
        fs::write(temp.path().join("a.md"), [255])?;
        assert!(read_document(temp.path(), "a.md").is_err());
        fs::write(temp.path().join("a.md"), vec![b'a'; 512001])?;
        assert!(read_document(temp.path(), "a.md").is_err());
        Ok(())
    }
}
