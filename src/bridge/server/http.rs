//! Low-level HTTP request reading, query parsing, CORS/PNA headers, and response helpers.

use tiny_http::{Header, Method, Request, Response, StatusCode};

use super::super::protocol::{ErrorResponse, HealthResponse};
use super::MAX_BODY_BYTES;

pub(super) type HttpResponse = Response<std::io::Cursor<Vec<u8>>>;

pub(super) enum BodyReadError {
    TooLarge,
    Io,
}

pub(super) fn get_header<'a>(req: &'a Request, name: &str) -> Option<&'a str> {
    req.headers()
        .iter()
        .find(|header| header.field.as_str().as_str().eq_ignore_ascii_case(name))
        .map(|header| header.value.as_str())
}

/// Attach standard CORS headers and the Private Network Access (PNA) header to a response.
pub(super) fn add_cors_headers(response: &mut HttpResponse, origin: Option<&str>) {
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
    if let Some(origin_value) =
        origin.filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("null"))
    {
        headers.insert(0, ("Access-Control-Allow-Origin", origin_value));
    }
    for (name, val) in headers {
        if let Ok(header) = Header::from_bytes(name.as_bytes(), val.as_bytes()) {
            response.add_header(header);
        }
    }
}

pub(super) fn json_response<T: serde::Serialize>(
    status_code: u16,
    data: &T,
    origin: Option<&str>,
) -> HttpResponse {
    let body = serde_json::to_vec(data).unwrap_or_default();
    let mut resp = Response::from_data(body).with_status_code(StatusCode(status_code));
    if let Ok(header) = Header::from_bytes(
        &b"Content-Type"[..],
        &b"application/json; charset=utf-8"[..],
    ) {
        resp.add_header(header);
    }
    add_cors_headers(&mut resp, origin);
    resp
}

pub(super) fn error_response(status_code: u16, msg: &str, origin: Option<&str>) -> HttpResponse {
    json_response(
        status_code,
        &ErrorResponse {
            status: "error".into(),
            error: msg.into(),
        },
        origin,
    )
}

pub(super) fn empty_cors_response(status_code: u16, origin: Option<&str>) -> HttpResponse {
    let mut resp = Response::from_data(Vec::new()).with_status_code(StatusCode(status_code));
    add_cors_headers(&mut resp, origin);
    resp
}

pub(super) fn read_body(req: &mut Request) -> Result<String, BodyReadError> {
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

pub(super) fn body_error_response(err: BodyReadError, origin: Option<&str>) -> HttpResponse {
    match err {
        BodyReadError::TooLarge => error_response(413, "payload too large", origin),
        BodyReadError::Io => {
            error_response(400, "invalid or unreadable UTF-8 request body", origin)
        }
    }
}

pub(super) fn parse_query_param<'a>(query: &'a str, key: &str) -> Option<&'a str> {
    for pair in query.split('&') {
        if let Some((param_key, param_value)) = pair.split_once('=') {
            if param_key == key {
                return Some(param_value);
            }
        }
    }
    None
}

pub(super) fn is_cors_preflight(method: &Method) -> bool {
    *method == Method::Options
}

pub(super) fn respond(req: Request, response: HttpResponse) {
    let _ = req.respond(response);
}

pub(super) fn health_response(origin: Option<&str>) -> HttpResponse {
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
