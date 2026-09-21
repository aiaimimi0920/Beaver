use super::*;
use crate::project_storage::ProjectStore;
use std::path::Path;

impl ProjectStorageRouter {
    /// Explicitly replace a host location without copying or rewriting project data.
    pub fn reassociate(
        &self,
        project_id: &str,
        expected_path: &str,
        directory: &Path,
        data: &Path,
    ) -> Result<Value> {
        ensure!(directory.is_absolute(), "项目路径必须是绝对路径");
        let root = layout::root(directory)?;
        ensure!(
            !root.starts_with(fs::canonicalize(data)?),
            "不能关联应用内部数据"
        );
        ensure!(
            crate::files::safe_path(&root, "project.godot")?.is_file(),
            "不是有效 Godot 项目"
        );
        layout::read_manifest(&root, Some(project_id))?;
        let mut projects = self
            .projects
            .lock()
            .map_err(|_| anyhow::anyhow!("项目注册表锁不可用"))?;
        let host = self
            .host
            .lock()
            .map_err(|_| anyhow::anyhow!("宿主数据库锁不可用"))?;
        let mut project = host
            .get::<Value>("project", project_id)?
            .context("宿主未登记项目")?;
        ensure!(project["id"] == project_id, "项目登记 key 与内部 ID 不一致");
        let current = project["path"].as_str().context("项目登记缺少路径")?;
        ensure!(current == expected_path, "项目登记已变化，请刷新后重新关联");
        if fs::canonicalize(current).ok().as_ref() == Some(&root) {
            return Ok(project);
        }
        for (id, other) in host.list_with_ids::<Value>("project")? {
            if id != project_id {
                ensure!(
                    other["path"]
                        .as_str()
                        .and_then(|path| fs::canonicalize(path).ok())
                        .as_ref()
                        != Some(&root),
                    "目标路径已登记给其他项目"
                );
            }
        }
        // Validate the target before releasing the old runtime or changing registration.
        let candidate = ProjectStore::open(&root, project_id)?;
        let local = candidate
            .store()
            .get::<Value>("project", project_id)?
            .context("项目本地存储缺少项目实体")?;
        ensure!(local["id"] == project_id, "项目本地实体 ID 与清单不一致");
        let mut index = self
            .task_projects
            .lock()
            .map_err(|_| anyhow::anyhow!("任务路由锁不可用"))?;
        projects.ensure_can_close(project_id)?;
        project["path"] = json!(root);
        host.put("project", project_id, &project)?;
        // The registry lock prevents new runtime references after the close precheck.
        projects.close(project_id)?;
        index.retain(|_, owner| owner != project_id);
        Ok(project)
    }
}

#[cfg(test)]
#[path = "project_reassociation_tests.rs"]
mod tests;
