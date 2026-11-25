//! Blocking I/O adapters. All bridge requests run off the GUI thread.

use crate::Event;
use crate::EventSender as Sender;
use eqntide::bridge::{DesktopWindowRequest, HealthResponse, WindowRequestCallback};
use std::sync::Arc;
use std::time::Duration;

/// Creates an HTTP client agent configured with the default 15-second global timeout.
pub fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .into()
}

/// Loads the per-user authentication token used to talk to the local bridge server.
pub fn bridge_token() -> Result<String, String> {
    let path = eqntide::bridge::bridge_token_path()
        .ok_or_else(|| "Could not resolve per-user bridge token path".to_string())?;
    let raw = std::fs::read_to_string(&path)
        .map_err(|err| format!("Failed to read bridge token '{}': {err}", path.display()))?;
    let token = raw.trim();
    if token.is_empty() {
        return Err("Bridge authentication token is empty".into());
    }
    Ok(token.to_owned())
}

/// Performs an authenticated GET request against `base_url` and parses JSON.
pub fn get_with_base(base_url: &str, path: &str, token: &str) -> Result<serde_json::Value, String> {
    agent()
        .get(format!("{base_url}{path}"))
        .header(eqntide::bridge::BRIDGE_TOKEN_HEADER, token)
        .call()
        .map_err(|err| err.to_string())?
        .body_mut()
        .read_json()
        .map_err(|err| err.to_string())
}

/// Performs an authenticated JSON POST request against `base_url` and parses the response JSON.
pub fn post_with_base(
    base_url: &str,
    path: &str,
    body: &impl serde::Serialize,
    token: &str,
) -> Result<serde_json::Value, String> {
    agent()
        .post(format!("{base_url}{path}"))
        .header(eqntide::bridge::BRIDGE_TOKEN_HEADER, token)
        .send_json(body)
        .map_err(|err| err.to_string())?
        .body_mut()
        .read_json()
        .map_err(|err| err.to_string())
}

/// Performs an authenticated GET request against the local EqnTide bridge.
pub fn get(path: &str) -> Result<serde_json::Value, String> {
    let token = bridge_token()?;
    get_with_base("http://127.0.0.1:17890", path, &token)
}

/// Performs an authenticated JSON POST request against the local EqnTide bridge.
pub fn post(path: &str, body: &impl serde::Serialize) -> Result<serde_json::Value, String> {
    let token = bridge_token()?;
    post_with_base("http://127.0.0.1:17890", path, body, &token)
}

/// Spawns or attaches to the local EqnTide bridge HTTP server on port 17890.
pub fn start_bridge(tx: Sender<Event>) {
    std::thread::spawn(move || {
        let events = tx.clone();
        let callback: WindowRequestCallback = Arc::new(move |req: DesktopWindowRequest| {
            let _ = events.send(Event::Window(req));
        });
        let result = match eqntide::bridge::start_bridge_server_with_window_requests(
            "127.0.0.1:17890",
            Some(callback),
        ) {
            Ok(handle) => Ok(Some(handle)),
            Err(err) => healthy()
                .map(|_| None)
                .map_err(|_| format!("Bridge unavailable: {err}")),
        };
        let _ = tx.send(Event::Bridge(result));
    });
}

/// Opens `path` (or its parent directory if `path` is a file) in the system file manager.
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
        .map_err(|err| err.to_string())
}

fn healthy() -> Result<(), String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(2)))
        .build()
        .into();
    let health: HealthResponse = agent
        .get("http://127.0.0.1:17890/health")
        .call()
        .map_err(|err| err.to_string())?
        .body_mut()
        .read_json()
        .map_err(|err| err.to_string())?;
    if health.status == "ok" && health.server == "eqntide-bridge" {
        Ok(())
    } else {
        Err("Port 17890 is occupied by another service".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_client_authenticates_against_token_protected_bridge() {
        let token = "test-bridge-token-0123456789abcdef".to_string();
        let server =
            eqntide::bridge::start_bridge_server_with_token("127.0.0.1:0", None, token.clone())
                .unwrap();
        let base = format!("http://{}", server.addr());
        let sid = server.session_manager().create_session(
            "<math><mi>x</mi></math>".into(),
            None,
            Some(12.0),
        );

        assert!(get_with_base(&base, &format!("/session/{sid}"), "wrong-token").is_err());
        let fetched =
            get_with_base(&base, &format!("/session/{sid}"), server.auth_token()).unwrap();
        assert_eq!(fetched["session_id"], sid);

        let updated = post_with_base(
            &base,
            &format!("/session/{sid}/cancel"),
            &serde_json::json!({}),
            server.auth_token(),
        )
        .unwrap();
        assert_eq!(updated["status"], "cancelled");
    }
}
