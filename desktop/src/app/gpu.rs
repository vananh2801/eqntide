//! GPU tier detection, host/VM heuristics, and embedded font registration.

use eframe::egui;
use std::sync::Arc;

/// Rendering hardware capability tier controlling animation and raster scaling choices.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum GpuProfile {
    /// Hardware-accelerated native GPU adapter.
    Native,
    /// Virtualized GPU adapter running inside a VM or remote session.
    VirtualGpu,
    /// CPU software rasterizer fallback.
    Software,
}

/// Returns `true` if the adapter name belongs to a known software rasterizer.
pub(crate) fn is_software_adapter_name(name: &str) -> bool {
    let lower_name = name.to_lowercase();
    [
        "llvmpipe",
        "lavapipe",
        "swiftshader",
        "basic render",
        "basic display",
        "microsoft basic",
        "gdi generic",
        "software",
    ]
    .iter()
    .any(|keyword| lower_name.contains(keyword))
}

/// Registers the bundled Nunito, JetBrains Mono, and STIX Two fonts with `egui_ctx`.
pub(super) fn register_fonts(egui_ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "nunito".into(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../../../assets/fonts/Nunito/Nunito-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "jetbrains-mono".into(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../../../assets/fonts/JetBrainsMono-2.304/JetBrainsMono-Regular.ttf"
        ))),
    );
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "nunito".into());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, "jetbrains-mono".into());
    fonts.font_data.insert(
        "stix-text".into(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../../../assets/fonts/stixfonts-2.13b171/STIXTwoText-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "stix-math".into(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../../../assets/fonts/stixfonts-2.13b171/STIXTwoMath-Regular.ttf"
        ))),
    );
    for family in fonts.families.values_mut() {
        family.push("stix-text".into());
        family.push("stix-math".into());
    }
    egui_ctx.set_fonts(fonts);
}

/// Detects the active [`GpuProfile`] and applies matching animation, shadow, and DPI tweaks.
pub(super) fn detect_and_apply_gpu_profile(
    egui_ctx: &egui::Context,
    wgpu_render_state: Option<&eframe::egui_wgpu::RenderState>,
    is_software_backend: bool,
    args: &[String],
) -> GpuProfile {
    let gpu_profile = detect_gpu_profile(egui_ctx, wgpu_render_state, is_software_backend);
    if matches!(gpu_profile, GpuProfile::VirtualGpu | GpuProfile::Software) {
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            egui_ctx.style_mut_of(theme, |style| {
                style.animation_time = 0.0;
                style.visuals.text_cursor.blink = false;
                style.visuals.window_shadow = egui::Shadow::NONE;
                style.visuals.popup_shadow = egui::Shadow::NONE;
                if gpu_profile == GpuProfile::Software {
                    style.interaction.tooltip_delay = 0.8;
                }
            });
        }
        egui_ctx.tessellation_options_mut(|options| options.feathering = false);
    }
    let force_scale_1 = args.iter().any(|arg| arg == "--scale1")
        || std::env::var("EQNTIDE_SOFTWARE_SCALE")
            .is_ok_and(|val| val.trim() == "1" || val.trim() == "1.0");
    if gpu_profile == GpuProfile::Software && force_scale_1 {
        egui_ctx.set_pixels_per_point(1.0);
    }
    gpu_profile
}

fn detect_host_vm_profile() -> GpuProfile {
    if std::env::var("LIBGL_ALWAYS_SOFTWARE")
        .is_ok_and(|val| matches!(val.trim(), "1" | "true" | "yes"))
    {
        return GpuProfile::Software;
    }
    #[cfg(target_os = "linux")]
    {
        for path in [
            "/sys/class/dmi/id/product_name",
            "/sys/class/dmi/id/sys_vendor",
            "/sys/class/dmi/id/board_vendor",
        ] {
            if let Ok(raw) = std::fs::read_to_string(path) {
                let lower = raw.to_lowercase();
                if [
                    "qemu",
                    "kvm",
                    "vmware",
                    "virtualbox",
                    "vbox",
                    "bochs",
                    "hyper-v",
                    "parallels",
                    "xen",
                    "bhyve",
                    "microsoft corporation",
                ]
                .iter()
                .any(|keyword| lower.contains(keyword))
                {
                    return GpuProfile::VirtualGpu;
                }
            }
        }
        if let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") {
            if cpuinfo.contains("hypervisor") {
                return GpuProfile::VirtualGpu;
            }
        }
    }
    GpuProfile::Native
}

fn detect_gpu_profile(
    egui_ctx: &egui::Context,
    wgpu_render_state: Option<&eframe::egui_wgpu::RenderState>,
    is_software_backend: bool,
) -> GpuProfile {
    let env_profile = std::env::var("EQNTIDE_GPU_PROFILE").unwrap_or_default();
    if env_profile.eq_ignore_ascii_case("software") {
        return GpuProfile::Software;
    }
    if env_profile.eq_ignore_ascii_case("virtual") || env_profile.eq_ignore_ascii_case("virtualgpu")
    {
        return GpuProfile::VirtualGpu;
    }
    if env_profile.eq_ignore_ascii_case("native") {
        return GpuProfile::Native;
    }

    if is_software_backend {
        if let Some(log_path) = crate::log_file_path("desktop_gpu.log") {
            let summary = format!(
                "profile=Software backend=Software device_type=Cpu ppp={:.2}\n",
                egui_ctx.pixels_per_point()
            );
            let _ = std::fs::write(log_path, summary);
        }
        return GpuProfile::Software;
    }

    let Some(render_state) = wgpu_render_state else {
        let profile = detect_host_vm_profile();
        if let Some(log_path) = crate::log_file_path("desktop_gpu.log") {
            let summary = format!(
                "profile={profile:?} backend=Glow device_type=Unknown ppp={:.2}\n",
                egui_ctx.pixels_per_point()
            );
            let _ = std::fs::write(log_path, summary);
        }
        return profile;
    };
    let info = render_state.adapter.get_info();
    let lower_name = info.name.to_lowercase();

    let software = info.device_type == eframe::egui_wgpu::wgpu::DeviceType::Cpu
        || is_software_adapter_name(&info.name);
    let profile = if software {
        GpuProfile::Software
    } else {
        let virtual_gpu = info.device_type == eframe::egui_wgpu::wgpu::DeviceType::VirtualGpu
            || [
                "virtio",
                "vmware",
                "virtualbox",
                "svga",
                "qxl",
                "parallels",
                "hyper-v",
                "qemu",
                "red hat",
                "bochs",
                "paravirtual",
                "remote display",
            ]
            .iter()
            .any(|keyword| lower_name.contains(keyword));
        if virtual_gpu {
            GpuProfile::VirtualGpu
        } else {
            detect_host_vm_profile()
        }
    };

    if let Some(log_path) = crate::log_file_path("desktop_gpu.log") {
        let summary = format!(
            "profile={profile:?} backend={:?} device_type={:?} name={:?} driver={:?} driver_info={:?} ppp={:.2}\n",
            info.backend,
            info.device_type,
            info.name,
            info.driver,
            info.driver_info,
            egui_ctx.pixels_per_point()
        );
        let _ = std::fs::write(log_path, summary);
    }

    profile
}
