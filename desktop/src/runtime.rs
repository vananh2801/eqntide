//! Blocking I/O adapters. All bridge requests run off the GUI thread.
use crate::Event;
use crate::EventSender as Sender;
use eqntide::bridge::{DesktopWindowRequest, HealthResponse, WindowRequestCallback};
use std::sync::Arc;
use std::time::Duration;

pub fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .into()
}
pub fn get(path: &str) -> Result<serde_json::Value, String> {
    agent()
        .get(format!("http://127.0.0.1:17890{path}"))
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .read_json()
        .map_err(|e| e.to_string())
}
pub fn post(path: &str, body: &impl serde::Serialize) -> Result<serde_json::Value, String> {
    agent()
        .post(format!("http://127.0.0.1:17890{path}"))
        .send_json(body)
        .map_err(|e| e.to_string())?
        .body_mut()
        .read_json()
        .map_err(|e| e.to_string())
}
fn healthy() -> Result<(), String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(2)))
        .build()
        .into();
    let health: HealthResponse = agent
        .get("http://127.0.0.1:17890/health")
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .read_json()
        .map_err(|e| e.to_string())?;
    if health.status == "ok" && health.server == "eqntide-bridge" {
        Ok(())
    } else {
        Err("Port 17890 is occupied by another service".into())
    }
}
pub fn start_bridge(tx: Sender<Event>) {
    std::thread::spawn(move || {
        let events = tx.clone();
        let callback: WindowRequestCallback = Arc::new(move |r: DesktopWindowRequest| {
            let _ = events.send(Event::Window(r));
        });
        let result = match eqntide::bridge::start_bridge_server_with_window_requests(
            "127.0.0.1:17890",
            Some(callback),
        ) {
            Ok(handle) => Ok(Some(handle)),
            Err(e) => healthy()
                .map(|_| None)
                .map_err(|_| format!("Bridge unavailable: {e}")),
        };
        let _ = tx.send(Event::Bridge(result));
    });
}

pub fn open_folder(path: &std::path::Path) -> Result<(), String> {
    let path = if path.is_file() {
        path.parent().unwrap_or(path)
    } else {
        path
    };
    #[cfg(target_os = "windows")]
    let program = "explorer";
    #[cfg(target_os = "macos")]
    let program = "open";
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let program = "xdg-open";
    std::process::Command::new(program)
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}
