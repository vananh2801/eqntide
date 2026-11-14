//! Tauri desktop application entry point.
//!
//! This crate adapts the public `eqntide` library for the bundled web UI. It
//! owns window lifecycle and Tauri command boundaries; MathML, OMML, MTEF, and
//! DOCX conversion rules remain in the library so the CLI and desktop app
//! produce the same results.

#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;
use tauri::{Emitter, Manager, WindowEvent};

#[tauri::command]
fn get_server_status() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "status": "running",
        "bridge_port": eqntide::DEFAULT_PORT,
        "bridge_host": eqntide::DEFAULT_HOST,
        "version": env!("CARGO_PKG_VERSION")
    }))
}

#[tauri::command]
fn convert_equation(
    data: String,
    from_format: String,
    to_format: String,
    font_size_pt: Option<f32>,
) -> Result<String, String> {
    eqntide::service::convert_snippet(&eqntide::bridge::ConvertSnippetRequest {
        data,
        from: from_format,
        to: to_format,
        font_pt: font_size_pt.unwrap_or(12.0),
    })
    .map(|response| response.result)
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn hide_window(app: tauri::AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}

#[tauri::command]
fn show_window(window: tauri::Window) -> Result<(), String> {
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_always_on_top(true);
    let _ = window.set_focus();
    let _ = window.set_always_on_top(false);
    Ok(())
}

#[tauri::command]
fn convert_document(
    input_path: String,
    output_path: Option<String>,
    direction: String,
    font_size_pt: Option<f32>,
) -> Result<serde_json::Value, String> {
    eqntide::service::convert_document(&eqntide::bridge::ConvertDocumentRequest {
        input_path: input_path.clone(), output_path, target: direction, preview: "emf".into(), font_pt: font_size_pt.unwrap_or(12.0),
    }).map(|response| serde_json::json!({
        "status": response.status, "input_path": input_path, "output_path": response.output_path,
        "found": response.found, "converted": response.converted, "skipped": response.skipped, "errors": response.errors,
    })).map_err(|error| error.to_string())
}

#[tauri::command]
fn open_folder(path: String) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    let target_dir = if p.is_file() {
        p.parent().unwrap_or(p)
    } else {
        p
    };

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(target_dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(target_dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(target_dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn main() {
    let app_handle_state = Arc::new(std::sync::Mutex::new(None::<tauri::AppHandle>));
    let app_handle_cb = Arc::clone(&app_handle_state);

    let window_cb: eqntide::WindowCallback = Arc::new(move |session_id, mathml, latex, font_pt| {
        let lock = app_handle_cb.lock().unwrap();
        if let Some(ref app) = *lock {
            let app_clone = app.clone();
            let sid = session_id.to_string();
            let m = mathml.map(|s| s.to_string());
            let l = latex.map(|s| s.to_string());
            let _ = app.run_on_main_thread(move || {
                let payload = serde_json::json!({
                    "session_id": sid,
                    "mathml": m,
                    "latex": l,
                    "font_pt": font_pt,
                });
                let _ = app_clone.emit("open-session", &payload);
                let _ = app_clone.emit_to(tauri::EventTarget::any(), "open-session", &payload);

                if let Some(window) = app_clone.get_webview_window("main") {
                    let _ = window.unminimize();
                    let _ = window.show();
                    let _ = window.set_always_on_top(true);
                    let _ = window.set_focus();
                    let _ = window.set_always_on_top(false);
                    let _ = window.emit("open-session", &payload);

                    if let Ok(json_str) = serde_json::to_string(&payload) {
                        let eval_js = format!(
                            "if (window.__loadSession) {{ window.__loadSession({}); }}",
                            json_str
                        );
                        let _ = window.eval(&eval_js);
                    }
                }
            });
        }
    });

    let window_cb_setup = Arc::clone(&window_cb);

    let app_handle_setup = Arc::clone(&app_handle_state);

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_always_on_top(true);
                let _ = window.set_focus();
                let _ = window.set_always_on_top(false);
                // Pass command line arguments (e.g. custom URI eqntide://open?...)
                let _ = window.emit("single-instance-launch", argv);
            }
        }))
        .setup(move |app| {
            *app_handle_setup.lock().unwrap() = Some(app.handle().clone());

            // Check if daemon is already running on 127.0.0.1:17890
            let bind_addr = format!("{}:{}", eqntide::DEFAULT_HOST, eqntide::DEFAULT_PORT);
            if std::net::TcpStream::connect(&bind_addr).is_ok() {
                log::info!(
                    "Daemon bridge server already active on {bind_addr}, using existing daemon."
                );
            } else {
                match eqntide::start_bridge_server_with_callback(
                    &bind_addr,
                    Some(Arc::clone(&window_cb_setup)),
                ) {
                    Ok(handle) => {
                        log::info!("Local bridge HTTP server active on {}", handle.addr());
                        app.manage(handle);
                    }
                    Err(e) => {
                        log::warn!("Could not bind bridge server on {bind_addr}: {e}");
                    }
                }
            }

            Ok(())
        })
        .on_window_event(move |_window, event| {
            if let WindowEvent::CloseRequested { .. } = event {
                std::process::exit(0);
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_server_status,
            convert_equation,
            hide_window,
            show_window,
            convert_document,
            open_folder
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(move |_app_handle, event| {
        println!("Tauri RunEvent: {:?}", event);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_status_command() {
        let status = get_server_status().expect("get status");
        assert_eq!(status["status"], "running");
        assert_eq!(status["bridge_port"], 17890);
    }

    #[test]
    fn test_convert_equation_command_roundtrip() {
        let mathml = "<math><mi>x</mi><mo>+</mo><mn>1</mn></math>";
        let flat_opc = convert_equation(
            mathml.to_string(),
            "mathml".to_string(),
            "flat_opc_omml".to_string(),
            None,
        )
        .expect("convert to flat opc");

        assert!(flat_opc.contains("pkg:package"));
        assert!(flat_opc.contains("m:oMath"));

        let roundtrip_mathml =
            convert_equation(flat_opc, "flat_opc".to_string(), "mathml".to_string(), None)
                .expect("convert back to mathml");

        assert!(roundtrip_mathml.contains("<math"));
        assert!(roundtrip_mathml.contains("x"));
    }

    #[test]
    fn test_convert_equation_mathtype_mode() {
        let mathml = "<math><mi>y</mi><mo>=</mo><msup><mi>x</mi><mn>2</mn></msup></math>";
        let flat_opc = convert_equation(
            mathml.to_string(),
            "mathml".to_string(),
            "flat_opc_mathtype".to_string(),
            Some(14.0),
        )
        .expect("convert to flat opc mathtype");

        assert!(flat_opc.contains("pkg:package"));
        assert!(flat_opc.contains("oleObject"));
        assert!(flat_opc.contains(".emf"));
    }

    #[test]
    fn test_convert_equation_emf_and_png() {
        let mathml = "<math><mi>x</mi><mo>+</mo><mn>1</mn></math>";
        let emf_b64 = convert_equation(
            mathml.to_string(),
            "mathml".to_string(),
            "emf".to_string(),
            Some(12.0),
        )
        .expect("convert to emf");
        assert!(!emf_b64.is_empty());

        let png_b64 = convert_equation(
            mathml.to_string(),
            "mathml".to_string(),
            "png".to_string(),
            Some(12.0),
        )
        .expect("convert to png");
        assert!(!png_b64.is_empty());

        let omml_xml = convert_equation(
            mathml.to_string(),
            "mathml".to_string(),
            "omml".to_string(),
            None,
        )
        .expect("convert to omml");
        assert!(omml_xml.contains("m:oMath"));
    }

    #[test]
    fn test_convert_document_nonexistent_fails() {
        let res = convert_document(
            "/nonexistent/file.docx".into(),
            None,
            "upgrade_modern_omml".into(),
            None,
        );
        assert!(res.is_err());
    }
}
