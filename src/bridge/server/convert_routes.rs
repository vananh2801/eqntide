//! Stateless snippet and batch document conversion HTTP handlers.

use tiny_http::Request;

use super::super::protocol::{ConvertDocumentRequest, ConvertSnippetRequest};
use super::http::{body_error_response, error_response, json_response, read_body};

/// Convert one equation snippet via `crate::service::convert_snippet`.
pub(super) fn handle_snippet_conversion(mut req: Request, origin: Option<&str>) {
    let body = match read_body(&mut req) {
        Ok(body) => body,
        Err(err) => {
            let _ = req.respond(body_error_response(err, origin));
            return;
        }
    };
    let result = serde_json::from_str::<ConvertSnippetRequest>(&body);
    let response = match result {
        Ok(input) => match crate::service::convert_snippet(&input) {
            Ok(output) => json_response(200, &output, origin),
            Err(err) => error_response(err.status, &err.message, origin),
        },
        Err(err) => error_response(400, &format!("invalid JSON payload: {err}"), origin),
    };
    let _ = req.respond(response);
}

/// Convert a `.docx` file via `crate::service::convert_document`.
pub(super) fn handle_document_conversion(mut req: Request, origin: Option<&str>) {
    let body = match read_body(&mut req) {
        Ok(body) => body,
        Err(err) => {
            let _ = req.respond(body_error_response(err, origin));
            return;
        }
    };
    let result = serde_json::from_str::<ConvertDocumentRequest>(&body);
    let response = match result {
        Ok(input) => match crate::service::convert_document(&input) {
            Ok(output) => json_response(200, &output, origin),
            Err(err) => error_response(err.status, &err.message, origin),
        },
        Err(err) => error_response(400, &format!("invalid JSON payload: {err}"), origin),
    };
    let _ = req.respond(response);
}
