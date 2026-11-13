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
    let font_pt = font_size_pt.unwrap_or(12.0);
    match (from_format.as_str(), to_format.as_str()) {
        ("mathml", "flat_opc_omml") => {
            let node =
                eqntide::MathNode::parse(&data).map_err(|e| format!("parse MathML failed: {e}"))?;
            eqntide::mathml_to_flat_opc(&node, eqntide::FlatOpcMode::ModernOmml, font_pt)
                .map_err(|e| format!("flat opc pack failed: {e}"))
        }
        ("mathml", "flat_opc_mathtype") => {
            let node =
                eqntide::MathNode::parse(&data).map_err(|e| format!("parse MathML failed: {e}"))?;
            eqntide::mathml_to_flat_opc(&node, eqntide::FlatOpcMode::ModernMathtypeOleEmf, font_pt)
                .map_err(|e| format!("flat opc pack failed: {e}"))
        }
        ("flat_opc", "mathml") | ("omml", "mathml") => {
            let node = eqntide::flat_opc_to_mathml(&data)
                .map_err(|e| format!("extract mathml failed: {e}"))?;
            Ok(node.to_xml())
        }
        ("mathml", "emf") | ("mathml", "emf_base64") => {
            let node =
                eqntide::MathNode::parse(&data).map_err(|e| format!("parse MathML failed: {e}"))?;
            let p = eqntide::render_emf(&node, font_pt)
                .map_err(|e| format!("render EMF failed: {e}"))?;
            Ok(eqntide::base64_encode(&p.emf))
        }
        ("mathml", "png") | ("mathml", "png_base64") => {
            let node =
                eqntide::MathNode::parse(&data).map_err(|e| format!("parse MathML failed: {e}"))?;
            let prev = eqntide::render_preview(&node, font_pt)
                .map_err(|e| format!("render PNG failed: {e}"))?;
            Ok(eqntide::base64_encode(&prev.png))
        }
        ("mathml", "omml") => {
            let node =
                eqntide::MathNode::parse(&data).map_err(|e| format!("parse MathML failed: {e}"))?;
            eqntide::mathml_to_omml_xml(&node).map_err(|e| format!("render OMML failed: {e}"))
        }
        _ => Err(format!(
            "unsupported conversion from '{from_format}' to '{to_format}'"
        )),
    }
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
    let in_p = std::path::Path::new(&input_path);
    if !in_p.exists() {
        return Err(format!("Input file does not exist: {input_path}"));
    }
    let out_p = output_path.as_deref().unwrap_or(&input_path);
    let font_pt = font_size_pt.unwrap_or(12.0);

    let summary = match direction.as_str() {
        "legacy_to_omml" | "upgrade_modern_omml" | "upgrade_omml" => {
            let opts = eqntide::UpgradeOptions {
                target: eqntide::UpgradeTarget::ModernOmml,
                font_pt,
                math_font: None,
            };
            eqntide::upgrade_legacy_mathtype_docx_with(in_p, out_p, &opts)
                .map_err(|e| e.to_string())?
        }
        "legacy_to_mathtype" | "upgrade_modern_mathtype" | "upgrade_mathtype" => {
            let opts = eqntide::UpgradeOptions {
                target: eqntide::UpgradeTarget::ModernMathType,
                font_pt,
                math_font: None,
            };
            eqntide::upgrade_legacy_mathtype_docx_with(in_p, out_p, &opts)
                .map_err(|e| e.to_string())?
        }
        "omml_to_mathtype" | "mathtype" => {
            let opts = eqntide::Options {
                font_pt,
                preview: eqntide::PreviewFormat::Emf,
                ..Default::default()
            };
            eqntide::convert_with(in_p, out_p, &opts).map_err(|e| e.to_string())?
        }
        "mathtype_to_omml" | "omml" => {
            let opts = eqntide::ToOmmlOptions::default();
            eqntide::convert_to_omml_with(in_p, out_p, &opts).map_err(|e| e.to_string())?
        }
        other => return Err(format!("unsupported conversion direction: {other}")),
    };

    Ok(serde_json::json!({
        "status": "ok",
        "input_path": input_path,
        "output_path": out_p,
        "found": summary.found,
        "converted": summary.converted,
        "skipped": summary.skipped,
        "errors": summary.errors
    }))
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
                // Pass command line arguments (e.g. custom URI matheditor://open?...)
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
