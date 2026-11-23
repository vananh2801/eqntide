//! Thread-safe session manager with condvar-based long-polling.

use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Current lifecycle state of an equation editing session.
#[derive(Debug, Clone)]
pub enum SessionState {
    /// Waiting for user edits in the editor.
    Waiting,
    /// Equation was updated/inserted by the editor.
    Ready {
        mathml: String,
        flat_opc: String,
        omml: String,
        emf_path: Option<String>,
        width_pt: Option<f32>,
        height_pt: Option<f32>,
        depth_pt: Option<f32>,
        font_pt: Option<f32>,
    },
    /// Session was closed or cancelled by user without saving.
    Cancelled,
}

/// Internal session data.
pub struct SessionData {
    pub id: String,
    pub initial_mathml: String,
    pub initial_latex: Option<String>,
    pub initial_font_pt: Option<f32>,
    pub target_mode: String,
    pub created_at: Instant,
    pub state: SessionState,
}

/// A synchronization entry pairing session data with a conditional variable for waiting.
pub struct SessionEntry {
    pub data: Mutex<SessionData>,
    pub cond: Condvar,
}

/// Thread-safe manager for active Add-in editing sessions.
#[derive(Clone, Default)]
pub struct SessionManager {
    sessions: Arc<Mutex<HashMap<String, Arc<SessionEntry>>>>,
}

impl SessionManager {
    /// Create a new session manager instance.
    pub fn new() -> Self {
        SessionManager {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Retrieve the most recently created waiting session ID, if any.
    pub fn get_latest_session_id(&self) -> Option<String> {
        let lock = lock_unpoisoned(&self.sessions);
        let mut latest: Option<(String, Instant)> = None;
        for (id, entry) in lock.iter() {
            let data = lock_unpoisoned(&entry.data);
            if matches!(data.state, SessionState::Waiting)
                && latest.as_ref().is_none_or(|(_, t)| data.created_at > *t)
            {
                latest = Some((id.clone(), data.created_at));
            }
        }
        latest.map(|(id, _)| id)
    }

    /// Register a new session with initial MathML, optional LaTeX, and optional document font size.
    pub fn create_session(
        &self,
        initial_mathml: String,
        initial_latex: Option<String>,
        initial_font_pt: Option<f32>,
    ) -> String {
        self.create_session_with_target(initial_mathml, initial_latex, initial_font_pt, "omml")
    }

    /// Open a session while preserving the Office insertion target.
    pub fn create_session_with_target(
        &self,
        initial_mathml: String,
        initial_latex: Option<String>,
        initial_font_pt: Option<f32>,
        target_mode: &str,
    ) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let entry = Arc::new(SessionEntry {
            data: Mutex::new(SessionData {
                id: id.clone(),
                initial_mathml,
                initial_latex,
                initial_font_pt,
                target_mode: target_mode.to_owned(),
                created_at: Instant::now(),
                state: SessionState::Waiting,
            }),
            cond: Condvar::new(),
        });

        let mut lock = lock_unpoisoned(&self.sessions);
        lock.insert(id.clone(), entry);
        id
    }

    /// Retrieve the initial MathML of an active session.
    pub fn get_initial_mathml(&self, id: &str) -> Option<String> {
        let lock = lock_unpoisoned(&self.sessions);
        let entry = lock.get(id)?;
        let data = lock_unpoisoned(&entry.data);
        Some(data.initial_mathml.clone())
    }

    /// Retrieve the initial MathML, LaTeX, and font_pt of an active session.
    pub fn get_session(&self, id: &str) -> Option<(String, Option<String>, Option<f32>)> {
        let lock = lock_unpoisoned(&self.sessions);
        let entry = lock.get(id)?;
        let data = lock_unpoisoned(&entry.data);
        Some((
            data.initial_mathml.clone(),
            data.initial_latex.clone(),
            data.initial_font_pt,
        ))
    }

    /// Target and lifecycle metadata used by native editors. Existing getters stay compatible.
    pub fn metadata(&self, id: &str) -> Option<(String, &'static str)> {
        let sessions = lock_unpoisoned(&self.sessions);
        let entry = sessions.get(id)?;
        let data = lock_unpoisoned(&entry.data);
        let state = match data.state {
            SessionState::Waiting => "waiting",
            SessionState::Ready { .. } => "ready",
            SessionState::Cancelled => "cancelled",
        };
        Some((data.target_mode.clone(), state))
    }

    /// Update a session with the newly generated equation payloads, notifying any waiting long-poll request.
    #[allow(clippy::too_many_arguments)]
    pub fn update_session(
        &self,
        id: &str,
        mathml: String,
        flat_opc: String,
        omml: String,
        emf_path: Option<String>,
        width_pt: Option<f32>,
        height_pt: Option<f32>,
        depth_pt: Option<f32>,
        font_pt: Option<f32>,
    ) -> Result<(), &'static str> {
        let entry = {
            let lock = lock_unpoisoned(&self.sessions);
            lock.get(id).cloned().ok_or("session not found")?
        };

        {
            let mut data = lock_unpoisoned(&entry.data);
            data.state = SessionState::Ready {
                mathml,
                flat_opc,
                omml,
                emf_path,
                width_pt,
                height_pt,
                depth_pt,
                font_pt,
            };
        }
        entry.cond.notify_all();
        Ok(())
    }

    /// Cancel a session, notifying any waiting long-poll request.
    pub fn cancel_session(&self, id: &str) -> Result<(), &'static str> {
        let entry = {
            let lock = lock_unpoisoned(&self.sessions);
            lock.get(id).cloned().ok_or("session not found")?
        };

        let old_emf = {
            let mut data = lock_unpoisoned(&entry.data);
            let old = match &data.state {
                SessionState::Ready { emf_path, .. } => emf_path.clone(),
                _ => None,
            };
            data.state = SessionState::Cancelled;
            old
        };
        if let Some(path) = old_emf {
            let _ = std::fs::remove_file(path);
        }
        entry.cond.notify_all();
        Ok(())
    }

    /// Block the calling thread until the session completes, is cancelled, or the timeout expires.
    pub fn wait_session(&self, id: &str, timeout: Duration) -> Option<SessionState> {
        let entry = {
            let lock = lock_unpoisoned(&self.sessions);
            lock.get(id).cloned()?
        };

        let mut data = lock_unpoisoned(&entry.data);
        let start = Instant::now();

        while matches!(data.state, SessionState::Waiting) {
            let elapsed = start.elapsed();
            if elapsed >= timeout {
                return None; // Timeout
            }
            let remaining = timeout - elapsed;
            let result = entry
                .cond
                .wait_timeout(data, remaining)
                .unwrap_or_else(PoisonError::into_inner);
            data = result.0;
            if result.1.timed_out() && matches!(data.state, SessionState::Waiting) {
                return None;
            }
        }

        Some(data.state.clone())
    }

    /// Remove a session from the manager.
    pub fn remove_session(&self, id: &str) -> bool {
        let removed = {
            let mut lock = lock_unpoisoned(&self.sessions);
            lock.remove(id)
        };
        if let Some(entry) = removed {
            let data = lock_unpoisoned(&entry.data);
            if let SessionState::Ready {
                emf_path: Some(ref path),
                ..
            } = data.state
            {
                let _ = std::fs::remove_file(path);
            }
            true
        } else {
            false
        }
    }

    /// Clean up sessions older than `max_age` and remove associated temporary EMF files.
    pub fn cleanup_stale(&self, max_age: Duration) {
        let mut stale_emfs = Vec::new();
        {
            let mut lock = lock_unpoisoned(&self.sessions);
            lock.retain(|_, entry| {
                let data = lock_unpoisoned(&entry.data);
                if data.created_at.elapsed() < max_age {
                    true
                } else {
                    if let SessionState::Ready {
                        emf_path: Some(ref path),
                        ..
                    } = data.state
                    {
                        stale_emfs.push(path.clone());
                    }
                    false
                }
            });
        }
        for path in stale_emfs {
            let _ = std::fs::remove_file(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poisoned_sessions_map_recovers_across_all_operations() {
        let mgr = SessionManager::new();
        let pre_id = mgr.create_session("<math/>".into(), Some("x".into()), Some(12.0));

        // Poison the outer sessions mutex.
        let sessions_arc = Arc::clone(&mgr.sessions);
        let _ = std::thread::spawn(move || {
            let _guard = sessions_arc.lock().unwrap();
            panic!("intentional poison of sessions map");
        })
        .join();
        assert!(mgr.sessions.is_poisoned());

        let new_id = mgr.create_session_with_target(
            "<math><mi>y</mi></math>".into(),
            Some("y".into()),
            Some(11.0),
            "mathtype",
        );
        assert!(mgr.get_latest_session_id().is_some());
        assert_eq!(mgr.get_initial_mathml(&pre_id).as_deref(), Some("<math/>"));
        assert_eq!(
            mgr.get_session(&new_id),
            Some((
                "<math><mi>y</mi></math>".into(),
                Some("y".into()),
                Some(11.0)
            ))
        );
        assert_eq!(mgr.metadata(&new_id), Some(("mathtype".into(), "waiting")));
        assert!(mgr
            .update_session(
                &pre_id,
                "<math/>".into(),
                "opc".into(),
                "<omml/>".into(),
                None,
                None,
                None,
                None,
                None,
            )
            .is_ok());
        assert!(matches!(
            mgr.wait_session(&pre_id, Duration::from_millis(50)),
            Some(SessionState::Ready { .. })
        ));
        assert!(mgr.cancel_session(&new_id).is_ok());
        assert!(matches!(
            mgr.wait_session(&new_id, Duration::from_millis(50)),
            Some(SessionState::Cancelled)
        ));
        assert!(mgr.remove_session(&pre_id));
        mgr.cleanup_stale(Duration::from_secs(60));
    }

    #[test]
    fn poisoned_session_entry_data_recovers_and_cleans_emf() {
        let mgr = SessionManager::new();
        let id = mgr.create_session("<math><mi>z</mi></math>".into(), None, None);

        let entry = {
            let lock = lock_unpoisoned(&mgr.sessions);
            Arc::clone(lock.get(&id).unwrap())
        };

        // Poison the inner SessionEntry.data mutex.
        let _ = std::thread::spawn(move || {
            let _guard = entry.data.lock().unwrap();
            panic!("intentional poison of session entry data");
        })
        .join();

        assert_eq!(mgr.get_latest_session_id().as_deref(), Some(id.as_str()));
        assert_eq!(
            mgr.get_initial_mathml(&id).as_deref(),
            Some("<math><mi>z</mi></math>")
        );
        assert_eq!(
            mgr.get_session(&id),
            Some(("<math><mi>z</mi></math>".into(), None, None))
        );
        assert_eq!(mgr.metadata(&id), Some(("omml".into(), "waiting")));

        // Wait should time out cleanly even on a poisoned mutex.
        assert!(mgr.wait_session(&id, Duration::from_millis(10)).is_none());

        let emf_path =
            std::env::temp_dir().join(format!("eqntide-poison-test-{}.emf", uuid::Uuid::new_v4()));
        std::fs::write(&emf_path, b"emf").unwrap();

        assert!(mgr
            .update_session(
                &id,
                "<math/>".into(),
                "opc".into(),
                "<omml/>".into(),
                Some(emf_path.to_string_lossy().into_owned()),
                None,
                None,
                None,
                None,
            )
            .is_ok());
        assert!(matches!(
            mgr.wait_session(&id, Duration::from_millis(10)),
            Some(SessionState::Ready { .. })
        ));

        // cleanup_stale(ZERO) evicts the poisoned entry and removes its EMF file.
        std::thread::sleep(Duration::from_millis(2));
        mgr.cleanup_stale(Duration::ZERO);
        assert!(!emf_path.exists());
        assert!(mgr.get_session(&id).is_none());
        assert!(!mgr.remove_session(&id));

        // Also test cancel_session on an entry poisoned while Ready.
        let id2 = mgr.create_session("<math/>".into(), None, None);
        let emf2 = std::env::temp_dir().join(format!(
            "eqntide-poison-cancel-{}.emf",
            uuid::Uuid::new_v4()
        ));
        std::fs::write(&emf2, b"emf2").unwrap();
        mgr.update_session(
            &id2,
            "<math/>".into(),
            "opc".into(),
            "<omml/>".into(),
            Some(emf2.to_string_lossy().into_owned()),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let entry2 = {
            let lock = lock_unpoisoned(&mgr.sessions);
            Arc::clone(lock.get(&id2).unwrap())
        };
        let _ = std::thread::spawn(move || {
            let _guard = entry2.data.lock().unwrap();
            panic!("intentional poison before cancel");
        })
        .join();
        assert!(mgr.cancel_session(&id2).is_ok());
        assert!(!emf2.exists());
        assert!(mgr.remove_session(&id2));
    }
}
