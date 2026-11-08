#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};

#[tauri::command]
fn get_server_status() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "status": "running",
        "bridge_port": omml_converter::DEFAULT_PORT,
        "bridge_host": omml_converter::DEFAULT_HOST,
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
            let node = omml_converter::MathNode::parse(&data)
                .map_err(|e| format!("parse MathML failed: {e}"))?;
            omml_converter::mathml_to_flat_opc(&node, omml_converter::FlatOpcMode::ModernOmml, font_pt)
                .map_err(|e| format!("flat opc pack failed: {e}"))
        }
        ("mathml", "flat_opc_mathtype") => {
            let node = omml_converter::MathNode::parse(&data)
                .map_err(|e| format!("parse MathML failed: {e}"))?;
            omml_converter::mathml_to_flat_opc(
                &node,
                omml_converter::FlatOpcMode::ModernMathtypeOleEmf,
                font_pt,
            )
            .map_err(|e| format!("flat opc pack failed: {e}"))
        }
        ("flat_opc", "mathml") | ("omml", "mathml") => {
            let node = omml_converter::flat_opc_to_mathml(&data)
                .map_err(|e| format!("extract mathml failed: {e}"))?;
            Ok(node.to_xml())
        }
        ("mathml", "emf") | ("mathml", "emf_base64") => {
            let node = omml_converter::MathNode::parse(&data)
                .map_err(|e| format!("parse MathML failed: {e}"))?;
            let p = omml_converter::render_emf(&node, font_pt)
                .map_err(|e| format!("render EMF failed: {e}"))?;
            Ok(omml_converter::base64_encode(&p.emf))
        }
        ("mathml", "png") | ("mathml", "png_base64") => {
            let node = omml_converter::MathNode::parse(&data)
                .map_err(|e| format!("parse MathML failed: {e}"))?;
            let prev = omml_converter::render_preview(&node, font_pt)
                .map_err(|e| format!("render PNG failed: {e}"))?;
            Ok(omml_converter::base64_encode(&prev.png))
        }
        ("mathml", "omml") => {
            let node = omml_converter::MathNode::parse(&data)
                .map_err(|e| format!("parse MathML failed: {e}"))?;
            omml_converter::mathml_to_omml_xml(&node)
                .map_err(|e| format!("render OMML failed: {e}"))
        }
        _ => Err(format!("unsupported conversion from '{from_format}' to '{to_format}'")),
    }
}

#[tauri::command]
fn hide_window(window: tauri::Window) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())
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
            let opts = omml_converter::UpgradeOptions {
                target: omml_converter::UpgradeTarget::ModernOmml,
                font_pt,
                math_font: None,
            };
            omml_converter::upgrade_legacy_mathtype_docx_with(in_p, out_p, &opts)
                .map_err(|e| e.to_string())?
        }
        "legacy_to_mathtype" | "upgrade_modern_mathtype" | "upgrade_mathtype" => {
            let opts = omml_converter::UpgradeOptions {
                target: omml_converter::UpgradeTarget::ModernMathType,
                font_pt,
                math_font: None,
            };
            omml_converter::upgrade_legacy_mathtype_docx_with(in_p, out_p, &opts)
                .map_err(|e| e.to_string())?
        }
        "omml_to_mathtype" | "mathtype" => {
            let opts = omml_converter::Options {
                font_pt,
                preview: omml_converter::PreviewFormat::Emf,
                ..Default::default()
            };
            omml_converter::convert_with(in_p, out_p, &opts)
                .map_err(|e| e.to_string())?
        }
        "mathtype_to_omml" | "omml" => {
            let opts = omml_converter::ToOmmlOptions::default();
            omml_converter::convert_to_omml_with(in_p, out_p, &opts)
                .map_err(|e| e.to_string())?
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

    let window_cb: omml_converter::WindowCallback = Arc::new(move |session_id, mathml, latex| {
        let lock = app_handle_cb.lock().unwrap();
        if let Some(ref app) = *lock {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
                let _ = window.emit("open-session", serde_json::json!({
                    "session_id": session_id,
                    "mathml": mathml,
                    "latex": latex
                }));
            }
        }
    });

    // 1. Start background local loopback bridge server (127.0.0.1:17890)
    let bind_addr = format!(
        "{}:{}",
        omml_converter::DEFAULT_HOST,
        omml_converter::DEFAULT_PORT
    );
    let _bridge_server = match omml_converter::start_bridge_server_with_callback(&bind_addr, Some(window_cb)) {
        Ok(handle) => {
            log::info!("Local bridge HTTP server active on {}", handle.addr());
            Some(handle)
        }
        Err(e) => {
            log::warn!("Could not start bridge server (possibly already running on port {bind_addr}): {e}");
            None
        }
    };

    let is_quitting = Arc::new(AtomicBool::new(false));
    let is_quitting_clone = Arc::clone(&is_quitting);
    let is_quitting_setup = Arc::clone(&is_quitting);
    let is_quitting_exit = Arc::clone(&is_quitting);

    let app_handle_setup = Arc::clone(&app_handle_state);

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
                // Pass command line arguments (e.g. custom URI matheditor://open?...)
                let _ = window.emit("single-instance-launch", argv);
            }
        }))
        .setup(move |app| {
            *app_handle_setup.lock().unwrap() = Some(app.handle().clone());
            // Build Tray Menu
            let show_item = MenuItem::with_id(app, "show", "Show Equation Editor", true, None::<&str>)?;
            let batch_item = MenuItem::with_id(app, "batch", "Batch Document Converter", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&show_item, &batch_item, &quit_item])?;

            let tray_builder = TrayIconBuilder::new()
                .menu(&tray_menu)
                .tooltip("Math Equation Editor & Converter (Word/WPS)")
                .show_menu_on_left_click(false)
                .on_menu_event({
                    let is_quitting_menu = Arc::clone(&is_quitting_setup);
                    move |app, event| match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            }
                        }
                        "batch" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                                let _ = window.emit("open-batch-tab", ());
                            }
                        }
                        "quit" => {
                            is_quitting_menu.store(true, Ordering::SeqCst);
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let is_visible = window.is_visible().unwrap_or(false);
                            if is_visible {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            }
                        }
                    }
                });

            // Set tray icon if default window icon exists
            if let Some(icon) = app.default_window_icon() {
                let _ = tray_builder.icon(icon.clone()).build(app)?;
            } else {
                let _ = tray_builder.build(app)?;
            }

            Ok(())
        })
        .on_window_event(move |window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if !is_quitting_clone.load(Ordering::SeqCst) {
                    api.prevent_close();
                    let _ = window.hide();
                    let _ = window.emit("window-close-requested", ());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_server_status,
            convert_equation,
            hide_window,
            convert_document,
            open_folder
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(move |_app_handle, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = event {
            if !is_quitting_exit.load(Ordering::SeqCst) {
                api.prevent_exit();
            }
        }
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

        let roundtrip_mathml = convert_equation(
            flat_opc,
            "flat_opc".to_string(),
            "mathml".to_string(),
            None,
        )
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
