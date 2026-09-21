use crate::store::Store;
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::sync::{Arc, Mutex, MutexGuard};

/// Serializes new project work with changes to its host registration.
#[derive(Clone, Default)]
pub struct ProjectWorkGate {
    registration: Option<(Arc<Mutex<Store>>, String)>,
}

pub struct ProjectWorkPermit<'a> {
    _host: Option<MutexGuard<'a, Store>>,
}

impl ProjectWorkGate {
    pub fn registered(host: Arc<Mutex<Store>>, project_id: impl Into<String>) -> Self {
        Self {
            registration: Some((host, project_id.into())),
        }
    }

    /// Acquire before the project Store and retain through coordination or claim writes.
    /// Do not call the router, callbacks or external workers while holding the permit.
    /// Host-only runtimes use the default gate; their Store lock guards registration.
    pub fn acquire(&self) -> Result<Option<ProjectWorkPermit<'_>>> {
        let Some((host, project_id)) = &self.registration else {
            return Ok(Some(ProjectWorkPermit { _host: None }));
        };
        let host = host
            .lock()
            .map_err(|_| anyhow!("Host database lock unavailable"))?;
        if host.get::<Value>("project", project_id)?.is_none() {
            return Ok(None);
        }
        Ok(Some(ProjectWorkPermit { _host: Some(host) }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permit_excludes_registration_writes_and_rechecks_old_clones() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let host = Arc::new(Mutex::new(Store::open(temp.path())?));
        let project = serde_json::json!({"id":"p"});
        host.lock().unwrap().put("project", "p", &project)?;
        let gate = ProjectWorkGate::registered(host.clone(), "p");
        let snapshot = gate.clone();
        let permit = gate.acquire()?.unwrap();
        assert!(matches!(
            host.try_lock(),
            Err(std::sync::TryLockError::WouldBlock)
        ));
        drop(permit);
        host.lock().unwrap().remove("project", "p")?;
        assert!(snapshot.acquire()?.is_none());
        host.lock().unwrap().put("project", "p", &project)?;
        assert!(snapshot.acquire()?.is_some());
        Ok(())
    }
}
