//! Thread-safe session manager with condvar-based long-polling.

use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

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
        let lock = self.sessions.lock().unwrap();
        let mut latest: Option<(String, Instant)> = None;
        for (id, entry) in lock.iter() {
            if let Ok(data) = entry.data.lock() {
                if matches!(data.state, SessionState::Waiting)
                    && latest.as_ref().is_none_or(|(_, t)| data.created_at > *t)
                {
                    latest = Some((id.clone(), data.created_at));
                }
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
        let id = uuid::Uuid::new_v4().to_string();
        let entry = Arc::new(SessionEntry {
            data: Mutex::new(SessionData {
                id: id.clone(),
                initial_mathml,
                initial_latex,
                initial_font_pt,
                created_at: Instant::now(),
                state: SessionState::Waiting,
            }),
            cond: Condvar::new(),
        });

        let mut lock = self.sessions.lock().unwrap();
        lock.insert(id.clone(), entry);
        id
    }

    /// Retrieve the initial MathML of an active session.
    pub fn get_initial_mathml(&self, id: &str) -> Option<String> {
        let lock = self.sessions.lock().unwrap();
        let entry = lock.get(id)?;
        let data = entry.data.lock().unwrap();
        Some(data.initial_mathml.clone())
    }

    /// Retrieve the initial MathML, LaTeX, and font_pt of an active session.
    pub fn get_session(&self, id: &str) -> Option<(String, Option<String>, Option<f32>)> {
        let lock = self.sessions.lock().unwrap();
        let entry = lock.get(id)?;
        let data = entry.data.lock().unwrap();
        Some((
            data.initial_mathml.clone(),
            data.initial_latex.clone(),
            data.initial_font_pt,
        ))
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
            let lock = self.sessions.lock().unwrap();
            lock.get(id).cloned().ok_or("session not found")?
        };

        {
            let mut data = entry.data.lock().unwrap();
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
            let lock = self.sessions.lock().unwrap();
            lock.get(id).cloned().ok_or("session not found")?
        };

        {
            let mut data = entry.data.lock().unwrap();
            data.state = SessionState::Cancelled;
        }
        entry.cond.notify_all();
        Ok(())
    }

    /// Block the calling thread until the session completes, is cancelled, or the timeout expires.
    pub fn wait_session(&self, id: &str, timeout: Duration) -> Option<SessionState> {
        let entry = {
            let lock = self.sessions.lock().unwrap();
            lock.get(id).cloned()?
        };

        let mut data = entry.data.lock().unwrap();
        let start = Instant::now();

        while matches!(data.state, SessionState::Waiting) {
            let elapsed = start.elapsed();
            if elapsed >= timeout {
                return None; // Timeout
            }
            let remaining = timeout - elapsed;
            let result = entry.cond.wait_timeout(data, remaining).unwrap();
            data = result.0;
            if result.1.timed_out() && matches!(data.state, SessionState::Waiting) {
                return None;
            }
        }

        Some(data.state.clone())
    }

    /// Remove a session from the manager.
    pub fn remove_session(&self, id: &str) -> bool {
        let mut lock = self.sessions.lock().unwrap();
        lock.remove(id).is_some()
    }

    /// Clean up sessions older than `max_age`.
    pub fn cleanup_stale(&self, max_age: Duration) {
        let mut lock = self.sessions.lock().unwrap();
        lock.retain(|_, entry| {
            if let Ok(data) = entry.data.lock() {
                data.created_at.elapsed() < max_age
            } else {
                false
            }
        });
    }
}
