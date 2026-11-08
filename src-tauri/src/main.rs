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
        _ => Err(format!("unsupported conversion from '{from_format}' to '{to_format}'")),
    }
}

#[tauri::command]
fn hide_window(window: tauri::Window) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())
}

fn main() {
    // 1. Start background local loopback bridge server (127.0.0.1:17890)
    let bind_addr = format!(
        "{}:{}",
        omml_converter::DEFAULT_HOST,
        omml_converter::DEFAULT_PORT
    );
    let _bridge_server = match omml_converter::start_bridge_server(&bind_addr) {
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

    tauri::Builder::default()
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
                    let is_quitting_menu = Arc::clone(&is_quitting);
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
                // If user did not explicitly select "Quit" from tray, prevent close and emit event to frontend
                if !is_quitting_clone.load(Ordering::SeqCst) {
                    api.prevent_close();
                    let _ = window.emit("window-close-requested", ());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_server_status,
            convert_equation,
            hide_window
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
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
}
