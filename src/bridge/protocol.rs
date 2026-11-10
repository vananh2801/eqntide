//! Data Transfer Objects (DTOs) for the local HTTP communication bridge.

use serde::{Deserialize, Serialize};

fn default_format() -> String {
    "auto".into()
}

fn default_target_mode() -> String {
    "omml".into()
}

fn default_font_pt() -> f32 {
    12.0
}

fn default_to_target() -> String {
    "omml".into()
}

fn default_doc_target() -> String {
    "omml".into()
}

fn default_preview() -> String {
    "emf".into()
}

/// Response returned by `GET /health``.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub server: String,
}

/// Request payload for `POST /session/open`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionOpenRequest {
    /// Raw equation data (Flat OPC XML, OMML `<m:oMath>`, MathML `<math>`, or LaTeX string).
    #[serde(default)]
    pub data: String,
    /// Explicit data format hint ("auto", "flat_opc", "omml", "mathml", "latex").
    #[serde(default = "default_format")]
    pub format: String,
    /// Default target mode for the session ("omml" or "mathtype").
    #[serde(default = "default_target_mode")]
    pub target_mode: String,
    /// Document font size in typographic points (e.g. 12.0, 14.0) at the cursor position.
    #[serde(default)]
    pub font_pt: Option<f32>,
}

/// Response returned by `POST /session/open`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionOpenResponse {
    pub status: String,
    pub session_id: String,
    pub mathml: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_pt: Option<f32>,
}

/// Response returned by `GET /session/:id/wait` upon completion, cancellation, or timeout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionWaitResponse {
    /// Status: "ready", "cancelled", or "timeout".
    pub status: String,
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mathml: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flat_opc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub omml: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emf_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width_pt: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height_pt: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth_pt: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_pt: Option<f32>,
}

/// Request payload for `POST /session/:id/update`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionUpdateRequest {
    pub mathml: String,
    #[serde(default = "default_target_mode")]
    pub target_mode: String,
    #[serde(default = "default_font_pt")]
    pub font_pt: f32,
}

/// Generic response for session actions (`update`, `close`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionActionResponse {
    pub status: String,
    pub session_id: String,
}

/// Request payload for `POST /convert/snippet`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertSnippetRequest {
    /// Input data string.
    pub data: String,
    /// Source format: "auto", "flat_opc", "omml", "mathml", "ole_base64", "emf_base64", "wmf_base64".
    #[serde(default = "default_format")]
    pub from: String,
    /// Target format: "flat_opc_omml", "flat_opc_mathtype", "omml", "mathml".
    #[serde(default = "default_to_target")]
    pub to: String,
    /// Point size for vector EMF preview rendering.
    #[serde(default = "default_font_pt")]
    pub font_pt: f32,
}

/// Response returned by `POST /convert/snippet`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertSnippetResponse {
    pub status: String,
    pub result: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emf_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width_pt: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height_pt: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth_pt: Option<f32>,
}

/// Request payload for `POST /convert/document`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertDocumentRequest {
    pub input_path: String,
    pub output_path: Option<String>,
    /// Target mode: "omml", "mathtype", "upgrade_omml", "upgrade_mathtype".
    #[serde(default = "default_doc_target")]
    pub target: String,
    /// Preview format for MathType: "emf" or "png".
    #[serde(default = "default_preview")]
    pub preview: String,
    #[serde(default = "default_font_pt")]
    pub font_pt: f32,
}

/// Response returned by `POST /convert/document`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertDocumentResponse {
    pub status: String,
    pub found: usize,
    pub converted: usize,
    pub skipped: usize,
    pub errors: usize,
    pub output_path: String,
}

/// Error response structure returned on HTTP 4xx/5xx errors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub status: String,
    pub error: String,
}

/// Request payload for `POST /window/show`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WindowShowRequest {
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub tab: Option<String>,
}
