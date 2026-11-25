//! Local HTTP bridge server for Office/WPS Add-ins and the native desktop editor.
//!
//! ```text
//! HTTP Request (tiny_http)
//!   ├─► Host validation (`is_valid_host`: loopback only, anti-DNS-rebinding)
//!   ├─► Origin validation (`is_allowed_origin`: localhost / HTTPS Office / WPS)
//!   ├─► CORS preflight (OPTIONS -> 204 + PNA headers)
//!   ├─► Public GET routes (`/health`, `/wps-addin/*`)
//!   └─► Token auth (`X-EqnTide-Token`, constant-time check)
//!         ├─► `/session/*` -> `session_routes` (open / long-poll wait / update / cancel)
//!         ├─► `/window/show` -> `DesktopWindowRequest` callback or spawned UI
//!         ├─► `/convert/{snippet,document}` -> `convert_routes` (`crate::service`)
//!         └─► `/log` -> diagnostic log sink
//! ```

mod addin_assets;
mod auth;
mod convert_routes;
mod http;
mod session_routes;

#[cfg(test)]
mod tests;

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use tiny_http::{Method, Request, Server};

#[cfg(test)]
use addin_assets::is_safe_relative_addin_path;
use addin_assets::serve_addin_route;
pub use auth::{
    bridge_token_path, is_allowed_origin, is_valid_host, load_or_create_bridge_token,
    load_or_create_bridge_token_in, user_data_dir,
};
use auth::{constant_time_eq, generate_random_token, is_valid_token_format};
use convert_routes::{handle_document_conversion, handle_snippet_conversion};
use http::{
    body_error_response, empty_cors_response, error_response, get_header, health_response,
    is_cors_preflight, json_response, read_body, respond,
};
use session_routes::{handle_session_route, handle_window_show};

use super::session::SessionManager;

/// Default bind port for the local bridge server.
pub const DEFAULT_PORT: u16 = 17890;

/// Default bind host address.
pub const DEFAULT_HOST: &str = "127.0.0.1";

/// HTTP header carrying the per-user bridge authentication token.
pub const BRIDGE_TOKEN_HEADER: &str = "X-EqnTide-Token";

/// Maximum accepted HTTP request body size (16 MiB).
pub(super) const MAX_BODY_BYTES: usize = 16 * 1024 * 1024;

/// Maximum concurrent HTTP worker threads (including long-poll waiters).
const MAX_CONCURRENT_REQUESTS: usize = 128;

/// Maximum session retention age before stale cleanup evicts entries and temp EMF files.
pub(super) const SESSION_MAX_AGE: Duration = Duration::from_secs(3600);

/// Legacy callback invoked when a session opens or window activation is requested.
///
/// Arguments: `(session_id, mathml, latex, font_pt)`.
pub type WindowCallback =
    Arc<dyn Fn(&str, Option<&str>, Option<&str>, Option<f32>) + Send + Sync + 'static>;

/// Window activation payload delivered to the embedded desktop UI callback.
#[derive(Clone, Debug)]
pub struct DesktopWindowRequest {
    /// Active bridge session ID, or empty when merely focusing the window.
    pub session_id: String,
    /// Optional initial MathML document payload to load into the canvas.
    pub mathml: Option<String>,
    /// Optional initial LaTeX source associated with the equation.
    pub latex: Option<String>,
    /// Optional preferred base font size in points.
    pub font_pt: Option<f32>,
    /// Optional desktop tab to focus (such as `"batch"`).
    pub tab: Option<String>,
    /// Optional target mode override (`"omml"` or `"mathtype"`).
    pub target_mode: Option<String>,
}

/// Structured callback invoked when an HTTP request asks to open or focus the desktop UI.
pub type WindowRequestCallback = Arc<dyn Fn(DesktopWindowRequest) + Send + Sync + 'static>;

/// Handle to a running bridge server instance.
pub struct ServerHandle {
    server: Arc<Server>,
    session_manager: SessionManager,
    running: Arc<AtomicBool>,
    active_requests: Arc<AtomicUsize>,
    thread: Option<JoinHandle<()>>,
    addr: SocketAddr,
    auth_token: String,
}

impl ServerHandle {
    /// Return the number of HTTP requests whose responses are still in flight.
    pub fn pending_requests(&self) -> usize {
        self.active_requests.load(Ordering::SeqCst)
    }

    /// Return the actual bound socket address of the server.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Return the per-user authentication token required by protected bridge endpoints.
    pub fn auth_token(&self) -> &str {
        &self.auth_token
    }

    /// Access the shared session manager.
    pub fn session_manager(&self) -> &SessionManager {
        &self.session_manager
    }

    /// Stop the server and wait for its accept loop to exit.
    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        self.server.unblock();
        if let Some(thread_handle) = self.thread.take() {
            let _ = thread_handle.join();
        }
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

struct ActiveRequest(Arc<AtomicUsize>);

impl Drop for ActiveRequest {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Start the local HTTP bridge server on `addr` (for example `"127.0.0.1:17890"`).
pub fn start_bridge_server(addr: &str) -> crate::error::Result<ServerHandle> {
    start_bridge_server_with_callback(addr, None)
}

/// Start the local HTTP bridge server with an optional legacy window-show callback.
pub fn start_bridge_server_with_callback(
    addr: &str,
    window_cb: Option<WindowCallback>,
) -> crate::error::Result<ServerHandle> {
    let callback = window_cb.map(|legacy_cb| {
        Arc::new(move |req: DesktopWindowRequest| {
            legacy_cb(
                &req.session_id,
                req.mathml.as_deref(),
                req.latex.as_deref(),
                req.font_pt,
            );
        }) as WindowRequestCallback
    });
    start_bridge_server_with_window_requests(addr, callback)
}

/// Start the local HTTP bridge server with a structured desktop window-request callback.
pub fn start_bridge_server_with_window_requests(
    addr: &str,
    window_cb: Option<WindowRequestCallback>,
) -> crate::error::Result<ServerHandle> {
    let auth_token = if cfg!(test) {
        generate_random_token()
    } else {
        load_or_create_bridge_token()?
    };
    start_bridge_server_with_token(addr, window_cb, auth_token)
}

/// Start the local HTTP bridge server with an explicit authentication token.
pub fn start_bridge_server_with_token(
    addr: &str,
    window_cb: Option<WindowRequestCallback>,
    auth_token: String,
) -> crate::error::Result<ServerHandle> {
    if !is_valid_token_format(&auth_token) {
        return Err(crate::error::Error::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "invalid bridge authentication token format",
        )));
    }
    let server = Server::http(addr).map_err(|err| {
        crate::error::Error::Io(std::io::Error::other(format!(
            "failed to bind HTTP server to {addr}: {err}"
        )))
    })?;

    let bound_addr = server.server_addr().to_ip().ok_or_else(|| {
        crate::error::Error::Io(std::io::Error::other("failed to query server IP address"))
    })?;

    let server_arc = Arc::new(server);
    let session_mgr = SessionManager::new();
    let running = Arc::new(AtomicBool::new(true));
    let active_requests = Arc::new(AtomicUsize::new(0));
    let active_for_server = Arc::clone(&active_requests);

    let server_clone = Arc::clone(&server_arc);
    let session_clone = session_mgr.clone();
    let running_clone = Arc::clone(&running);
    let token_clone = auth_token.clone();

    let thread = std::thread::spawn(move || {
        while running_clone.load(Ordering::SeqCst) {
            let request = match server_clone.recv() {
                Ok(req) => req,
                Err(err) => {
                    log::debug!("HTTP server recv ended: {err}");
                    break;
                }
            };

            if active_for_server.load(Ordering::SeqCst) >= MAX_CONCURRENT_REQUESTS {
                let _ = request.respond(error_response(
                    503,
                    "service unavailable: too many concurrent requests",
                    None,
                ));
                continue;
            }

            let session_for_worker = session_clone.clone();
            let window_cb_for_worker = window_cb.clone();
            let active_for_worker = Arc::clone(&active_for_server);
            let token_for_worker = token_clone.clone();
            active_for_worker.fetch_add(1, Ordering::SeqCst);
            std::thread::spawn(move || {
                let _active = ActiveRequest(active_for_worker);
                handle_http_request(
                    request,
                    session_for_worker,
                    window_cb_for_worker,
                    &token_for_worker,
                );
            });
        }
    });

    Ok(ServerHandle {
        server: server_arc,
        session_manager: session_mgr,
        running,
        active_requests,
        thread: Some(thread),
        addr: bound_addr,
        auth_token,
    })
}

fn handle_http_request(
    mut req: Request,
    session_mgr: SessionManager,
    window_cb: Option<WindowRequestCallback>,
    auth_token: &str,
) {
    let method = req.method().clone();
    let raw_url = req.url().to_owned();
    let (path, query) = match raw_url.split_once('?') {
        Some((route_path, query_str)) => (route_path, Some(query_str)),
        None => (raw_url.as_str(), None),
    };

    // 1. Validate Host header strictly against loopback to prevent DNS rebinding.
    let Some(host) = get_header(&req, "Host") else {
        log::warn!("Rejected request missing Host header");
        let _ = req.respond(error_response(
            403,
            "forbidden: missing Host header (DNS rebinding protection)",
            None,
        ));
        return;
    };
    if !is_valid_host(host) {
        log::warn!("Rejected request with untrusted Host header: {host}");
        let _ = req.respond(error_response(
            403,
            "forbidden: invalid Host header (DNS rebinding protection)",
            None,
        ));
        return;
    }

    // 2. Validate Origin header against localhost and trusted Office/WPS hosts.
    let origin_owned = get_header(&req, "Origin").map(|origin_str| origin_str.to_string());
    if let Some(ref origin_value) = origin_owned {
        if !is_allowed_origin(origin_value) {
            log::warn!("Rejected request from untrusted Origin: {origin_value}");
            let _ = req.respond(error_response(403, "forbidden: untrusted Origin", None));
            return;
        }
    }
    let origin = origin_owned.as_deref();

    // 3. Respond to CORS/PNA preflight requests before authentication checks.
    if is_cors_preflight(&method) {
        let resp = empty_cors_response(204, origin);
        let _ = req.respond(resp);
        return;
    }

    // 4. Serve public read-only health checks and static WPS add-in assets.
    if method == Method::Get && path == "/health" {
        respond(req, health_response(origin));
        return;
    }
    if method == Method::Get {
        if let Some(resp) = serve_addin_route(path, origin) {
            let _ = req.respond(resp);
            return;
        }
    }

    // 5. Require the per-user authentication token on all API routes.
    let authorized = get_header(&req, BRIDGE_TOKEN_HEADER).is_some_and(|provided| {
        constant_time_eq(provided.trim().as_bytes(), auth_token.as_bytes())
    });
    if !authorized {
        let _ = req.respond(error_response(
            401,
            "unauthorized: invalid or missing bridge token",
            origin,
        ));
        return;
    }

    if let Some(response) = handle_session_route(
        &mut req,
        &method,
        path,
        query,
        &session_mgr,
        window_cb.as_ref(),
        origin,
    ) {
        respond(req, response);
        return;
    }

    // 6. Dispatch window, logging, and stateless conversion routes.
    match (method, path) {
        (Method::Post, "/window/show") => {
            let response = handle_window_show(&mut req, &session_mgr, window_cb.as_ref(), origin);
            respond(req, response);
        }

        (Method::Post, "/log") => {
            let body = match read_body(&mut req) {
                Ok(body) => body,
                Err(err) => {
                    respond(req, body_error_response(err, origin));
                    return;
                }
            };
            let trimmed = body.trim();
            eprintln!("[WPS Addin] {trimmed}");
            log::info!("[WPS Addin] {trimmed}");
            let resp = json_response(200, &serde_json::json!({ "status": "ok" }), origin);
            let _ = req.respond(resp);
        }

        // Preserve `/convert` alongside `/convert/snippet` for legacy VBA templates.
        (Method::Post, "/convert/snippet" | "/convert") => handle_snippet_conversion(req, origin),

        (Method::Post, "/convert/document") => handle_document_conversion(req, origin),

        _ => {
            let resp = error_response(404, "not found", origin);
            let _ = req.respond(resp);
        }
    }
}

/// Launch the desktop equation editor binary for an existing bridge `session_id`.
pub(crate) fn spawn_desktop_editor(session_id: &str) {
    spawn_desktop_action(session_id, None);
}

pub(super) fn spawn_desktop_action(session_id: &str, tab: Option<&str>) {
    // Library tests must never open an installed desktop application.
    if cfg!(test) {
        return;
    }
    let mut candidates = Vec::new();
    if let Ok(env_path) = std::env::var("EQNTIDE_DESKTOP_BIN") {
        candidates.push(std::path::PathBuf::from(env_path));
    }
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(dir) = current_exe.parent() {
            candidates.push(dir.join("eqntide-desktop"));
            candidates.push(dir.join("eqntide-desktop.exe"));
            candidates.push(dir.join("EqnTide.app/Contents/MacOS/eqntide-desktop"));
        }
    }
    #[cfg(target_os = "windows")]
    candidates.push(std::path::PathBuf::from("eqntide-desktop.exe"));
    #[cfg(not(target_os = "windows"))]
    candidates.push(std::path::PathBuf::from("/usr/bin/eqntide-desktop"));
    candidates.push(std::path::PathBuf::from("eqntide-desktop"));

    for path in &candidates {
        if path.is_absolute() && !path.exists() {
            continue;
        }
        let mut cmd = std::process::Command::new(path);
        let action = super::uri::CustomUriAction {
            command: "open".into(),
            session_id: (!session_id.is_empty()).then(|| session_id.to_owned()),
            tab: tab.map(str::to_owned),
            data: None,
            format: None,
            target_mode: None,
        };
        cmd.arg(super::uri::format_custom_uri(&action));
        match cmd.spawn() {
            Ok(child) => {
                log::info!(
                    "Spawned desktop editor UI (PID {}) via {}",
                    child.id(),
                    path.display()
                );
                eprintln!(
                    "Spawned desktop editor UI (PID {}) via {}",
                    child.id(),
                    path.display()
                );
                reap_desktop_child(child);
                return;
            }
            Err(err) => {
                log::debug!("Failed to spawn {}: {err}", path.display());
            }
        }
    }
    log::warn!("Could not find or launch EqnTide UI binary");
}

pub(super) fn reap_desktop_child(mut child: std::process::Child) -> JoinHandle<()> {
    // Wait off the request thread so exited desktop processes never remain zombies.
    std::thread::spawn(move || {
        if let Err(error) = child.wait() {
            log::warn!("Failed to reap desktop process {}: {error}", child.id());
        }
    })
}
