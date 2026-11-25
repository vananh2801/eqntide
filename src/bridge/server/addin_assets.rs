//! Static asset serving and path-traversal guards for installed WPS add-in files.

use std::path::{Component, Path, PathBuf};

use tiny_http::{Header, Response, StatusCode};

use super::http::{add_cors_headers, error_response, HttpResponse};

/// Dispatch static add-in routes (`/wps-addin/*`), returning `None` for other paths.
pub(super) fn serve_addin_route(path: &str, origin: Option<&str>) -> Option<HttpResponse> {
    let (folder, rel_path) = match path {
        route_path if route_path.starts_with("/wps-addin/") => (
            "wps-addin",
            route_path.strip_prefix("/wps-addin/").unwrap_or(""),
        ),
        _ => return None,
    };
    Some(
        serve_addin_file(folder, rel_path, origin)
            .unwrap_or_else(|| error_response(404, "file not found", origin)),
    )
}

/// Validate that `rel_path` is a safe relative path without traversal, drive prefixes, or secrets.
pub(super) fn is_safe_relative_addin_path(rel_path: &str) -> bool {
    if rel_path.is_empty()
        || rel_path.contains('\0')
        || rel_path.contains('%')
        || rel_path.contains('\\')
        || rel_path.contains(':')
        || rel_path.starts_with('/')
    {
        return false;
    }
    let path = Path::new(rel_path);
    let mut has_normal = false;
    for comp in path.components() {
        match comp {
            Component::Normal(seg) => {
                let segment_str = seg.to_string_lossy();
                if segment_str.starts_with('.')
                    || segment_str.eq_ignore_ascii_case("bridge-token.js")
                {
                    return false;
                }
                has_normal = true;
            }
            _ => return false,
        }
    }
    has_normal
}

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
    base_candidates.push(PathBuf::from(folder));
    base_candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(folder));

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
        .and_then(|ext_os| ext_os.to_str())
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
    if let Ok(header) = Header::from_bytes(&b"Content-Type"[..], mime.as_bytes()) {
        resp.add_header(header);
    }
    if let Ok(header) = Header::from_bytes(&b"Cache-Control"[..], &b"no-cache"[..]) {
        resp.add_header(header);
    }
    add_cors_headers(&mut resp, origin);
    Some(resp)
}
