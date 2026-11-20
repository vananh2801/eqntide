//! Local HTTP bridge server implementation for Office/WPS Add-ins and the native desktop.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

type HttpResponse = Response<std::io::Cursor<Vec<u8>>>;

use crate::{
    mathml_to_flat_opc, mathml_to_mtef_bytes, mathml_to_omml_xml, render_emf_with_mtef,
    FlatOpcMode, MathNode,
};

use super::protocol::*;
use super::session::{SessionManager, SessionState};
use crate::service::{decode_input_to_mathml, parse_latex_to_mathml};

/// Default bind port for the local bridge server.
pub const DEFAULT_PORT: u16 = 17890;

/// Default bind host address.
pub const DEFAULT_HOST: &str = "127.0.0.1";

/// HTTP header carrying the per-user bridge authentication token.
pub const BRIDGE_TOKEN_HEADER: &str = "X-EqnTide-Token";

/// Maximum accepted HTTP request body size (16 MiB).
const MAX_BODY_BYTES: usize = 16 * 1024 * 1024;

/// Maximum concurrent HTTP worker threads (including long-poll waiters).
const MAX_CONCURRENT_REQUESTS: usize = 128;

/// Maximum session retention age before stale cleanup evicts entries and temp EMF files.
const SESSION_MAX_AGE: Duration = Duration::from_secs(3600);

/// Callback invoked when a session is opened or window show is requested.
/// Arguments: `(session_id, mathml, latex)`
pub type WindowCallback =
    Arc<dyn Fn(&str, Option<&str>, Option<&str>, Option<f32>) + Send + Sync + 'static>;

/// Window request including the tab, without changing the legacy callback API.
#[derive(Clone, Debug)]
pub struct DesktopWindowRequest {
    pub session_id: String,
    pub mathml: Option<String>,
    pub latex: Option<String>,
    pub font_pt: Option<f32>,
    pub tab: Option<String>,
    pub target_mode: Option<String>,
}
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

struct ActiveRequest(Arc<AtomicUsize>);
impl Drop for ActiveRequest {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

impl ServerHandle {
    /// Requests whose responses have not finished writing to the client yet.
    pub fn pending_requests(&self) -> usize {
        self.active_requests.load(Ordering::SeqCst)
    }

    /// The actual bound address of the server.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Per-user authentication token required by protected bridge endpoints.
    pub fn auth_token(&self) -> &str {
        &self.auth_token
    }

    /// Access the shared session manager.
    pub fn session_manager(&self) -> &SessionManager {
        &self.session_manager
    }

    /// Stop the server and wait for its worker loop to exit.
    pub fn stop(mut self) {
        self.running.store(false, Ordering::SeqCst);
        self.server.unblock();
        if let Some(th) = self.thread.take() {
            let _ = th.join();
        }
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        self.server.unblock();
        if let Some(th) = self.thread.take() {
            let _ = th.join();
        }
    }
}

// ── Per-User Bridge Token & Private Storage ───────────────────────────────────

/// Return the per-user local data directory for EqnTide state/tokens.
pub fn user_data_dir() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("com", "eqntide", "EqnTide")
        .map(|dirs| dirs.data_local_dir().to_path_buf())
}

/// Return the path to the per-user bridge authentication token file.
pub fn bridge_token_path() -> Option<std::path::PathBuf> {
    user_data_dir().map(|dir| dir.join("bridge.token"))
}

fn generate_random_token() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

fn is_valid_token_format(token: &str) -> bool {
    token.len() >= 16 && token.bytes().all(|b| b.is_ascii_graphic())
}

fn ensure_private_dir(dir: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
    }
    Ok(())
}

fn write_private_file(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    ensure_private_dir(parent)?;
    let temp_path = parent.join(format!(".tmp-{}", uuid::Uuid::new_v4().simple()));
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let write_res = (|| -> std::io::Result<()> {
        let mut f = opts.open(&temp_path)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&temp_path, std::fs::Permissions::from_mode(0o600));
        }
        std::fs::rename(&temp_path, path)?;
        Ok(())
    })();
    if write_res.is_err() {
        let _ = std::fs::remove_file(&temp_path);
    }
    write_res
}

fn wps_jsaddons_install_dirs() -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    #[cfg(target_os = "windows")]
    if let Some(appdata) = std::env::var_os("APPDATA") {
        let base = std::path::PathBuf::from(appdata).join("Kingsoft/wps/jsaddons");
        out.push(base.join("EqnTide"));
        out.push(base.join("EqnTide_1.0.0"));
    }
    #[cfg(not(target_os = "windows"))]
    if let Some(home) = std::env::var_os("HOME") {
        let home = std::path::PathBuf::from(home);
        for sub in [
            ".local/share/Kingsoft/wps/jsaddons",
            ".local/share/Kingsoft/wps_intl/jsaddons",
        ] {
            let base = home.join(sub);
            out.push(base.join("EqnTide"));
            out.push(base.join("EqnTide_1.0.0"));
        }
    }
    out
}

fn sync_wps_addin_token(token: &str) {
    let payload = format!(
        "// Generated per-user EqnTide bridge token\nwindow.EQNTIDE_BRIDGE_TOKEN = \"{token}\";\n"
    );
    for dir in wps_jsaddons_install_dirs() {
        if dir.is_dir() {
            let target = dir.join("bridge-token.js");
            let _ = write_private_file(&target, payload.as_bytes());
        }
    }
}

/// Load or create the persistent per-user bridge token inside `dir`.
pub fn load_or_create_bridge_token_in(dir: &std::path::Path) -> crate::error::Result<String> {
    ensure_private_dir(dir)?;
    let path = dir.join("bridge.token");
    if let Ok(raw) = std::fs::read_to_string(&path) {
        let trimmed = raw.trim();
        if is_valid_token_format(trimmed) {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
            }
            return Ok(trimmed.to_owned());
        }
    }
    let token = generate_random_token();
    write_private_file(&path, format!("{token}\n").as_bytes())?;
    Ok(token)
}

/// Load or create the persistent per-user bridge authentication token and sync it
/// to any installed per-user WPS JS Add-in directory.
pub fn load_or_create_bridge_token() -> crate::error::Result<String> {
    let dir = user_data_dir().ok_or_else(|| {
        crate::error::Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "could not resolve per-user EqnTide data directory",
        ))
    })?;
    let token = load_or_create_bridge_token_in(&dir)?;
    sync_wps_addin_token(&token);
    Ok(token)
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (&x, &y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

// ── Security Hardening: Host Validation & Origin Whitelist ──────────────────────

/// Validate that the `Host` header resolves strictly to local loopback (anti-DNS Rebinding).
pub fn is_valid_host(host: &str) -> bool {
    let host_trimmed = host.trim();
    let host_part = if host_trimmed.starts_with('[') {
        if let Some(end) = host_trimmed.find(']') {
            &host_trimmed[1..end]
        } else {
            host_trimmed
        }
    } else {
        host_trimmed.split(':').next().unwrap_or(host_trimmed)
    };

    matches!(host_part, "127.0.0.1" | "localhost" | "::1")
}

/// Validate that the `Origin` header matches trusted localhost, HTTPS Office Add-in, or WPS origins.
pub fn is_allowed_origin(origin: &str) -> bool {
    let origin = origin.trim();
    if origin.is_empty() || origin.eq_ignore_ascii_case("null") {
        return false;
    }
    // Custom/local schemes used by WPS Add-in WebViews
    if origin.starts_with("file://") || origin.starts_with("wps-addon:") {
        return true;
    }

    let Some((scheme, rest)) = origin.split_once("://") else {
        return false;
    };
    let host_and_port = rest.split('/').next().unwrap_or("").trim();
    let host = if host_and_port.starts_with('[') {
        if let Some(end) = host_and_port.find(']') {
            &host_and_port[1..end]
        } else {
            host_and_port
        }
    } else {
        host_and_port.split(':').next().unwrap_or(host_and_port)
    };

    // 1. Localhost origins (HTTP or HTTPS)
    if matches!(host, "127.0.0.1" | "localhost" | "::1") {
        return scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https");
    }

    // Remote Office/WPS origins MUST use HTTPS
    if !scheme.eq_ignore_ascii_case("https") {
        return false;
    }

    // 2. Microsoft Office Add-in origins
    if host.ends_with(".office.com")
        || host == "office.com"
        || host.ends_with(".office365.com")
        || host == "office365.com"
        || host.ends_with(".microsoft.com")
        || host == "microsoft.com"
        || host.ends_with(".live.com")
        || host == "live.com"
        || host.ends_with(".officeapps.live.com")
    {
        return true;
    }

    // 3. WPS Office Add-in origins
    if host.ends_with(".wps.cn")
        || host == "wps.cn"
        || host.ends_with(".wps.com")
        || host == "wps.com"
        || host.ends_with(".kdocs.cn")
        || host == "kdocs.cn"
    {
        return true;
    }

    false
}

fn get_header<'a>(req: &'a Request, name: &str) -> Option<&'a str> {
    req.headers()
        .iter()
        .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(name))
        .map(|h| h.value.as_str())
}

/// Attach standard CORS headers and Private Network Access (PNA) header to a response.
fn add_cors_headers(response: &mut HttpResponse, origin: Option<&str>) {
    let mut headers: Vec<(&str, &str)> = vec![
        (
            "Access-Control-Allow-Methods",
            "GET, POST, OPTIONS, PUT, DELETE",
        ),
        (
            "Access-Control-Allow-Headers",
            "Content-Type, Authorization, X-Requested-With, X-EqnTide-Token, Access-Control-Request-Private-Network",
        ),
        ("Access-Control-Allow-Private-Network", "true"),
        ("Access-Control-Max-Age", "86400"),
        ("Vary", "Origin"),
    ];
    if let Some(o) = origin.filter(|o| !o.is_empty() && !o.eq_ignore_ascii_case("null")) {
        headers.insert(0, ("Access-Control-Allow-Origin", o));
    }
    for (name, val) in headers {
        if let Ok(h) = Header::from_bytes(name.as_bytes(), val.as_bytes()) {
            response.add_header(h);
        }
    }
}

fn json_response<T: serde::Serialize>(
    status_code: u16,
    data: &T,
    origin: Option<&str>,
) -> HttpResponse {
    let body = serde_json::to_vec(data).unwrap_or_default();
    let mut resp = Response::from_data(body).with_status_code(StatusCode(status_code));
    if let Ok(h) = Header::from_bytes(
        &b"Content-Type"[..],
        &b"application/json; charset=utf-8"[..],
    ) {
        resp.add_header(h);
    }
    add_cors_headers(&mut resp, origin);
    resp
}

fn error_response(status_code: u16, msg: &str, origin: Option<&str>) -> HttpResponse {
    json_response(
        status_code,
        &ErrorResponse {
            status: "error".into(),
            error: msg.into(),
        },
        origin,
    )
}

fn empty_cors_response(status_code: u16, origin: Option<&str>) -> HttpResponse {
    let mut resp = Response::from_data(Vec::new()).with_status_code(StatusCode(status_code));
    add_cors_headers(&mut resp, origin);
    resp
}

enum BodyReadError {
    TooLarge,
    Io,
}

fn read_body(req: &mut Request) -> Result<String, BodyReadError> {
    use std::io::Read;
    let mut raw = Vec::new();
    let mut limited = req.as_reader().take((MAX_BODY_BYTES + 1) as u64);
    limited
        .read_to_end(&mut raw)
        .map_err(|_| BodyReadError::Io)?;
    if raw.len() > MAX_BODY_BYTES {
        return Err(BodyReadError::TooLarge);
    }
    String::from_utf8(raw).map_err(|_| BodyReadError::Io)
}

fn body_error_response(err: BodyReadError, origin: Option<&str>) -> HttpResponse {
    match err {
        BodyReadError::TooLarge => error_response(413, "payload too large", origin),
        BodyReadError::Io => {
            error_response(400, "invalid or unreadable UTF-8 request body", origin)
        }
    }
}

fn parse_query_param<'a>(query: &'a str, key: &str) -> Option<&'a str> {
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k == key {
                return Some(v);
            }
        }
    }
    None
}

fn is_cors_preflight(method: &Method) -> bool {
    *method == Method::Options
}

fn respond(req: Request, response: HttpResponse) {
    let _ = req.respond(response);
}

fn health_response(origin: Option<&str>) -> HttpResponse {
    json_response(
        200,
        &HealthResponse {
            status: "ok".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            server: "eqntide-bridge".into(),
        },
        origin,
    )
}

fn handle_window_show(
    req: &mut Request,
    session_mgr: &SessionManager,
    window_cb: Option<&WindowRequestCallback>,
    origin: Option<&str>,
) -> HttpResponse {
    let body = match read_body(req) {
        Ok(b) => b,
        Err(err) => return body_error_response(err, origin),
    };
    let show_req: WindowShowRequest = serde_json::from_str(&body).unwrap_or_default();
    let session_id = show_req
        .session_id
        .filter(|id| !id.is_empty())
        .unwrap_or_default();
    eprintln!("POST /window/show for session_id='{session_id}'");

    let (mathml, latex, font_pt) = if session_id.is_empty() {
        (None, None, None)
    } else {
        session_mgr
            .get_session(&session_id)
            .map(|(mathml, latex, font_pt)| (Some(mathml), latex, font_pt))
            .unwrap_or((None, None, None))
    };
    if let Some(callback) = window_cb {
        callback(DesktopWindowRequest {
            session_id: session_id.clone(),
            mathml,
            latex,
            font_pt,
            tab: show_req.tab.clone(),
            target_mode: session_mgr.metadata(&session_id).map(|m| m.0),
        });
    } else {
        spawn_desktop_action(&session_id, show_req.tab.as_deref());
    }
    json_response(
        200,
        &serde_json::json!({ "status": "ok", "session_id": session_id }),
        origin,
    )
}

fn is_safe_relative_addin_path(rel_path: &str) -> bool {
    if rel_path.is_empty()
        || rel_path.contains('\0')
        || rel_path.contains('%')
        || rel_path.contains('\\')
        || rel_path.contains(':')
        || rel_path.starts_with('/')
    {
        return false;
    }
    let path = std::path::Path::new(rel_path);
    let mut has_normal = false;
    for comp in path.components() {
        match comp {
            std::path::Component::Normal(seg) => {
                let s = seg.to_string_lossy();
                if s.starts_with('.') || s.eq_ignore_ascii_case("bridge-token.js") {
                    return false;
                }
                has_normal = true;
            }
            _ => return false,
        }
    }
    has_normal
}

/// Serve static files for Office Word/WPS Add-ins directly from the bridge server.
fn serve_addin_file(folder: &str, rel_path: &str, origin: Option<&str>) -> Option<HttpResponse> {
    if !is_safe_relative_addin_path(rel_path) {
        return None;
    }

    // Installed resources belong to the binary's version, even on build hosts
    // where the original repository path still exists.
    let mut base_candidates = Vec::new();
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            base_candidates.push(parent.join(folder));
        }
    }
    base_candidates.push(std::path::PathBuf::from(folder));
    base_candidates.push(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(folder));

    let mut found_bytes = None;
    let mut resolved_path = None;

    for base in &base_candidates {
        let Ok(canon_base) = std::fs::canonicalize(base) else {
            continue;
        };
        let candidate = canon_base.join(rel_path);
        let Ok(canon_candidate) = std::fs::canonicalize(&candidate) else {
            continue;
        };
        if !canon_candidate.starts_with(&canon_base) || !canon_candidate.is_file() {
            continue;
        }
        if let Ok(bytes) = std::fs::read(&canon_candidate) {
            found_bytes = Some(bytes);
            resolved_path = Some(canon_candidate);
            break;
        }
    }

    let bytes = found_bytes?;
    let path_ref = resolved_path?;
    let ext = path_ref
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let mime = match ext.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "js" | "mjs" => "application/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "xml" => "application/xml; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "svg" => "image/svg+xml",
        "json" => "application/json",
        _ => "application/octet-stream",
    };

    let mut resp = Response::from_data(bytes).with_status_code(StatusCode(200));
    if let Ok(h) = Header::from_bytes(&b"Content-Type"[..], mime.as_bytes()) {
        resp.add_header(h);
    }
    if let Ok(h) = Header::from_bytes(&b"Cache-Control"[..], &b"no-cache"[..]) {
        resp.add_header(h);
    }
    add_cors_headers(&mut resp, origin);
    Some(resp)
}

fn serve_addin_route(path: &str, origin: Option<&str>) -> Option<HttpResponse> {
    let (folder, rel_path) = match path {
        path if path.starts_with("/wps-addin/") => {
            ("wps-addin", path.strip_prefix("/wps-addin/").unwrap_or(""))
        }
        _ => return None,
    };
    Some(
        serve_addin_file(folder, rel_path, origin)
            .unwrap_or_else(|| error_response(404, "file not found", origin)),
    )
}

/// Start the local HTTP bridge server on `addr` (e.g. `"127.0.0.1:17890"`).
pub fn start_bridge_server(addr: &str) -> crate::error::Result<ServerHandle> {
    start_bridge_server_with_callback(addr, None)
}

/// Start the local HTTP bridge server with an optional window-show callback.
pub fn start_bridge_server_with_callback(
    addr: &str,
    window_cb: Option<WindowCallback>,
) -> crate::error::Result<ServerHandle> {
    let callback = window_cb.map(|old| {
        Arc::new(move |r: DesktopWindowRequest| {
            old(
                &r.session_id,
                r.mathml.as_deref(),
                r.latex.as_deref(),
                r.font_pt,
            );
        }) as WindowRequestCallback
    });
    start_bridge_server_with_window_requests(addr, callback)
}

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
    let server = Server::http(addr).map_err(|e| {
        crate::error::Error::Io(std::io::Error::other(format!(
            "failed to bind HTTP server to {addr}: {e}"
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
                Err(e) => {
                    log::debug!("HTTP server recv ended: {e}");
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
        Some((p, q)) => (p, Some(q)),
        None => (raw_url.as_str(), None),
    };

    // 1. Host Header Validation (anti-DNS Rebinding, including HTTP/1.0 without Host)
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

    // 2. Origin Header Validation (anti-CSRF from untrusted websites)
    let origin_owned = get_header(&req, "Origin").map(|s| s.to_string());
    if let Some(ref orig) = origin_owned {
        if !is_allowed_origin(orig) {
            log::warn!("Rejected request from untrusted Origin: {orig}");
            let _ = req.respond(error_response(403, "forbidden: untrusted Origin", None));
            return;
        }
    }
    let origin = origin_owned.as_deref();

    // 3. CORS Preflight
    if is_cors_preflight(&method) {
        let resp = empty_cors_response(204, origin);
        let _ = req.respond(resp);
        return;
    }

    // 4. Public read-only routes (health check & static add-in assets)
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

    // 5. Per-user Authentication Token Validation for all API routes
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

    // 6. Routing
    match (method, path) {
        (Method::Post, "/window/show") => {
            let response = handle_window_show(&mut req, &session_mgr, window_cb.as_ref(), origin);
            respond(req, response)
        }

        (Method::Post, "/log") => {
            let body = match read_body(&mut req) {
                Ok(b) => b,
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

        // Compatibility for installed VBA templates using the original route.
        (Method::Post, "/convert/snippet" | "/convert") => handle_snippet_conversion(req, origin),

        (Method::Post, "/convert/document") => handle_document_conversion(req, origin),

        _ => {
            let resp = error_response(404, "not found", origin);
            let _ = req.respond(resp);
        }
    }
}

/// Resolve the complete session lifecycle before the general route dispatcher.
fn handle_session_route(
    req: &mut Request,
    method: &Method,
    path: &str,
    query: Option<&str>,
    session_mgr: &SessionManager,
    window_cb: Option<&WindowRequestCallback>,
    origin: Option<&str>,
) -> Option<HttpResponse> {
    match (method, path) {
        (Method::Get, p)
            if p.starts_with("/session/")
                && !p.ends_with("/wait")
                && !p.ends_with("/update")
                && !p.ends_with("/submit")
                && !p.ends_with("/cancel")
                && !p.ends_with("/close")
                && p != "/session/latest" =>
        {
            let sid = p.strip_prefix("/session/").unwrap_or("");
            Some(match session_mgr.get_session(sid) {
                Some((mathml, latex, font_pt)) => json_response(
                    200,
                    &serde_json::json!({
                        "status": "ok", "session_id": sid, "mathml": mathml,
                        "latex": latex, "font_pt": font_pt,
                        "target_mode": session_mgr.metadata(sid).map(|m|m.0),
                        "state": session_mgr.metadata(sid).map(|m|m.1),
                    }),
                    origin,
                ),
                None => error_response(404, "session not found", origin),
            })
        }
        (Method::Get, "/session/latest") => {
            let active = session_mgr.get_latest_session_id();
            let font_pt = active
                .as_deref()
                .and_then(|id| session_mgr.get_session(id))
                .and_then(|(_, _, f)| f);
            Some(json_response(
                200,
                &serde_json::json!({ "status": "ok", "session_id": active, "font_pt": font_pt }),
                origin,
            ))
        }
        (Method::Post, "/session/open") => {
            Some(handle_session_open(req, session_mgr, window_cb, origin))
        }
        (Method::Get, p) if p.starts_with("/session/") && p.ends_with("/wait") => {
            Some(handle_session_wait(path, query, session_mgr, origin))
        }
        (Method::Post, p)
            if p.starts_with("/session/") && (p.ends_with("/update") || p.ends_with("/submit")) =>
        {
            Some(handle_session_update(req, path, session_mgr, origin))
        }
        (Method::Post, p)
            if p.starts_with("/session/") && (p.ends_with("/close") || p.ends_with("/cancel")) =>
        {
            Some(handle_session_cancel(path, session_mgr, origin))
        }
        _ => None,
    }
}

fn handle_session_open(
    req: &mut Request,
    session_mgr: &SessionManager,
    window_cb: Option<&WindowRequestCallback>,
    origin: Option<&str>,
) -> HttpResponse {
    session_mgr.cleanup_stale(SESSION_MAX_AGE);
    let body = match read_body(req) {
        Ok(b) => b,
        Err(err) => return body_error_response(err, origin),
    };
    let open_req: SessionOpenRequest = match serde_json::from_str(&body) {
        Ok(request) => request,
        Err(error) => {
            return error_response(400, &format!("invalid JSON payload: {error}"), origin)
        }
    };
    let trimmed_data = open_req.data.trim();
    eprintln!(
        "POST /session/open received payload len={}",
        trimmed_data.len()
    );
    log::info!(
        "POST /session/open received payload len={}",
        trimmed_data.len()
    );
    let is_latex = open_req.format == "latex"
        || trimmed_data.starts_with('$')
        || trimmed_data.starts_with('\\')
        || trimmed_data.contains("\\frac")
        || trimmed_data.contains("\\sqrt");
    let (mathml, latex) = if is_latex {
        let clean = trimmed_data.trim_matches('$').trim().to_string();
        let mathml = parse_latex_to_mathml(trimmed_data)
            .ok()
            .map(|math| math.to_xml())
            .unwrap_or_default();
        (mathml, Some(clean))
    } else if trimmed_data.is_empty() {
        (String::new(), None)
    } else {
        match decode_input_to_mathml(trimmed_data, &open_req.format) {
            Ok(math) => {
                let latex = math.to_latex();
                eprintln!(
                    "POST /session/open successfully parsed equation to latex: '{}'",
                    latex
                );
                (math.to_xml(), Some(latex))
            }
            Err(error) => {
                eprintln!("POST /session/open could not parse equation data: {error}");
                log::warn!("Could not parse equation data in /session/open ({error}), opening session as empty canvas");
                (String::new(), None)
            }
        }
    };
    let session_id = session_mgr.create_session_with_target(
        mathml.clone(),
        latex.clone(),
        open_req.font_pt,
        &open_req.target_mode,
    );
    if let Some(callback) = window_cb {
        callback(DesktopWindowRequest {
            session_id: session_id.clone(),
            mathml: Some(mathml.clone()),
            latex: latex.clone(),
            font_pt: open_req.font_pt,
            tab: None,
            target_mode: Some(open_req.target_mode.clone()),
        });
    } else {
        spawn_desktop_editor(&session_id);
    }
    json_response(
        200,
        &SessionOpenResponse {
            status: "opened".into(),
            session_id,
            mathml,
            latex,
            font_pt: open_req.font_pt,
        },
        origin,
    )
}

fn handle_session_wait(
    path: &str,
    query: Option<&str>,
    session_mgr: &SessionManager,
    origin: Option<&str>,
) -> HttpResponse {
    let session_id = path
        .strip_prefix("/session/")
        .and_then(|path| path.strip_suffix("/wait"))
        .unwrap_or("");
    if session_id.is_empty() {
        return error_response(400, "missing session id in path", origin);
    }
    let timeout_secs = query
        .and_then(|query| parse_query_param(query, "timeout"))
        .and_then(|timeout| timeout.parse::<u64>().ok())
        .unwrap_or(300)
        .clamp(1, 300);
    match session_mgr.wait_session(session_id, Duration::from_secs(timeout_secs)) {
        Some(SessionState::Ready {
            mathml,
            flat_opc,
            omml,
            emf_path,
            width_pt,
            height_pt,
            depth_pt,
            font_pt,
        }) => json_response(
            200,
            &SessionWaitResponse {
                status: "ready".into(),
                session_id: session_id.into(),
                mathml: Some(mathml),
                flat_opc: Some(flat_opc),
                omml: Some(omml),
                emf_path,
                width_pt,
                height_pt,
                depth_pt,
                font_pt,
            },
            origin,
        ),
        Some(SessionState::Cancelled) => {
            session_wait_empty_response("cancelled", session_id, origin)
        }
        Some(SessionState::Waiting) | None => {
            session_wait_empty_response("timeout", session_id, origin)
        }
    }
}

fn handle_session_update(
    req: &mut Request,
    path: &str,
    session_mgr: &SessionManager,
    origin: Option<&str>,
) -> HttpResponse {
    let session_id = path
        .strip_prefix("/session/")
        .and_then(|path| {
            path.strip_suffix("/update")
                .or_else(|| path.strip_suffix("/submit"))
        })
        .unwrap_or("");
    if session_id.is_empty() {
        return error_response(400, "missing session id in path", origin);
    }
    if uuid::Uuid::parse_str(session_id).is_err() || session_mgr.get_session(session_id).is_none() {
        return error_response(404, "session not found", origin);
    }
    let body = match read_body(req) {
        Ok(b) => b,
        Err(err) => return body_error_response(err, origin),
    };
    let update: SessionUpdateRequest = match serde_json::from_str(&body) {
        Ok(update) => update,
        Err(error) => {
            return error_response(400, &format!("invalid JSON payload: {error}"), origin)
        }
    };
    if !update.font_pt.is_finite() || !(1.0..=200.0).contains(&update.font_pt) {
        return error_response(
            400,
            "font_pt must be a finite number between 1.0 and 200.0",
            origin,
        );
    }
    let math = match MathNode::parse(&update.mathml) {
        Ok(math) => math,
        Err(error) => {
            return error_response(422, &format!("failed to parse MathML: {error}"), origin)
        }
    };
    let is_mathtype = update.target_mode.eq_ignore_ascii_case("mathtype");
    if is_mathtype {
        if let Some(reason) = crate::mtef::check_mathtype_unsupported(&math) {
            return error_response(
                422,
                &format!("{reason} is not supported in MathType"),
                origin,
            );
        }
    } else if let Some(reason) = crate::omml::check_omml_unsupported(&math) {
        return error_response(422, &format!("{reason} is not supported in OMML"), origin);
    }
    let mode = if is_mathtype {
        FlatOpcMode::ModernMathtypeOleEmf
    } else {
        FlatOpcMode::ModernOmml
    };
    let flat_opc = match mathml_to_flat_opc(&math, mode, update.font_pt) {
        Ok(flat_opc) => flat_opc,
        Err(error) => {
            return error_response(500, &format!("failed to package Flat OPC: {error}"), origin)
        }
    };
    let omml = match mathml_to_omml_xml(&math) {
        Ok(omml) => omml,
        Err(error) => {
            return error_response(500, &format!("failed to generate OMML: {error}"), origin)
        }
    };
    // WPS prefers emf_path over flat_opc. Supplying a picture for OMML
    // silently turns a native equation into a MathType picture on insertion.
    let picture = if update.target_mode.eq_ignore_ascii_case("mathtype") {
        let mtef = mathml_to_mtef_bytes(&math)
            .ok()
            .map(|records| crate::mtef::build_mtef(&records));
        render_emf_with_mtef(&math, update.font_pt, mtef.as_deref()).ok()
    } else {
        None
    };
    let (emf_path, width_pt, height_pt, depth_pt) = if let Some(picture) = picture {
        let path = std::env::temp_dir().join(format!("eqntide_{session_id}.emf"));
        let path = path.to_string_lossy().into_owned();
        if let Err(error) = std::fs::write(&path, &picture.emf) {
            return error_response(500, &format!("failed to write EMF: {error}"), origin);
        }
        (
            Some(path),
            Some(picture.width_pt),
            Some(picture.height_pt),
            Some(picture.depth_pt),
        )
    } else {
        (None, None, None, None)
    };
    if let Err(error) = session_mgr.update_session(
        session_id,
        update.mathml,
        flat_opc,
        omml,
        emf_path,
        width_pt,
        height_pt,
        depth_pt,
        Some(update.font_pt),
    ) {
        return error_response(404, error, origin);
    }
    json_response(
        200,
        &SessionActionResponse {
            status: "ok".into(),
            session_id: session_id.into(),
        },
        origin,
    )
}

fn session_wait_empty_response(
    status: &str,
    session_id: &str,
    origin: Option<&str>,
) -> HttpResponse {
    json_response(
        200,
        &SessionWaitResponse {
            status: status.into(),
            session_id: session_id.into(),
            mathml: None,
            flat_opc: None,
            omml: None,
            emf_path: None,
            width_pt: None,
            height_pt: None,
            depth_pt: None,
            font_pt: None,
        },
        origin,
    )
}

fn handle_session_cancel(
    path: &str,
    session_mgr: &SessionManager,
    origin: Option<&str>,
) -> HttpResponse {
    let session_id = path
        .strip_prefix("/session/")
        .and_then(|path| {
            path.strip_suffix("/close")
                .or_else(|| path.strip_suffix("/cancel"))
        })
        .unwrap_or("");
    if let Err(error) = session_mgr.cancel_session(session_id) {
        return error_response(404, error, origin);
    }
    json_response(
        200,
        &SessionActionResponse {
            status: "cancelled".into(),
            session_id: session_id.into(),
        },
        origin,
    )
}

/// Convert one equation snippet without coupling conversion details to routing.
fn handle_snippet_conversion(mut req: Request, origin: Option<&str>) {
    let body = match read_body(&mut req) {
        Ok(b) => b,
        Err(err) => {
            let _ = req.respond(body_error_response(err, origin));
            return;
        }
    };
    let result = serde_json::from_str::<ConvertSnippetRequest>(&body);
    let response = match result {
        Ok(input) => match crate::service::convert_snippet(&input) {
            Ok(output) => json_response(200, &output, origin),
            Err(e) => error_response(e.status, &e.message, origin),
        },
        Err(e) => error_response(400, &format!("invalid JSON payload: {e}"), origin),
    };
    let _ = req.respond(response);
}

fn handle_document_conversion(mut req: Request, origin: Option<&str>) {
    let body = match read_body(&mut req) {
        Ok(b) => b,
        Err(err) => {
            let _ = req.respond(body_error_response(err, origin));
            return;
        }
    };
    let result = serde_json::from_str::<ConvertDocumentRequest>(&body);
    let response = match result {
        Ok(input) => match crate::service::convert_document(&input) {
            Ok(output) => json_response(200, &output, origin),
            Err(e) => error_response(e.status, &e.message, origin),
        },
        Err(e) => error_response(400, &format!("invalid JSON payload: {e}"), origin),
    };
    let _ = req.respond(response);
}

pub(crate) fn spawn_desktop_editor(session_id: &str) {
    spawn_desktop_action(session_id, None);
}

fn reap_desktop_child(mut child: std::process::Child) -> JoinHandle<()> {
    // Dropping Child does not reap it. Wait off the request thread so both
    // short-lived IPC launchers and full desktop sessions cannot leave zombies.
    std::thread::spawn(move || {
        if let Err(error) = child.wait() {
            log::warn!("Failed to reap desktop process {}: {error}", child.id());
        }
    })
}

fn spawn_desktop_action(session_id: &str, tab: Option<&str>) {
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
            Err(e) => {
                log::debug!("Failed to spawn {}: {e}", path.display());
            }
        }
    }
    log::warn!("Could not find or launch EqnTide UI binary");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::net::TcpStream;

    #[cfg(target_os = "linux")]
    #[test]
    fn repeated_desktop_child_exits_are_reaped() {
        let mut children = Vec::new();
        for _ in 0..8 {
            let child = std::process::Command::new("/bin/sh")
                .args(["-c", "exit 0"])
                .spawn()
                .unwrap();
            children.push((child.id(), reap_desktop_child(child)));
        }
        for (pid, reaper) in children {
            reaper.join().unwrap();
            assert!(
                !std::path::Path::new(&format!("/proc/{pid}")).exists(),
                "child {pid} must be reaped, not left as a zombie"
            );
        }
    }

    struct RawHttpResponse {
        status: u16,
        headers: HashMap<String, String>,
        body: String,
    }

    fn send_raw_http(addr: SocketAddr, req: &str) -> RawHttpResponse {
        let mut stream = TcpStream::connect(addr).expect("connect to test server");
        stream.write_all(req.as_bytes()).expect("write request");
        let mut response_bytes = Vec::new();
        stream
            .read_to_end(&mut response_bytes)
            .expect("read response");
        let response_str = String::from_utf8_lossy(&response_bytes);

        let (head, body) = response_str
            .split_once("\r\n\r\n")
            .unwrap_or((&response_str, ""));
        let mut lines = head.lines();
        let status_line = lines.next().unwrap_or("");
        let status: u16 = status_line
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let mut headers = HashMap::new();
        for line in lines {
            if let Some((k, v)) = line.split_once(':') {
                headers.insert(k.trim().to_lowercase(), v.trim().to_string());
            }
        }

        RawHttpResponse {
            status,
            headers,
            body: body.to_string(),
        }
    }

    #[test]
    fn origin_policy_after_legacy_desktop_removal() {
        for origin in [
            "",
            "null",
            "NULL",
            "tauri://app",
            "http://tauri.localhost",
            "vscode-webview://123",
            "http://appsforoffice.microsoft.com",
            "http://wps.cn",
        ] {
            assert!(!is_allowed_origin(origin), "should reject origin: {origin}");
        }
        for origin in [
            "http://127.0.0.1:17890",
            "http://localhost:3000",
            "https://appsforoffice.microsoft.com",
            "https://www.wps.cn",
            "wps-addon://eqntide",
        ] {
            assert!(is_allowed_origin(origin), "should allow origin: {origin}");
        }
    }

    #[test]
    fn test_server_health_and_cors() {
        let server_handle = start_bridge_server("127.0.0.1:0").expect("start test server");
        let addr = server_handle.addr();

        // 1. Test OPTIONS preflight (PNA & CORS)
        let opt_req = format!(
            "OPTIONS /health HTTP/1.1\r\nHost: {addr}\r\nOrigin: http://localhost:3000\r\nAccess-Control-Request-Private-Network: true\r\nConnection: close\r\n\r\n"
        );
        let resp = send_raw_http(addr, &opt_req);
        assert_eq!(resp.status, 204);
        assert_eq!(
            resp.headers
                .get("access-control-allow-private-network")
                .map(|s| s.as_str()),
            Some("true")
        );
        assert_eq!(
            resp.headers
                .get("access-control-allow-origin")
                .map(|s| s.as_str()),
            Some("http://localhost:3000")
        );
        assert!(resp
            .headers
            .get("access-control-allow-headers")
            .is_some_and(|h| h.contains("X-EqnTide-Token")));

        // 2. Test GET /health without Origin does not emit wildcard Access-Control-Allow-Origin
        let get_req = format!("GET /health HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
        let resp = send_raw_http(addr, &get_req);
        assert_eq!(resp.status, 200);
        assert!(!resp.headers.contains_key("access-control-allow-origin"));
        let health: HealthResponse = serde_json::from_str(&resp.body).expect("json parse");
        assert_eq!(health.status, "ok");
        assert_eq!(health.server, "eqntide-bridge");
    }

    #[test]
    fn test_security_dns_rebinding_and_missing_host_rejected() {
        let server_handle = start_bridge_server("127.0.0.1:0").expect("start test server");
        let addr = server_handle.addr();

        // Send a request with spoofed/malicious Host header
        let rebinding_req = format!(
            "GET /health HTTP/1.1\r\nHost: evil-attacker.com:{}\r\nConnection: close\r\n\r\n",
            addr.port()
        );
        let resp = send_raw_http(addr, &rebinding_req);
        assert_eq!(resp.status, 403);
        assert!(resp.body.contains("DNS rebinding"));

        // HTTP/1.0 request without Host header must also be rejected
        let no_host_req = "GET /health HTTP/1.0\r\nConnection: close\r\n\r\n";
        let resp = send_raw_http(addr, no_host_req);
        assert_eq!(resp.status, 403);
        assert!(resp.body.contains("missing Host"));
    }

    #[test]
    fn test_security_untrusted_and_null_origin_rejected() {
        let server_handle = start_bridge_server("127.0.0.1:0").expect("start test server");
        let addr = server_handle.addr();

        for bad_origin in ["https://malicious-website.xyz", "null", "http://office.com"] {
            let req = format!(
                "GET /health HTTP/1.1\r\nHost: {addr}\r\nOrigin: {bad_origin}\r\nConnection: close\r\n\r\n"
            );
            let resp = send_raw_http(addr, &req);
            assert_eq!(resp.status, 403, "origin={bad_origin}");
            assert!(resp.body.contains("untrusted Origin"));
            assert!(!resp.headers.contains_key("access-control-allow-origin"));
        }
    }

    #[test]
    fn test_security_office_add_in_origin_allowed() {
        let server_handle = start_bridge_server("127.0.0.1:0").expect("start test server");
        let addr = server_handle.addr();

        // Send request from official Microsoft Office Add-in host
        let req = format!(
            "GET /health HTTP/1.1\r\nHost: {addr}\r\nOrigin: https://appsforoffice.microsoft.com\r\nConnection: close\r\n\r\n"
        );
        let resp = send_raw_http(addr, &req);
        assert_eq!(resp.status, 200);
        assert_eq!(
            resp.headers
                .get("access-control-allow-origin")
                .map(|s| s.as_str()),
            Some("https://appsforoffice.microsoft.com")
        );
    }

    #[test]
    fn test_session_lifecycle_longpoll() {
        let server_handle = start_bridge_server("127.0.0.1:0").expect("start test server");
        let addr = server_handle.addr();
        let token = server_handle.auth_token().to_owned();

        // 1. Open a new session with OMML input
        let raw_omml = r#"<w:p xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" xmlns:m=\"http://schemas.openxmlformats.org/officeDocument/2006/math\"><m:oMath><m:r><m:t>x+1</m:t></m:r></m:oMath></w:p>"#;
        let open_payload = format!("{{\"data\": \"{}\", \"format\": \"omml\"}}", raw_omml);
        let open_req = format!(
            "POST /session/open HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            open_payload.len(),
            open_payload
        );
        let resp = send_raw_http(addr, &open_req);
        assert_eq!(resp.status, 200);
        let open_res: SessionOpenResponse = serde_json::from_str(&resp.body).expect("json parse");
        assert_eq!(open_res.status, "opened");
        let session_id = open_res.session_id;

        // 2. In background thread, perform the long-poll wait
        let sid_clone = session_id.clone();
        let token_clone = token.clone();
        let wait_handle = std::thread::spawn(move || {
            let wait_req = format!(
                "GET /session/{sid_clone}/wait?timeout=10 HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token_clone}\r\nConnection: close\r\n\r\n"
            );
            let resp = send_raw_http(addr, &wait_req);
            let wait_res: SessionWaitResponse =
                serde_json::from_str(&resp.body).expect("json parse");
            wait_res
        });

        // 3. Update the session from the editor side
        std::thread::sleep(Duration::from_millis(50));
        let upd_payload =
            r#"{"mathml": "<math><mi>y</mi><mo>=</mo><mn>2</mn></math>", "target_mode": "omml"}"#;
        let upd_req = format!(
            "POST /session/{session_id}/update HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            upd_payload.len(),
            upd_payload
        );
        let resp = send_raw_http(addr, &upd_req);
        assert_eq!(resp.status, 200);
        let upd_res: SessionActionResponse = serde_json::from_str(&resp.body).expect("json parse");
        assert_eq!(upd_res.status, "ok");

        // 4. Verify the waiting thread woke up with the result
        let wait_result = wait_handle.join().expect("thread join");
        assert_eq!(wait_result.status, "ready");
        assert!(wait_result
            .flat_opc
            .as_ref()
            .unwrap()
            .contains("pkg:package"));
        assert!(wait_result.omml.as_ref().unwrap().contains("m:oMath"));
    }

    #[test]
    fn session_output_honors_the_saved_mode_for_insert_and_edit() {
        let server =
            start_bridge_server_with_window_requests("127.0.0.1:0", Some(Arc::new(|_| {})))
                .unwrap();
        let addr = server.addr();
        let token = server.auth_token().to_owned();
        for (initial, saved, data) in [
            ("mathtype", "omml", ""),
            ("mathtype", "omml", "<math><mi>x</mi></math>"),
            ("omml", "mathtype", ""),
            ("omml", "mathtype", "<math><mi>x</mi></math>"),
        ] {
            let body = serde_json::json!({"data":data,"format":"mathml","target_mode":initial})
                .to_string();
            let response = send_raw_http(addr, &format!(
                "POST /session/open HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()));
            assert_eq!(response.status, 200);
            let opened: SessionOpenResponse = serde_json::from_str(&response.body).unwrap();
            let sid = opened.session_id;
            let body = serde_json::json!({"mathml":"<math><mi>y</mi></math>","target_mode":saved})
                .to_string();
            let response = send_raw_http(addr, &format!(
                "POST /session/{sid}/update HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()));
            assert_eq!(response.status, 200);
            let response = send_raw_http(addr, &format!(
                "GET /session/{sid}/wait?timeout=1 HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nConnection: close\r\n\r\n"));
            assert_eq!(response.status, 200);
            let result: SessionWaitResponse = serde_json::from_str(&response.body).unwrap();
            assert_eq!(result.status, "ready");
            let package = result.flat_opc.unwrap();
            if saved == "omml" {
                assert!(package.contains("m:oMath"));
                assert!(!package.contains("Equation.DSMT4"));
                assert!(result.emf_path.is_none());
                assert!(result.width_pt.is_none());
                assert!(result.height_pt.is_none());
                assert!(result.depth_pt.is_none());
            } else {
                assert!(package.contains("Equation.DSMT4"));
                let path = result.emf_path.expect("MathType must retain EMF insertion");
                let image = std::fs::read(&path).unwrap();
                let mtef = crate::extract_mtef_from_emf(&image).unwrap();
                assert!(crate::mtef::decode::mtef_to_mathml(&mtef).is_some());
                std::fs::remove_file(path).unwrap();
            }
        }
    }

    #[test]
    fn test_convert_snippet_stateless() {
        let server_handle = start_bridge_server("127.0.0.1:0").expect("start test server");
        let addr = server_handle.addr();
        let token = server_handle.auth_token().to_owned();

        let snip_payload = r#"{"data": "<math><mi>a</mi><mo>+</mo><mi>b</mi></math>", "from": "mathml", "to": "flat_opc_omml"}"#;
        let snip_req = format!(
            "POST /convert/snippet HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            snip_payload.len(),
            snip_payload
        );
        let resp = send_raw_http(addr, &snip_req);
        assert_eq!(resp.status, 200);

        let snip_res: ConvertSnippetResponse =
            serde_json::from_str(&resp.body).expect("json parse");
        assert_eq!(snip_res.status, "ok");
        assert!(snip_res.result.contains("pkg:package"));
        assert!(snip_res.result.contains("m:oMath"));

        // Test EMF snippet export
        let emf_payload = r#"{"data": "<math><mi>x</mi><mo>+</mo><mn>1</mn></math>", "from": "mathml", "to": "emf"}"#;
        let emf_req = format!(
            "POST /convert/snippet HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            emf_payload.len(),
            emf_payload
        );
        let resp_emf = send_raw_http(addr, &emf_req);
        assert_eq!(resp_emf.status, 200);
        let emf_res: ConvertSnippetResponse =
            serde_json::from_str(&resp_emf.body).expect("json parse");
        assert_eq!(emf_res.status, "ok");
        assert!(!emf_res.result.is_empty());

        // Test PNG snippet export
        let png_payload = r#"{"data": "<math><mi>x</mi><mo>+</mo><mn>1</mn></math>", "from": "mathml", "to": "png"}"#;
        let png_req = format!(
            "POST /convert/snippet HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            png_payload.len(),
            png_payload
        );
        let resp_png = send_raw_http(addr, &png_req);
        assert_eq!(resp_png.status, 200);
        let png_res: ConvertSnippetResponse =
            serde_json::from_str(&resp_png.body).expect("json parse");
        assert_eq!(png_res.status, "ok");
        assert!(!png_res.result.is_empty());
    }

    #[test]
    fn test_session_open_with_mathml_generates_latex() {
        let server_handle = start_bridge_server("127.0.0.1:0").expect("start test server");
        let addr = server_handle.addr();
        let token = server_handle.auth_token().to_owned();

        let open_payload = r#"{"data": "<math><mi>x</mi><mo>=</mo><mfrac><mi>a</mi><mi>b</mi></mfrac></math>", "format": "mathml", "target_mode": "mathtype"}"#;
        let open_req = format!(
            "POST /session/open HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            open_payload.len(),
            open_payload
        );
        let resp = send_raw_http(addr, &open_req);
        assert_eq!(resp.status, 200);
        let open_res: SessionOpenResponse = serde_json::from_str(&resp.body).expect("json parse");
        assert_eq!(open_res.status, "opened");
        assert_eq!(open_res.latex, Some("x=\\frac{a}{b}".into()));
    }

    #[test]
    fn test_server_serves_addin_files() {
        let server_handle = start_bridge_server("127.0.0.1:0").expect("start test server");
        let addr = server_handle.addr();
        let token = server_handle.auth_token().to_owned();

        // Removed Word web endpoints stay unavailable even on upgraded installs.
        // Word now uses the VBA template and the session/conversion API only.
        for path in [
            "/manifest.xml",
            "/word-addin/manifest.xml",
            "/word-addin/commands.js",
        ] {
            let unauth =
                format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
            assert_eq!(send_raw_http(addr, &unauth).status, 401);

            let request = format!(
                "GET {path} HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nConnection: close\r\n\r\n"
            );
            assert_eq!(send_raw_http(addr, &request).status, 404);
        }

        // 2. WPS Add-in files
        let req_wps = format!(
            "GET /wps-addin/ribbon.xml HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n"
        );
        let resp_wps = send_raw_http(addr, &req_wps);
        assert_eq!(resp_wps.status, 200);
        assert!(resp_wps.body.contains("TabEqnTide"));
        assert!(resp_wps.body.contains("btnInsertEquation"));

        let req_wps_js =
            format!("GET /wps-addin/main.js HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
        let resp_wps_js = send_raw_http(addr, &req_wps_js);
        assert_eq!(resp_wps_js.status, 200);
        assert!(resp_wps_js.body.contains("InsertEquation"));
        assert!(resp_wps_js.body.contains("WordOpenXML"));
    }
    #[test]
    fn native_callback_receives_batch_tab_and_legacy_callback_remains_compatible() {
        let (tx, rx) = std::sync::mpsc::channel();
        let callback: WindowRequestCallback = Arc::new(move |r| {
            tx.send(r).unwrap();
        });
        let server =
            start_bridge_server_with_window_requests("127.0.0.1:0", Some(callback)).unwrap();
        let addr = server.addr();
        let token = server.auth_token().to_owned();
        let body = r#"{"tab":"batch"}"#;
        let response=send_raw_http(addr,&format!("POST /window/show HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()));
        assert_eq!(response.status, 200);
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(2))
                .unwrap()
                .tab
                .as_deref(),
            Some("batch")
        );
        let (tx, rx) = std::sync::mpsc::channel();
        let callback: WindowCallback = Arc::new(move |id, _, _, _| {
            tx.send(id.to_owned()).unwrap();
        });
        let legacy = start_bridge_server_with_callback("127.0.0.1:0", Some(callback)).unwrap();
        let addr = legacy.addr();
        let token = legacy.auth_token().to_owned();
        let response=send_raw_http(addr,&format!("POST /window/show HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}"));
        assert_eq!(response.status, 200);
        assert_eq!(rx.recv_timeout(Duration::from_secs(2)).unwrap(), "");
    }
    #[test]
    fn session_get_preserves_target_and_reports_completed_state() {
        let callback: WindowCallback = Arc::new(|_, _, _, _| {});
        let server = start_bridge_server_with_callback("127.0.0.1:0", Some(callback)).unwrap();
        let token = server.auth_token().to_owned();
        let id = server.session_manager().create_session_with_target(
            "<math><mi>x</mi></math>".into(),
            None,
            Some(14.0),
            "mathtype",
        );
        let addr = server.addr();
        let get = || {
            send_raw_http(
                addr,
                &format!("GET /session/{id} HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nConnection: close\r\n\r\n"),
            )
        };
        let response: serde_json::Value = serde_json::from_str(&get().body).unwrap();
        assert_eq!(response["target_mode"], "mathtype");
        assert_eq!(response["state"], "waiting");
        server.session_manager().cancel_session(&id).unwrap();
        let response: serde_json::Value = serde_json::from_str(&get().body).unwrap();
        assert_eq!(response["state"], "cancelled");
    }
    #[test]
    fn legacy_vba_convert_route_uses_the_native_snippet_service() {
        let server = start_bridge_server("127.0.0.1:0").unwrap();
        let addr = server.addr();
        let token = server.auth_token().to_owned();
        let body = r#"{"data":"x+1","from":"latex","to":"omml"}"#;
        let response=send_raw_http(addr,&format!("POST /convert HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()));
        assert_eq!(response.status, 200);
        let json: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert!(json["result"].as_str().unwrap().contains("oMath"));
    }
    #[test]
    fn bridge_tracks_longpoll_until_its_response_is_written() {
        let server = start_bridge_server("127.0.0.1:0").unwrap();
        let addr = server.addr();
        let token = server.auth_token().to_owned();
        let id = server
            .session_manager()
            .create_session("<math/>".into(), None, None);
        let sid = id.clone();
        let request = std::thread::spawn(move || {
            send_raw_http(addr,&format!("GET /session/{sid}/wait?timeout=5 HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nConnection: close\r\n\r\n"))
        });
        let start = std::time::Instant::now();
        while server.pending_requests() == 0 && start.elapsed() < Duration::from_secs(2) {
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(server.pending_requests(), 1);
        server.session_manager().cancel_session(&id).unwrap();
        assert!(request.join().unwrap().body.contains("cancelled"));
        while server.pending_requests() != 0 && start.elapsed() < Duration::from_secs(2) {
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(server.pending_requests(), 0);
    }
    #[test]
    fn session_update_validates_target_compatibility() {
        let server = start_bridge_server("127.0.0.1:0").unwrap();
        let addr = server.addr();
        let token = server.auth_token().to_owned();

        // 1. OMML session rejects longdiv with 422
        let omml_sid = server.session_manager().create_session_with_target(
            "<math/>".into(),
            None,
            Some(12.0),
            "omml",
        );
        let body = serde_json::json!({
            "mathml": "<math><menclose notation=\"longdiv\"><mn>144</mn></menclose></math>",
            "target_mode": "omml",
            "font_pt": 12.0
        })
        .to_string();
        let resp = send_raw_http(
            addr,
            &format!("POST /session/{omml_sid}/update HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()),
        );
        assert_eq!(resp.status, 422, "body: {}", resp.body);

        // 2. MathType session accepts longdiv with 200
        let mt_sid = server.session_manager().create_session_with_target(
            "<math/>".into(),
            None,
            Some(12.0),
            "mathtype",
        );
        let body_mt = serde_json::json!({
            "mathml": "<math><menclose notation=\"longdiv\"><mn>144</mn></menclose></math>",
            "target_mode": "mathtype",
            "font_pt": 12.0
        })
        .to_string();
        let resp_mt = send_raw_http(
            addr,
            &format!("POST /session/{mt_sid}/update HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body_mt}", body_mt.len()),
        );
        assert_eq!(resp_mt.status, 200, "body: {}", resp_mt.body);
        assert!(server.session_manager().remove_session(&mt_sid));

        // 3. MathType session rejects verticalstrike with 422
        let mt_sid2 = server.session_manager().create_session_with_target(
            "<math/>".into(),
            None,
            Some(12.0),
            "mathtype",
        );
        let body_vs = serde_json::json!({
            "mathml": "<math><menclose notation=\"verticalstrike\"><mi>x</mi></menclose></math>",
            "target_mode": "mathtype",
            "font_pt": 12.0
        })
        .to_string();
        let resp_vs = send_raw_http(
            addr,
            &format!("POST /session/{mt_sid2}/update HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body_vs}", body_vs.len()),
        );
        assert_eq!(resp_vs.status, 422, "body: {}", resp_vs.body);
    }

    #[test]
    fn unauthenticated_and_wrong_token_requests_are_rejected_with_401() {
        let server = start_bridge_server("127.0.0.1:0").unwrap();
        let addr = server.addr();

        for (method, path, body) in [
            ("GET", "/session/latest", ""),
            ("POST", "/session/open", "{}"),
            ("POST", "/window/show", "{}"),
            ("POST", "/log", "hi"),
            ("POST", "/convert/snippet", "{}"),
            ("POST", "/convert/document", "{}"),
        ] {
            let unauth = format!(
                "{method} {path} HTTP/1.1\r\nHost: {addr}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            assert_eq!(send_raw_http(addr, &unauth).status, 401, "{method} {path}");

            let wrong = format!(
                "{method} {path} HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: wrong-token-value\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            assert_eq!(send_raw_http(addr, &wrong).status, 401, "{method} {path}");
        }
    }

    #[test]
    fn addin_static_serving_blocks_traversal_and_drive_prefixes() {
        assert!(!is_safe_relative_addin_path("C:/Windows/win.ini"));
        assert!(!is_safe_relative_addin_path(r"C:\Users\secret.txt"));
        assert!(!is_safe_relative_addin_path("../Cargo.toml"));
        assert!(!is_safe_relative_addin_path("..%2fCargo.toml"));
        assert!(!is_safe_relative_addin_path("bridge-token.js"));
        assert!(!is_safe_relative_addin_path(".hidden"));
        assert!(is_safe_relative_addin_path("main.js"));
        assert!(is_safe_relative_addin_path("assets/icons/tex-32.png"));

        let server = start_bridge_server("127.0.0.1:0").unwrap();
        let addr = server.addr();
        for bad_path in [
            "/wps-addin/C:/Windows/win.ini",
            "/wps-addin/C:%5CWindows%5Cwin.ini",
            "/wps-addin/%2e%2e/Cargo.toml",
            "/wps-addin/../Cargo.toml",
            "/wps-addin/bridge-token.js",
        ] {
            let req =
                format!("GET {bad_path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
            assert_eq!(send_raw_http(addr, &req).status, 404, "{bad_path}");
        }
    }

    #[test]
    fn open_folder_removed_and_document_output_extension_validated() {
        let server = start_bridge_server("127.0.0.1:0").unwrap();
        let addr = server.addr();
        let token = server.auth_token().to_owned();

        let body = r#"{"path":"/tmp"}"#;
        let open_folder = format!(
            "POST /open/folder HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        assert_eq!(send_raw_http(addr, &open_folder).status, 404);

        let dir = std::env::temp_dir().join(format!(
            "eqntide_doc_test_{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let input_docx = dir.join("sample.docx");
        std::fs::write(&input_docx, b"not-a-zip").unwrap();
        let bad_out = dir.join("evil.bat");
        let doc_body = serde_json::json!({
            "input_path": input_docx.to_string_lossy(),
            "output_path": bad_out.to_string_lossy(),
            "target": "omml"
        })
        .to_string();
        let doc_req = format!(
            "POST /convert/document HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{doc_body}",
            doc_body.len()
        );
        assert_eq!(send_raw_http(addr, &doc_req).status, 400);
        assert!(!bad_out.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn invalid_session_update_writes_no_emf_and_oversized_body_returns_413() {
        let server = start_bridge_server("127.0.0.1:0").unwrap();
        let addr = server.addr();
        let token = server.auth_token().to_owned();

        let bogus_id = "not-a-uuid-and-missing";
        let expected_emf = std::env::temp_dir().join(format!("eqntide_{bogus_id}.emf"));
        let _ = std::fs::remove_file(&expected_emf);
        let upd_body = r#"{"mathml":"<math><mi>x</mi></math>","target_mode":"mathtype"}"#;
        let upd_req = format!(
            "POST /session/{bogus_id}/update HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{upd_body}",
            upd_body.len()
        );
        assert_eq!(send_raw_http(addr, &upd_req).status, 404);
        assert!(!expected_emf.exists());

        let too_big = "a".repeat(MAX_BODY_BYTES + 16);
        let big_req = format!(
            "POST /log HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{too_big}",
            too_big.len()
        );
        assert_eq!(send_raw_http(addr, &big_req).status, 413);
    }

    #[test]
    fn token_persistence_and_stale_session_cleanup_remove_temp_emfs() {
        let dir = std::env::temp_dir().join(format!(
            "eqntide_tok_test_{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let tok1 = load_or_create_bridge_token_in(&dir).unwrap();
        let tok2 = load_or_create_bridge_token_in(&dir).unwrap();
        assert_eq!(tok1, tok2);
        assert!(tok1.len() >= 32);

        let mgr = SessionManager::new();
        let sid = mgr.create_session_with_target("<math/>".into(), None, Some(12.0), "mathtype");
        let emf_path = dir.join(format!("eqntide_{sid}.emf"));
        std::fs::write(&emf_path, b"emf-data").unwrap();
        mgr.update_session(
            &sid,
            "<math/>".into(),
            "<pkg:package/>".into(),
            "<m:oMath/>".into(),
            Some(emf_path.to_string_lossy().into_owned()),
            Some(10.0),
            Some(10.0),
            Some(2.0),
            Some(12.0),
        )
        .unwrap();
        std::thread::sleep(Duration::from_millis(20));
        mgr.cleanup_stale(Duration::from_millis(5));
        assert!(mgr.get_session(&sid).is_none());
        assert!(!emf_path.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
