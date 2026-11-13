//! Local communication bridge server (127.0.0.1:17890) for Office/WPS Add-ins.
//!
//! Exposes a high-performance HTTP bridge enabling Office.js and WPS JS Add-ins
//! to communicate with the local equation conversion engine and popup editor.
//!
//! # Key Capabilities
//!
//! * **CORS & Private Network Access (PNA):** Automatically responds to Chromium /
//!   WebView2 preflight requests with `Access-Control-Allow-Private-Network: true`.
//! * **Session Lifecycle Management:** Manages active equation sessions with
//!   long-polling synchronization (`Condvar`) between the Add-in and Desktop GUI.
//! * **Instant Stateless Conversions:** Rapid snippet and batch document conversion.
//! * **Custom URI Handler:** Parses `matheditor://open?...` URLs for launching
//!   the editor when the server is in cold state.

pub mod protocol;
pub mod server;
pub mod session;
pub mod uri;

pub use protocol::*;
pub use server::{
    start_bridge_server, start_bridge_server_with_callback, ServerHandle, WindowCallback,
    DEFAULT_HOST, DEFAULT_PORT,
};
pub use session::{SessionManager, SessionState};
pub use uri::{format_custom_uri, parse_custom_uri, CustomUriAction};
