use crate::{asset_checkpoint, asset_preview::Client, asset_task, blender_session::Session};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

const MAX_SESSIONS: usize = 12;

#[derive(Clone, Default)]
pub struct Sessions(Arc<Mutex<HashMap<String, Option<Session>>>>);

impl Sessions {
    /// Reserve before starting a process. A parked task retains its existing port.
    pub fn reserve(&self, id: &str) -> Result<Option<u16>, String> {
        let dead = {
            let mut sessions = self
                .0
                .lock()
                .map_err(|_| "Asset session lock unavailable")?;
            if let Some(Some(session)) = sessions.get_mut(id) {
                if session.alive() {
                    return Ok(Some(session.pub_port));
                }
            }
            let dead = sessions.remove(id).flatten();
            if sessions.len() >= MAX_SESSIONS {
                return Err("最多保留 12 个 Blender 现场；请先完成或中止一个资产任务".into());
            }
            sessions.insert(id.into(), None);
            dead
        };
        // Session::drop writes the Store; never run it under the registry lock.
        drop(dead);
        Ok(None)
    }

    pub fn insert(&self, id: &str, session: Session) -> Result<(), String> {
        let previous = self
            .0
            .lock()
            .map_err(|_| "Asset session lock unavailable")?
            .insert(id.into(), Some(session));
        drop(previous);
        Ok(())
    }

    pub fn client(&self, id: &str) -> Result<Client, String> {
        let mut sessions = self
            .0
            .lock()
            .map_err(|_| "Asset session lock unavailable")?;
        let session = sessions
            .get_mut(id)
            .and_then(Option::as_mut)
            .ok_or("Blender 观察会话尚未就绪或已经结束")?;
        if !session.alive() {
            return Err("Blender 已退出；仅已保存的场景可以恢复".into());
        }
        Ok(session.client.clone())
    }

    pub fn ids(&self) -> Vec<String> {
        self.0
            .lock()
            .map(|sessions| sessions.keys().cloned().collect())
            .unwrap_or_default()
    }

    pub async fn close(&self, id: &str) {
        let session = self
            .0
            .lock()
            .ok()
            .and_then(|mut sessions| sessions.remove(id).flatten());
        if let Some(session) = session {
            let store = session.store();
            let saved = asset_checkpoint::save(&store, &session.files, &session.client).await;
            if let Ok(db) = store.lock() {
                if let Ok(mut state) = asset_task::get(&db, id) {
                    if let Err(error) = saved {
                        let message = format!(
                            "关闭 Blender 前保存失败，最近保存点之后的变化可能丢失：{error}"
                        );
                        state.recovery = Some(message.clone());
                        let _ = db.event(id, &asset_task::now(), "assetRecovery", &message);
                    }
                    state.session_id = None;
                    let _ = asset_task::save(&db, &state);
                }
            }
            let _ = tokio::task::spawn_blocking(move || drop(session)).await;
        }
    }

    pub async fn close_all(&self) {
        for id in self.ids() {
            self.close(&id).await;
        }
    }
}
