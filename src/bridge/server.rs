//! Local HTTP bridge server implementation for Office/WPS Add-ins and Tauri GUI.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

use crate::docx::{
    base64_decode, base64_encode, convert_to_omml_with, convert_with, flat_opc_to_mathml, mathml_to_flat_opc,
    upgrade_legacy_mathtype_docx_with, FlatOpcMode, Options, PreviewFormat, ToOmmlOptions,
    UpgradeOptions, UpgradeTarget,
};
use crate::render::emf::{render_emf, render_emf_with_mtef};
use crate::mtef::encode::mathml_to_mtef_bytes;
use crate::render::render_preview;
use crate::mathml::MathNode;
use crate::mtef::decode::{mtef_to_mathml, ole_to_mathml};
use crate::ole::{extract_mtef_from_emf, extract_mtef_from_wmf};
use crate::omml::encode::mathml_to_omml_xml;

use super::protocol::*;
use super::session::{SessionManager, SessionState};

/// Default bind port for the local bridge server.
pub const DEFAULT_PORT: u16 = 17890;

/// Default bind host address.
pub const DEFAULT_HOST: &str = "127.0.0.1";

/// Callback invoked when a session is opened or window show is requested.
/// Arguments: `(session_id, mathml, latex)`
pub type WindowCallback = Arc<dyn Fn(&str, Option<&str>, Option<&str>, Option<f32>) + Send + Sync + 'static>;

/// Handle to a running bridge server instance.
pub struct ServerHandle {
    server: Arc<Server>,
    session_manager: SessionManager,
    running: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    addr: SocketAddr,
}

impl ServerHandle {
    /// The actual bound address of the server.
    pub fn addr(&self) -> SocketAddr {
        self.addr
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

/// Validate that the `Origin` header matches trusted localhost, Office Add-in, or WPS origins.
pub fn is_allowed_origin(origin: &str) -> bool {
    let origin = origin.trim();
    if origin.is_empty() || origin == "null" {
        return true;
    }
    // File or custom schemes used by desktop WebViews / Add-in iframes
    if origin.starts_with("file://")
        || origin.starts_with("wps-addon:")
        || origin.starts_with("tauri://")
        || origin.starts_with("vscode-webview://")
    {
        return true;
    }

    let Some((_scheme, rest)) = origin.split_once("://") else {
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

    // 1. Localhost & Tauri origins
    if matches!(host, "127.0.0.1" | "localhost" | "::1" | "tauri.localhost") {
        return true;
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
fn add_cors_headers(response: &mut Response<std::io::Cursor<Vec<u8>>>, origin: Option<&str>) {
    let allow_origin = match origin {
        Some(o) if !o.is_empty() && o != "null" => o,
        _ => "*",
    };

    let headers = [
        ("Access-Control-Allow-Origin", allow_origin),
        (
            "Access-Control-Allow-Methods",
            "GET, POST, OPTIONS, PUT, DELETE",
        ),
        (
            "Access-Control-Allow-Headers",
            "Content-Type, Authorization, X-Requested-With, Access-Control-Request-Private-Network",
        ),
        ("Access-Control-Allow-Private-Network", "true"),
        ("Access-Control-Max-Age", "86400"),
        ("Vary", "Origin"),
    ];
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
) -> Response<std::io::Cursor<Vec<u8>>> {
    let body = serde_json::to_vec(data).unwrap_or_default();
    let mut resp = Response::from_data(body).with_status_code(StatusCode(status_code));
    if let Ok(h) = Header::from_bytes(&b"Content-Type"[..], &b"application/json; charset=utf-8"[..])
    {
        resp.add_header(h);
    }
    add_cors_headers(&mut resp, origin);
    resp
}

fn error_response(
    status_code: u16,
    msg: &str,
    origin: Option<&str>,
) -> Response<std::io::Cursor<Vec<u8>>> {
    json_response(
        status_code,
        &ErrorResponse {
            status: "error".into(),
            error: msg.into(),
        },
        origin,
    )
}

fn empty_cors_response(status_code: u16, origin: Option<&str>) -> Response<std::io::Cursor<Vec<u8>>> {
    let mut resp = Response::from_data(Vec::new()).with_status_code(StatusCode(status_code));
    add_cors_headers(&mut resp, origin);
    resp
}

fn read_body(req: &mut Request) -> String {
    let mut buf = String::new();
    let _ = req.as_reader().read_to_string(&mut buf);
    buf
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

/// Serve static files for Office Word/WPS Add-ins directly from the bridge server.
fn serve_addin_file(
    folder: &str,
    rel_path: &str,
    origin: Option<&str>,
) -> Option<Response<std::io::Cursor<Vec<u8>>>> {
    if rel_path.contains("..") || rel_path.starts_with('/') || rel_path.starts_with('\\') {
        return None;
    }

    let mut candidates = vec![
        std::path::PathBuf::from(folder).join(rel_path),
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(folder)
            .join(rel_path),
    ];
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            candidates.push(parent.join(folder).join(rel_path));
        }
    }

    let mut found_bytes = None;
    let mut resolved_path = None;

    for candidate in &candidates {
        if candidate.is_file() {
            if let Ok(bytes) = std::fs::read(candidate) {
                found_bytes = Some(bytes);
                resolved_path = Some(candidate.clone());
                break;
            }
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

/// Start the local HTTP bridge server on `addr` (e.g. `"127.0.0.1:17890"`).
pub fn start_bridge_server(addr: &str) -> crate::error::Result<ServerHandle> {
    start_bridge_server_with_callback(addr, None)
}

/// Start the local HTTP bridge server with an optional window-show callback.
pub fn start_bridge_server_with_callback(
    addr: &str,
    window_cb: Option<WindowCallback>,
) -> crate::error::Result<ServerHandle> {
    let server = Server::http(addr).map_err(|e| {
        crate::error::Error::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("failed to bind HTTP server to {addr}: {e}"),
        ))
    })?;

    let bound_addr = server.server_addr().to_ip().ok_or_else(|| {
        crate::error::Error::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            "failed to query server IP address",
        ))
    })?;

    let server_arc = Arc::new(server);
    let session_mgr = SessionManager::new();
    let running = Arc::new(AtomicBool::new(true));

    let server_clone = Arc::clone(&server_arc);
    let session_clone = session_mgr.clone();
    let running_clone = Arc::clone(&running);

    let thread = std::thread::spawn(move || {
        while running_clone.load(Ordering::SeqCst) {
            let request = match server_clone.recv() {
                Ok(req) => req,
                Err(e) => {
                    log::debug!("HTTP server recv ended: {e}");
                    break;
                }
            };

            let session_for_worker = session_clone.clone();
            let window_cb_for_worker = window_cb.clone();
            std::thread::spawn(move || {
                handle_http_request(request, session_for_worker, window_cb_for_worker);
            });
        }
    });

    Ok(ServerHandle {
        server: server_arc,
        session_manager: session_mgr,
        running,
        thread: Some(thread),
        addr: bound_addr,
    })
}

fn handle_http_request(
    mut req: Request,
    session_mgr: SessionManager,
    window_cb: Option<WindowCallback>,
) {
    let method = req.method().clone();
    let raw_url = req.url().to_owned();
    let (path, query) = match raw_url.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (raw_url.as_str(), None),
    };

    // 1. Host Header Validation (anti-DNS Rebinding)
    if let Some(host) = get_header(&req, "Host") {
        if !is_valid_host(host) {
            log::warn!("Rejected request with untrusted Host header: {host}");
            let _ = req.respond(error_response(
                403,
                "forbidden: invalid Host header (DNS rebinding protection)",
                None,
            ));
            return;
        }
    }

    // 2. Origin Header Validation (anti-CSRF from untrusted websites)
    let origin_owned = get_header(&req, "Origin").map(|s| s.to_string());
    if let Some(ref orig) = origin_owned {
        if !is_allowed_origin(orig) {
            log::warn!("Rejected request from untrusted Origin: {orig}");
            let _ = req.respond(error_response(
                403,
                "forbidden: untrusted Origin",
                None,
            ));
            return;
        }
    }
    let origin = origin_owned.as_deref();

    // 3. CORS Preflight
    if method == Method::Options {
        let resp = empty_cors_response(204, origin);
        let _ = req.respond(resp);
        return;
    }

    // 4. Routing
    match (method, path) {
        (Method::Get, "/health") => {
            let resp = json_response(
                200,
                &HealthResponse {
                    status: "ok".into(),
                    version: env!("CARGO_PKG_VERSION").into(),
                    server: "omml-converter-bridge".into(),
                },
                origin,
            );
            let _ = req.respond(resp);
        }

        (Method::Get, "/manifest.xml") => {
            if let Some(resp) = serve_addin_file("word-addin", "manifest.xml", origin) {
                let _ = req.respond(resp);
            } else {
                let _ = req.respond(error_response(404, "file not found", origin));
            }
        }

        (Method::Get, p) if p.starts_with("/word-addin/") => {
            let rel_path = p.strip_prefix("/word-addin/").unwrap_or("");
            if let Some(resp) = serve_addin_file("word-addin", rel_path, origin) {
                let _ = req.respond(resp);
            } else {
                let _ = req.respond(error_response(404, "file not found", origin));
            }
        }

        (Method::Get, p) if p.starts_with("/wps-addin/") => {
            let rel_path = p.strip_prefix("/wps-addin/").unwrap_or("");
            if let Some(resp) = serve_addin_file("wps-addin", rel_path, origin) {
                let _ = req.respond(resp);
            } else {
                let _ = req.respond(error_response(404, "file not found", origin));
            }
        }

        (Method::Post, "/window/show") => {
            let body = read_body(&mut req);
            let show_req: WindowShowRequest = serde_json::from_str(&body).unwrap_or_default();
            let sid = show_req
                .session_id
                .filter(|s| !s.is_empty())
                // do not revive stale session
                .unwrap_or_default();
            eprintln!("POST /window/show for session_id='{}'", sid);

            let (m, l, f) = if !sid.is_empty() {
                session_mgr
                    .get_session(&sid)
                    .map(|(m, l, f)| (Some(m), l, f))
                    .unwrap_or((None, None, None))
            } else {
                (None, None, None)
            };

            if let Some(ref cb) = window_cb {
                cb(&sid, m.as_deref(), l.as_deref(), f);
            } else {
                spawn_desktop_editor(&sid);
            }

            let resp = json_response(
                200,
                &serde_json::json!({
                    "status": "ok",
                    "session_id": sid
                }),
                origin,
            );
            let _ = req.respond(resp);
        }

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
            if let Some((mathml, latex, font_pt)) = session_mgr.get_session(sid) {
                let resp = json_response(
                    200,
                    &serde_json::json!({
                        "status": "ok",
                        "session_id": sid,
                        "mathml": mathml,
                        "latex": latex,
                        "font_pt": font_pt,
                    }),
                    origin,
                );
                let _ = req.respond(resp);
            } else {
                let _ = req.respond(error_response(404, "session not found", origin));
            }
        }

        (Method::Get, "/session/latest") => {
            let active = session_mgr.get_latest_session_id();
            let font_pt = active.as_deref().and_then(|id| session_mgr.get_session(id)).and_then(|(_, _, f)| f);
            let resp = json_response(
                200,
                &serde_json::json!({
                    "status": "ok",
                    "session_id": active,
                    "font_pt": font_pt,
                }),
                origin,
            );
            let _ = req.respond(resp);
        }

        (Method::Post, "/log") => {
            let body = read_body(&mut req);
            let trimmed = body.trim();
            eprintln!("[WPS Addin] {trimmed}");
            log::info!("[WPS Addin] {trimmed}");
            let resp = json_response(200, &serde_json::json!({ "status": "ok" }), origin);
            let _ = req.respond(resp);
        }

        (Method::Post, "/session/open") => {
            let body = read_body(&mut req);
            let open_req: SessionOpenRequest = match serde_json::from_str(&body) {
                Ok(r) => r,
                Err(e) => {
                    let _ = req.respond(error_response(
                        400,
                        &format!("invalid JSON payload: {e}"),
                        origin,
                    ));
                    return;
                }
            };

            let trimmed_data = open_req.data.trim();
            let _ = std::fs::write("/tmp/last_open_req.xml", trimmed_data.as_bytes());
            eprintln!("POST /session/open received payload len={}", trimmed_data.len());
            log::info!("POST /session/open received payload len={}", trimmed_data.len());
            let is_latex = open_req.format == "latex"
                || trimmed_data.starts_with('$')
                || trimmed_data.starts_with('\\')
                || trimmed_data.contains("\\frac")
                || trimmed_data.contains("\\sqrt");

            let (mathml, latex) = if is_latex {
                let clean = trimmed_data.trim_matches('$').trim().to_string();
                let mml = parse_latex_to_mathml(trimmed_data).ok().map(|m| m.to_xml()).unwrap_or_default();
                (mml, Some(clean))
            } else if trimmed_data.is_empty() {
                (String::new(), None)
            } else {
                match decode_input_to_mathml(trimmed_data, &open_req.format) {
                    Ok(m) => {
                        let latex = m.to_latex();
                        eprintln!("POST /session/open successfully parsed equation to latex: '{}'", latex);
                        (m.to_xml(), Some(latex))
                    },
                    Err(e) => {
                        eprintln!("POST /session/open could not parse equation data: {e}");
                        log::warn!("Could not parse equation data in /session/open ({e}), opening session as empty canvas");
                        (String::new(), None)
                    }
                }
            };

            let session_id = session_mgr.create_session(mathml.clone(), latex.clone(), open_req.font_pt);
            if let Some(ref cb) = window_cb {
                cb(&session_id, Some(&mathml), latex.as_deref(), open_req.font_pt);
            } else {
                spawn_desktop_editor(&session_id);
            }
            let resp = json_response(
                200,
                &SessionOpenResponse {
                    status: "opened".into(),
                    session_id,
                    mathml,
                    latex,
                    font_pt: open_req.font_pt,
                },
                origin,
            );
            let _ = req.respond(resp);
        }

        (Method::Get, p) if p.starts_with("/session/") && p.ends_with("/wait") => {
            let session_id = p
                .strip_prefix("/session/")
                .and_then(|s| s.strip_suffix("/wait"))
                .unwrap_or("");

            if session_id.is_empty() {
                let _ = req.respond(error_response(400, "missing session id in path", origin));
                return;
            }

            let timeout_secs = query
                .and_then(|q| parse_query_param(q, "timeout"))
                .and_then(|t| t.parse::<u64>().ok())
                .unwrap_or(300);

            let timeout = Duration::from_secs(timeout_secs);
            match session_mgr.wait_session(session_id, timeout) {
                Some(SessionState::Ready {
                    mathml,
                    flat_opc,
                    omml,
                    emf_path,
                    width_pt,
                    height_pt,
                    depth_pt,
                    font_pt,
                }) => {
                    let resp = json_response(
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
                    );
                    let _ = req.respond(resp);
                }
                Some(SessionState::Cancelled) => {
                    let resp = json_response(
                        200,
                        &SessionWaitResponse {
                            status: "cancelled".into(),
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
                    );
                    let _ = req.respond(resp);
                }
                Some(SessionState::Waiting) | None => {
                    let resp = json_response(
                        200,
                        &SessionWaitResponse {
                            status: "timeout".into(),
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
                    );
                    let _ = req.respond(resp);
                }
            }
        }

        (Method::Post, p)
            if p.starts_with("/session/")
                && (p.ends_with("/update") || p.ends_with("/submit")) =>
        {
            let session_id = p
                .strip_prefix("/session/")
                .and_then(|s| s.strip_suffix("/update").or_else(|| s.strip_suffix("/submit")))
                .unwrap_or("");

            if session_id.is_empty() {
                let _ = req.respond(error_response(400, "missing session id in path", origin));
                return;
            }

            let body = read_body(&mut req);
            let upd_req: SessionUpdateRequest = match serde_json::from_str(&body) {
                Ok(r) => r,
                Err(e) => {
                    let _ = req.respond(error_response(
                        400,
                        &format!("invalid JSON payload: {e}"),
                        origin,
                    ));
                    return;
                }
            };

            let math_node = match MathNode::parse(&upd_req.mathml) {
                Ok(m) => m,
                Err(e) => {
                    let _ = req.respond(error_response(
                        422,
                        &format!("failed to parse MathML: {e}"),
                        origin,
                    ));
                    return;
                }
            };

            let mode = if upd_req.target_mode.eq_ignore_ascii_case("mathtype") {
                FlatOpcMode::ModernMathtypeOleEmf
            } else {
                FlatOpcMode::ModernOmml
            };

            let flat_opc = match mathml_to_flat_opc(&math_node, mode, upd_req.font_pt) {
                Ok(opc) => opc,
                Err(e) => {
                    let _ = req.respond(error_response(
                        500,
                        &format!("failed to package Flat OPC: {e}"),
                        origin,
                    ));
                    return;
                }
            };

            let omml = match mathml_to_omml_xml(&math_node) {
                Ok(o) => o,
                Err(e) => {
                    let _ = req.respond(error_response(
                        500,
                        &format!("failed to generate OMML: {e}"),
                        origin,
                    ));
                    return;
                }
            };

            let mtef_bytes = mathml_to_mtef_bytes(&math_node).ok();
            let emf_pic = render_emf_with_mtef(&math_node, upd_req.font_pt, mtef_bytes.as_deref()).ok();
            let (emf_path, width_pt, height_pt, depth_pt) = if let Some(ref pic) = emf_pic {
                let path = format!("/tmp/matheditor_{}.emf", session_id);
                let _ = std::fs::write(&path, &pic.emf);
                (Some(path), Some(pic.width_pt), Some(pic.height_pt), Some(pic.depth_pt))
            } else {
                (None, None, None, None)
            };

            if let Err(e) = session_mgr.update_session(
                session_id,
                upd_req.mathml,
                flat_opc,
                omml,
                emf_path,
                width_pt,
                height_pt,
                depth_pt,
                Some(upd_req.font_pt),
            ) {
                let _ = req.respond(error_response(404, e, origin));
                return;
            }

            let resp = json_response(
                200,
                &SessionActionResponse {
                    status: "ok".into(),
                    session_id: session_id.into(),
                },
                origin,
            );
            let _ = req.respond(resp);
        }

        (Method::Post, p)
            if p.starts_with("/session/")
                && (p.ends_with("/close") || p.ends_with("/cancel")) =>
        {
            let session_id = p
                .strip_prefix("/session/")
                .and_then(|s| s.strip_suffix("/close").or_else(|| s.strip_suffix("/cancel")))
                .unwrap_or("");

            if let Err(e) = session_mgr.cancel_session(session_id) {
                let _ = req.respond(error_response(404, e, origin));
                return;
            }

            let resp = json_response(
                200,
                &SessionActionResponse {
                    status: "cancelled".into(),
                    session_id: session_id.into(),
                },
                origin,
            );
            let _ = req.respond(resp);
        }

        (Method::Post, "/convert/snippet") => {
            let body = read_body(&mut req);
            let snip_req: ConvertSnippetRequest = match serde_json::from_str(&body) {
                Ok(r) => r,
                Err(e) => {
                    let _ = req.respond(error_response(
                        400,
                        &format!("invalid JSON payload: {e}"),
                        origin,
                    ));
                    return;
                }
            };

            let math = match decode_input_to_mathml(&snip_req.data, &snip_req.from) {
                Ok(m) => m,
                Err(e) => {
                    let _ = req.respond(error_response(
                        422,
                        &format!("failed to parse input equation: {e}"),
                        origin,
                    ));
                    return;
                }
            };

            let mut emf_path = None;
            let mut width_pt = None;
            let mut height_pt = None;
            let mut depth_pt = None;

            let converted = match snip_req.to.as_str() {
                "flat_opc_omml" => {
                    mathml_to_flat_opc(&math, FlatOpcMode::ModernOmml, snip_req.font_pt)
                }
                "flat_opc_mathtype" | "mathtype" => {
                    let mtef_bytes = mathml_to_mtef_bytes(&math).ok();
                    if let Ok(pic) = render_emf_with_mtef(&math, snip_req.font_pt, mtef_bytes.as_deref()) {
                        let path = format!("/tmp/matheditor_snippet_{}.emf", uuid::Uuid::new_v4());
                        let _ = std::fs::write(&path, &pic.emf);
                        emf_path = Some(path);
                        width_pt = Some(pic.width_pt);
                        height_pt = Some(pic.height_pt);
                        depth_pt = Some(pic.depth_pt);
                    }
                    mathml_to_flat_opc(&math, FlatOpcMode::ModernMathtypeOleEmf, snip_req.font_pt)
                }
                "omml" => mathml_to_omml_xml(&math),
                "mathml" => Ok(math.to_xml()),
                "latex" => Ok(math.to_latex()),
                "emf" | "emf_base64" => {
                    render_emf(&math, snip_req.font_pt)
                        .map(|p| base64_encode(&p.emf))
                        .map_err(|e| crate::error::Error::Xml(format!("EMF rendering error: {e}")))
                }
                "png" | "png_base64" => {
                    render_preview(&math, snip_req.font_pt)
                        .map(|prev| base64_encode(&prev.png))
                        .map_err(|e| crate::error::Error::Xml(format!("PNG rendering error: {e}")))
                }
                other => {
                    let _ = req.respond(error_response(
                        400,
                        &format!("unknown target format '{other}'"),
                        origin,
                    ));
                    return;
                }
            };

            match converted {
                Ok(result) => {
                    let resp = json_response(
                        200,
                        &ConvertSnippetResponse {
                            status: "ok".into(),
                            result,
                            emf_path,
                            width_pt,
                            height_pt,
                            depth_pt,
                        },
                        origin,
                    );
                    let _ = req.respond(resp);
                }
                Err(e) => {
                    let _ = req.respond(error_response(
                        500,
                        &format!("conversion failed: {e}"),
                        origin,
                    ));
                }
            }
        }

        (Method::Post, "/convert/document") => {
            let body = read_body(&mut req);
            let doc_req: ConvertDocumentRequest = match serde_json::from_str(&body) {
                Ok(r) => r,
                Err(e) => {
                    let _ = req.respond(error_response(
                        400,
                        &format!("invalid JSON payload: {e}"),
                        origin,
                    ));
                    return;
                }
            };

            let in_path = std::path::Path::new(&doc_req.input_path);
            if !in_path.exists() {
                let _ = req.respond(error_response(
                    404,
                    &format!("input file does not exist: {}", in_path.display()),
                    origin,
                ));
                return;
            }

            if !in_path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("docx"))
            {
                let _ = req.respond(error_response(
                    400,
                    "only .docx files are supported for document conversion",
                    origin,
                ));
                return;
            }

            let out_path = doc_req.output_path.as_deref().unwrap_or(&doc_req.input_path);
            let preview_fmt = if doc_req.preview.eq_ignore_ascii_case("png") {
                PreviewFormat::Png
            } else {
                PreviewFormat::Emf
            };

            let res = match doc_req.target.as_str() {
                "omml" => {
                    let opts = ToOmmlOptions::default();
                    convert_to_omml_with(in_path, out_path, &opts)
                }
                "mathtype" => {
                    let opts = Options {
                        font_pt: doc_req.font_pt,
                        preview: preview_fmt,
                        ..Default::default()
                    };
                    convert_with(in_path, out_path, &opts)
                }
                "upgrade_omml" => {
                    let opts = UpgradeOptions {
                        target: UpgradeTarget::ModernOmml,
                        font_pt: doc_req.font_pt,
                        math_font: None,
                    };
                    upgrade_legacy_mathtype_docx_with(in_path, out_path, &opts)
                }
                "upgrade_mathtype" => {
                    let opts = UpgradeOptions {
                        target: UpgradeTarget::ModernMathType,
                        font_pt: doc_req.font_pt,
                        math_font: None,
                    };
                    upgrade_legacy_mathtype_docx_with(in_path, out_path, &opts)
                }
                other => {
                    let _ = req.respond(error_response(
                        400,
                        &format!("unknown document target '{other}'"),
                        origin,
                    ));
                    return;
                }
            };

            match res {
                Ok(summary) => {
                    let resp = json_response(
                        200,
                        &ConvertDocumentResponse {
                            status: "ok".into(),
                            found: summary.found,
                            converted: summary.converted,
                            skipped: summary.skipped,
                            errors: summary.errors,
                            output_path: out_path.to_owned(),
                        },
                        origin,
                    );
                    let _ = req.respond(resp);
                }
                Err(e) => {
                    let _ = req.respond(error_response(
                        500,
                        &format!("document conversion error: {e}"),
                        origin,
                    ));
                }
            }
        }

        (Method::Post, "/open/folder") => {
            let body = read_body(&mut req);
            let payload: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
            if let Some(path_str) = payload.get("path").and_then(|v| v.as_str()) {
                let p = std::path::Path::new(path_str);
                let target_dir = if p.is_file() {
                    p.parent().unwrap_or(p)
                } else {
                    p
                };
                #[cfg(target_os = "windows")]
                let _ = std::process::Command::new("explorer").arg(target_dir).spawn();
                #[cfg(target_os = "macos")]
                let _ = std::process::Command::new("open").arg(target_dir).spawn();
                #[cfg(not(any(target_os = "windows", target_os = "macos")))]
                let _ = std::process::Command::new("xdg-open").arg(target_dir).spawn();
            }
            let resp = json_response(200, &serde_json::json!({"status": "ok"}), origin);
            let _ = req.respond(resp);
        }

        _ => {
            let resp = error_response(404, "not found", origin);
            let _ = req.respond(resp);
        }
    }
}

/// Parse a LaTeX math string into a [`MathNode`] via latex2mathml.
fn parse_latex_to_mathml(latex_str: &str) -> crate::error::Result<MathNode> {
    let mut clean = latex_str.trim();
    if clean.starts_with("$$") && clean.ends_with("$$") && clean.len() >= 4 {
        clean = clean[2..clean.len() - 2].trim();
    } else if clean.starts_with('$') && clean.ends_with('$') && clean.len() >= 2 {
        clean = clean[1..clean.len() - 1].trim();
    } else if clean.starts_with(r"\[") && clean.ends_with(r"\]") && clean.len() >= 4 {
        clean = clean[2..clean.len() - 2].trim();
    } else if clean.starts_with(r"\(") && clean.ends_with(r"\)") && clean.len() >= 4 {
        clean = clean[2..clean.len() - 2].trim();
    }
    let mml = latex2mathml::latex_to_mathml(clean, latex2mathml::DisplayStyle::Inline)
        .map_err(|e| crate::error::Error::Xml(format!("LaTeX parsing error: {e}")))?;
    MathNode::parse(&mml)
}

/// Helper to decode any supported equation input representation into [`MathNode`].
fn decode_input_to_mathml(data: &str, format: &str) -> crate::error::Result<MathNode> {
    let trimmed = data.trim();

    if format == "latex"
        || trimmed.starts_with('$')
        || trimmed.starts_with(r"\[")
        || trimmed.starts_with(r"\(")
    {
        return parse_latex_to_mathml(trimmed);
    }

    if format == "flat_opc" || trimmed.contains("<pkg:package") || trimmed.starts_with("<?xml") {
        return flat_opc_to_mathml(trimmed);
    }

    if format == "omml"
        || trimmed.contains("<m:oMath")
        || trimmed.contains("<w:p")
        || trimmed.contains("<w:wordDocument")
        || trimmed.contains("<m:oMathPara")
    {
        return flat_opc_to_mathml(trimmed);
    }

    if format == "mathml" || trimmed.starts_with("<math") {
        return MathNode::parse(trimmed);
    }

    if format == "ole_base64" {
        let bin = base64_decode(trimmed).ok_or_else(|| {
            crate::error::Error::Xml("invalid base64 encoding for OLE data".into())
        })?;
        return ole_to_mathml(&bin).ok_or_else(|| {
            crate::error::Error::InvalidMtef("failed to decode MTEF from OLE data".into())
        });
    }

    if format == "emf_base64" {
        let bin = base64_decode(trimmed).ok_or_else(|| {
            crate::error::Error::Xml("invalid base64 encoding for EMF data".into())
        })?;
        let mtef = extract_mtef_from_emf(&bin).ok_or_else(|| {
            crate::error::Error::InvalidMtef("no embedded MTEF comment found in EMF".into())
        })?;
        return mtef_to_mathml(&mtef).ok_or_else(|| {
            crate::error::Error::InvalidMtef("failed to decode MTEF from EMF comment".into())
        });
    }

    if format == "wmf_base64" {
        let bin = base64_decode(trimmed).ok_or_else(|| {
            crate::error::Error::Xml("invalid base64 encoding for WMF data".into())
        })?;
        let mtef = extract_mtef_from_wmf(&bin).ok_or_else(|| {
            crate::error::Error::InvalidMtef("no embedded MTEF found in WMF".into())
        })?;
        return mtef_to_mathml(&mtef).ok_or_else(|| {
            crate::error::Error::InvalidMtef("failed to decode MTEF from WMF".into())
        });
    }

    // Fallback: Attempt Flat OPC / XML parse
    if let Ok(m) = flat_opc_to_mathml(trimmed) {
        return Ok(m);
    }

    // Fallback: Attempt MathNode parse
    if let Ok(m) = MathNode::parse(trimmed) {
        return Ok(m);
    }

    Err(crate::error::Error::Xml(format!(
        "unrecognized equation input format: '{format}'"
    )))
}

pub(crate) fn spawn_desktop_editor(session_id: &str) {
    let mut candidates = Vec::new();
    if let Ok(env_path) = std::env::var("MATH_EDITOR_UI_BIN") {
        candidates.push(std::path::PathBuf::from(env_path));
    }
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(dir) = current_exe.parent() {
            candidates.push(dir.join("omml-editor-desktop"));
            candidates.push(dir.join("omml-editor-desktop.exe"));
            candidates.push(dir.join("Math Equation Editor.app/Contents/MacOS/omml-editor-desktop"));
        }
    }
    #[cfg(target_os = "windows")]
    candidates.push(std::path::PathBuf::from("omml-editor-desktop.exe"));
    #[cfg(not(target_os = "windows"))]
    candidates.push(std::path::PathBuf::from("/home/vananh2801/Documents/mathtype-convert/omml-converter-rust/src-tauri/target/release/omml-editor-desktop"));
    candidates.push(std::path::PathBuf::from("omml-editor-desktop"));

    for path in &candidates {
        if path.is_absolute() && !path.exists() {
            continue;
        }
        let mut cmd = std::process::Command::new(path);
        if !session_id.is_empty() {
            cmd.arg(format!("matheditor://open?session_id={session_id}"));
        }
        match cmd.spawn() {
            Ok(child) => {
                log::info!("Spawned desktop editor UI (PID {}) via {}", child.id(), path.display());
                eprintln!("Spawned desktop editor UI (PID {}) via {}", child.id(), path.display());
                return;
            }
            Err(e) => {
                log::debug!("Failed to spawn {}: {e}", path.display());
            }
        }
    }
    log::warn!("Could not find or launch omml-editor-desktop UI binary");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::net::TcpStream;

    struct RawHttpResponse {
        status: u16,
        headers: HashMap<String, String>,
        body: String,
    }

    fn send_raw_http(addr: SocketAddr, req: &str) -> RawHttpResponse {
        let mut stream = TcpStream::connect(addr).expect("connect to test server");
        stream.write_all(req.as_bytes()).expect("write request");
        let mut response_bytes = Vec::new();
        stream.read_to_end(&mut response_bytes).expect("read response");
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

        // 2. Test GET /health
        let get_req = format!("GET /health HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
        let resp = send_raw_http(addr, &get_req);
        assert_eq!(resp.status, 200);
        let health: HealthResponse = serde_json::from_str(&resp.body).expect("json parse");
        assert_eq!(health.status, "ok");
        assert_eq!(health.server, "omml-converter-bridge");
    }

    #[test]
    fn test_security_dns_rebinding_rejected() {
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
    }

    #[test]
    fn test_security_untrusted_origin_rejected() {
        let server_handle = start_bridge_server("127.0.0.1:0").expect("start test server");
        let addr = server_handle.addr();

        // Send request from random malicious web origin
        let req = format!(
            "GET /health HTTP/1.1\r\nHost: {addr}\r\nOrigin: https://malicious-website.xyz\r\nConnection: close\r\n\r\n"
        );
        let resp = send_raw_http(addr, &req);
        assert_eq!(resp.status, 403);
        assert!(resp.body.contains("untrusted Origin"));
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

        // 1. Open a new session with OMML input
        let raw_omml = r#"<w:p xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" xmlns:m=\"http://schemas.openxmlformats.org/officeDocument/2006/math\"><m:oMath><m:r><m:t>x+1</m:t></m:r></m:oMath></w:p>"#;
        let open_payload = format!(
            "{{\"data\": \"{}\", \"format\": \"omml\"}}",
            raw_omml
        );
        let open_req = format!(
            "POST /session/open HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
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
        let wait_handle = std::thread::spawn(move || {
            let wait_req = format!(
                "GET /session/{sid_clone}/wait?timeout=10 HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n"
            );
            let resp = send_raw_http(addr, &wait_req);
            let wait_res: SessionWaitResponse =
                serde_json::from_str(&resp.body).expect("json parse");
            wait_res
        });

        // 3. Update the session from the editor side
        std::thread::sleep(Duration::from_millis(50));
        let upd_payload = r#"{"mathml": "<math><mi>y</mi><mo>=</mo><mn>2</mn></math>", "target_mode": "omml"}"#;
        let upd_req = format!(
            "POST /session/{session_id}/update HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
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
        assert!(wait_result.flat_opc.as_ref().unwrap().contains("pkg:package"));
        assert!(wait_result.omml.as_ref().unwrap().contains("m:oMath"));
    }

    #[test]
    fn test_convert_snippet_stateless() {
        let server_handle = start_bridge_server("127.0.0.1:0").expect("start test server");
        let addr = server_handle.addr();

        let snip_payload = r#"{"data": "<math><mi>a</mi><mo>+</mo><mi>b</mi></math>", "from": "mathml", "to": "flat_opc_omml"}"#;
        let snip_req = format!(
            "POST /convert/snippet HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
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
            "POST /convert/snippet HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            emf_payload.len(),
            emf_payload
        );
        let resp_emf = send_raw_http(addr, &emf_req);
        assert_eq!(resp_emf.status, 200);
        let emf_res: ConvertSnippetResponse = serde_json::from_str(&resp_emf.body).expect("json parse");
        assert_eq!(emf_res.status, "ok");
        assert!(!emf_res.result.is_empty());

        // Test PNG snippet export
        let png_payload = r#"{"data": "<math><mi>x</mi><mo>+</mo><mn>1</mn></math>", "from": "mathml", "to": "png"}"#;
        let png_req = format!(
            "POST /convert/snippet HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            png_payload.len(),
            png_payload
        );
        let resp_png = send_raw_http(addr, &png_req);
        assert_eq!(resp_png.status, 200);
        let png_res: ConvertSnippetResponse = serde_json::from_str(&resp_png.body).expect("json parse");
        assert_eq!(png_res.status, "ok");
        assert!(!png_res.result.is_empty());
    }

    #[test]
    fn test_session_open_with_mathml_generates_latex() {
        let server_handle = start_bridge_server("127.0.0.1:0").expect("start test server");
        let addr = server_handle.addr();

        let open_payload = r#"{"data": "<math><mi>x</mi><mo>=</mo><mfrac><mi>a</mi><mi>b</mi></mfrac></math>", "format": "mathml", "target_mode": "mathtype"}"#;
        let open_req = format!(
            "POST /session/open HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
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

        // 1. Word Add-in files
        let req = format!("GET /word-addin/manifest.xml HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
        let resp = send_raw_http(addr, &req);
        assert_eq!(resp.status, 200);
        assert!(resp.body.contains("Math Editor"));
        assert!(resp.body.contains("OfficeApp"));
        assert_eq!(resp.headers.get("content-type").map(|s| s.as_str()), Some("application/xml; charset=utf-8"));

        let req2 = format!("GET /word-addin/commands.js HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
        let resp2 = send_raw_http(addr, &req2);
        assert_eq!(resp2.status, 200);
        assert!(resp2.body.contains("insertEquation"));
        assert_eq!(resp2.headers.get("content-type").map(|s| s.as_str()), Some("application/javascript; charset=utf-8"));

        let req3 = format!("GET /manifest.xml HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
        let resp3 = send_raw_http(addr, &req3);
        assert_eq!(resp3.status, 200);
        assert!(resp3.body.contains("Math Editor"));

        // 2. WPS Add-in files
        let req_wps = format!("GET /wps-addin/ribbon.xml HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
        let resp_wps = send_raw_http(addr, &req_wps);
        assert_eq!(resp_wps.status, 200);
        assert!(resp_wps.body.contains("TabMathEditor"));
        assert!(resp_wps.body.contains("btnInsertEquation"));

        let req_wps_js = format!("GET /wps-addin/main.js HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
        let resp_wps_js = send_raw_http(addr, &req_wps_js);
        assert_eq!(resp_wps_js.status, 200);
        assert!(resp_wps_js.body.contains("InsertEquation"));
        assert!(resp_wps_js.body.contains("WordOpenXML"));
    }
}
