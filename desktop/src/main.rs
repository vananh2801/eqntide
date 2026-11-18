//! Native Rust desktop. No webview, JavaScript runtime, or MathLive dependency.
#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app;
mod batch;
mod commands;
mod instance;
mod palette;
mod runtime;
mod settings;
mod theme;
mod wgpu_runner;

use eqntide::bridge::{DesktopWindowRequest, ServerHandle};
use eqntide::render::InteractivePreview;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};

pub struct EventSender<T = Event> {
    channel: mpsc::Sender<T>,
    repaint: Arc<std::sync::OnceLock<eframe::egui::Context>>,
    urgent: Arc<AtomicBool>,
}
impl<T> Clone for EventSender<T> {
    fn clone(&self) -> Self {
        Self {
            channel: self.channel.clone(),
            repaint: self.repaint.clone(),
            urgent: self.urgent.clone(),
        }
    }
}
impl<T> EventSender<T> {
    fn new(channel: mpsc::Sender<T>) -> Self {
        Self {
            channel,
            repaint: Default::default(),
            urgent: Arc::new(AtomicBool::new(false)),
        }
    }
    fn send(&self, event: T) -> Result<(), mpsc::SendError<T>> {
        self.channel.send(event)?;
        self.urgent.store(true, Ordering::Release);
        if let Some(ctx) = self.repaint.get() {
            ctx.request_repaint();
        }
        Ok(())
    }
    fn send_local(&self, event: T) -> Result<(), mpsc::SendError<T>> {
        self.channel.send(event)
    }
    fn attach(&self, ctx: eframe::egui::Context) {
        let _ = self.repaint.set(ctx);
    }
    fn urgent_handle(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.urgent)
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

pub(crate) fn log_file_path(filename: &str) -> Option<std::path::PathBuf> {
    let dirs = directories::ProjectDirs::from("com", "eqntide", "EqnTide")?;
    let log_dir = dirs.data_local_dir().join("logs");
    let _ = std::fs::create_dir_all(&log_dir);
    Some(log_dir.join(filename))
}

#[cfg(not(target_os = "windows"))]
fn has_wgpu_adapter(config: eframe::egui_wgpu::WgpuConfiguration) -> bool {
    let eframe::egui_wgpu::WgpuSetup::CreateNew(setup) = config.wgpu_setup else {
        return true;
    };
    let instance = eframe::egui_wgpu::wgpu::Instance::new(setup.instance_descriptor);
    pollster::block_on(
        instance.request_adapter(&eframe::egui_wgpu::wgpu::RequestAdapterOptions {
            power_preference: setup.power_preference,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }),
    )
    .is_ok()
}

fn desktop_viewport() -> eframe::egui::ViewportBuilder {
    let icon =
        eframe::icon_data::from_png_bytes(include_bytes!("../../assets/icons/icon.png")).ok();
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_app_id("eqntide")
        .with_title("EqnTide — Equation Editor")
        .with_inner_size(app::MIN_WINDOW_SIZE)
        .with_min_inner_size(app::MIN_WINDOW_SIZE);
    if let Some(icon) = icon {
        viewport = viewport.with_icon(icon);
    }
    viewport
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
    let viewport = desktop_viewport();

    let renderer_env = std::env::var("EQNTIDE_RENDERER").unwrap_or_default();
    let use_wgpu = args.iter().any(|a| a == "--wgpu") || renderer_env.eq_ignore_ascii_case("wgpu");
    let use_glow = args.iter().any(|a| a == "--glow") || renderer_env.eq_ignore_ascii_case("glow");

    let present_env = std::env::var("EQNTIDE_PRESENT_MODE").unwrap_or_default();
    let use_novsync =
        args.iter().any(|a| a == "--novsync") || present_env.eq_ignore_ascii_case("novsync");

    let wgpu_options = {
        let mut setup = eframe::egui_wgpu::WgpuSetupCreateNew::without_display_handle();
        #[cfg(not(debug_assertions))]
        if std::env::var_os("WGPU_VALIDATION").is_none() {
            setup.instance_descriptor.flags = eframe::egui_wgpu::wgpu::InstanceFlags::empty();
        }
        setup.device_descriptor = Arc::new(|adapter| {
            let info = adapter.get_info();
            let adapter_limits = adapter.limits();
            let base = if info.backend == eframe::egui_wgpu::wgpu::Backend::Gl {
                eframe::egui_wgpu::wgpu::Limits::downlevel_webgl2_defaults()
            } else {
                eframe::egui_wgpu::wgpu::Limits::default()
            };
            let max_dim = if info.device_type == eframe::egui_wgpu::wgpu::DeviceType::Cpu {
                adapter_limits.max_texture_dimension_2d.min(8192)
            } else {
                adapter_limits.max_texture_dimension_2d
            };
            eframe::egui_wgpu::wgpu::DeviceDescriptor {
                label: Some("egui wgpu device"),
                required_limits: eframe::egui_wgpu::wgpu::Limits {
                    max_texture_dimension_2d: max_dim,
                    ..base
                },
                ..Default::default()
            }
        });

        let mut config = eframe::egui_wgpu::WgpuConfiguration {
            wgpu_setup: eframe::egui_wgpu::WgpuSetup::CreateNew(setup),
            ..Default::default()
        };
        config.surface.present_mode = if use_novsync {
            eframe::egui_wgpu::wgpu::PresentMode::AutoNoVsync
        } else {
            eframe::egui_wgpu::wgpu::PresentMode::AutoVsync
        };
        config.surface.desired_maximum_frame_latency = Some(2);
        config
    };

    let renderer = if use_wgpu {
        eframe::Renderer::Wgpu
    } else if use_glow {
        eframe::Renderer::Glow
    } else {
        #[cfg(target_os = "windows")]
        {
            eframe::Renderer::Wgpu
        }
        #[cfg(not(target_os = "windows"))]
        {
            if has_wgpu_adapter(wgpu_options.clone()) {
                eframe::Renderer::Wgpu
            } else {
                eframe::Renderer::Glow
            }
        }
    };

    if renderer == eframe::Renderer::Wgpu {
        return wgpu_runner::run(viewport, wgpu_options, instance, tx, rx, args);
    }

    let options = eframe::NativeOptions {
        viewport,
        renderer,
        wgpu_options,
        multisampling: 0,
        depth_buffer: 0,
        stencil_buffer: 0,
        dithering: false,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_viewport_sets_app_id_icon_and_compact_default_size() {
        let viewport = desktop_viewport();
        assert_eq!(viewport.app_id.as_deref(), Some("eqntide"));
        assert!(viewport.icon.is_some());
        assert_eq!(
            viewport.inner_size,
            Some(eframe::egui::Vec2::from(app::MIN_WINDOW_SIZE))
        );
        assert_eq!(
            viewport.min_inner_size,
            Some(eframe::egui::Vec2::from(app::MIN_WINDOW_SIZE))
        );
    }
}
