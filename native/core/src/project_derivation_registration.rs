//! Adopt an activated derivation without mutating its files before the host commit.
use super::*;
use crate::{project_derivation_assembly as assembly, project_storage::ProjectStore};
use sha2::{Digest, Sha256};
use std::path::Path;

fn proof(receipt: &assembly::Receipt) -> Result<Value> {
    Ok(json!({
        "format": "beaver-project-derivation-registration-v1",
        "assemblySha256": format!("{:x}", Sha256::digest(serde_json::to_vec(receipt)?)),
        "binding": receipt.binding,
    }))
}

fn check_local(project: &Value, id: &str, root: &Path) -> Result<()> {
    ensure!(
        project["id"] == id,
        "derived project entity identity mismatch"
    );
    ensure!(
        project["path"].as_str() == root.to_str(),
        "derived project entity binding mismatch"
    );
    Ok(())
}

impl ProjectStorageRouter {
    /// Commit host adoption only after read-only identity, content and route preflight.
    /// The desktop opens the committed project through its ordinary recovery lifecycle.
    pub fn register_assembly(
        &self,
        preparation: &Path,
        destination: &Path,
        data: &Path,
    ) -> Result<Value> {
        ensure!(
            preparation.is_absolute() && destination.is_absolute(),
            "assembly paths must be absolute"
        );
        let preparation = layout::partition_root(preparation)?;
        let destination = layout::partition_root(destination)?;
        let data = fs::canonicalize(data)?;
        for path in [&preparation, &destination] {
            ensure!(
                !path.starts_with(&data) && !data.starts_with(path),
                "assembly overlaps host data"
            );
        }
        let root = layout::root(&destination.join("project"))?;
        let receipt = assembly::registration_receipt(&preparation, &destination)?;
        let id = &receipt.project_id;
        let expected = proof(&receipt)?;
        // Shared order with reassociation/unregistration; also excludes new-work permits.
        let projects = self
            .projects
            .lock()
            .map_err(|_| anyhow::anyhow!("项目注册表锁不可用"))?;
        let host = self
            .host
            .lock()
            .map_err(|_| anyhow::anyhow!("宿主数据库锁不可用"))?;
        let registered = host.get::<Value>("project", id)?;
        if let Some(project) = &registered {
            ensure!(project["id"] == *id, "项目登记 key 与内部 ID 不一致");
            let path = Path::new(project["path"].as_str().context("项目登记缺少路径")?);
            ensure!(
                path.is_absolute() && fs::canonicalize(path).ok().as_ref() == Some(&root),
                "同一项目 ID 已登记到其他位置；请明确重新关联"
            );
        }
        for (other_id, project) in host.list_with_ids::<Value>("project")? {
            if other_id != *id {
                ensure!(
                    project["path"]
                        .as_str()
                        .and_then(|path| fs::canonicalize(path).ok())
                        .as_ref()
                        != Some(&root),
                    "目标路径已登记给其他项目"
                );
            }
        }
        for runtime in projects.runtimes() {
            ensure!(
                (runtime.project_id() != id || runtime.project_root() == root)
                    && (runtime.project_root() != root || runtime.project_id() == id),
                "derived project conflicts with an open runtime"
            );
        }
        let index = self
            .task_projects
            .lock()
            .map_err(|_| anyhow::anyhow!("任务路由锁不可用"))?;
        if let Some(project) = registered
            .as_ref()
            .filter(|project| project["derivationAssembly"] == expected)
        {
            // Successful adoption authorizes normal project changes. Never compare its
            // current database/files to the pre-use inventory or overwrite host metadata.
            let local = if let Some(runtime) = projects.get(id) {
                runtime
                    .store()
                    .lock()
                    .map_err(|_| anyhow::anyhow!("项目数据库锁不可用"))?
                    .get::<Value>("project", id)?
                    .context("derived project entity missing")?
            } else {
                ProjectStore::read_project(&root, id)?
            };
            check_local(&local, id, &root)?;
            return Ok(json!({"projectId":id,"project":project,"hostRegistrationChanged":false}));
        }
        ensure!(
            projects.get(id).is_none(),
            "derived project is already open without an adoption receipt"
        );
        let verified = assembly::verified_activation(&preparation, &destination)?;
        ensure!(
            proof(&verified.receipt)? == expected,
            "assembly changed during registration"
        );
        let mut local = verified
            .snapshot
            .project(id)?
            .context("derived project entity missing")?;
        check_local(&local, id, &root)?;
        for (task_id, task) in verified.snapshot.tasks()? {
            validate_task(&task_id, id, &task)?;
            ensure!(
                index.get(&task_id).is_none_or(|owner| owner == id),
                "任务 ID 同时归属于多个项目：{task_id}"
            );
            if let Some(shadow) = host.get::<Value>("task", &task_id)? {
                validate_task(&task_id, id, &shadow)?;
            }
        }
        // No target SQLite connection has been opened. A failed host write leaves the
        // immutable activation inventory intact, so first-adoption retry remains valid.
        if let Some(project) = registered {
            local = project;
        }
        local["derivationAssembly"] = expected;
        host.put("project", id, &local)?;
        Ok(json!({"projectId":id,"project":local,"hostRegistrationChanged":true}))
    }
}

#[cfg(all(test, windows))]
#[path = "project_derivation_registration_tests.rs"]
mod tests;
