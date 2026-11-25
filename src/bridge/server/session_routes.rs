//! HTTP handlers for the equation-editing session lifecycle and window activation.

use std::time::Duration;

use tiny_http::{Method, Request};

use crate::service::{decode_input_to_mathml, parse_latex_to_mathml};
use crate::{
    mathml_to_flat_opc, mathml_to_mtef_bytes, mathml_to_omml_xml, render_emf_with_mtef,
    FlatOpcMode, MathNode,
};

use super::super::protocol::{
    SessionActionResponse, SessionOpenRequest, SessionOpenResponse, SessionUpdateRequest,
    SessionWaitResponse, WindowShowRequest,
};
use super::super::session::{SessionManager, SessionState};
use super::http::{
    body_error_response, error_response, json_response, parse_query_param, read_body, HttpResponse,
};
use super::{
    spawn_desktop_action, spawn_desktop_editor, DesktopWindowRequest, WindowRequestCallback,
    SESSION_MAX_AGE,
};

/// Resolve session lifecycle routes (`/session/*`) before general endpoint dispatch.
pub(super) fn handle_session_route(
    req: &mut Request,
    method: &Method,
    path: &str,
    query: Option<&str>,
    session_mgr: &SessionManager,
    window_cb: Option<&WindowRequestCallback>,
    origin: Option<&str>,
) -> Option<HttpResponse> {
    match (method, path) {
        (Method::Get, route_path)
            if route_path.starts_with("/session/")
                && !route_path.ends_with("/wait")
                && !route_path.ends_with("/update")
                && !route_path.ends_with("/submit")
                && !route_path.ends_with("/cancel")
                && !route_path.ends_with("/close")
                && route_path != "/session/latest" =>
        {
            let session_id = route_path.strip_prefix("/session/").unwrap_or("");
            Some(match session_mgr.get_session(session_id) {
                Some((mathml, latex, font_pt)) => json_response(
                    200,
                    &serde_json::json!({
                        "status": "ok",
                        "session_id": session_id,
                        "mathml": mathml,
                        "latex": latex,
                        "font_pt": font_pt,
                        "target_mode": session_mgr.metadata(session_id).map(|meta| meta.0),
                        "state": session_mgr.metadata(session_id).map(|meta| meta.1),
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
                .and_then(|(_, _, font_pt)| font_pt);
            Some(json_response(
                200,
                &serde_json::json!({ "status": "ok", "session_id": active, "font_pt": font_pt }),
                origin,
            ))
        }
        (Method::Post, "/session/open") => {
            Some(handle_session_open(req, session_mgr, window_cb, origin))
        }
        (Method::Get, route_path)
            if route_path.starts_with("/session/") && route_path.ends_with("/wait") =>
        {
            Some(handle_session_wait(path, query, session_mgr, origin))
        }
        (Method::Post, route_path)
            if route_path.starts_with("/session/")
                && (route_path.ends_with("/update") || route_path.ends_with("/submit")) =>
        {
            Some(handle_session_update(req, path, session_mgr, origin))
        }
        (Method::Post, route_path)
            if route_path.starts_with("/session/")
                && (route_path.ends_with("/close") || route_path.ends_with("/cancel")) =>
        {
            Some(handle_session_cancel(path, session_mgr, origin))
        }
        _ => None,
    }
}

/// Focus the desktop UI or open it to a requested tab/session via `POST /window/show`.
pub(super) fn handle_window_show(
    req: &mut Request,
    session_mgr: &SessionManager,
    window_cb: Option<&WindowRequestCallback>,
    origin: Option<&str>,
) -> HttpResponse {
    let body = match read_body(req) {
        Ok(body) => body,
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
            target_mode: session_mgr.metadata(&session_id).map(|meta| meta.0),
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

fn handle_session_open(
    req: &mut Request,
    session_mgr: &SessionManager,
    window_cb: Option<&WindowRequestCallback>,
    origin: Option<&str>,
) -> HttpResponse {
    session_mgr.cleanup_stale(SESSION_MAX_AGE);
    let body = match read_body(req) {
        Ok(body) => body,
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
                log::warn!(
                    "Could not parse equation data in /session/open ({error}), opening session as empty canvas"
                );
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
        .and_then(|rest| rest.strip_suffix("/wait"))
        .unwrap_or("");
    if session_id.is_empty() {
        return error_response(400, "missing session id in path", origin);
    }
    let timeout_secs = query
        .and_then(|query_str| parse_query_param(query_str, "timeout"))
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
        .and_then(|rest| {
            rest.strip_suffix("/update")
                .or_else(|| rest.strip_suffix("/submit"))
        })
        .unwrap_or("");
    if session_id.is_empty() {
        return error_response(400, "missing session id in path", origin);
    }
    if uuid::Uuid::parse_str(session_id).is_err() || session_mgr.get_session(session_id).is_none() {
        return error_response(404, "session not found", origin);
    }
    let body = match read_body(req) {
        Ok(body) => body,
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
    // WPS prefers `emf_path` over `flat_opc`; supplying a picture for OMML would
    // silently turn a native equation into a MathType picture on insertion.
    let picture = if is_mathtype {
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
        .and_then(|rest| {
            rest.strip_suffix("/close")
                .or_else(|| rest.strip_suffix("/cancel"))
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
