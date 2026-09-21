use super::*;

impl ProjectStorageRouter {
    /// Remove only host registration; admitted workers retain their original storage.
    pub fn unregister(&self, project_id: &str, expected_path: &str) -> Result<Value> {
        let mut projects = self
            .projects
            .lock()
            .map_err(|_| anyhow::anyhow!("项目注册表锁不可用"))?;
        let host = self
            .host
            .lock()
            .map_err(|_| anyhow::anyhow!("宿主数据库锁不可用"))?;
        let project = host
            .get::<Value>("project", project_id)?
            .context("宿主未登记项目")?;
        ensure!(project["id"] == project_id, "项目登记 key 与内部 ID 不一致");
        let path = project["path"].as_str().context("项目登记缺少路径")?;
        ensure!(path == expected_path, "项目登记已变化，请刷新后注销");
        let mut index = self
            .task_projects
            .lock()
            .map_err(|_| anyhow::anyhow!("任务路由锁不可用"))?;
        // This host lock also excludes Scheduler and Validation admission permits.
        // Persist first: a failed write must leave the runtime and routes intact.
        host.remove("project", project_id)?;
        let draining = projects.close(project_id).is_err();
        if !draining {
            index.retain(|_, owner| owner != project_id);
        }
        Ok(json!({"id":project_id,"path":path,"draining":draining}))
    }
}

#[cfg(test)]
#[path = "project_unregistration_tests.rs"]
mod tests;
