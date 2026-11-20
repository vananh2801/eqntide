//! Local communication bridge server (127.0.0.1:17890) for Office/WPS Add-ins.
//!
//! Exposes a high-performance HTTP bridge enabling the Word VBA template and WPS JS add-in
//! to communicate with the local equation conversion engine and popup editor.
//!
//! # Key Capabilities
//!
//! * **CORS & Private Network Access (PNA):** Automatically responds to Chromium /
//!   WebView2 preflight requests with `Access-Control-Allow-Private-Network: true`.
//! * **Session Lifecycle Management:** Manages active equation sessions with
//!   long-polling synchronization (`Condvar`) between the Add-in and Desktop GUI.
//! * **Instant Stateless Conversions:** Rapid snippet and batch document conversion.
//! * **Custom URI Handler:** Parses `eqntide://open?...` URLs for launching
//!   the editor when the server is in cold state.

pub mod protocol;
pub mod server;
pub mod session;
pub mod uri;

pub use protocol::*;
pub use server::{
    bridge_token_path, load_or_create_bridge_token, start_bridge_server,
    start_bridge_server_with_callback, start_bridge_server_with_token,
    start_bridge_server_with_window_requests, user_data_dir, DesktopWindowRequest, ServerHandle,
    WindowCallback, WindowRequestCallback, BRIDGE_TOKEN_HEADER, DEFAULT_HOST, DEFAULT_PORT,
};
pub use session::{SessionManager, SessionState};
pub use uri::{format_custom_uri, parse_custom_uri, CustomUriAction};
