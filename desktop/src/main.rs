//! Native Rust desktop. No webview, JavaScript runtime, or MathLive dependency.
#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app;
mod batch;
mod commands;
mod instance;
mod palette;
mod runtime;

use eqntide::bridge::{DesktopWindowRequest, ServerHandle};
use eqntide::render::InteractivePreview;
use std::sync::mpsc;

pub struct EventSender<T = Event> {
    channel: mpsc::Sender<T>,
    repaint: std::sync::Arc<std::sync::OnceLock<eframe::egui::Context>>,
}
impl<T> Clone for EventSender<T> {
    fn clone(&self) -> Self {
        Self {
            channel: self.channel.clone(),
            repaint: self.repaint.clone(),
        }
    }
}
impl<T> EventSender<T> {
    fn new(channel: mpsc::Sender<T>) -> Self {
        Self {
            channel,
            repaint: Default::default(),
        }
    }
    fn send(&self, event: T) -> Result<(), mpsc::SendError<T>> {
        self.channel.send(event)?;
        if let Some(ctx) = self.repaint.get() {
            ctx.request_repaint();
        }
        Ok(())
    }
    fn attach(&self, ctx: eframe::egui::Context) {
        let _ = self.repaint.set(ctx);
    }
}

pub struct Rendered {
    placeholders: Vec<eqntide::render::HitRegion>,
    preview: InteractivePreview,
    image: eframe::egui::ColorImage,
}
pub enum Event {
    Launch(Vec<String>),
    Window(DesktopWindowRequest),
    Bridge(Result<Option<ServerHandle>, String>),
    Render(u64, Result<Rendered, String>),
    Latex(u64, Result<eqntide::MathNode, String>),
    Session(String, Result<serde_json::Value, String>),
    Finished(Result<(), String>),
    Paths(Vec<std::path::PathBuf>),
    Batch(
        usize,
        Result<eqntide::bridge::ConvertDocumentResponse, String>,
    ),
    BatchDone,
}

fn log_file_path(filename: &str) -> Option<std::path::PathBuf> {
    let dirs = directories::ProjectDirs::from("com", "eqntide", "EqnTide")?;
    let log_dir = dirs.data_local_dir().join("logs");
    let _ = std::fs::create_dir_all(&log_dir);
    Some(log_dir.join(filename))
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = mpsc::channel();
    let tx = EventSender::new(tx);
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--version") {
        println!("eqntide-desktop {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let Some(instance) = instance::acquire(&args, tx.clone())? else {
        return Ok(());
    };
    let icon =
        eframe::icon_data::from_png_bytes(include_bytes!("../../assets/icons/icon.png")).ok();
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title("EqnTide — Equation Editor")
        .with_inner_size([1100.0, 650.0])
        .with_min_inner_size(app::MIN_WINDOW_SIZE);
    if let Some(icon) = icon {
        viewport = viewport.with_icon(icon);
    }

    let renderer_env = std::env::var("EQNTIDE_RENDERER").unwrap_or_default();
    let use_wgpu = args.iter().any(|a| a == "--wgpu") || renderer_env.eq_ignore_ascii_case("wgpu");
    let use_glow = args.iter().any(|a| a == "--glow") || renderer_env.eq_ignore_ascii_case("glow");

    let renderer = if use_wgpu {
        eframe::Renderer::Wgpu
    } else if use_glow {
        eframe::Renderer::Glow
    } else {
        // Default to Glow (OpenGL) across all platforms.
        // In VMs (VMware, VirtualBox, Hyper-V) and remote sessions, Glow uses direct
        // guest hardware-accelerated OpenGL / WGL, whereas Wgpu defaults to DirectX 12
        // which falls back to unaccelerated WARP CPU rasterization and severe lag.
        eframe::Renderer::Glow
    };

    let wgpu_options = {
        let mut config = eframe::egui_wgpu::WgpuConfiguration::default();
        config.surface.present_mode = eframe::egui_wgpu::wgpu::PresentMode::AutoNoVsync;
        config
    };

    let options = eframe::NativeOptions {
        viewport,
        renderer,
        wgpu_options,
        ..Default::default()
    };
    eframe::run_native(
        "EqnTide",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, instance, tx, rx, args)))),
    )?;
    Ok(())
}

fn main() {
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("EqnTide crashed unexpectedly:\n\n{info}");
        if let Some(path) = log_file_path("desktop_panic.log") {
            let _ = std::fs::write(&path, &msg);
        }
        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title("EqnTide Crash")
            .set_description(&msg)
            .show();
    }));

    if let Err(err) = run() {
        let msg = format!("Failed to start EqnTide desktop:\n\n{err}");
        if let Some(path) = log_file_path("desktop_error.log") {
            let _ = std::fs::write(&path, &msg);
        }
        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title("EqnTide Startup Failed")
            .set_description(&msg)
            .show();
        std::process::exit(1);
    }
}
