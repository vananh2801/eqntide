//! Native Rust desktop. No webview, JavaScript runtime, or MathLive dependency.
#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app;
mod batch;
mod instance;
mod palette;
mod runtime;

use eqntide::bridge::{DesktopWindowRequest, ServerHandle};
use eqntide::render::InteractivePreview;
use std::sync::mpsc;

/// Wake the event loop whenever a worker or IPC connection produces a result.
/// Idle windows do not need to poll a channel or redraw continuously.
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
    Copied(Result<String, String>),
    Image(arboard::ImageData<'static>),
    Paths(Vec<std::path::PathBuf>),
    Batch(
        usize,
        Result<eqntide::bridge::ConvertDocumentResponse, String>,
    ),
    BatchDone,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
        .with_inner_size([940.0, 650.0])
        .with_min_inner_size([600.0, 450.0]);
    if let Some(icon) = icon {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "EqnTide",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, instance, tx, rx, args)))),
    )?;
    Ok(())
}
