use crate::EventSender as Sender;
use crate::{
    batch, commands,
    instance::Instance,
    palette, runtime,
    settings::{self, Settings, StartupFocus, ThemeMode, MAX_RECENT_PALETTE, ZOOM_RANGE},
    theme::{self, CanvasPalette, CanvasPreset, ThemeTokens},
    Event, Rendered,
};
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use eqntide::{
    bridge::{DesktopWindowRequest, ServerHandle},
    MathNode,
};
use eqntide_editor::{Caret, Editor, Move};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Receiver,
        Arc,
    },
    time::{Duration, Instant},
};

const CONTENT_PADDING: i8 = theme::SPACE_MD as i8;
const FONT_SIZE_CONTROL_WIDTH: f32 = 96.0;
const FOOTER_CONTROL_HEIGHT: f32 = theme::BUTTON_HEIGHT;
pub(crate) const MIN_WINDOW_SIZE: [f32; 2] = [1056.0, 560.0];
const PALETTE_BUTTON_SIZE: Vec2 = Vec2::new(46.0, 32.0);
const PALETTE_GAP: f32 = theme::SPACE_XS;
const SIDEBAR_CATEGORY_ICON_SIZE: f32 = 22.0;
const UI_ICON_SIZE: f32 = 16.0;
const PALETTE_ICON_TARGET_EM: f32 = 17.0;
const PALETTE_HEIGHT: f32 =
    3.0 * PALETTE_BUTTON_SIZE.y + 2.0 * PALETTE_GAP + CONTENT_PADDING as f32;
const LATEX_PANEL_HEIGHT: f32 = 88.0;
const LATEX_INPUT_HEIGHT: f32 = LATEX_PANEL_HEIGHT - CONTENT_PADDING as f32;
const PLACEHOLDER_ATTR: &str = "data-eqntide-placeholder";

fn tabler_icon_bytes(name: &str) -> &'static [u8] {
    match name {
        "alpha" => include_bytes!("../../assets/icons/rendered/alpha.png"),
        "arrow-autofit-right" => {
            include_bytes!("../../assets/icons/rendered/arrow-autofit-right.png")
        }
        "arrow-back-up" => include_bytes!("../../assets/icons/rendered/arrow-back-up.png"),
        "arrow-forward-up" => include_bytes!("../../assets/icons/rendered/arrow-forward-up.png"),
        "arrows-left-right" => include_bytes!("../../assets/icons/rendered/arrows-left-right.png"),
        "brand-github" => include_bytes!("../../assets/icons/rendered/brand-github.png"),
        "chevron-left" => include_bytes!("../../assets/icons/rendered/chevron-left.png"),
        "file" => include_bytes!("../../assets/icons/rendered/file.png"),
        "file-type-docx" => include_bytes!("../../assets/icons/rendered/file-type-docx.png"),
        "file-upload" => include_bytes!("../../assets/icons/rendered/file-upload.png"),
        "folder" => include_bytes!("../../assets/icons/rendered/folder.png"),
        "folder-open" => include_bytes!("../../assets/icons/rendered/folder-open.png"),
        "history" => include_bytes!("../../assets/icons/rendered/history.png"),
        "info-circle" => include_bytes!("../../assets/icons/rendered/info-circle.png"),
        "license" => include_bytes!("../../assets/icons/rendered/license.png"),
        "math-avg" => include_bytes!("../../assets/icons/rendered/math-avg.png"),
        "math-equal-lower" => include_bytes!("../../assets/icons/rendered/math-equal-lower.png"),
        "math-integral" => include_bytes!("../../assets/icons/rendered/math-integral.png"),
        "math-sin" => include_bytes!("../../assets/icons/rendered/math-sin.png"),
        "math-x-divide-y" => include_bytes!("../../assets/icons/rendered/math-x-divide-y.png"),
        "matrix" => include_bytes!("../../assets/icons/rendered/matrix.png"),
        "omega" => include_bytes!("../../assets/icons/rendered/omega.png"),
        "overline" => include_bytes!("../../assets/icons/rendered/overline.png"),
        "parentheses" => include_bytes!("../../assets/icons/rendered/parentheses.png"),
        "settings" => include_bytes!("../../assets/icons/rendered/settings.png"),
        "square-root-2" => include_bytes!("../../assets/icons/rendered/square-root-2.png"),
        "sum" => include_bytes!("../../assets/icons/rendered/sum.png"),
        "underline" => include_bytes!("../../assets/icons/rendered/underline.png"),
        "user" => include_bytes!("../../assets/icons/rendered/user.png"),
        "x-power-y" => include_bytes!("../../assets/icons/rendered/x-power-y.png"),
        other => panic!("unknown tabler icon: {other}"),
    }
}

#[derive(Clone)]
struct MatrixDialog {
    rows: usize,
    columns: usize,
    alignment: &'static str,
    fence: &'static str,
}

impl Default for MatrixDialog {
    fn default() -> Self {
        Self {
            rows: 2,
            columns: 2,
            alignment: "center",
            fence: "parentheses",
        }
    }
}

#[derive(Clone, Copy)]
enum Intent {
    Close,
    Next,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum GpuProfile {
    Native,
    VirtualGpu,
    Software,
}

struct PaletteAtlasEntry {
    uv: Rect,
    display_size: Vec2,
}

struct PaletteAtlas {
    texture: egui::TextureHandle,
    entries: HashMap<&'static str, PaletteAtlasEntry>,
}

fn detect_host_vm_profile() -> GpuProfile {
    if std::env::var("LIBGL_ALWAYS_SOFTWARE")
        .is_ok_and(|v| matches!(v.trim(), "1" | "true" | "yes"))
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
                .any(|k| lower.contains(k))
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

    let Some(rs) = wgpu_render_state else {
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
    let info = rs.adapter.get_info();
    let name = info.name.to_lowercase();

    let software = info.device_type == eframe::egui_wgpu::wgpu::DeviceType::Cpu
        || [
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
        .any(|k| name.contains(k));
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
            .any(|k| name.contains(k));
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

fn frame_cap_for_profile(profile: GpuProfile) -> Option<Duration> {
    if let Ok(raw) = std::env::var("EQNTIDE_SOFTWARE_FRAME_MS") {
        if let Ok(ms) = raw.parse::<u64>() {
            return (ms > 0).then(|| Duration::from_millis(ms));
        }
    }
    match profile {
        GpuProfile::Native => None,
        GpuProfile::VirtualGpu => Some(Duration::from_millis(33)),
        GpuProfile::Software => Some(Duration::from_millis(66)),
    }
}

pub struct App {
    _instance: Option<Instance>,
    tx: Sender<Event>,
    rx: Receiver<Event>,
    bridge: Option<ServerHandle>,
    bridge_ready: bool,
    bridge_status: String,
    editor: Editor,
    saved_xml: String,
    saved_font: f32,
    saved_mode: String,
    latex: String,
    draft_dirty: bool,
    latex_generation: u64,
    latex_deadline: Option<Instant>,
    latex_parsing: Option<u64>,
    latex_error: Option<String>,
    command_error: Option<String>,
    session: Option<String>,
    loading_session: Option<String>,
    queue: VecDeque<DesktopWindowRequest>,
    startup: Vec<String>,
    busy: bool,
    save_clicked: bool,
    pending: Option<Intent>,
    allow_close: bool,
    close_deadline: Option<Instant>,
    font_pt: f32,
    target: String,
    zoom: f32,
    tab: usize,
    category: String,
    matrix_dialog: Option<MatrixDialog>,
    palette_atlases: HashMap<String, PaletteAtlas>,
    ui_icons: HashMap<&'static str, egui::TextureHandle>,
    palette_ppp: f32,
    recent_palette: Vec<&'static str>,
    gpu_profile: GpuProfile,
    software_frame_cap: Option<Duration>,
    passive_canvas_rect: Option<Rect>,
    last_enforced_viewport: Option<(Vec2, bool)>,
    logo_texture: Option<egui::TextureHandle>,
    status: String,
    texture: Option<egui::TextureHandle>,
    rendered: Option<Rendered>,
    render_key: u64,
    requested_key: u64,
    render_busy: bool,
    ime: String,
    command_input: Option<commands::Input>,
    latex_groups: Vec<u64>,
    expects_latex_group: bool,
    files: Vec<batch::BatchItem>,
    batch_rows: Vec<batch::BatchRowState>,
    batch_selection_mode: batch::BatchSelectionMode,
    batch_collapsed_dirs: HashSet<String>,
    batch_logs: Vec<String>,
    batch_target: usize,
    batch_overwrite: bool,
    batch_totals: [usize; 4],
    batch_completed: usize,
    batch_run_total: usize,
    batch_run_target: String,
    batch_run_overwrite: bool,
    batch_run_timestamp: String,
    batch_running: bool,
    stop: Arc<AtomicBool>,
    warning_cache: Option<(u64, u64, String, Option<String>)>,
    keyboard_handled: bool,
    settings_path: Option<PathBuf>,
    persisted: Settings,
    settings_dirty: bool,
    canvas_light: CanvasPreset,
    canvas_dark: CanvasPreset,
    startup_focus: StartupFocus,
    active_editor_focus: StartupFocus,
    pending_focus_restore: Option<StartupFocus>,
    caret_blink: Option<(Caret, u64, usize, f64)>,
}

impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        instance: Instance,
        tx: Sender<Event>,
        rx: Receiver<Event>,
        args: Vec<String>,
    ) -> Self {
        Self::new_with_state(
            &cc.egui_ctx,
            cc.wgpu_render_state.as_ref(),
            instance,
            tx,
            rx,
            args,
        )
    }

    pub(crate) fn new_with_state(
        egui_ctx: &egui::Context,
        wgpu_render_state: Option<&eframe::egui_wgpu::RenderState>,
        instance: Instance,
        tx: Sender<Event>,
        rx: Receiver<Event>,
        args: Vec<String>,
    ) -> Self {
        egui_ctx.set_theme(egui::ThemePreference::System);
        ThemeTokens::apply_style(egui_ctx);
        tx.attach(egui_ctx.clone());
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "nunito".into(),
            Arc::new(egui::FontData::from_static(include_bytes!(
                "../../assets/fonts/Nunito/Nunito-Regular.ttf"
            ))),
        );
        fonts.font_data.insert(
            "jetbrains-mono".into(),
            Arc::new(egui::FontData::from_static(include_bytes!(
                "../../assets/fonts/JetBrainsMono-2.304/JetBrainsMono-Regular.ttf"
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
                "../../assets/fonts/stixfonts-2.13b171/STIXTwoText-Regular.ttf"
            ))),
        );
        fonts.font_data.insert(
            "stix-math".into(),
            Arc::new(egui::FontData::from_static(include_bytes!(
                "../../assets/fonts/stixfonts-2.13b171/STIXTwoMath-Regular.ttf"
            ))),
        );
        for family in fonts.families.values_mut() {
            family.push("stix-text".into());
            family.push("stix-math".into());
        }
        let gpu_profile = detect_gpu_profile(egui_ctx, wgpu_render_state);
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
                    style.spacing.scroll.dormant_handle_opacity =
                        style.spacing.scroll.active_handle_opacity;
                    style.spacing.scroll.interact_handle_opacity =
                        style.spacing.scroll.active_handle_opacity;
                    style.spacing.scroll.dormant_background_opacity =
                        style.spacing.scroll.active_background_opacity;
                    style.spacing.scroll.interact_background_opacity =
                        style.spacing.scroll.active_background_opacity;
                    style.spacing.scroll.floating_allocated_width = style.spacing.scroll.bar_width;
                });
            }
            egui_ctx.tessellation_options_mut(|o| o.feathering = false);
        }
        let force_scale_1 = args.iter().any(|a| a == "--scale1")
            || std::env::var("EQNTIDE_SOFTWARE_SCALE")
                .is_ok_and(|v| v.trim() == "1" || v.trim() == "1.0");
        if gpu_profile == GpuProfile::Software && force_scale_1 {
            egui_ctx.set_pixels_per_point(1.0);
        }
        egui_ctx.set_fonts(fonts);
        runtime::start_bridge(tx.clone());
        let mut app = Self::from_parts(Some(instance), tx, rx, args);
        app.gpu_profile = gpu_profile;
        app.software_frame_cap = frame_cap_for_profile(gpu_profile);
        if let Some(path) = Settings::default_path() {
            app = app.with_settings_path(path, Some(egui_ctx));
        }
        app
    }

    pub(crate) fn with_settings_path(
        mut self,
        path: PathBuf,
        egui_ctx: Option<&egui::Context>,
    ) -> Self {
        let loaded = Settings::load_from_path(&path);
        self.settings_path = Some(path);
        self.apply_settings(loaded, egui_ctx);
        self
    }

    fn apply_settings(&mut self, settings: Settings, egui_ctx: Option<&egui::Context>) {
        let settings = settings.sanitized();
        self.zoom = settings.zoom;
        self.font_pt = settings.font_pt;
        self.saved_font = settings.font_pt;
        self.target = settings.target.clone();
        self.saved_mode = settings.target.clone();
        self.category = settings.category.clone();
        self.recent_palette = settings.resolved_recent_palette();
        self.batch_target = settings.batch_target;
        self.batch_overwrite = settings.batch_overwrite;
        self.canvas_light = settings.canvas_light;
        self.canvas_dark = settings.canvas_dark;
        self.startup_focus = settings.startup_focus;
        self.active_editor_focus = settings.startup_focus;
        self.pending_focus_restore = Some(settings.startup_focus);
        if let Some(ctx) = egui_ctx {
            ctx.set_theme(settings.theme.to_egui());
        }
        self.persisted = settings;
        self.settings_dirty = false;
    }

    fn active_canvas_palette(&self, dark_mode: bool) -> CanvasPalette {
        let preset = if dark_mode {
            self.canvas_dark
        } else {
            self.canvas_light
        };
        preset.palette(dark_mode)
    }

    fn editor_focus_id(target: StartupFocus) -> egui::Id {
        match target {
            StartupFocus::Canvas => egui::Id::unique("equation-canvas"),
            StartupFocus::Latex => egui::Id::unique("latex-source"),
        }
    }

    fn sync_editor_focus(&mut self, ui: &mut egui::Ui) {
        if self.tab != 0 || !ui.is_enabled() {
            return;
        }
        let canvas_id = Self::editor_focus_id(StartupFocus::Canvas);
        let latex_id = Self::editor_focus_id(StartupFocus::Latex);
        let (has_canvas, has_latex) =
            ui.memory(|m| (m.has_focus(canvas_id), m.has_focus(latex_id)));
        if let Some(target) = self.pending_focus_restore.take() {
            if has_canvas {
                self.active_editor_focus = StartupFocus::Canvas;
            } else if has_latex {
                self.active_editor_focus = StartupFocus::Latex;
                self.command_input = None;
            } else {
                ui.memory_mut(|m| m.request_focus(Self::editor_focus_id(target)));
                self.active_editor_focus = target;
            }
        } else if has_canvas {
            self.active_editor_focus = StartupFocus::Canvas;
        } else if has_latex {
            self.active_editor_focus = StartupFocus::Latex;
            self.command_input = None;
        }
    }

    fn canvas_caret_visible(&mut self, ui: &egui::Ui) -> bool {
        let style = ui.visuals().text_cursor.clone();
        if !style.blink {
            self.caret_blink = None;
            return true;
        }
        let now = ui.input(|i| i.time);
        let caret = self.editor.caret();
        let revision = self.editor.revision;
        let command_len = self.command_input.as_ref().map_or(0, |c| c.name.len() + 1);
        let start = match self.caret_blink {
            Some((prev_caret, prev_rev, prev_cmd, start))
                if prev_caret == caret && prev_rev == revision && prev_cmd == command_len =>
            {
                start
            }
            _ => {
                self.caret_blink = Some((caret, revision, command_len, now));
                now
            }
        };
        let on = f64::from(style.on_duration.max(0.05));
        let off = f64::from(style.off_duration.max(0.05));
        let period = on + off;
        let elapsed = (now - start).max(0.0);
        let phase = elapsed % period;
        let visible = phase < on;
        let next_toggle = if visible { on - phase } else { period - phase };
        ui.ctx()
            .request_repaint_after(Duration::from_secs_f64(next_toggle.max(0.01)));
        visible
    }

    fn primary_button_with<R>(
        ui: &mut egui::Ui,
        enabled: bool,
        add_button: impl FnOnce(&mut egui::Ui) -> R,
    ) -> R {
        let tokens = ThemeTokens::from_ui(ui);
        ui.scope(|ui| {
            let widgets = &mut ui.visuals_mut().widgets;
            if enabled {
                widgets.inactive.bg_fill = tokens.primary_bg;
                widgets.inactive.weak_bg_fill = tokens.primary_bg;
                widgets.inactive.bg_stroke = Stroke::new(1.0, tokens.primary_bg);
                widgets.inactive.fg_stroke = Stroke::new(1.0, tokens.primary_fg);

                widgets.hovered.bg_fill = tokens.primary_hover_bg;
                widgets.hovered.weak_bg_fill = tokens.primary_hover_bg;
                widgets.hovered.bg_stroke = Stroke::new(1.0, tokens.primary_hover_bg);
                widgets.hovered.fg_stroke = Stroke::new(1.0, tokens.primary_fg);

                widgets.active.bg_fill = tokens.primary_active_bg;
                widgets.active.weak_bg_fill = tokens.primary_active_bg;
                widgets.active.bg_stroke = Stroke::new(1.0, tokens.primary_active_bg);
                widgets.active.fg_stroke = Stroke::new(1.0, tokens.primary_fg);
            } else {
                widgets.noninteractive.bg_fill = tokens.control_bg;
                widgets.noninteractive.weak_bg_fill = tokens.control_bg;
                widgets.noninteractive.bg_stroke = Stroke::new(1.0, tokens.border);
                widgets.noninteractive.fg_stroke = Stroke::new(1.0, tokens.text_primary);
                widgets.inactive.bg_fill = tokens.control_bg;
                widgets.inactive.weak_bg_fill = tokens.control_bg;
                widgets.inactive.bg_stroke = Stroke::new(1.0, tokens.border);
                widgets.inactive.fg_stroke = Stroke::new(1.0, tokens.text_primary);
            }
            ui.add_enabled_ui(enabled, add_button).inner
        })
        .inner
    }

    fn primary_button(ui: &mut egui::Ui, enabled: bool, label: &str) -> egui::Response {
        Self::primary_button_with(ui, enabled, |ui| {
            ui.add(egui::Button::new(label).min_size(Vec2::new(0.0, theme::BUTTON_HEIGHT)))
        })
    }

    fn icon_primary_button(
        &mut self,
        ui: &mut egui::Ui,
        enabled: bool,
        icon: &'static str,
        label: &str,
    ) -> egui::Response {
        let tokens = ThemeTokens::from_ui(ui);
        let tint = if enabled {
            tokens.primary_fg
        } else {
            tokens.text_primary
        };
        let image = self.ui_icon(ui.ctx(), icon, UI_ICON_SIZE, tint);
        Self::primary_button_with(ui, enabled, |ui| {
            ui.add(
                egui::Button::image_and_text(image, label)
                    .min_size(Vec2::new(0.0, theme::BUTTON_HEIGHT)),
            )
        })
    }

    fn standard_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
        ui.add(egui::Button::new(label).min_size(Vec2::new(0.0, theme::BUTTON_HEIGHT)))
    }

    fn icon_standard_button(
        &mut self,
        ui: &mut egui::Ui,
        icon: &'static str,
        label: &str,
    ) -> egui::Response {
        let tint = ThemeTokens::from_ui(ui).icon_tint(false, ui.is_enabled());
        let image = self.ui_icon(ui.ctx(), icon, UI_ICON_SIZE, tint);
        ui.add(
            egui::Button::image_and_text(image, label)
                .min_size(Vec2::new(0.0, theme::BUTTON_HEIGHT)),
        )
    }

    fn compact_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
        ui.scope(|ui| {
            ui.spacing_mut().button_padding = Vec2::new(theme::SPACE_MD, 3.0);
            ui.add(egui::Button::new(label).min_size(Vec2::new(0.0, theme::COMPACT_BUTTON_HEIGHT)))
        })
        .inner
    }

    fn with_selectable_style<R>(
        ui: &mut egui::Ui,
        selected: bool,
        add_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> R {
        let tokens = ThemeTokens::from_ui(ui);
        ui.scope(|ui| {
            if selected {
                let widgets = &mut ui.visuals_mut().widgets;
                widgets.inactive.bg_fill = tokens.selected_bg;
                widgets.inactive.weak_bg_fill = tokens.selected_bg;
                widgets.inactive.bg_stroke = Stroke::new(1.0, tokens.selected_border);
                widgets.inactive.fg_stroke = Stroke::new(1.0, tokens.selected_fg);

                widgets.hovered.bg_fill = tokens.selected_hover_bg;
                widgets.hovered.weak_bg_fill = tokens.selected_hover_bg;
                widgets.hovered.bg_stroke = Stroke::new(1.0, tokens.selected_border);
                widgets.hovered.fg_stroke = Stroke::new(1.0, tokens.selected_fg);

                widgets.active.bg_fill = tokens.selected_hover_bg;
                widgets.active.weak_bg_fill = tokens.selected_hover_bg;
                widgets.active.bg_stroke = Stroke::new(1.0, tokens.selected_border);
                widgets.active.fg_stroke = Stroke::new(1.0, tokens.selected_fg);
            }
            add_contents(ui)
        })
        .inner
    }

    fn full_width_card<R>(
        ui: &mut egui::Ui,
        tokens: ThemeTokens,
        add_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> egui::InnerResponse<R> {
        tokens.group_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            add_contents(ui)
        })
    }

    fn flush_settings(&mut self) {
        if !self.settings_dirty {
            return;
        }
        self.settings_dirty = false;
        let Some(path) = self.settings_path.as_deref() else {
            return;
        };
        let to_save = self.persisted.clone().sanitized();
        if let Err(err) = to_save.save_to_path(path) {
            eprintln!("Failed to save settings {}: {err}", path.display());
        }
    }

    pub(crate) fn frame_cap(&self) -> Option<Duration> {
        self.software_frame_cap
    }
    pub(crate) fn passive_hover_rect(&self) -> Option<Rect> {
        self.passive_canvas_rect
    }
    pub(crate) fn has_active_overlay(&self) -> bool {
        self.busy
            || self.pending.is_some()
            || self.matrix_dialog.is_some()
            || self.command_input.is_some()
    }
    pub(crate) fn has_active_animation(&self) -> bool {
        (self.busy || self.batch_running) && self.gpu_profile != GpuProfile::Software
    }
    fn from_parts(
        instance: Option<Instance>,
        tx: Sender<Event>,
        rx: Receiver<Event>,
        args: Vec<String>,
    ) -> Self {
        let editor = Editor::default();
        let xml = editor.math().to_xml();
        Self {
            _instance: instance,
            tx,
            rx,
            bridge: None,
            bridge_ready: false,
            bridge_status: "Connecting bridge…".into(),
            editor,
            saved_xml: xml,
            saved_font: 12.0,
            saved_mode: "omml".into(),
            latex: String::new(),
            draft_dirty: false,
            latex_generation: 0,
            latex_deadline: None,
            latex_parsing: None,
            latex_error: None,
            command_error: None,
            session: None,
            loading_session: None,
            queue: VecDeque::new(),
            startup: args,
            busy: false,
            save_clicked: false,
            pending: None,
            allow_close: false,
            close_deadline: None,
            font_pt: 12.0,
            target: "omml".into(),
            zoom: 2.0,
            tab: 0,
            category: palette::DEFAULT_GROUP.into(),
            matrix_dialog: None,
            palette_atlases: HashMap::new(),
            ui_icons: HashMap::new(),
            palette_ppp: 0.0,
            recent_palette: Vec::new(),
            gpu_profile: GpuProfile::Native,
            software_frame_cap: None,
            passive_canvas_rect: None,
            last_enforced_viewport: None,
            logo_texture: None,
            status: "Click the canvas to type. Use the palette or Ctrl+F for a fraction.".into(),
            texture: None,
            rendered: None,
            render_key: 1,
            requested_key: 0,
            render_busy: false,
            ime: String::new(),
            command_input: None,
            latex_groups: vec![],
            expects_latex_group: false,
            files: vec![],
            batch_rows: vec![],
            batch_selection_mode: batch::BatchSelectionMode::default(),
            batch_collapsed_dirs: HashSet::new(),
            batch_logs: vec![],
            batch_target: 0,
            batch_overwrite: false,
            batch_totals: [0; 4],
            batch_completed: 0,
            batch_run_total: 0,
            batch_run_target: String::new(),
            batch_run_overwrite: false,
            batch_run_timestamp: String::new(),
            batch_running: false,
            stop: Arc::new(AtomicBool::new(false)),
            warning_cache: None,
            keyboard_handled: false,
            settings_path: None,
            persisted: Settings::default(),
            settings_dirty: false,
            canvas_light: CanvasPreset::SoftPaper,
            canvas_dark: CanvasPreset::SoftPaper,
            startup_focus: StartupFocus::Canvas,
            active_editor_focus: StartupFocus::Canvas,
            pending_focus_restore: Some(StartupFocus::Canvas),
            caret_blink: None,
        }
    }
    fn dirty(&self) -> bool {
        self.draft_dirty
            || self.command_input.is_some()
            || self.editor.math().to_xml() != self.saved_xml
            || self.font_pt != self.saved_font
            || self.target != self.saved_mode
    }
    fn changed(&mut self) {
        self.render_key += 1;
        self.latex = self.editor.math().to_latex();
        self.draft_dirty = false;
        self.latex_generation += 1;
        self.latex_deadline = None;
        self.latex_error = None;
        self.command_error = None;
    }
    fn source_changed(&mut self, now: Instant) {
        self.command_input = None;
        self.latex_groups.clear();
        self.expects_latex_group = false;
        self.draft_dirty = true;
        self.latex_generation += 1;
        self.latex_deadline = Some(now + Duration::from_millis(180));
        self.latex_error = None;
    }
    fn take_latex_request(&mut self, now: Instant) -> Option<(u64, String)> {
        if !self.draft_dirty || self.latex_parsing.is_some() || now < self.latex_deadline? {
            return None;
        }
        self.latex_deadline = None;
        self.latex_parsing = Some(self.latex_generation);
        Some((self.latex_generation, self.latex.clone()))
    }
    fn parse_desktop_latex(source: &str) -> Result<MathNode, String> {
        if source.trim().is_empty() {
            return Ok(MathNode::new("math"));
        }
        commands::validate_desktop_latex(source)?;
        let math = eqntide::service::parse_latex_to_mathml(source).map_err(|e| e.to_string())?;
        Editor::try_new(&math).map(|_| math).map_err(str::to_owned)
    }
    fn request_latex_parse(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        if let Some((generation, source)) = self.take_latex_request(now) {
            let tx = self.tx.clone();
            std::thread::spawn(move || {
                let result = Self::parse_desktop_latex(&source);
                let _ = tx.send(Event::Latex(generation, result));
            });
        }
        if let Some(deadline) = self.latex_deadline.filter(|_| self.latex_parsing.is_none()) {
            ctx.request_repaint_after(deadline.saturating_duration_since(now));
        }
    }
    fn launch(&mut self, args: Vec<String>, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        for arg in args {
            if arg.starts_with("eqntide:") {
                match eqntide::bridge::parse_custom_uri(&arg) {
                    Ok(action) if action.command == "open" => {
                        if action.tab.as_deref() == Some("batch") {
                            self.tab = 1;
                        }
                        if let Some(id) = action.session_id {
                            if self.session.as_ref() != Some(&id)
                                && !self.queue.iter().any(|q| q.session_id == id)
                            {
                                self.window(DesktopWindowRequest {
                                    session_id: id,
                                    mathml: None,
                                    latex: None,
                                    font_pt: None,
                                    tab: None,
                                    target_mode: action.target_mode.clone(),
                                });
                            }
                        } else if let Some(data) = action.data {
                            match eqntide::service::decode_input_to_mathml(
                                &data,
                                action.format.as_deref().unwrap_or("auto"),
                            ) {
                                Ok(math) => {
                                    self.window(DesktopWindowRequest {
                                        session_id: String::new(),
                                        mathml: Some(math.to_xml()),
                                        latex: None,
                                        font_pt: None,
                                        tab: None,
                                        target_mode: action.target_mode.clone(),
                                    });
                                }
                                Err(e) => self.status = e.to_string(),
                            }
                        }
                    }
                    Ok(_) => self.status = "Unsupported EqnTide URI command".into(),
                    Err(e) => self.status = e.into(),
                }
            } else {
                let path = PathBuf::from(arg);
                if path.exists() {
                    self.add_files(vec![path]);
                    self.tab = 1;
                }
            }
        }
    }
    fn window(&mut self, request: DesktopWindowRequest) {
        if request.tab.as_deref() == Some("batch") {
            self.tab = 1;
        }
        if request.session_id.is_empty() && request.mathml.is_none() {
            return;
        }
        if !request.session_id.is_empty()
            && (self.session.as_ref() == Some(&request.session_id)
                || self.loading_session.as_ref() == Some(&request.session_id)
                || self
                    .queue
                    .iter()
                    .any(|r| r.session_id == request.session_id))
        {
            return;
        }
        self.queue.push_back(request);
        if self.busy {
            return;
        }
        if self.dirty() || self.session.is_some() {
            if self.pending.is_none() {
                self.pending = Some(Intent::Next);
            }
        } else {
            self.load_next();
        }
    }
    fn load_next(&mut self) {
        self.pending = None;
        if let Some(request) = self.queue.pop_front() {
            self.tab = 0;
            self.active_editor_focus = self.startup_focus;
            self.pending_focus_restore = Some(self.startup_focus);
            if request.session_id.is_empty() {
                if self.load_math(
                    request.mathml.as_deref().unwrap_or(""),
                    request.latex.as_deref(),
                    request.font_pt,
                ) {
                    if let Some(mode) = request
                        .target_mode
                        .filter(|m| m == "omml" || m == "mathtype")
                    {
                        self.target = mode;
                        self.saved_mode = self.target.clone();
                    }
                }
            } else {
                self.busy = true;
                let id = request.session_id;
                self.loading_session = Some(id.clone());
                let tx = self.tx.clone();
                std::thread::spawn(move || {
                    let result = runtime::get(&format!("/session/{id}"));
                    let _ = tx.send(Event::Session(id, result));
                });
            }
        }
    }
    fn load_math(&mut self, xml: &str, latex: Option<&str>, font: Option<f32>) -> bool {
        let result = if !xml.trim().is_empty() {
            MathNode::parse(xml)
        } else if let Some(l) = latex.filter(|l| !l.trim().is_empty()) {
            eqntide::service::parse_latex_to_mathml(l)
        } else {
            Ok(MathNode::new("math"))
        };
        match result {
            Ok(math) => {
                match Editor::try_new(&math) {
                    Ok(editor) => {
                        self.editor = editor;
                        self.matrix_dialog = None;
                        self.command_input = None;
                        self.latex_groups.clear();
                        self.expects_latex_group = false;
                        self.active_editor_focus = self.startup_focus;
                        self.pending_focus_restore = Some(self.startup_focus);
                    }
                    Err(e) => {
                        self.status = e.into();
                        return false;
                    }
                }
                self.font_pt = font
                    .filter(|f| f.is_finite() && *f > 0.0)
                    .unwrap_or(self.persisted.font_pt);
                self.saved_font = self.font_pt;
                self.changed();
                self.saved_xml = self.editor.math().to_xml();
                self.saved_mode = self.target.clone();
                true
            }
            Err(e) => {
                self.status = format!("Could not load equation; existing document retained: {e}");
                false
            }
        }
    }
    fn events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                Event::Launch(args) => {
                    if self.bridge_ready {
                        self.launch(args, ctx);
                    } else {
                        self.startup.extend(args);
                    }
                }
                Event::Window(r) => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    self.window(r);
                }
                Event::Bridge(result) => match result {
                    Ok(handle) => {
                        self.bridge_status = if handle.is_some() {
                            "Bridge: hosted by desktop"
                        } else {
                            "Bridge: external CLI/service"
                        }
                        .into();
                        self.bridge = handle;
                        self.bridge_ready = true;
                        let args = std::mem::take(&mut self.startup);
                        if !args.is_empty() {
                            self.launch(args, ctx);
                        }
                    }
                    Err(e) => {
                        self.bridge_status = e;
                        self.status="Standalone editing remains available. Free port 17890, then retry the bridge.".into();
                    }
                },
                Event::Latex(generation, result) => {
                    if self.latex_parsing == Some(generation) {
                        self.latex_parsing = None;
                    }
                    if generation != self.latex_generation || !self.draft_dirty {
                        continue;
                    }
                    match result {
                        Ok(math) => {
                            if self.editor.math() != math {
                                self.editor.replace(&math);
                                self.render_key += 1;
                            }
                            // Keep the user's source, spacing and cursor position while typing.
                            self.draft_dirty = false;
                            self.latex_error = None;
                        }
                        Err(error) => self.latex_error = Some(error),
                    }
                }
                Event::Render(key, result) => {
                    self.render_busy = false;
                    if key == self.render_key {
                        match result {
                            Ok(mut rendered) => {
                                let image = std::mem::take(&mut rendered.image);
                                if let Some(texture) = &mut self.texture {
                                    texture.set(image, egui::TextureOptions::LINEAR);
                                } else {
                                    self.texture = Some(ctx.load_texture(
                                        "equation",
                                        image,
                                        egui::TextureOptions::LINEAR,
                                    ));
                                }
                                self.rendered = Some(rendered);
                            }
                            Err(e) => {
                                self.status = format!("Render error: {e}");
                                self.texture = None;
                                self.rendered = None;
                            }
                        }
                    }
                }
                Event::Session(id, result) => {
                    if self.loading_session.as_ref() != Some(&id) {
                        continue;
                    }
                    self.loading_session = None;
                    self.busy = false;
                    match result {
                        Ok(value) => {
                            if value["state"].as_str().is_some_and(|s| s != "waiting") {
                                self.status = format!(
                                    "Session {id} has already completed; current work retained."
                                );
                                continue;
                            }
                            if self.load_math(
                                value["mathml"].as_str().unwrap_or(""),
                                value["latex"].as_str(),
                                value["font_pt"].as_f64().map(|f| f as f32),
                            ) {
                                if let Some(mode) = value["target_mode"]
                                    .as_str()
                                    .filter(|m| *m == "omml" || *m == "mathtype")
                                {
                                    self.target = mode.into();
                                    self.saved_mode = self.target.clone();
                                }
                                self.session = Some(id);
                                if !self.queue.is_empty() && self.pending.is_none() {
                                    self.pending = Some(Intent::Next);
                                }
                                self.status="Editing Office session. Save returns the equation to the add-in.".into();
                            }
                        }
                        Err(e) => {
                            self.status = format!("Session {id} unavailable: {e}");
                        }
                    }
                }
                Event::Finished(result) => {
                    self.busy = false;
                    match result {
                        Ok(()) => {
                            self.flush_settings();
                            self.saved_xml = self.editor.math().to_xml();
                            self.saved_font = self.font_pt;
                            self.saved_mode = self.target.clone();
                            self.draft_dirty = false;
                            let had_session = self.session.take().is_some();
                            if matches!(self.pending, Some(Intent::Close)) {
                                self.begin_close(ctx);
                            } else if !self.queue.is_empty() {
                                self.load_next();
                            } else if had_session {
                                self.begin_close(ctx);
                            } else {
                                self.pending = None;
                                self.status = "Saved.".into();
                            }
                        }
                        Err(e) => {
                            self.status = e;
                            self.pending = None;
                        }
                    }
                }
                Event::Paths(paths) => self.add_files(paths),
                Event::Batch(i, result) => {
                    self.batch_completed += 1;
                    self.sync_batch_row_state();
                    let log = match result {
                        Ok(r) => {
                            for (total, value) in self.batch_totals.iter_mut().zip([
                                r.found,
                                r.converted,
                                r.skipped,
                                r.errors,
                            ]) {
                                *total += value;
                            }
                            if let (Some(item), Some(row_state)) =
                                (self.files.get(i), self.batch_rows.get_mut(i))
                            {
                                row_state.last_output =
                                    Some(batch::BatchRowOutput::from_conversion(
                                        item,
                                        Path::new(&r.output_path),
                                        &self.batch_run_target,
                                        self.batch_run_overwrite,
                                        &self.batch_run_timestamp,
                                    ));
                            }
                            format!(
                                "{} converted, {} skipped, {} errors → {}",
                                r.converted, r.skipped, r.errors, r.output_path
                            )
                        }
                        Err(e) => {
                            self.batch_totals[3] += 1;
                            if let Some(row_state) = self.batch_rows.get_mut(i) {
                                row_state.last_output = None;
                            }
                            format!("Error: {e}")
                        }
                    };
                    if let Some(row) = self.batch_logs.get_mut(i) {
                        *row = log;
                    }
                }
                Event::BatchDone => {
                    self.batch_running = false;
                    for row in &mut self.batch_logs {
                        if row == "Waiting" {
                            *row = "Stopped".into();
                        }
                    }
                    self.status = if self.stop.load(Ordering::Relaxed) {
                        "Batch stopped."
                    } else {
                        "Batch finished."
                    }
                    .into();
                }
            }
        }
    }
    fn request_render(&mut self, ctx: &egui::Context) {
        if self.render_busy || self.requested_key == self.render_key {
            return;
        }
        self.render_busy = true;
        self.requested_key = self.render_key;
        let key = self.render_key;
        let font = self.font_pt;
        let mut math = self.editor.math();
        fn placeholders(n: &mut MathNode) {
            if eqntide_editor::is_opaque_tag(&n.tag) {
                n.text = format!("[{}]", n.tag);
                n.tag = "mtext".into();
                n.children.clear();
                n.attrs.clear();
                return;
            }
            if n.children.is_empty() && matches!(n.tag.as_str(), "math" | "mtd") {
                let mut slot = MathNode::leaf("mi", "□");
                slot.attrs.push((PLACEHOLDER_ATTR.into(), "true".into()));
                n.children.push(slot);
                return;
            }
            if n.text.is_empty()
                && n.children.is_empty()
                && matches!(
                    n.tag.as_str(),
                    "math" | "mrow" | "mtd" | "mi" | "mn" | "mtext"
                )
            {
                n.tag = "mi".into();
                n.text = "□".into();
                n.attrs.push((PLACEHOLDER_ATTR.into(), "true".into()));
            }
            for c in &mut n.children {
                placeholders(c);
            }
        }
        placeholders(&mut math);
        // The root is always a container, including the initially empty equation.
        if math.tag != "math" {
            math = MathNode::with_children("math", vec![math]);
        }
        let raster_scale = match self.gpu_profile {
            GpuProfile::Software | GpuProfile::VirtualGpu => {
                let ppp = ctx.pixels_per_point().max(1.0);
                (ppp * 2.5 / eqntide::render::PX_PER_PT).clamp(0.5, 1.0)
            }
            GpuProfile::Native => 1.0,
        };
        let result = eqntide::render::render_interactive_alpha_scaled(&math, font, raster_scale)
            .map_err(|e| e.to_string())
            .map(|(mask, preview)| {
                let placeholders = preview
                    .regions
                    .iter()
                    .filter(|region| {
                        region
                            .path
                            .iter()
                            .try_fold(&math, |node, &index| node.children.get(index))
                            .is_some_and(|node| {
                                node.attrs.iter().any(|(key, _)| key == PLACEHOLDER_ATTR)
                            })
                    })
                    .cloned()
                    .collect();
                let pixels = mask
                    .alpha
                    .into_iter()
                    .map(Color32::from_white_alpha)
                    .collect();
                let image = egui::ColorImage {
                    size: [mask.width as usize, mask.height as usize],
                    source_size: Vec2::new(mask.width as f32, mask.height as f32),
                    pixels,
                };
                Rendered {
                    placeholders,
                    image,
                    preview,
                }
            });
        let _ = self.tx.send_local(Event::Render(key, result));
    }
    fn sync_render(&mut self, ctx: &egui::Context) {
        if !self.render_busy && self.requested_key != self.render_key {
            self.request_render(ctx);
            self.events(ctx);
        }
    }
    fn sync_batch_row_state(&mut self) {
        let len = self.files.len();
        if self.batch_rows.len() < len {
            self.batch_rows
                .resize_with(len, batch::BatchRowState::default);
        } else if self.batch_rows.len() > len {
            self.batch_rows.truncate(len);
        }
        if self.batch_logs.len() < len {
            self.batch_logs.resize_with(len, || "Ready".into());
        } else if self.batch_logs.len() > len {
            self.batch_logs.truncate(len);
        }
    }

    fn selected_batch_count(&self) -> usize {
        self.batch_rows
            .iter()
            .take(self.files.len())
            .filter(|row| row.is_selected(self.batch_selection_mode))
            .count()
    }

    fn push_batch_item(&mut self, item: batch::BatchItem) -> bool {
        if let Some(idx) = self
            .files
            .iter()
            .position(|existing| existing.input == item.input)
        {
            if self.files[idx].source_root.is_none() && item.source_root.is_some() {
                self.files[idx].source_root = item.source_root;
            }
            return false;
        }
        self.files.push(item);
        self.batch_rows.push(batch::BatchRowState::default());
        self.batch_logs.push("Ready".into());
        true
    }

    fn add_files(&mut self, paths: Vec<PathBuf>) {
        if self.batch_running {
            return;
        }
        let mut added = 0usize;
        let mut saw_empty_dir = false;
        for path in paths {
            if path.is_dir() {
                let mut found = Vec::new();
                match eqntide::docx::collect_docx_files_recursive(&path, None, &mut found) {
                    Ok(()) => {
                        if found.is_empty() {
                            saw_empty_dir = true;
                        }
                        for file in found {
                            if self
                                .push_batch_item(batch::BatchItem::from_folder(file, path.clone()))
                            {
                                added += 1;
                            }
                        }
                    }
                    Err(e) => {
                        self.status = format!("Failed to scan folder {}: {e}", path.display());
                    }
                }
            } else if eqntide::docx::is_docx_file(&path)
                && self.push_batch_item(batch::BatchItem::file(path))
            {
                added += 1;
            }
        }
        if added > 0 {
            self.batch_run_total = 0;
            self.batch_completed = 0;
        } else if saw_empty_dir {
            self.status = "No .docx files found in selected folder.".into();
        }
    }
    fn finish(&mut self, save: bool) {
        if self.busy {
            return;
        }
        if save && self.command_input.is_some() {
            self.status =
                "Complete the LaTeX command with Tab/Enter or cancel with Esc before saving."
                    .into();
            return;
        }
        if save && self.draft_dirty {
            self.status = "Wait for valid LaTeX before saving.".into();
            return;
        }
        if save && self.session.is_some() {
            let math = self.editor.math();
            if self.target == "omml" {
                if let Some(unsupported) = eqntide::check_omml_unsupported(&math) {
                    self.status = format!(
                        "{unsupported} is not supported in OMML. Switch target to MathType to insert."
                    );
                    return;
                }
            } else if self.target == "mathtype" {
                if let Some(unsupported) = eqntide::check_mathtype_unsupported(&math) {
                    self.status = format!("{unsupported} is not supported in MathType.");
                    return;
                }
            }
        }
        if self.session.is_none() && save {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("MathML", &["mathml"])
                .set_file_name("equation.mathml")
                .save_file()
            {
                self.busy = true;
                let xml = self.editor.math().to_xml();
                let tx = self.tx.clone();
                std::thread::spawn(move || {
                    let _ = tx.send(Event::Finished(
                        std::fs::write(path, xml).map_err(|e| e.to_string()),
                    ));
                });
            }
            return;
        }
        self.busy = true;
        let session = self.session.clone();
        let mathml = self.editor.math().to_xml();
        let font = self.font_pt;
        let mode = self.target.clone();
        let tx = self.tx.clone();
        let queued: Vec<_> = if matches!(self.pending, Some(Intent::Close)) {
            self.queue
                .iter()
                .filter(|r| !r.session_id.is_empty())
                .map(|r| r.session_id.clone())
                .collect()
        } else {
            vec![]
        };
        std::thread::spawn(move || {
            let result = (|| {
                if let Some(id) = session {
                    let path = format!("/session/{id}/{}", if save { "update" } else { "cancel" });
                    let body =
                        serde_json::json!({"mathml":mathml,"target_mode":mode,"font_pt":font});
                    runtime::post(&path, &body)?;
                }
                for id in queued {
                    runtime::post(&format!("/session/{id}/cancel"), &serde_json::json!({}))?;
                }
                Ok(())
            })();
            let _ = tx.send(Event::Finished(result));
        });
    }
    fn insert_template(&mut self, source: &str) {
        match palette::template(source) {
            Ok(template) => {
                if palette::attaches_to_previous(source) {
                    self.editor.insert_template_wrapping_previous(&template);
                } else {
                    self.editor.insert_template(&template);
                }
                self.changed();
            }
            Err(e) => self.status = e,
        }
    }
    fn begin_close(&mut self, ctx: &egui::Context) {
        self.flush_settings();
        self.close_deadline = Some(Instant::now() + Duration::from_secs(3));
        self.busy = true;
        self.status = "Returning the result to Office…".into();
        ctx.request_repaint();
    }
    fn drain_before_close(&mut self, ctx: &egui::Context) {
        if let Some(deadline) = self.close_deadline {
            let now = Instant::now();
            // Give the client's next long-poll a brief chance to arrive, then
            // wait for active response writes, with a bounded shutdown timeout.
            let grace_elapsed = now >= deadline - Duration::from_millis(2500);
            if grace_elapsed
                && (self
                    .bridge
                    .as_ref()
                    .is_none_or(|server| server.pending_requests() == 0)
                    || now >= deadline)
            {
                self.close_deadline = None;
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                ctx.request_repaint_after(Duration::from_millis(20));
            }
        }
    }
    fn selected_equation(&self) -> MathNode {
        let math = self
            .editor
            .selected_math()
            .unwrap_or_else(|| self.editor.math());
        if math.tag == "math" {
            math
        } else {
            MathNode::with_children("math", vec![math])
        }
    }
    fn copy(&self, ctx: &egui::Context) {
        ctx.copy_text(self.selected_equation().to_latex());
    }
    fn save_button(&mut self, ui: &mut egui::Ui) {
        let label = if self.session.is_some() {
            "Save & Return to Document (Ctrl+S)"
        } else {
            "Save MathML (Ctrl+S)"
        };
        let enabled = self.save_available() && !self.draft_dirty && self.command_input.is_none();
        if Self::primary_button(ui, enabled, label).clicked() {
            self.save_clicked = true;
        }
    }

    fn footer(&mut self, ui: &mut egui::Ui) {
        ui.add_enabled_ui(
            !self.busy && self.pending.is_none() && self.matrix_dialog.is_none(),
            |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = theme::SPACE_MD;
                    ui.add_enabled_ui(!self.draft_dirty, |ui| {
                        let font = egui::TextStyle::Body.resolve(ui.style());
                        let color = ui.visuals().widgets.inactive.fg_stroke.color;
                        let caption =
                            |text: &str| egui::RichText::new(text).font(font.clone()).color(color);
                        let mt_resp = ui.radio_value(
                            &mut self.target,
                            "mathtype".into(),
                            caption("MathType"),
                        );
                        ui.add_space(theme::SPACE_MD);
                        let omml_resp =
                            ui.radio_value(&mut self.target, "omml".into(), caption("OMML"));
                        if mt_resp.changed() || omml_resp.changed() {
                            self.persisted.target = self.target.clone();
                            self.settings_dirty = true;
                        }
                        if mt_resp.clicked() || omml_resp.clicked() {
                            self.pending_focus_restore = Some(self.active_editor_focus);
                        }
                        ui.separator();
                        ui.label(caption("Font size"));
                        let drag_resp = ui
                            .scope(|ui| {
                                ui.style_mut().explanation_tooltips = false;
                                ui.add(
                                    egui::DragValue::from_get_set(|value| {
                                        if let Some(value) = value {
                                            self.font_pt = ((value * 2.0).round() / 2.0) as f32;
                                        }
                                        f64::from(self.font_pt)
                                    })
                                    .range(settings::FONT_PT_RANGE)
                                    .min_size(Vec2::new(
                                        FONT_SIZE_CONTROL_WIDTH,
                                        FOOTER_CONTROL_HEIGHT,
                                    ))
                                    // A quarter of the control's width changes the size by 1 pt.
                                    .speed(4.0 / FONT_SIZE_CONTROL_WIDTH)
                                    .max_decimals(1)
                                    .suffix(" pt"),
                                )
                                .on_hover_text("Click and drag left/right to adjust")
                            })
                            .inner;
                        if drag_resp.changed() {
                            self.render_key += 1;
                            self.persisted.font_pt = self.font_pt;
                            self.settings_dirty = true;
                        }
                        if drag_resp.drag_stopped() && !drag_resp.has_focus() {
                            self.pending_focus_restore = Some(self.active_editor_focus);
                        }
                        let history_tint =
                            ThemeTokens::from_ui(ui).icon_tint(false, ui.is_enabled());
                        let undo_img =
                            self.ui_icon(ui.ctx(), "arrow-back-up", UI_ICON_SIZE, history_tint);
                        let undo_resp = ui.add(
                            egui::Button::new(undo_img)
                                .min_size(Vec2::new(32.0, FOOTER_CONTROL_HEIGHT)),
                        );
                        undo_resp.widget_info(|| {
                            egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), "Undo")
                        });
                        if undo_resp.on_hover_text("Undo (Ctrl+Z)").clicked() {
                            self.command_input = None;
                            self.latex_groups.clear();
                            self.expects_latex_group = false;
                            self.editor.undo();
                            self.changed();
                            self.pending_focus_restore = Some(self.active_editor_focus);
                        }
                        let redo_img =
                            self.ui_icon(ui.ctx(), "arrow-forward-up", UI_ICON_SIZE, history_tint);
                        let redo_resp = ui.add(
                            egui::Button::new(redo_img)
                                .min_size(Vec2::new(32.0, FOOTER_CONTROL_HEIGHT)),
                        );
                        redo_resp.widget_info(|| {
                            egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), "Redo")
                        });
                        if redo_resp.on_hover_text("Redo (Ctrl+Y)").clicked() {
                            self.command_input = None;
                            self.latex_groups.clear();
                            self.expects_latex_group = false;
                            self.editor.redo();
                            self.changed();
                            self.pending_focus_restore = Some(self.active_editor_focus);
                        }
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        self.save_button(ui);
                    });
                });
            },
        );
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        const SIDEBAR_GAP: f32 = theme::SPACE_SM;
        const SIDEBAR_BUTTON_HEIGHT: f32 = 42.0;
        let tokens = ThemeTokens::from_ui(ui);
        let enabled = ui.is_enabled();
        egui::Panel::bottom("sidebar-batch")
            .exact_size(120.0)
            .frame(egui::Frame::NONE.inner_margin(egui::Margin {
                left: 0,
                right: 0,
                top: theme::SPACE_SM as i8,
                bottom: 0,
            }))
            .show_separator_line(false)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = theme::SPACE_SM;
                for (tab, icon, label) in [
                    (1, "file-type-docx", "Batch DOCX"),
                    (2, "settings", "Settings"),
                    (3, "info-circle", "About"),
                ] {
                    let selected = self.tab == tab;
                    let icon_tint = tokens.icon_tint(selected, enabled);
                    let icon_img = self.ui_icon(ui.ctx(), icon, UI_ICON_SIZE, icon_tint);
                    if Self::with_selectable_style(ui, selected, |ui| {
                        ui.add_sized(
                            [ui.available_width(), theme::SIDEBAR_TAB_HEIGHT],
                            egui::Button::image_and_text(icon_img, label),
                        )
                    })
                    .clicked()
                    {
                        self.tab = tab;
                    }
                }
            });
        let button_width = ((ui.available_width() - SIDEBAR_GAP) / 2.0)
            .floor()
            .max(48.0);
        let button_size = Vec2::new(button_width, SIDEBAR_BUTTON_HEIGHT);
        let mut clicked_group = None;
        egui::Grid::new("sidebar-categories")
            .num_columns(2)
            .spacing(Vec2::splat(SIDEBAR_GAP))
            .show(ui, |ui| {
                ui.spacing_mut().button_padding = Vec2::splat(theme::SPACE_XS);
                for (index, group) in palette::UI_GROUPS.iter().enumerate() {
                    let selected = self.category == group.id;
                    let icon_color = tokens.icon_tint(selected, enabled);
                    let button = match group.icons {
                        [first, second] => {
                            let left = self.ui_icon(
                                ui.ctx(),
                                first,
                                SIDEBAR_CATEGORY_ICON_SIZE,
                                icon_color,
                            );
                            let right = self.ui_icon(
                                ui.ctx(),
                                second,
                                SIDEBAR_CATEGORY_ICON_SIZE,
                                icon_color,
                            );
                            egui::Button::new((left, right)).gap(0.0)
                        }
                        [single] => {
                            let image = self.ui_icon(
                                ui.ctx(),
                                single,
                                SIDEBAR_CATEGORY_ICON_SIZE,
                                icon_color,
                            );
                            egui::Button::new(image)
                        }
                        _ => unreachable!("sidebar category must have 1 or 2 icons"),
                    };
                    let response = Self::with_selectable_style(ui, selected, |ui| {
                        ui.add_sized(button_size, button)
                    });
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::Role::Button, enabled, group.label)
                    });
                    if response.on_hover_text(group.label).clicked() {
                        clicked_group = Some(group.id);
                    }
                    if (index + 1) % 2 == 0 {
                        ui.end_row();
                    }
                }
            });
        if let Some(group_id) = clicked_group {
            self.category = group_id.into();
            self.persisted.category = self.category.clone();
            self.settings_dirty = true;
            self.pending_focus_restore = Some(self.active_editor_focus);
        }
    }

    fn insert_custom_matrix(&mut self, settings: &MatrixDialog) {
        let matrix = eqntide::catalog::matrix(
            settings.rows,
            settings.columns,
            settings.alignment,
            settings.fence,
        );
        self.command_input = None;
        self.latex_groups.clear();
        self.expects_latex_group = false;
        self.editor.insert_template(&matrix);
        self.changed();
        self.status = format!("Inserted {} × {} matrix.", settings.rows, settings.columns);
    }

    fn matrix_dialog_ui(&mut self, ctx: &egui::Context) {
        let Some(mut settings) = self.matrix_dialog.take() else {
            return;
        };
        if self.pending.is_some() || self.tab != 0 {
            return;
        }
        let mut insert = false;
        let response = egui::Modal::new(egui::Id::unique("custom-matrix-dialog")).show(ctx, |ui| {
            ui.set_min_width(330.0);
            ui.heading("Custom matrix");
            ui.add_space(6.0);
            egui::Grid::new("matrix-settings")
                .spacing([16.0, 10.0])
                .show(ui, |ui| {
                    ui.label("Rows");
                    ui.add(egui::DragValue::new(&mut settings.rows).range(1..=20));
                    ui.end_row();
                    ui.label("Columns");
                    ui.add(egui::DragValue::new(&mut settings.columns).range(1..=20));
                    ui.end_row();
                    ui.label("Alignment");
                    egui::ComboBox::from_id_salt("matrix-alignment")
                        .selected_text(match settings.alignment {
                            "left" => "Left",
                            "right" => "Right",
                            _ => "Center",
                        })
                        .show_ui(ui, |ui| {
                            for (value, label) in
                                [("left", "Left"), ("center", "Center"), ("right", "Right")]
                            {
                                ui.selectable_value(&mut settings.alignment, value, label);
                            }
                        });
                    ui.end_row();
                    ui.label("Delimiters");
                    const FENCES: &[(&str, &str)] = &[
                        ("none", "None"),
                        ("parentheses", "Parentheses ( )"),
                        ("brackets", "Brackets [ ]"),
                        ("braces", "Braces { }"),
                        ("bars", "Bars | |"),
                        ("double-bars", "Double bars ‖ ‖"),
                        ("left-brace", "Left brace {"),
                        ("left-bracket", "Left bracket ["),
                    ];
                    egui::ComboBox::from_id_salt("matrix-fence")
                        .selected_text(
                            FENCES
                                .iter()
                                .find(|(value, _)| *value == settings.fence)
                                .map_or("Parentheses ( )", |&(_, label)| label),
                        )
                        .show_ui(ui, |ui| {
                            for &(value, label) in FENCES {
                                ui.selectable_value(&mut settings.fence, value, label);
                            }
                        });
                    ui.end_row();
                });
            ui.add_space(theme::SPACE_MD);
            ui.horizontal(|ui| {
                insert = Self::primary_button(ui, true, "Insert matrix").clicked();
                if Self::standard_button(ui, "Cancel").clicked() {
                    ui.close();
                }
            });
        });
        if insert {
            self.insert_custom_matrix(&settings);
            self.pending_focus_restore = Some(self.active_editor_focus);
        } else if !response.should_close() {
            self.matrix_dialog = Some(settings);
        } else {
            self.pending_focus_restore = Some(self.active_editor_focus);
        }
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if self.busy {
                if self.gpu_profile == GpuProfile::Software {
                    ui.weak("Working…");
                } else {
                    ui.spinner();
                }
            }
            let ready = if self.bridge_ready { "●" } else { "○" };
            ui.label(ready).on_hover_text(&self.bridge_status);
            if !self.bridge_ready && Self::compact_button(ui, "Retry").clicked() {
                runtime::start_bridge(self.tx.clone());
                self.pending_focus_restore = Some(self.active_editor_focus);
            }
            if !self.queue.is_empty()
                && Self::compact_button(ui, &format!("Next session ({})", self.queue.len()))
                    .clicked()
            {
                self.pending = Some(Intent::Next);
                self.pending_focus_restore = Some(self.active_editor_focus);
            }
            let target_warning = if self.tab == 0 {
                let cache_hit = self.warning_cache.as_ref().is_some_and(
                    |(rk, lg, t, _)| *rk == self.render_key && *lg == self.latex_generation && *t == self.target,
                );
                if !cache_hit {
                    let math = self.editor.math();
                    let computed = if self.target == "mathtype" {
                        eqntide::check_mathtype_unsupported(&math)
                            .or_else(|| eqntide::check_mathtype_unsupported_latex(&self.latex))
                            .map(|u| format!("Warning: {u} is not supported in MathType."))
                            .or_else(|| {
                                eqntide::check_omml_unsupported(&math)
                                    .or_else(|| eqntide::check_omml_unsupported_latex(&self.latex))
                                    .map(|u| {
                                        format!(
                                            "Warning: {u} is not supported in OMML (MathType target active)."
                                        )
                                    })
                            })
                    } else {
                        eqntide::check_omml_unsupported(&math)
                            .or_else(|| eqntide::check_omml_unsupported_latex(&self.latex))
                            .map(|u| {
                                if eqntide::check_mathtype_unsupported(&math)
                                    .or_else(|| eqntide::check_mathtype_unsupported_latex(&self.latex))
                                    .is_some()
                                {
                                    format!("Warning: {u} is not supported in OMML or MathType.")
                                } else {
                                    format!(
                                        "Warning: {u} is not supported in OMML. Switch target to MathType to insert."
                                    )
                                }
                            })
                    };
                    self.warning_cache = Some((
                        self.render_key,
                        self.latex_generation,
                        self.target.clone(),
                        computed,
                    ));
                }
                self.warning_cache
                    .as_ref()
                    .and_then(|(_, _, _, w)| w.clone())
            } else {
                None
            };
            let active_latex_error = if self.tab == 0 {
                self.command_error
                    .as_deref()
                    .filter(|_| self.command_input.is_some())
                    .or(self.latex_error.as_deref())
            } else {
                None
            };
            if let Some(latex_error) = active_latex_error {
                let error = format!("LaTeX error: {latex_error}");
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(&error).color(ui.visuals().error_fg_color),
                    )
                    .truncate(),
                )
                .on_hover_text(error);
            } else if self.tab == 0 && self.draft_dirty {
                ui.label("Updating equation…");
            } else if let Some(warning) = target_warning {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(&warning).color(ui.visuals().error_fg_color),
                    )
                    .truncate(),
                )
                .on_hover_text(warning);
            } else {
                ui.add(egui::Label::new(&self.status).truncate())
                    .on_hover_text(&self.status);
            }
        });
    }

    fn record_recent_palette(&mut self, source: &'static str) {
        self.recent_palette.retain(|&existing| existing != source);
        self.recent_palette.insert(0, source);
        self.recent_palette.truncate(MAX_RECENT_PALETTE);
        self.palette_atlases.remove(palette::RECENT_GROUP);
        self.persisted.recent_palette = self.recent_palette.iter().map(|&s| s.to_owned()).collect();
        self.settings_dirty = true;
    }

    fn palette_ui(&mut self, ui: &mut egui::Ui) {
        let height = ui.available_height();
        let items = palette::items_for_ui_group(&self.category, &self.recent_palette);
        if self.category == palette::RECENT_GROUP && items.is_empty() {
            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), height),
                egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                |ui| {
                    ui.weak("Click any symbol or template in the palette to pin it here for quick reuse.");
                },
            );
            return;
        }
        let has_matrix_tools = palette::group_contains(&self.category, "Matrices");
        let fitting_columns = ((ui.available_width() + PALETTE_GAP)
            / (PALETTE_BUTTON_SIZE.x + PALETTE_GAP))
            .floor() as usize;
        // Native DPI/startup adjustments may briefly give us less than the minimum width.
        // Keep three rows in that frame too, scrolling horizontally if necessary.
        let item_count = items.len() + usize::from(has_matrix_tools);
        let columns = fitting_columns.max(item_count.div_ceil(3)).max(1);
        ui.scope(|ui| {
            ui.style_mut().always_scroll_the_only_direction = true;
            egui::ScrollArea::horizontal()
                .id_salt(("palette", &self.category))
                .vertical_scroll_offset(0.0)
                .max_height(height)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    egui::Grid::new(("palette-grid", &self.category))
                        .num_columns(columns)
                        .spacing(Vec2::splat(PALETTE_GAP))
                        .show(ui, |ui| {
                            ui.spacing_mut().button_padding = Vec2::splat(3.0);
                            ui.spacing_mut().item_spacing = Vec2::splat(PALETTE_GAP);
                            let ppp = ui.ctx().pixels_per_point();
                            self.sync_atlas_dpi(ppp);
                            if !self.palette_atlases.contains_key(&self.category) {
                                if let Some(atlas) =
                                    Self::build_palette_atlas(ui.ctx(), &self.category, &items, ppp)
                                {
                                    self.palette_atlases.insert(self.category.clone(), atlas);
                                }
                            }
                            let icon_color = Self::icon_tint(ui);
                            let mut clicked_source = None;
                            {
                                let atlas = self.palette_atlases.get(&self.category);
                                for (index, (_, label, source)) in items.iter().enumerate() {
                                    let response = if let Some((atlas, entry)) =
                                        atlas.and_then(|a| a.entries.get(source).map(|e| (a, e)))
                                    {
                                        let image = egui::Image::new((
                                            atlas.texture.id(),
                                            entry.display_size,
                                        ))
                                        .uv(entry.uv)
                                        .tint(icon_color);
                                        ui.add_sized(PALETTE_BUTTON_SIZE, egui::Button::new(image))
                                    } else {
                                        ui.add_sized(PALETTE_BUTTON_SIZE, egui::Button::new(*label))
                                    };
                                    response.widget_info(|| {
                                        egui::WidgetInfo::labeled(
                                            egui::Role::Button,
                                            ui.is_enabled(),
                                            *label,
                                        )
                                    });
                                    if response
                                        .on_hover_ui(|ui| {
                                            ui.label(*label);
                                            ui.weak(*source);
                                        })
                                        .clicked()
                                    {
                                        clicked_source = Some(*source);
                                    }
                                    if (index + 1) % columns == 0 {
                                        ui.end_row();
                                    }
                                }
                            }
                            if let Some(source) = clicked_source {
                                self.command_input = None;
                                self.latex_groups.clear();
                                self.expects_latex_group = false;
                                self.record_recent_palette(source);
                                self.insert_template(source);
                                self.pending_focus_restore = Some(self.active_editor_focus);
                            }
                            if has_matrix_tools
                                && ui
                                    .add_sized(PALETTE_BUTTON_SIZE, egui::Button::new("⋯"))
                                    .on_hover_text("Custom matrix")
                                    .clicked()
                            {
                                self.command_input = None;
                                self.matrix_dialog = Some(MatrixDialog::default());
                                ui.memory_mut(|m| {
                                    m.surrender_focus(Self::editor_focus_id(
                                        self.active_editor_focus,
                                    ))
                                });
                            }
                        });
                });
        });
    }

    fn icon_tint(ui: &egui::Ui) -> Color32 {
        ThemeTokens::from_ui(ui).icon_tint(false, ui.is_enabled())
    }

    fn ui_icon(
        &mut self,
        ctx: &egui::Context,
        name: &'static str,
        size: f32,
        tint: Color32,
    ) -> egui::Image<'static> {
        let texture_id = self
            .ui_icons
            .entry(name)
            .or_insert_with(|| {
                let pixels = image::load_from_memory(tabler_icon_bytes(name))
                    .expect("bundled tabler icon is valid")
                    .to_rgba8();
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [pixels.width() as usize, pixels.height() as usize],
                    pixels.as_raw(),
                );
                ctx.load_texture(
                    format!("tabler-icon-{name}"),
                    image,
                    egui::TextureOptions::LINEAR,
                )
            })
            .id();
        egui::Image::new((texture_id, Vec2::splat(size))).tint(tint)
    }

    fn sync_atlas_dpi(&mut self, ppp: f32) {
        if (self.palette_ppp - ppp).abs() > f32::EPSILON {
            self.palette_atlases.clear();
            self.palette_ppp = ppp;
        }
    }

    fn build_palette_atlas(
        ctx: &egui::Context,
        category: &str,
        items: &[&palette::PaletteItem],
        ppp: f32,
    ) -> Option<PaletteAtlas> {
        let mut rendered = Vec::new();
        for (_, _, source) in items {
            let Ok(math) = palette::preview_template(source) else {
                continue;
            };
            let Ok(mask) = eqntide::render_alpha_at(
                &math,
                PALETTE_ICON_TARGET_EM * ppp,
                40.0 * ppp,
                26.0 * ppp,
            ) else {
                continue;
            };
            if mask.width > 0 && mask.height > 0 {
                rendered.push((*source, mask));
            }
        }
        Self::build_mask_atlas(ctx, &format!("palette-atlas-{category}"), rendered, ppp)
    }

    fn build_mask_atlas(
        ctx: &egui::Context,
        texture_name: &str,
        rendered: Vec<(&'static str, eqntide::AlphaMask)>,
        ppp: f32,
    ) -> Option<PaletteAtlas> {
        if rendered.is_empty() {
            return None;
        }
        const PAD: usize = 1;
        const MAX_ATLAS_W: usize = 512;
        let mut placements = Vec::with_capacity(rendered.len());
        let mut cur_x = PAD;
        let mut cur_y = PAD;
        let mut row_h = 0usize;
        let mut atlas_w = 0usize;
        for (idx, (_, mask)) in rendered.iter().enumerate() {
            let w = mask.width as usize;
            let h = mask.height as usize;
            if cur_x > PAD && cur_x + w + PAD > MAX_ATLAS_W {
                cur_x = PAD;
                cur_y += row_h + PAD;
                row_h = 0;
            }
            placements.push((idx, cur_x, cur_y, w, h));
            cur_x += w + PAD;
            atlas_w = atlas_w.max(cur_x);
            row_h = row_h.max(h);
        }
        let atlas_h = (cur_y + row_h + PAD).max(1);
        let atlas_w = atlas_w.max(1);
        let mut rgba = vec![0u8; atlas_w * atlas_h * 4];
        let mut entries = HashMap::with_capacity(placements.len());
        let mut gamma_lut = [0u8; 256];
        for (i, slot) in gamma_lut.iter_mut().enumerate().skip(1) {
            let a = i as f32 / 255.0;
            *slot = (a.powf(0.70) * 255.0).round().min(255.0) as u8;
        }
        for (idx, px, py, w, h) in placements {
            let (key, mask) = &rendered[idx];
            for y in 0..h {
                let src_row = y * w;
                let dst_row = (py + y) * atlas_w + px;
                for x in 0..w {
                    let alpha = mask.alpha[src_row + x];
                    if alpha == 0 {
                        continue;
                    }
                    let dst = (dst_row + x) * 4;
                    rgba[dst] = 255;
                    rgba[dst + 1] = 255;
                    rgba[dst + 2] = 255;
                    rgba[dst + 3] = gamma_lut[alpha as usize];
                }
            }
            let uv = Rect::from_min_max(
                Pos2::new(px as f32 / atlas_w as f32, py as f32 / atlas_h as f32),
                Pos2::new(
                    (px + w) as f32 / atlas_w as f32,
                    (py + h) as f32 / atlas_h as f32,
                ),
            );
            entries.insert(
                *key,
                PaletteAtlasEntry {
                    uv,
                    display_size: Vec2::new(w as f32 / ppp, h as f32 / ppp),
                },
            );
        }
        let image = egui::ColorImage::from_rgba_unmultiplied([atlas_w, atlas_h], &rgba);
        let texture = ctx.load_texture(texture_name, image, egui::TextureOptions::NEAREST);
        Some(PaletteAtlas { texture, entries })
    }

    #[cfg(test)]
    fn palette_texture_ids(&self) -> std::collections::HashSet<egui::TextureId> {
        self.palette_atlases
            .values()
            .map(|a| a.texture.id())
            .collect()
    }

    #[cfg(test)]
    fn ui_icon_texture_id(&self, name: &str) -> Option<egui::TextureId> {
        self.ui_icons.get(name).map(|t| t.id())
    }

    fn enforce_window_minimum_if_needed(&mut self, ctx: &egui::Context) {
        let state = ctx.input(|input| {
            let viewport = input.viewport();
            let size = viewport.inner_rect.unwrap_or(input.viewport_rect()).size();
            let fullscreen_or_max =
                viewport.maximized == Some(true) || viewport.fullscreen == Some(true);
            (size, fullscreen_or_max)
        });
        if self
            .last_enforced_viewport
            .is_some_and(|(prev_size, prev_flag)| {
                prev_flag == state.1
                    && (prev_size.x - state.0.x).abs() < 0.5
                    && (prev_size.y - state.0.y).abs() < 0.5
            })
        {
            return;
        }
        self.last_enforced_viewport = Some(state);
        Self::enforce_window_minimum(ctx);
    }

    fn enforce_window_minimum(ctx: &egui::Context) {
        let request = ctx.input(|input| {
            let viewport = input.viewport();
            if viewport.maximized == Some(true) || viewport.fullscreen == Some(true) {
                return None;
            }
            // Wayland does not provide native inner_rect; the UI viewport still has its size.
            let size = viewport.inner_rect.unwrap_or(input.viewport_rect()).size();
            // Respect a physically smaller display, avoiding resize loops with the compositor.
            let minimum =
                Vec2::from(MIN_WINDOW_SIZE).min(viewport.monitor_size.unwrap_or(Vec2::INFINITY));
            let required = size.max(minimum);
            (required.x > size.x + 0.5 || required.y > size.y + 0.5).then_some((minimum, required))
        });
        if let Some((minimum, required)) = request {
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(minimum));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(required));
        }
    }

    fn shell(&mut self, ui: &mut egui::Ui) {
        self.save_clicked = false;
        self.keyboard_handled = false;
        if self.tab == 0
            && !self.draft_dirty
            && !self.busy
            && self.pending.is_none()
            && self.matrix_dialog.is_none()
            && ui.is_enabled()
            && ui.memory(|m| m.has_focus(egui::Id::unique("equation-canvas")))
            && !ui.input(|i| {
                i.pointer.any_pressed() || i.pointer.any_released() || i.pointer.any_down()
            })
        {
            self.keyboard(ui);
            self.keyboard_handled = true;
        }
        self.sync_render(ui.ctx());
        let tokens = ThemeTokens::from_ui(ui);
        egui::Panel::bottom("app-status")
            .exact_size(32.0)
            .frame(
                egui::Frame::NONE
                    .fill(tokens.panel_bg)
                    .stroke(Stroke::new(1.0, tokens.border))
                    .inner_margin(egui::Margin::symmetric(
                        theme::SPACE_LG as i8,
                        theme::SPACE_XS as i8,
                    )),
            )
            .show_separator_line(false)
            .show(ui, |ui| self.status_bar(ui));
        let sidebar_rect = if self.tab == 0 {
            Some(
                egui::Panel::left("app-sidebar")
                    .exact_size(174.0)
                    .resizable(false)
                    .frame(egui::Frame::NONE.fill(tokens.panel_bg).inner_margin(
                        egui::Margin::symmetric(theme::SPACE_LG as i8, CONTENT_PADDING),
                    ))
                    .show_separator_line(false)
                    .show(ui, |ui| {
                        ui.add_enabled_ui(
                            !self.busy && self.pending.is_none() && self.matrix_dialog.is_none(),
                            |ui| self.sidebar(ui),
                        );
                    })
                    .response
                    .rect,
            )
        } else {
            None
        };
        let footer_rect = if self.tab == 0 || self.session.is_some() {
            Some(
                egui::Panel::bottom("app-footer")
                    .exact_size(56.0)
                    .frame(egui::Frame::NONE.fill(tokens.panel_bg).inner_margin(
                        egui::Margin::symmetric(theme::SPACE_LG as i8, CONTENT_PADDING),
                    ))
                    .show_separator_line(false)
                    .show(ui, |ui| self.footer(ui))
                    .response
                    .rect,
            )
        } else {
            None
        };
        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(tokens.app_bg)
                    .inner_margin(egui::Margin::symmetric(
                        theme::SPACE_LG as i8,
                        CONTENT_PADDING,
                    )),
            )
            .show(ui, |ui| {
                ui.add_enabled_ui(
                    !self.busy && self.pending.is_none() && self.matrix_dialog.is_none(),
                    |ui| match self.tab {
                        0 => self.editor_ui(ui),
                        1 => self.batch_ui(ui),
                        2 => self.settings_ui(ui),
                        3 => self.about_ui(ui),
                        _ => self.editor_ui(ui),
                    },
                );
            });
        let divider_stroke = Stroke::new(1.0, tokens.border);
        if let Some(rect) = sidebar_rect {
            ui.painter()
                .vline(rect.max.x - 0.5, rect.y_range(), divider_stroke);
        }
        if let Some(rect) = footer_rect {
            ui.painter()
                .hline(rect.x_range(), rect.min.y + 0.5, divider_stroke);
        }
    }

    fn save_available(&self) -> bool {
        !self.busy
            && !self.batch_running
            && self.pending.is_none()
            && !self.allow_close
            && self.matrix_dialog.is_none()
    }
    fn take_save_shortcut(ctx: &egui::Context) -> bool {
        // Consume before widgets see the events, but save after they apply this
        // frame's edits. Ctrl+Shift+S remains the canvas's sum shortcut.
        ctx.input_mut(|input| {
            let mut save = false;
            input.events.retain(|event| {
                let matches = matches!(event, egui::Event::Key {
                    key: egui::Key::S,
                    pressed: true,
                    modifiers,
                    ..
                } if modifiers.command && !modifiers.shift && !modifiers.alt);
                save |= matches;
                !matches
            });
            save
        })
    }
    fn editor_ui(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("templates-panel")
            .exact_size(PALETTE_HEIGHT)
            .frame(egui::Frame::NONE.inner_margin(egui::Margin {
                bottom: CONTENT_PADDING,
                ..Default::default()
            }))
            .show_separator_line(false)
            .show(ui, |ui| {
                ui.add_enabled_ui(!self.draft_dirty, |ui| self.palette_ui(ui));
            });
        egui::Panel::bottom("latex-panel")
            .exact_size(LATEX_PANEL_HEIGHT)
            .show_separator_line(false)
            .frame(egui::Frame::NONE.inner_margin(egui::Margin {
                top: CONTENT_PADDING,
                ..Default::default()
            }))
            .show(ui, |ui| self.latex_ui(ui));
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                ui.set_clip_rect(ui.clip_rect().intersect(ui.max_rect()));
                ui.add_enabled_ui(!self.draft_dirty, |ui| {
                    if palette::group_contains(&self.category, "Matrices") {
                        ui.horizontal(|ui| {
                            for (label, column, grow) in [
                                ("+ row", false, true),
                                ("− row", false, false),
                                ("+ col", true, true),
                                ("− col", true, false),
                            ] {
                                if Self::compact_button(ui, label).clicked() {
                                    let changed = if grow {
                                        self.editor.grow_matrix(column)
                                    } else {
                                        self.editor.shrink_matrix(column)
                                    };
                                    if changed {
                                        self.changed();
                                    } else {
                                        self.status =
                                            "Place the caret inside a matrix first.".into();
                                    }
                                    self.pending_focus_restore = Some(self.active_editor_focus);
                                }
                            }
                        });
                    }
                    let scroll_out = self.canvas(ui);
                    if !self.has_active_overlay() && !self.draft_dirty {
                        let r = Rect::from_min_max(
                            scroll_out.inner_rect.min + Vec2::splat(2.0),
                            scroll_out.inner_rect.max - Vec2::splat(16.0),
                        );
                        if r.is_positive() {
                            self.passive_canvas_rect = Some(r);
                        }
                    }
                });
            });
        self.sync_editor_focus(ui);
    }

    fn configure_canvas_scroll_style(&self, ui: &mut egui::Ui, pal: CanvasPalette) {
        ui.visuals_mut().widgets.inactive.bg_fill = pal.scroll_inactive;
        ui.visuals_mut().widgets.inactive.fg_stroke.color = pal.scroll_inactive;
        ui.visuals_mut().widgets.hovered.bg_fill = pal.scroll_hover;
        ui.visuals_mut().widgets.hovered.fg_stroke.color = pal.scroll_hover;
        ui.visuals_mut().widgets.active.bg_fill = pal.scroll_active;
        ui.visuals_mut().widgets.active.fg_stroke.color = pal.scroll_active;
        ui.visuals_mut().extreme_bg_color = pal.scroll_track;

        let scroll = &mut ui.spacing_mut().scroll;
        if matches!(
            self.gpu_profile,
            GpuProfile::VirtualGpu | GpuProfile::Software
        ) {
            scroll.dormant_handle_opacity = 0.85;
            scroll.active_handle_opacity = 0.85;
            scroll.interact_handle_opacity = 0.85;
            scroll.dormant_background_opacity = 0.6;
            scroll.active_background_opacity = 0.6;
            scroll.interact_background_opacity = 0.6;
            scroll.floating_allocated_width = scroll.bar_width;
        } else {
            scroll.dormant_handle_opacity = 0.5;
            scroll.active_handle_opacity = 0.85;
            scroll.interact_handle_opacity = 1.0;
            scroll.dormant_background_opacity = 0.3;
            scroll.active_background_opacity = 0.6;
            scroll.interact_background_opacity = 0.85;
        }
    }

    fn latex_ui(&mut self, ui: &mut egui::Ui) {
        let pal = self.active_canvas_palette(ui.visuals().dark_mode);
        let latex_id = Self::editor_focus_id(StartupFocus::Latex);
        let box_height = ui.available_height().clamp(24.0, LATEX_INPUT_HEIGHT);
        let frame_resp = egui::Frame::new()
            .fill(pal.bg)
            .corner_radius(theme::CONTROL_CORNER_RADIUS)
            .show(ui, |ui| {
                self.configure_canvas_scroll_style(ui, pal);
                ui.spacing_mut().scroll.floating_allocated_width = 0.0;
                ui.spacing_mut().scroll.bar_outer_margin = 4.0;
                ui.visuals_mut().text_cursor.stroke = Stroke::new(1.5, pal.caret);
                ui.visuals_mut().selection.bg_fill = pal.selection_bg;
                ui.visuals_mut().selection.stroke = Stroke::new(1.0, pal.caret);
                egui::ScrollArea::vertical()
                    .id_salt("latex-scroll")
                    .max_height(box_height)
                    .min_scrolled_height(box_height)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let width = ui.available_width();
                        ui.add(
                            egui::TextEdit::multiline(&mut self.latex)
                                .id(latex_id)
                                .frame(egui::Frame::NONE.inner_margin(egui::Margin {
                                    left: 8,
                                    right: 12,
                                    top: 6,
                                    bottom: 6,
                                }))
                                .desired_rows(1)
                                .desired_width(f32::INFINITY)
                                .min_size(Vec2::new(width, box_height))
                                .font(egui::TextStyle::Monospace)
                                .text_color(pal.ink),
                        )
                    })
                    .inner
            });
        ui.painter().rect_stroke(
            frame_resp.response.rect,
            theme::CONTROL_CORNER_RADIUS,
            Stroke::new(1.0, pal.border),
            egui::StrokeKind::Inside,
        );
        if frame_resp.inner.changed() {
            self.source_changed(Instant::now());
        }
    }

    fn canvas(&mut self, ui: &mut egui::Ui) -> egui::scroll_area::ScrollAreaOutput<()> {
        self.sync_render(ui.ctx());
        // A pending/invalid source disables editing, but keeps the last preview legible.
        ui.set_opacity(1.0);
        let scale = self.zoom * 96.0 / 72.0;
        let height = (ui.available_height() - 26.0).max(12.0);
        let pal = self.active_canvas_palette(ui.visuals().dark_mode);
        egui::Frame::new()
            .fill(pal.bg)
            .corner_radius(theme::SURFACE_CORNER_RADIUS)
            .stroke(Stroke::new(1.0, pal.border))
            .inner_margin(theme::SPACE_LG)
            .show(ui, |ui| {
                self.configure_canvas_scroll_style(ui, pal);

                egui::ScrollArea::both()
                    .id_salt("formula-scroll")
                    .max_height(height)
                    .min_scrolled_height(height)
                    .auto_shrink([false, false])
                    .show_viewport(ui, |ui, viewport| {
                        let size = self
                            .rendered
                            .as_ref()
                            .map(|r| {
                                Vec2::new(
                                    r.preview.preview.width_pt * scale,
                                    r.preview.preview.height_pt * scale,
                                )
                            })
                            .unwrap_or(Vec2::new(60.0, 40.0));
                        // Only the formula contributes to scrollable content. Filling the
                        // viewport via allocate_space can round past its width at fractional DPI.
                        let content_size = size + Vec2::new(12.0, 20.0);
                        let (_, content_rect) = ui.allocate_space(content_size);
                        // Empty canvas space remains clickable without creating overflow.
                        let rect = Rect::from_min_size(
                            content_rect.min,
                            viewport.size().max(content_size),
                        );
                        let response = ui.interact(
                            rect,
                            egui::Id::unique("equation-canvas"),
                            egui::Sense::click_and_drag(),
                        );
                        response.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::Role::TextInput,
                                true,
                                "Visual equation editor",
                            )
                        });
                        let origin = rect.min + Vec2::new(4.0, 8.0);
                        if let Some(texture) = &self.texture {
                            ui.painter().image(
                                texture.id(),
                                Rect::from_min_size(origin, size),
                                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                pal.ink,
                            );
                        }
                        if let Some(rendered) = &self.rendered {
                            for slot in &rendered.placeholders {
                                let rect = Rect::from_min_size(
                                    origin + Vec2::new(slot.x, slot.y) * scale,
                                    Vec2::new(slot.width, slot.height) * scale,
                                );
                                Self::paint_placeholder(ui.painter(), rect, pal);
                            }
                        }
                        if response.clicked() || response.drag_started() {
                            response.request_focus();
                        }
                        // Enter/Space also set clicked() for a focused egui widget.
                        // Only a real pointer click may cancel command entry or reposition the caret.
                        let pointer_clicked = response.clicked_by(egui::PointerButton::Primary);
                        if pointer_clicked || response.drag_started() {
                            self.command_input = None;
                            self.latex_groups.clear();
                            self.expects_latex_group = false;
                        }
                        if response.drag_started() {
                            if let Some(pos) = ui.input(|i| i.pointer.press_origin()) {
                                if let Some(caret) = self.hit(pos, origin, scale) {
                                    self.editor.set_caret(caret, false);
                                }
                            }
                        }
                        if (pointer_clicked || response.dragged()) && !self.render_busy {
                            if let Some(pos) = response.interact_pointer_pos() {
                                if let Some(caret) = self.hit(pos, origin, scale) {
                                    let extend =
                                        ui.input(|i| i.modifiers.shift) || response.dragged();
                                    if self.editor.node(caret.node).is_some_and(|n| n.opaque()) {
                                        self.editor.select_node(caret.node);
                                    } else {
                                        self.editor.set_caret(caret, extend);
                                    }
                                    if response.double_clicked() {
                                        self.editor.select_node(caret.node);
                                    }
                                }
                            }
                        }
                        let selected = self.editor.selected_ids();
                        for id in selected {
                            if let Some(r) = self.node_rect(id, origin, scale) {
                                ui.painter().rect_filled(
                                    r,
                                    theme::CONTROL_CORNER_RADIUS,
                                    pal.selection_bg,
                                );
                            }
                        }
                        let has_canvas_focus =
                            response.has_focus() || ui.memory(|m| m.has_focus(response.id));
                        if has_canvas_focus && ui.is_enabled() {
                            ui.memory_mut(|m| {
                                m.set_focus_lock_filter(
                                    response.id,
                                    egui::EventFilter {
                                        tab: true,
                                        horizontal_arrows: true,
                                        vertical_arrows: true,
                                        escape: true,
                                    },
                                )
                            });
                            if !self.keyboard_handled {
                                self.keyboard(ui);
                                self.sync_render(ui.ctx());
                            }
                            let cursor = self
                                .caret_rect(origin, scale)
                                .unwrap_or(Rect::from_min_size(origin, Vec2::new(1.0, 24.0)));
                            if self.canvas_caret_visible(ui) {
                                let ppp = ui.ctx().pixels_per_point();
                                let snap = |v: f32| (v * ppp).round() / ppp;
                                let x = snap(cursor.left());
                                ui.painter().line_segment(
                                    [
                                        Pos2::new(x, snap(cursor.top())),
                                        Pos2::new(x, snap(cursor.bottom())),
                                    ],
                                    Stroke::new(
                                        1.5,
                                        if self.canvas_in_text() {
                                            pal.text_caret
                                        } else {
                                            pal.caret
                                        },
                                    ),
                                );
                            }
                            ui.output_mut(|o| {
                                o.ime = Some(egui::output::IMEOutput {
                                    purpose: egui::IMEPurpose::Normal,
                                    rect,
                                    cursor_rect: cursor,
                                    should_interrupt_composition: false,
                                })
                            });
                            if !self.ime.is_empty() {
                                ui.painter().text(
                                    cursor.right_bottom(),
                                    egui::Align2::LEFT_TOP,
                                    &self.ime,
                                    egui::FontId::proportional(18.0),
                                    pal.ime_fg,
                                );
                            }
                        } else {
                            self.caret_blink = None;
                        }
                        self.keyboard_handled = false;
                        if self.command_input.is_some() && ui.is_enabled() {
                            let cursor = self
                                .caret_rect(origin, scale)
                                .unwrap_or(Rect::from_min_size(origin, Vec2::new(1.0, 24.0)));
                            self.command_popup(ui, cursor.left_bottom() + Vec2::new(0.0, 6.0));
                        }
                    })
            })
            .inner
    }
    fn paint_placeholder(painter: &egui::Painter, rect: Rect, pal: CanvasPalette) {
        // Cover only the synthetic square; the formula texture and hit regions stay unchanged.
        painter.rect_filled(rect.expand(0.5), 0.0, pal.bg);
        let rect = rect.shrink(1.0);
        let radius = theme::CONTROL_CORNER_RADIUS;
        painter.rect_filled(rect, radius, pal.placeholder_bg);
        let mut path = vec![];
        egui::epaint::tessellator::path::rounded_rectangle(
            &mut path,
            egui::epaint::RoundedRect::new(rect, radius),
        );
        if let Some(&first) = path.first() {
            path.push(first);
        }
        painter.extend(egui::Shape::dashed_line(
            &path,
            Stroke::new(1.2, pal.placeholder_stroke),
            4.0,
            3.0,
        ));
    }

    fn region(&self, id: u64) -> Option<eqntide::render::HitRegion> {
        let path = self.editor.path(id)?;
        let rendered = self.rendered.as_ref()?;
        if let Some(r) = rendered.preview.regions.iter().find(|r| r.path == path) {
            return Some(r.clone());
        }
        let mut result: Option<eqntide::render::HitRegion> = None;
        for r in rendered
            .preview
            .regions
            .iter()
            .filter(|r| r.path.starts_with(&path))
        {
            if let Some(out) = &mut result {
                let right = (out.x + out.width).max(r.x + r.width);
                let bottom = (out.y + out.height).max(r.y + r.height);
                out.x = out.x.min(r.x);
                out.y = out.y.min(r.y);
                out.width = right - out.x;
                out.height = bottom - out.y;
            } else {
                result = Some(r.clone());
            }
        }
        result
    }
    fn node_rect(&self, id: u64, origin: Pos2, scale: f32) -> Option<Rect> {
        let r = self.region(id)?;
        Some(Rect::from_min_size(
            origin + Vec2::new(r.x * scale, r.y * scale),
            Vec2::new(r.width.max(2.0) * scale, r.height.max(6.0) * scale),
        ))
    }
    fn caret_em_pt(&self, id: u64) -> f32 {
        let Some(path) = self.editor.path(id) else {
            return self.font_pt;
        };
        let mut node = self.editor.root();
        let mut em = self.font_pt;
        let mut display = true;
        for &child_idx in &path {
            let ds = node
                .attrs
                .iter()
                .find(|(k, _)| k == "displaystyle")
                .map(|(_, v)| v.as_str());
            match node.tag.as_str() {
                "mstyle" => {
                    if ds == Some("false") {
                        display = false;
                    } else if ds == Some("true") {
                        display = true;
                    }
                }
                "mfrac" => {
                    if ds == Some("false") || !display {
                        em *= 0.71;
                    }
                    display = false;
                }
                "msub" | "msup" | "msubsup" | "munder" | "mover" | "munderover"
                    if child_idx >= 1 =>
                {
                    em *= 0.71;
                    display = false;
                }
                "mroot" if child_idx == 1 => {
                    em *= 0.71;
                    display = false;
                }
                "msqrt" => {
                    display = false;
                }
                _ => {}
            }
            let Some(next) = node.children.get(child_idx) else {
                break;
            };
            node = next;
        }
        em.max(self.font_pt * 0.45)
    }

    fn caret_rect(&self, origin: Pos2, scale: f32) -> Option<Rect> {
        let caret = self.editor.caret();
        let node = self.editor.node(caret.node)?;
        let region = self.region(caret.node)?;
        let is_placeholder =
            caret.node != self.editor.root().id && node.text.is_empty() && node.children.is_empty();
        if is_placeholder {
            let inset_x = (region.width * 0.5).min(2.5 / scale);
            let inset_y = (2.0 / scale).min(region.height * 0.25);
            let top = region.y + inset_y;
            let height = (region.height - 2.0 * inset_y).max(4.0 / scale);
            return Some(Rect::from_min_size(
                origin + Vec2::new((region.x + inset_x) * scale, top * scale),
                Vec2::new(1.0, height * scale),
            ));
        }
        let x = if node.token() {
            if region.carets.len() == node.text.chars().count() + 1 {
                region.carets[caret.offset.min(region.carets.len() - 1)]
            } else {
                region.x
                    + region.width * caret.offset as f32 / node.text.chars().count().max(1) as f32
            }
        } else if let Some(previous_text) = caret
            .offset
            .checked_sub(1)
            .and_then(|index| node.children.get(index))
            .filter(|child| child.tag == "mtext")
            .and_then(|child| self.region(child.id))
        {
            previous_text.x + previous_text.width
        } else if let Some(child) = node.children.get(caret.offset) {
            self.region(child.id).map_or(region.x, |r| r.x)
        } else if let Some(child) = node.children.last() {
            self.region(child.id)
                .map_or(region.x + region.width, |r| r.x + r.width)
        } else {
            region.x
        };
        let em_pt = self.caret_em_pt(caret.node);
        let top = region.baseline - 0.72 * em_pt;
        let height = (0.72 + 0.22) * em_pt;
        Some(Rect::from_min_size(
            origin + Vec2::new(x * scale, top * scale),
            Vec2::new(1.0, (height * scale).max(6.0)),
        ))
    }
    fn hit(&self, pos: Pos2, origin: Pos2, scale: f32) -> Option<Caret> {
        let rendered = self.rendered.as_ref()?;
        let p = (pos - origin) / scale;
        let r = rendered
            .preview
            .regions
            .iter()
            .filter(|r| {
                p.x >= r.x - 1.0
                    && p.x <= r.x + r.width + 1.0
                    && p.y >= r.y - 1.0
                    && p.y <= r.y + r.height + 1.0
            })
            .max_by_key(|r| r.path.len());
        let Some(r) = r else {
            return Some(Caret {
                node: self.editor.root().id,
                offset: self.editor.root().children.len(),
            });
        };
        let mut path = r.path.clone();
        let id = loop {
            if let Some(id) = self.editor.id_at_path(&path) {
                break id;
            }
            path.pop()?;
        };
        let mut id = id;
        for depth in 0..=path.len() {
            if let Some(ancestor) = self.editor.id_at_path(&path[..depth]) {
                if self.editor.node(ancestor).is_some_and(|n| n.opaque()) {
                    id = ancestor;
                    break;
                }
            }
        }
        let node = self.editor.node(id)?;
        // Unknown elements are selected atomically, never flattened into text.
        let offset = if node.token() {
            if r.carets.len() == node.text.chars().count() + 1 {
                r.carets
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| (**a - p.x).abs().total_cmp(&(**b - p.x).abs()))
                    .map_or(0, |(i, _)| i)
            } else {
                ((p.x - r.x) / r.width.max(1.0) * node.text.chars().count() as f32)
                    .round()
                    .clamp(0.0, node.text.chars().count() as f32) as usize
            }
        } else if node.row() {
            node.children
                .iter()
                .position(|c| self.region(c.id).is_some_and(|r| p.x < r.x + r.width / 2.0))
                .unwrap_or(node.children.len())
        } else {
            0
        };
        Some(Caret { node: id, offset })
    }
    fn vertical(&mut self, down: bool, extend: bool) {
        let Some(current) = self.region(self.editor.caret().node) else {
            return;
        };
        let x = current.x + current.width / 2.0;
        if let Some(mut path) = self.editor.path(self.editor.caret().node) {
            while let Some(index) = path.pop() {
                let Some(parent_id) = self.editor.id_at_path(&path) else {
                    break;
                };
                let Some(parent) = self.editor.node(parent_id) else {
                    break;
                };
                let target_index = match (parent.tag.as_str(), index, down) {
                    ("mfrac", 0, true) => Some(1),
                    ("mfrac", 1, false) => Some(0),
                    ("mroot", 0, false) => Some(1),
                    ("mroot", 1, true) => Some(0),
                    ("msup" | "mover", 0, false) => Some(1),
                    ("msup" | "mover", 1, true) => Some(0),
                    ("msub" | "munder", 0, true) => Some(1),
                    ("msub" | "munder", 1, false) => Some(0),
                    ("msubsup" | "munderover", 0, true) => Some(1),
                    ("msubsup" | "munderover", 0, false) => Some(2),
                    ("msubsup" | "munderover", 1, false) | ("msubsup" | "munderover", 2, true) => {
                        Some(0)
                    }
                    _ => None,
                };
                if let Some(target) = target_index
                    .and_then(|i| parent.children.get(i))
                    .map(|n| n.id)
                {
                    self.enter_subtree(target, x, extend);
                    return;
                }
                if parent.tag == "mtr" {
                    let column = index;
                    if let Some(row) = path.pop() {
                        if let Some(table) = self
                            .editor
                            .id_at_path(&path)
                            .and_then(|id| self.editor.node(id))
                        {
                            if table.tag == "mtable" {
                                let other = if down {
                                    row.checked_add(1)
                                } else {
                                    row.checked_sub(1)
                                };
                                if let Some(cell) = other
                                    .and_then(|r| table.children.get(r))
                                    .and_then(|r| r.children.get(column))
                                {
                                    let id = cell.id;
                                    self.enter_subtree(id, x, extend);
                                    return;
                                }
                            }
                        }
                        path.push(row);
                    }
                }
            }
        }
        let Some(rendered) = &self.rendered else {
            return;
        };
        let candidate = rendered
            .preview
            .regions
            .iter()
            .filter(|r| {
                if down {
                    r.baseline > current.baseline + 1.0
                } else {
                    r.baseline < current.baseline - 1.0
                }
            })
            .filter_map(|r| {
                let id = self.editor.id_at_path(&r.path)?;
                let n = self.editor.node(id)?;
                if !n.token() && !(n.row() && n.children.is_empty()) {
                    return None;
                }
                let score =
                    (r.baseline - current.baseline).abs() * 3.0 + (r.x + r.width / 2.0 - x).abs();
                Some((
                    score,
                    Caret {
                        node: id,
                        offset: 0,
                    },
                ))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0));
        if let Some((_, c)) = candidate {
            self.editor.set_caret(c, extend);
        }
    }
    fn enter_subtree(&mut self, id: u64, x: f32, extend: bool) {
        let Some(path) = self.editor.path(id) else {
            return;
        };
        let Some(rendered) = &self.rendered else {
            return;
        };
        let target = rendered
            .preview
            .regions
            .iter()
            .filter(|r| r.path.starts_with(&path))
            .filter_map(|r| {
                let node_id = self.editor.id_at_path(&r.path)?;
                let node = self.editor.node(node_id)?;
                if !node.token() && !(node.row() && node.children.is_empty()) {
                    return None;
                }
                let offset = if node.token() {
                    r.carets
                        .iter()
                        .enumerate()
                        .min_by(|(_, a), (_, b)| (**a - x).abs().total_cmp(&(**b - x).abs()))
                        .map_or(0, |(i, _)| i.min(node.text.chars().count()))
                } else {
                    0
                };
                Some((
                    (r.x + r.width / 2.0 - x).abs(),
                    Caret {
                        node: node_id,
                        offset,
                    },
                ))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0));
        if let Some((_, caret)) = target {
            self.editor.set_caret(caret, extend);
        }
    }
    fn commit_command(&mut self, choose_suggestion: bool) -> bool {
        let Some(input) = &self.command_input else {
            return true;
        };
        if input.name.trim().is_empty() {
            self.command_input = None;
            return true;
        }
        let command = if !input.is_command_prefix() {
            None
        } else if choose_suggestion {
            commands::suggestions(&input.name)
                .get(input.selected)
                .copied()
        } else {
            commands::exact(&input.name)
        };
        if let Some(command) = command.filter(|c| commands::is_argument_requiring_command(c.name)) {
            let msg = commands::bare_command_error(command.name);
            if let Some(cmd_input) = self.command_input.as_mut() {
                cmd_input.set_name_end(command.name);
            }
            self.command_error = Some(msg.clone());
            self.status = msg;
            return false;
        }
        let result = if let Some(command) = command {
            palette::template(command.source).map(|math| (math, command.source.contains("#?")))
        } else {
            Self::parse_desktop_latex(&format!("\\{}", input.name)).map(|math| (math, false))
        };
        match result {
            Ok((math, template)) => {
                self.command_input = None;
                self.command_error = None;
                self.expects_latex_group = template;
                if template {
                    if command.is_some_and(|c| palette::attaches_to_previous(c.source)) {
                        self.editor.insert_template_wrapping_previous(&math);
                    } else {
                        self.editor.insert_template(&math);
                    }
                } else {
                    self.editor.insert_math(&math);
                }
                self.changed();
                self.status = "Inserted LaTeX command.".into();
                true
            }
            Err(msg) => {
                self.command_error = Some(msg.clone());
                self.status = msg;
                false
            }
        }
    }

    fn canvas_in_text(&self) -> bool {
        self.editor
            .node(self.editor.caret().node)
            .is_some_and(|node| node.tag == "mtext")
    }

    fn canvas_text(&mut self, text: &str) {
        for ch in text.chars() {
            if let Some(input) = &mut self.command_input {
                if ch.is_whitespace() && input.is_command_prefix() {
                    continue;
                }
                input.insert_char(ch);
                self.command_error = None;
                continue;
            }
            if self.canvas_in_text() {
                if !ch.is_control() {
                    self.editor.insert_text(&ch.to_string());
                }
                continue;
            }
            match ch {
                '\\' => {
                    self.expects_latex_group = false;
                    self.command_input = Some(commands::Input::opened());
                }
                '{' if self.expects_latex_group || !self.latex_groups.is_empty() => {
                    self.latex_groups.push(self.editor.caret().node);
                    self.expects_latex_group = false;
                }
                '}' if !self.latex_groups.is_empty() => self.close_latex_group(),
                '^' | '_' => {
                    self.insert_template(if ch == '^' { "^{#?}" } else { "_{#?}" });
                    self.expects_latex_group = true;
                }
                '/' => {
                    if let Ok(template) = palette::template("\\frac{#?}{#?}") {
                        self.editor.insert_template_wrapping_previous(&template);
                    }
                }
                ch if ch.is_whitespace() => {}
                _ => {
                    self.expects_latex_group = false;
                    self.editor.insert_text(&ch.to_string());
                }
            }
        }
    }

    fn exit_text_run(&mut self, id: u64) -> bool {
        if !self.editor.node(id).is_some_and(|node| node.tag == "mtext") {
            return false;
        }
        let Some(mut path) = self.editor.path(id) else {
            return false;
        };
        while let Some(index) = path.pop() {
            let Some(parent_id) = self.editor.id_at_path(&path) else {
                return false;
            };
            if self.editor.node(parent_id).is_some_and(|node| node.row()) {
                self.editor.set_caret(
                    Caret {
                        node: parent_id,
                        offset: index + 1,
                    },
                    false,
                );
                if self.latex_groups.last() == Some(&id) {
                    self.latex_groups.pop();
                }
                self.expects_latex_group = false;
                return true;
            }
        }
        false
    }

    fn close_latex_group(&mut self) {
        let Some(start) = self.latex_groups.pop() else {
            return;
        };
        if self.exit_text_run(start) {
            return;
        }
        let Some(mut path) = self.editor.path(start) else {
            return;
        };
        while let Some(index) = path.pop() {
            let Some(parent_id) = self.editor.id_at_path(&path) else {
                return;
            };
            let Some(parent) = self.editor.node(parent_id) else {
                return;
            };
            if !matches!(
                parent.tag.as_str(),
                "mfrac"
                    | "mroot"
                    | "msqrt"
                    | "msub"
                    | "msup"
                    | "msubsup"
                    | "munder"
                    | "mover"
                    | "munderover"
            ) {
                continue;
            }
            if parent.tag == "mfrac" && index == 0 {
                if let Some(denom) = parent.children.get(1) {
                    self.expects_latex_group = true;
                    self.editor.set_caret(
                        Caret {
                            node: denom.id,
                            offset: 0,
                        },
                        false,
                    );
                }
                return;
            }
            while let Some(index) = path.pop() {
                let Some(id) = self.editor.id_at_path(&path) else {
                    return;
                };
                if self.editor.node(id).is_some_and(|n| n.row()) {
                    self.editor.set_caret(
                        Caret {
                            node: id,
                            offset: index + 1,
                        },
                        false,
                    );
                    return;
                }
            }
            return;
        }
    }

    fn command_key(&mut self, key: egui::Key, modifiers: egui::Modifiers) -> bool {
        use egui::Key;
        if modifiers.command {
            if key == Key::Z && self.command_input.take().is_some() {
                self.latex_groups.clear();
                self.expects_latex_group = false;
                return true;
            }
            if !matches!(key, Key::S | Key::C) {
                self.command_input = None;
                self.latex_groups.clear();
                self.expects_latex_group = false;
            }
            return false;
        }
        let Some(input) = &mut self.command_input else {
            if matches!(
                key,
                Key::ArrowLeft
                    | Key::ArrowRight
                    | Key::ArrowUp
                    | Key::ArrowDown
                    | Key::Home
                    | Key::End
            ) {
                self.latex_groups.clear();
                self.expects_latex_group = false;
            }
            return false;
        };
        match key {
            Key::ArrowDown | Key::ArrowUp => {
                if input.is_command_prefix() {
                    let count = commands::suggestions(&input.name).len();
                    if count > 0 {
                        input.selected = if key == Key::ArrowDown {
                            (input.selected + 1) % count
                        } else {
                            (input.selected + count - 1) % count
                        };
                        self.command_error = None;
                    }
                }
                true
            }
            Key::Tab => {
                if input.is_command_prefix() {
                    if let Some(command) = commands::suggestions(&input.name).get(input.selected) {
                        input.set_name_end(command.name);
                        self.command_error = None;
                    }
                }
                true
            }
            Key::Enter => {
                self.commit_command(true);
                true
            }
            Key::Escape => {
                self.command_input = None;
                self.command_error = None;
                self.status = "Canceled LaTeX command.".into();
                true
            }
            Key::Backspace => {
                if !input.backspace() {
                    self.command_input = None;
                }
                self.command_error = None;
                true
            }
            Key::Delete => {
                input.delete();
                self.command_error = None;
                true
            }
            Key::ArrowLeft => {
                input.move_left();
                true
            }
            Key::ArrowRight => {
                input.move_right();
                true
            }
            Key::Home => {
                input.move_home();
                true
            }
            Key::End => {
                input.move_end();
                true
            }
            _ => false,
        }
    }

    fn command_popup(&mut self, ui: &mut egui::Ui, anchor: Pos2) {
        if self.command_input.is_none() {
            return;
        }
        let cmd_id = egui::Id::unique("canvas-command-input");
        let canvas_id = egui::Id::unique("equation-canvas");
        let has_input_focus = ui.memory(|m| m.has_focus(cmd_id));
        let escape_blurred_new_input = ui.memory(|m| m.focused().is_none())
            && self
                .command_input
                .as_ref()
                .is_some_and(|input| !input.needs_focus);
        if (has_input_focus || escape_blurred_new_input)
            && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.command_input = None;
            self.command_error = None;
            self.status = "Canceled LaTeX command.".into();
            ui.memory_mut(|m| m.request_focus(canvas_id));
            return;
        }
        if has_input_focus {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    cmd_id,
                    egui::EventFilter {
                        tab: true,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: true,
                    },
                );
                // Clear any focus_direction queued by begin_pass before frame-1 filter took effect.
                m.move_focus(egui::FocusDirection::None);
            });
            let enter_pressed = ui.input_mut(|i| {
                i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                    || i.consume_key(egui::Modifiers::SHIFT, egui::Key::Enter)
            });
            if enter_pressed {
                if self.commit_command(true) {
                    ui.memory_mut(|m| m.request_focus(canvas_id));
                    ui.ctx().request_repaint();
                    return;
                }
                ui.memory_mut(|m| m.request_focus(cmd_id));
                ui.ctx().request_repaint();
            }
            let buffer_empty = self
                .command_input
                .as_ref()
                .is_some_and(|input| input.name.is_empty());
            if buffer_empty
                && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Backspace))
            {
                self.command_input = None;
                self.command_error = None;
                ui.memory_mut(|m| m.request_focus(canvas_id));
                return;
            }
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab)) {
                if let Some(input) = &mut self.command_input {
                    if input.is_command_prefix() {
                        if let Some(command) =
                            commands::suggestions(&input.name).get(input.selected)
                        {
                            input.set_name_end(command.name);
                            self.command_error = None;
                        }
                    }
                }
            }
            let has_suggestions = self.command_input.as_ref().is_some_and(|input| {
                input.is_command_prefix() && !commands::suggestions(&input.name).is_empty()
            });
            if has_suggestions {
                let down =
                    ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown));
                let up = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp));
                if down || up {
                    if let Some(input) = &mut self.command_input {
                        let count = commands::suggestions(&input.name).len();
                        if count > 0 {
                            input.selected = if down {
                                (input.selected + 1) % count
                            } else {
                                (input.selected + count - 1) % count
                            };
                            self.command_error = None;
                        }
                    }
                }
            }
            if self
                .command_input
                .as_ref()
                .is_some_and(|input| input.is_command_prefix())
            {
                ui.input_mut(|i| {
                    i.events.retain(|event| match event {
                        egui::Event::Text(text) => !text.chars().all(char::is_whitespace),
                        egui::Event::Key {
                            key: egui::Key::Space,
                            ..
                        } => false,
                        _ => true,
                    });
                });
            }
        }

        let tokens = ThemeTokens::from_ui(ui);
        let mut accept = None;
        let mut popup_edited = false;
        egui::Area::new(egui::Id::unique("canvas-command-popup"))
            .order(egui::Order::Foreground)
            .fixed_pos(anchor)
            .constrain(true)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_width(240.0);
                    let Some(input) = &mut self.command_input else {
                        return;
                    };
                    if input.sync_cursor {
                        let mut state =
                            egui::TextEdit::load_state(ui.ctx(), cmd_id).unwrap_or_default();
                        let ccursor = egui::text::CCursor::new(input.cursor.min(input.char_len()));
                        state
                            .cursor
                            .set_char_range(Some(egui::text::CCursorRange::one(ccursor)));
                        state.store(ui.ctx(), cmd_id);
                        input.sync_cursor = false;
                    }
                    let box_focused = has_input_focus || input.needs_focus;
                    let box_resp = egui::Frame::new()
                        .fill(Color32::TRANSPARENT)
                        .corner_radius(theme::CONTROL_CORNER_RADIUS)
                        .stroke(Stroke::new(
                            1.0,
                            if box_focused {
                                tokens.accent
                            } else {
                                tokens.border
                            },
                        ))
                        .inner_margin(egui::Margin::symmetric(8, 5))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 2.0;
                                let prefix_resp = ui.add(
                                    egui::Label::new(
                                        egui::RichText::new("\\")
                                            .monospace()
                                            .color(tokens.text_muted),
                                    )
                                    .sense(egui::Sense::click()),
                                );
                                let text_resp = ui.add(
                                    egui::TextEdit::singleline(&mut input.name)
                                        .id(cmd_id)
                                        .frame(egui::Frame::NONE)
                                        .background_color(Color32::TRANSPARENT)
                                        .font(egui::TextStyle::Monospace)
                                        .text_color(tokens.text_primary)
                                        .desired_width(f32::INFINITY)
                                        .lock_focus(true),
                                );
                                if prefix_resp.clicked() {
                                    let mut state = egui::TextEdit::load_state(ui.ctx(), cmd_id)
                                        .unwrap_or_default();
                                    state.cursor.set_char_range(Some(
                                        egui::text::CCursorRange::one(egui::text::CCursor::new(0)),
                                    ));
                                    state.store(ui.ctx(), cmd_id);
                                    input.cursor = 0;
                                    text_resp.request_focus();
                                }
                                text_resp
                            })
                            .inner
                        });
                    let text_resp = box_resp.inner;
                    if input.needs_focus {
                        text_resp.request_focus();
                        input.needs_focus = false;
                    }
                    popup_edited = text_resp.changed();
                    if popup_edited {
                        input.selected = 0;
                    }
                    if let Some(state) = egui::TextEdit::load_state(ui.ctx(), cmd_id) {
                        if let Some(range) = state.cursor.char_range() {
                            input.cursor = usize::from(range.primary.index).min(input.char_len());
                        }
                    }

                    let selected = input.selected;
                    let is_prefix = input.is_command_prefix();
                    let candidates = if is_prefix {
                        commands::suggestions(&input.name)
                    } else {
                        Vec::new()
                    };
                    if is_prefix && candidates.is_empty() {
                        ui.label("No matching command — Esc to cancel");
                    }
                    for (index, command) in candidates.iter().enumerate() {
                        if ui
                            .selectable_label(
                                index == selected,
                                format!("\\{}   {}", command.name, command.label),
                            )
                            .clicked()
                        {
                            accept = Some(index);
                        }
                    }
                    if is_prefix {
                        ui.weak("↑↓ select · Tab complete · Enter insert · Esc cancel");
                    } else {
                        ui.weak("Enter insert · Esc cancel");
                    }
                });
            });
        if popup_edited {
            self.command_error = None;
        }
        if let Some(index) = accept {
            if let Some(input) = &mut self.command_input {
                input.selected = index;
            }
            if self.commit_command(true) {
                ui.memory_mut(|m| m.request_focus(canvas_id));
                ui.ctx().request_repaint();
            } else {
                ui.memory_mut(|m| m.request_focus(cmd_id));
                ui.ctx().request_repaint();
            }
        }
    }

    fn keyboard(&mut self, ui: &mut egui::Ui) {
        let events = ui.input(|i| i.events.clone());
        let before = self.editor.revision;
        for event in events {
            if let egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } = &event
            {
                if self.command_key(*key, *modifiers) {
                    continue;
                }
            }
            match event {
                egui::Event::Copy => self.copy(ui.ctx()),
                egui::Event::Cut => {
                    self.command_input = None;
                    self.latex_groups.clear();
                    self.expects_latex_group = false;
                    if self.editor.selected_math().is_some() {
                        self.copy(ui.ctx());
                        self.editor.delete(true);
                    }
                }
                egui::Event::Paste(text) => {
                    self.command_input = None;
                    self.latex_groups.clear();
                    self.expects_latex_group = false;
                    if self.canvas_in_text() {
                        self.editor.insert_text(&text);
                    } else if text.trim_start().starts_with('<') {
                        match eqntide::service::decode_input_to_mathml(&text, "auto") {
                            Ok(math) => self.editor.insert_math(&math),
                            Err(e) => self.status = e.to_string(),
                        }
                    } else if text.contains('\\') {
                        match eqntide::service::parse_latex_to_mathml(&text) {
                            Ok(math) => self.editor.insert_math(&math),
                            Err(e) => self.status = e.to_string(),
                        }
                    } else {
                        self.editor.insert_text(&text);
                    }
                }
                egui::Event::Text(text) => {
                    if self.ime.is_empty() {
                        self.canvas_text(&text);
                    }
                }
                egui::Event::Ime(egui::ImeEvent::Preedit { text, .. }) => self.ime = text,
                egui::Event::Ime(egui::ImeEvent::Commit(text)) => {
                    self.ime.clear();
                    self.canvas_text(&text);
                }
                egui::Event::Ime(egui::ImeEvent::DeleteSurrounding {
                    before_chars,
                    after_chars,
                }) => {
                    for _ in 0..before_chars {
                        self.editor.delete(true);
                    }
                    for _ in 0..after_chars {
                        self.editor.delete(false);
                    }
                }
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    use egui::Key;
                    if modifiers.command {
                        match key {
                            Key::Z => {
                                if modifiers.shift {
                                    self.editor.redo()
                                } else {
                                    self.editor.undo()
                                }
                            }
                            Key::Y => self.editor.redo(),
                            Key::A => self.editor.select_all(),
                            Key::F => self.insert_template("\\frac{#?}{#?}"),
                            Key::R => self.insert_template(if modifiers.shift {
                                "\\sqrt[#?]{#?}"
                            } else {
                                "\\sqrt{#?}"
                            }),
                            Key::H => self.insert_template("^{#?}"),
                            Key::L => self.insert_template(if modifiers.shift {
                                "\\lim_{#? \\to #?}"
                            } else {
                                "_{#?}"
                            }),
                            Key::J => self.insert_template("_{#?}^{#?}"),
                            Key::I => self.insert_template(if modifiers.shift {
                                "\\int_{#?}^{#?} #? \\, d#?"
                            } else {
                                "\\int #? \\, d#?"
                            }),
                            Key::S if modifiers.shift => self.insert_template("\\sum_{#?}^{#?} #?"),
                            Key::P if modifiers.shift => {
                                self.insert_template("\\prod_{#?}^{#?} #?")
                            }
                            Key::M if modifiers.shift => self.insert_template(
                                "\\begin{pmatrix} #? & #? \\\\ #? & #? \\end{pmatrix}",
                            ),
                            Key::C if modifiers.shift => self.insert_template(
                                "\\begin{cases} #? & #? \\\\ #? & #? \\end{cases}",
                            ),
                            Key::Num9 => self.insert_template("\\left( #? \\right)"),
                            Key::OpenBracket | Key::CloseBracket => {
                                self.insert_template("\\left[ #? \\right]")
                            }
                            _ => {}
                        }
                    } else if self.ime.is_empty() {
                        match key {
                            Key::ArrowLeft => self.editor.move_caret(Move::Left, modifiers.shift),
                            Key::ArrowRight => self.editor.move_caret(Move::Right, modifiers.shift),
                            Key::ArrowUp => self.vertical(false, modifiers.shift),
                            Key::ArrowDown => self.vertical(true, modifiers.shift),
                            Key::Home => self.editor.move_caret(Move::Home, modifiers.shift),
                            Key::End => self.editor.move_caret(Move::End, modifiers.shift),
                            Key::Tab => self.editor.move_caret(
                                if modifiers.shift {
                                    Move::PreviousPlaceholder
                                } else {
                                    Move::NextPlaceholder
                                },
                                false,
                            ),
                            Key::Enter if !modifiers.alt => {
                                if !self.exit_text_run(self.editor.caret().node)
                                    && self.editor.insert_row_or_promote_fence()
                                {
                                    self.latex_groups.clear();
                                    self.expects_latex_group = false;
                                }
                            }
                            Key::Backspace => self.editor.delete(true),
                            Key::Delete => self.editor.delete(false),
                            Key::Escape => self.pending = Some(Intent::Close),
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        if self.editor.revision != before {
            self.changed();
        }
    }
    fn back_to_editor_button(&mut self, ui: &mut egui::Ui) {
        if self
            .icon_standard_button(ui, "chevron-left", "Back")
            .clicked()
        {
            self.tab = 0;
            self.active_editor_focus = self.startup_focus;
            self.pending_focus_restore = Some(self.startup_focus);
        }
        ui.add_space(theme::SPACE_MD);
    }

    fn batch_ui(&mut self, ui: &mut egui::Ui) {
        self.sync_batch_row_state();
        let tokens = ThemeTokens::from_ui(ui);
        self.back_to_editor_button(ui);
        Self::full_width_card(ui, tokens, |ui| {
            ui.label(
                egui::RichText::new("Input")
                    .small()
                    .color(tokens.text_muted),
            );
            ui.horizontal_wrapped(|ui| {
                ui.add_enabled_ui(!self.batch_running, |ui| {
                    if self
                        .icon_standard_button(ui, "file", "Add DOCX files…")
                        .clicked()
                    {
                        let tx = self.tx.clone();
                        std::thread::spawn(move || {
                            if let Some(paths) = rfd::FileDialog::new()
                                .add_filter("Word documents", &["docx"])
                                .pick_files()
                            {
                                let _ = tx.send(Event::Paths(paths));
                            }
                        });
                    }
                    if self
                        .icon_standard_button(ui, "folder", "Add folder…")
                        .clicked()
                    {
                        let tx = self.tx.clone();
                        std::thread::spawn(move || {
                            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                let _ = tx.send(Event::Paths(vec![folder]));
                            }
                        });
                    }
                });
                if ui
                    .add_enabled(
                        !self.batch_running && !self.files.is_empty(),
                        egui::Button::new("Clear list")
                            .min_size(Vec2::new(0.0, theme::BUTTON_HEIGHT)),
                    )
                    .clicked()
                {
                    self.files.clear();
                    self.batch_rows.clear();
                    self.batch_collapsed_dirs.clear();
                    self.batch_logs.clear();
                    self.batch_run_total = 0;
                    self.batch_completed = 0;
                    self.batch_totals = [0; 4];
                }
            });
            ui.add_space(theme::SPACE_XS);
            ui.separator();
            ui.add_space(theme::SPACE_XS);
            ui.label(
                egui::RichText::new("Options & Actions")
                    .small()
                    .color(tokens.text_muted),
            );
            ui.add_enabled_ui(!self.batch_running, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let before_batch_target = self.batch_target;
                    egui::ComboBox::from_id_salt("batch-target")
                        .selected_text(batch::TARGETS[self.batch_target].1)
                        .show_ui(ui, |ui| {
                            for (i, (_, label)) in batch::TARGETS.iter().enumerate() {
                                ui.selectable_value(&mut self.batch_target, i, *label);
                            }
                        });
                    if self.batch_target != before_batch_target {
                        self.persisted.batch_target = self.batch_target;
                        self.settings_dirty = true;
                    }
                    if ui
                        .checkbox(
                            &mut self.batch_overwrite,
                            "Overwrite original documents (replace each selected input)",
                        )
                        .changed()
                    {
                        self.persisted.batch_overwrite = self.batch_overwrite;
                        self.settings_dirty = true;
                    }
                });
            });
            ui.horizontal_wrapped(|ui| {
                let selected_count = self.selected_batch_count();
                if self
                    .icon_primary_button(
                        ui,
                        !self.batch_running && selected_count > 0,
                        "arrows-left-right",
                        "Convert",
                    )
                    .clicked()
                {
                    let target = batch::TARGETS[self.batch_target].0;
                    let timestamp = batch::timestamp_now();
                    let mode = self.batch_selection_mode;
                    let selected_items: Vec<(usize, batch::BatchItem)> = self
                        .files
                        .iter()
                        .zip(self.batch_rows.iter())
                        .enumerate()
                        .filter(|(_, (_, row))| row.is_selected(mode))
                        .map(|(idx, (item, _))| (idx, item.clone()))
                        .collect();
                    self.batch_running = true;
                    self.batch_totals = [0; 4];
                    self.batch_completed = 0;
                    self.batch_run_total = selected_items.len();
                    self.batch_run_target = target.into();
                    self.batch_run_overwrite = self.batch_overwrite;
                    self.batch_run_timestamp = timestamp.clone();
                    self.stop.store(false, Ordering::Relaxed);
                    for (idx, _) in &selected_items {
                        if let Some(log) = self.batch_logs.get_mut(*idx) {
                            *log = "Waiting".into();
                        }
                        if let Some(row_state) = self.batch_rows.get_mut(*idx) {
                            row_state.last_output = None;
                        }
                    }
                    batch::run_items(
                        selected_items,
                        target.into(),
                        self.font_pt,
                        self.batch_overwrite,
                        timestamp,
                        Arc::clone(&self.stop),
                        self.tx.clone(),
                    );
                }
                if ui
                    .add_enabled(
                        self.batch_running,
                        egui::Button::new("Stop").min_size(Vec2::new(0.0, theme::BUTTON_HEIGHT)),
                    )
                    .clicked()
                {
                    self.stop.store(true, Ordering::Relaxed);
                }
            });
        });
        ui.add_space(theme::SPACE_MD);
        if self.files.is_empty() {
            return;
        }
        let selected_count = self.selected_batch_count();
        let (display_completed, display_total) = if self.batch_run_total > 0 {
            (self.batch_completed, self.batch_run_total)
        } else {
            (0, selected_count)
        };
        let progress_fraction = if display_total > 0 && self.batch_run_total > 0 {
            display_completed as f32 / display_total as f32
        } else {
            0.0
        };
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), theme::COMPACT_BUTTON_HEIGHT),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                ui.add_enabled_ui(!self.batch_running, |ui| {
                    if Self::compact_button(ui, "Deselect all").clicked() {
                        let mode = self.batch_selection_mode;
                        for row in &mut self.batch_rows {
                            row.set_selected(mode, false);
                        }
                        self.batch_run_total = 0;
                        self.batch_completed = 0;
                    }
                    if Self::compact_button(ui, "Select all").clicked() {
                        let mode = self.batch_selection_mode;
                        for row in &mut self.batch_rows {
                            row.set_selected(mode, true);
                        }
                        self.batch_run_total = 0;
                        self.batch_completed = 0;
                    }
                });
                let bar_width = ui.available_width().max(80.0);
                ui.add(
                    egui::ProgressBar::new(progress_fraction)
                        .desired_width(bar_width)
                        .desired_height(theme::COMPACT_BUTTON_HEIGHT)
                        .text(format!("{display_completed}/{display_total} files")),
                );
            },
        );
        ui.add_space(theme::SPACE_XS);
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), theme::COMPACT_BUTTON_HEIGHT),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                for (mode, label) in [
                    (batch::BatchSelectionMode::FlatList, "File tree"),
                    (batch::BatchSelectionMode::FolderTree, "Folder tree"),
                ] {
                    let active = self.batch_selection_mode == mode;
                    if Self::with_selectable_style(ui, active, |ui| Self::compact_button(ui, label))
                        .clicked()
                    {
                        self.batch_selection_mode = mode;
                        if !self.batch_running {
                            self.batch_run_total = 0;
                            self.batch_completed = 0;
                        }
                    }
                }
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.label(format!(
                        "Found: {}   Converted: {}   Skipped: {}   Errors: {}",
                        self.batch_totals[0],
                        self.batch_totals[1],
                        self.batch_totals[2],
                        self.batch_totals[3]
                    ));
                });
            },
        );
        ui.add_space(theme::SPACE_XS);
        let text_h = ui.text_style_height(&egui::TextStyle::Body);
        let header_h = text_h.max(theme::COMPACT_BUTTON_HEIGHT);
        let row_height = header_h + theme::SPACE_MD;
        let outer_h = ui.available_height().max(140.0);
        const TREE_INDENT: f32 = 24.0;
        tokens.group_frame().show(ui, |ui| {
            let inner_h = (outer_h - 2.0 * theme::SPACE_LG).max(116.0);
            ui.set_min_size(Vec2::new(ui.available_width(), inner_h));
            ui.set_max_height(inner_h);
            ui.spacing_mut().item_spacing.y = 0.0;
            let scroll = egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible);
            match self.batch_selection_mode {
                batch::BatchSelectionMode::FlatList => {
                    scroll.show_rows(ui, row_height, self.files.len(), |ui, row_range| {
                        for idx in row_range {
                            let display_str = self.files[idx].display_label();
                            self.render_batch_file_row(
                                ui,
                                tokens,
                                idx,
                                &display_str,
                                0,
                                TREE_INDENT,
                                row_height,
                                None,
                            );
                        }
                    });
                }
                batch::BatchSelectionMode::FolderTree => {
                    let tree_rows = batch::build_tree_rows(&self.files, &self.batch_collapsed_dirs);
                    let guides = batch::compute_tree_guides(&tree_rows);
                    scroll.show_rows(ui, row_height, tree_rows.len(), |ui, row_range| {
                        for row_idx in row_range {
                            let guide = Some(&guides[row_idx]);
                            match &tree_rows[row_idx] {
                                batch::TreeRow::Group {
                                    key,
                                    label,
                                    hover_path,
                                    depth,
                                    collapsed,
                                    descendant_indices,
                                } => {
                                    self.render_batch_tree_group_row(
                                        ui,
                                        tokens,
                                        key,
                                        label,
                                        hover_path.as_deref(),
                                        *depth,
                                        *collapsed,
                                        descendant_indices,
                                        TREE_INDENT,
                                        row_height,
                                        guide,
                                    );
                                }
                                batch::TreeRow::File {
                                    item_index,
                                    label,
                                    depth,
                                } => {
                                    self.render_batch_file_row(
                                        ui,
                                        tokens,
                                        *item_index,
                                        label,
                                        *depth,
                                        TREE_INDENT,
                                        row_height,
                                        guide,
                                    );
                                }
                            }
                        }
                    });
                }
            }
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn paint_batch_row_chrome(
        ui: &egui::Ui,
        tokens: ThemeTokens,
        rect: Rect,
        depth: usize,
        indent_step: f32,
        guide: Option<&batch::TreeGuide>,
        branch_end_offset: f32,
        is_group: bool,
        has_expanded_children: bool,
    ) {
        let painter = ui.painter();
        let tree_stroke = Stroke::new(1.25, tokens.border_hover);
        let divider_stroke = Stroke::new(1.0, tokens.border.gamma_multiply(0.85));

        let divider_left = if guide.is_some() {
            let content_depth = if is_group { depth + 1 } else { depth };
            rect.left() + content_depth as f32 * indent_step + theme::COMPACT_BUTTON_HEIGHT
        } else {
            rect.left()
        };
        painter.hline(divider_left..=rect.right(), rect.bottom(), divider_stroke);

        if let Some(guide) = guide {
            for (d, &continues) in guide.ancestor_continues.iter().enumerate() {
                if continues {
                    let x = rect.left() + d as f32 * indent_step + 10.0;
                    painter.vline(x, rect.y_range(), tree_stroke);
                }
            }
            if depth > 0 {
                let parent_x = rect.left() + (depth - 1) as f32 * indent_step + 10.0;
                let mid_y = rect.center().y;
                let v_end = if guide.is_last_sibling {
                    mid_y
                } else {
                    rect.bottom()
                };
                painter.vline(parent_x, rect.top()..=v_end, tree_stroke);
                painter.hline(
                    parent_x..=(parent_x + branch_end_offset),
                    mid_y,
                    tree_stroke,
                );
            }
        }

        if has_expanded_children {
            let trunk_x = rect.left() + depth as f32 * indent_step + 10.0;
            let trunk_start =
                (rect.center().y + theme::COMPACT_BUTTON_HEIGHT * 0.5).min(rect.bottom());
            painter.vline(trunk_x, trunk_start..=rect.bottom(), tree_stroke);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_batch_tree_group_row(
        &mut self,
        ui: &mut egui::Ui,
        tokens: ThemeTokens,
        key: &str,
        label: &str,
        hover_path: Option<&str>,
        depth: usize,
        collapsed: bool,
        descendant_indices: &[usize],
        indent_step: f32,
        row_height: f32,
        guide: Option<&batch::TreeGuide>,
    ) {
        let total_desc = descendant_indices.len();
        let selected_desc = descendant_indices
            .iter()
            .filter(|&&i| self.batch_rows[i].tree_selected)
            .count();
        let mut all_checked = total_desc > 0 && selected_desc == total_desc;
        let partial = selected_desc > 0 && selected_desc < total_desc;
        let folder_icon = self.ui_icon(ui.ctx(), "folder", UI_ICON_SIZE, tokens.text_muted);

        let resp = ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), row_height),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                let indent = depth as f32 * indent_step;
                if indent > 0.0 {
                    ui.add_space(indent);
                }
                let toggle_label = if collapsed { "+" } else { "-" };
                if Self::compact_button(ui, toggle_label).clicked() {
                    if collapsed {
                        self.batch_collapsed_dirs.remove(key);
                    } else {
                        self.batch_collapsed_dirs.insert(key.to_owned());
                    }
                }
                ui.add_enabled_ui(!self.batch_running, |ui| {
                    if ui
                        .add(egui::Checkbox::new(&mut all_checked, "").indeterminate(partial))
                        .changed()
                    {
                        let target = all_checked;
                        for &idx in descendant_indices {
                            self.batch_rows[idx].tree_selected = target;
                        }
                        self.batch_run_total = 0;
                        self.batch_completed = 0;
                    }
                });
                ui.add(folder_icon);
                let group_output_dir = hover_path.and_then(|source_dir| {
                    descendant_indices.iter().find_map(|&i| {
                        let out = self.batch_rows[i].last_output.as_ref()?;
                        let root_out = out.root_output_dir.as_deref()?;
                        let source_root = self.files[i].source_root.as_deref()?;
                        let rel = Path::new(source_dir)
                            .strip_prefix(source_root)
                            .unwrap_or(Path::new(""));
                        Some(if rel.as_os_str().is_empty() {
                            root_out.to_path_buf()
                        } else {
                            root_out.join(rel)
                        })
                    })
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(dir_path) = group_output_dir.as_deref() {
                        if Self::compact_button(ui, "Open folder")
                            .on_hover_text(dir_path.display().to_string())
                            .clicked()
                        {
                            if let Err(e) = runtime::open_folder(dir_path) {
                                self.status = e;
                            }
                        }
                    }
                    ui.label(
                        egui::RichText::new(format!("{selected_desc}/{total_desc} selected"))
                            .small()
                            .color(tokens.text_muted),
                    );
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        let lbl = ui
                            .add(egui::Label::new(egui::RichText::new(label).strong()).truncate());
                        if let Some(hover) = hover_path {
                            lbl.on_hover_text(hover);
                        }
                    });
                });
            },
        );
        Self::paint_batch_row_chrome(
            ui,
            tokens,
            resp.response.rect,
            depth,
            indent_step,
            guide,
            11.0,
            true,
            !collapsed && !descendant_indices.is_empty(),
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn render_batch_file_row(
        &mut self,
        ui: &mut egui::Ui,
        tokens: ThemeTokens,
        idx: usize,
        display_str: &str,
        depth: usize,
        indent_step: f32,
        row_height: f32,
        guide: Option<&batch::TreeGuide>,
    ) {
        let full_path = self.files[idx].input.display().to_string();
        let log = self.batch_logs[idx].clone();
        let mode = self.batch_selection_mode;
        let mut checked = self.batch_rows[idx].is_selected(mode);
        let output_meta = self.batch_rows[idx].last_output.clone();

        let resp = ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), row_height),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                let indent = depth as f32 * indent_step;
                if indent > 0.0 {
                    ui.add_space(indent);
                }
                if depth > 0 {
                    // Align file checkboxes and labels past the parent folder's collapse toggle.
                    ui.add_space(theme::COMPACT_BUTTON_HEIGHT);
                }
                ui.add_enabled_ui(!self.batch_running, |ui| {
                    if ui.checkbox(&mut checked, "").changed() {
                        self.batch_rows[idx].set_selected(mode, checked);
                        self.batch_run_total = 0;
                        self.batch_completed = 0;
                    }
                });
                if depth > 0 {
                    let file_icon = self.ui_icon(ui.ctx(), "file", UI_ICON_SIZE, tokens.text_muted);
                    ui.add(file_icon);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(out) = output_meta.as_ref() {
                        let show_open_folder = match mode {
                            batch::BatchSelectionMode::FlatList => true,
                            batch::BatchSelectionMode::FolderTree => out.root_output_dir.is_none(),
                        };
                        if show_open_folder
                            && Self::compact_button(ui, "Open folder")
                                .on_hover_text(out.file_dir.display().to_string())
                                .clicked()
                        {
                            if let Err(e) = runtime::open_folder(&out.file_dir) {
                                self.status = e;
                            }
                        }
                    }
                    if log != "Ready" && log != "Waiting" {
                        let (status_text, status_color) = if log.starts_with("Error:") {
                            (log.as_str(), tokens.error_fg)
                        } else if log == "Stopped" {
                            ("Stopped", tokens.text_muted)
                        } else {
                            (
                                log.split(" → ").next().unwrap_or(log.as_str()),
                                tokens.text_muted,
                            )
                        };
                        ui.label(egui::RichText::new(status_text).small().color(status_color))
                            .on_hover_text(&log);
                    }
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        let name_color = if checked {
                            tokens.text_primary
                        } else {
                            tokens.text_disabled
                        };
                        ui.add(
                            egui::Label::new(egui::RichText::new(display_str).color(name_color))
                                .truncate(),
                        )
                        .on_hover_text(&full_path);
                    });
                });
            },
        );
        Self::paint_batch_row_chrome(
            ui,
            tokens,
            resp.response.rect,
            depth,
            indent_step,
            guide,
            30.0,
            false,
            false,
        );
    }

    fn canvas_preset_picker(
        ui: &mut egui::Ui,
        title: &str,
        presets: &[CanvasPreset],
        dark_mode_preview: bool,
        current: &mut CanvasPreset,
    ) -> bool {
        let tokens = ThemeTokens::from_ui(ui);
        ui.label(egui::RichText::new(title).color(tokens.text_muted));
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(theme::SPACE_MD);
            for &preset in presets {
                let selected = *current == preset;
                let pal = preset.palette(dark_mode_preview);
                let response = Self::with_selectable_style(ui, selected, |ui| {
                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::new(116.0, 32.0), egui::Sense::click());
                    if ui.is_rect_visible(rect) {
                        let visuals = ui.style().interact(&response);
                        let bg = if selected {
                            if response.hovered() {
                                tokens.selected_hover_bg
                            } else {
                                tokens.selected_bg
                            }
                        } else {
                            visuals.bg_fill
                        };
                        let stroke = if selected {
                            Stroke::new(1.0, tokens.selected_border)
                        } else {
                            visuals.bg_stroke
                        };
                        let fg = if selected {
                            tokens.selected_fg
                        } else {
                            visuals.text_color()
                        };
                        ui.painter().rect(
                            rect,
                            theme::CONTROL_CORNER_RADIUS,
                            bg,
                            stroke,
                            egui::StrokeKind::Inside,
                        );
                        let swatch = Rect::from_min_size(
                            Pos2::new(rect.left() + theme::SPACE_MD, rect.center().y - 9.0),
                            Vec2::splat(18.0),
                        );
                        ui.painter().rect(
                            swatch,
                            theme::CONTROL_CORNER_RADIUS,
                            pal.bg,
                            Stroke::new(1.0, pal.border),
                            egui::StrokeKind::Inside,
                        );
                        ui.painter().text(
                            swatch.center(),
                            egui::Align2::CENTER_CENTER,
                            "x",
                            egui::FontId::proportional(10.0),
                            pal.ink,
                        );
                        ui.painter().text(
                            Pos2::new(swatch.right() + theme::SPACE_SM, rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            preset.label(),
                            egui::TextStyle::Body.resolve(ui.style()),
                            fg,
                        );
                    }
                    response
                });
                response.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::Role::RadioButton,
                        ui.is_enabled(),
                        selected,
                        preset.label(),
                    )
                });
                if response.clicked() && *current != preset {
                    *current = preset;
                    changed = true;
                }
            }
        });
        changed
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) {
        let tokens = ThemeTokens::from_ui(ui);
        self.back_to_editor_button(ui);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                Self::full_width_card(ui, tokens, |ui| {
                    ui.label(egui::RichText::new("Canvas zoom").strong());
                    ui.add_space(theme::SPACE_XS);
                    ui.allocate_ui_with_layout(
                        Vec2::new(ui.available_size_before_wrap().x, theme::BUTTON_HEIGHT),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            let slider_changed = ui
                                .scope(|ui| {
                                    ui.spacing_mut().slider_width = 220.0;
                                    let visuals = ui.visuals_mut();
                                    let handle_stroke = Stroke::new(1.0, tokens.accent);
                                    visuals.widgets.inactive.bg_fill = tokens.border;
                                    visuals.widgets.inactive.fg_stroke = handle_stroke;
                                    visuals.widgets.hovered.fg_stroke = handle_stroke;
                                    visuals.widgets.active.fg_stroke = handle_stroke;
                                    visuals.selection.bg_fill = tokens.accent;
                                    ui.add(
                                        egui::Slider::new(&mut self.zoom, ZOOM_RANGE)
                                            .show_value(false)
                                            .trailing_fill(true)
                                            .handle_shape(egui::style::HandleShape::Rect {
                                                aspect_ratio: 1.0,
                                            }),
                                    )
                                    .changed()
                                })
                                .inner;
                            let value_changed = ui
                                .add(
                                    egui::DragValue::new(&mut self.zoom)
                                        .range(ZOOM_RANGE)
                                        .speed(0.01)
                                        .min_size(Vec2::new(60.0, theme::BUTTON_HEIGHT))
                                        .custom_formatter(|value, _| {
                                            format!("{:.0}%", value * 100.0)
                                        })
                                        .custom_parser(|text| {
                                            text.trim()
                                                .trim_end_matches('%')
                                                .trim()
                                                .parse::<f64>()
                                                .ok()
                                                .map(|pct| pct / 100.0)
                                        }),
                                )
                                .changed();
                            if slider_changed || value_changed {
                                self.persisted.zoom = self.zoom;
                                self.settings_dirty = true;
                            }
                            if Self::standard_button(ui, "Reset (200%)").clicked() {
                                self.zoom = settings::DEFAULT_ZOOM;
                                self.persisted.zoom = self.zoom;
                                self.settings_dirty = true;
                            }
                        },
                    );
                });
                ui.add_space(theme::SPACE_MD);
                Self::full_width_card(ui, tokens, |ui| {
                    ui.label(egui::RichText::new("Appearance").strong());
                    ui.add_space(theme::SPACE_XS);
                    let is_dark = ui.visuals().dark_mode;
                    ui.horizontal(|ui| {
                        for (choice, label) in
                            [(egui::Theme::Light, "Light"), (egui::Theme::Dark, "Dark")]
                        {
                            let selected = (choice == egui::Theme::Dark) == is_dark;
                            if Self::with_selectable_style(ui, selected, |ui| {
                                Self::standard_button(ui, label)
                            })
                            .clicked()
                                && !selected
                            {
                                ui.ctx().set_theme(choice);
                                self.persisted.theme = ThemeMode::from_egui_theme(choice);
                                self.settings_dirty = true;
                            }
                        }
                    });
                    ui.add_space(theme::SPACE_MD);
                    ui.label(egui::RichText::new("Equation canvas").strong());
                    ui.add_space(theme::SPACE_XS);
                    if Self::canvas_preset_picker(
                        ui,
                        "Light mode canvas",
                        theme::LIGHT_CANVAS_PRESETS,
                        false,
                        &mut self.canvas_light,
                    ) {
                        self.persisted.canvas_light = self.canvas_light;
                        self.settings_dirty = true;
                    }
                    ui.add_space(theme::SPACE_MD);
                    if Self::canvas_preset_picker(
                        ui,
                        "Dark mode canvas",
                        theme::DARK_CANVAS_PRESETS,
                        true,
                        &mut self.canvas_dark,
                    ) {
                        self.persisted.canvas_dark = self.canvas_dark;
                        self.settings_dirty = true;
                    }
                });
                ui.add_space(theme::SPACE_MD);
                Self::full_width_card(ui, tokens, |ui| {
                    ui.label(egui::RichText::new("Startup focus").strong());
                    ui.add_space(theme::SPACE_XS);
                    ui.horizontal(|ui| {
                        let mut focus_changed = false;
                        for (choice, label) in [
                            (StartupFocus::Canvas, "Equation canvas"),
                            (StartupFocus::Latex, "LaTeX source"),
                        ] {
                            let selected = self.startup_focus == choice;
                            if Self::with_selectable_style(ui, selected, |ui| {
                                Self::standard_button(ui, label)
                            })
                            .clicked()
                                && !selected
                            {
                                self.startup_focus = choice;
                                focus_changed = true;
                            }
                        }
                        if focus_changed {
                            self.persisted.startup_focus = self.startup_focus;
                            self.settings_dirty = true;
                        }
                    });
                });
            });
    }

    fn about_ui(&mut self, ui: &mut egui::Ui) {
        let tokens = ThemeTokens::from_ui(ui);
        self.back_to_editor_button(ui);
        let author_icon = self.ui_icon(ui.ctx(), "user", UI_ICON_SIZE, tokens.text_muted);
        let license_icon = self.ui_icon(ui.ctx(), "license", UI_ICON_SIZE, tokens.text_muted);
        let github_icon = self.ui_icon(ui.ctx(), "brand-github", UI_ICON_SIZE, tokens.text_muted);
        let logo_id = self
            .logo_texture
            .get_or_insert_with(|| {
                let bytes = include_bytes!("../../assets/icons/icon.png");
                let pixels = image::load_from_memory(bytes)
                    .expect("bundled icon is valid")
                    .to_rgba8();
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [pixels.width() as usize, pixels.height() as usize],
                    pixels.as_raw(),
                );
                ui.ctx()
                    .load_texture("eqntide-logo", image, egui::TextureOptions::LINEAR)
            })
            .id();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                Self::full_width_card(ui, tokens, |ui| {
                    const ABOUT_LOGO_SIZE: f32 = 72.0;
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::Image::new((logo_id, Vec2::splat(ABOUT_LOGO_SIZE)))
                                .corner_radius(theme::SURFACE_CORNER_RADIUS),
                        );
                        ui.add_space(theme::SPACE_LG);
                        let title = concat!("EqnTide ", env!("CARGO_PKG_VERSION"));
                        let desc = env!("CARGO_PKG_DESCRIPTION");
                        let desc = desc.strip_prefix("EqnTide: ").unwrap_or(desc);
                        let avail_w = ui.available_width().max(1.0);
                        let heading_font = egui::TextStyle::Heading.resolve(ui.style());
                        let body_font = egui::TextStyle::Body.resolve(ui.style());
                        let title_h = ui.fonts_mut(|f| {
                            f.layout(title.to_owned(), heading_font, tokens.text_primary, avail_w)
                                .size()
                                .y
                        });
                        let desc_h = ui.fonts_mut(|f| {
                            f.layout(desc.to_owned(), body_font, tokens.text_primary, avail_w)
                                .size()
                                .y
                        });
                        let content_h = title_h + ui.spacing().item_spacing.y + desc_h;
                        let top_pad = ((ABOUT_LOGO_SIZE - content_h) * 0.5).max(0.0);
                        ui.vertical(|ui| {
                            ui.add_space(top_pad);
                            ui.heading(title);
                            ui.label(desc);
                        });
                    });
                    ui.add_space(theme::SPACE_XL);
                    ui.separator();
                    ui.add_space(theme::SPACE_MD);
                    egui::Grid::new("about-info")
                        .num_columns(2)
                        .spacing([theme::SPACE_XL, theme::SPACE_MD])
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.add(author_icon);
                                ui.strong("Author:");
                            });
                            ui.label(env!("CARGO_PKG_AUTHORS"));
                            ui.end_row();

                            ui.horizontal(|ui| {
                                ui.add(license_icon);
                                ui.strong("License:");
                            });
                            ui.label(env!("CARGO_PKG_LICENSE"));
                            ui.end_row();

                            ui.horizontal(|ui| {
                                ui.add(github_icon);
                                ui.strong("GitHub:");
                            });
                            ui.hyperlink(env!("CARGO_PKG_REPOSITORY"));
                            ui.end_row();
                        });
                });
            });
    }
    fn dialogs(&mut self, ctx: &egui::Context) {
        // An Office session alone is not an unsaved change. Cancel an
        // untouched session before closing/switching so its add-in wait ends
        // without replacing the equation in the document.
        if self.pending.is_some() && !self.dirty() && !self.batch_running {
            if !self.busy {
                self.finish(false);
            }
            return;
        }
        if self.pending.is_some() {
            egui::Window::new("Finish current equation")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    if self.dirty() {
                        ui.label("Save your changes, discard them, or continue editing.");
                    }
                    if self.batch_running {
                        ui.label("A document is being converted. Stop the batch and wait before closing.");
                        if Self::standard_button(ui, "Stop batch").clicked() {
                            self.stop.store(true, Ordering::Relaxed);
                        }
                    }
                    ui.add_enabled_ui(!self.busy && !self.batch_running, |ui| {
                        ui.horizontal(|ui| {
                            if Self::primary_button(ui, true, "Save").clicked() {
                                self.finish(true);
                            }
                            if Self::standard_button(ui, "Discard / cancel session").clicked() {
                                self.finish(false);
                            }
                            if Self::standard_button(ui, "Continue editing").clicked() {
                                self.pending = None;
                            }
                        });
                    });
                });
        }
    }
}

impl App {
    pub(crate) fn ui_pass(&mut self, ui: &mut egui::Ui) {
        self.passive_canvas_rect = None;
        let ctx = ui.ctx().clone();
        self.enforce_window_minimum_if_needed(&ctx);
        self.events(&ctx);
        let save_requested = Self::take_save_shortcut(&ctx);
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close {
            if !self.dirty()
                && self.session.is_none()
                && self.queue.is_empty()
                && !self.batch_running
                && !self.busy
            {
                self.flush_settings();
                self.allow_close = true;
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.pending = Some(Intent::Close);
            }
        }
        self.sync_render(&ctx);
        self.shell(ui);
        if (save_requested || self.save_clicked) && self.save_available() {
            self.finish(true);
        }
        self.matrix_dialog_ui(&ctx);
        self.sync_render(&ctx);
        self.dialogs(&ctx);
        self.request_latex_parse(&ctx);
        self.drain_before_close(&ctx);
        if self.settings_dirty && !ctx.input(|i| i.pointer.any_down()) {
            self.flush_settings();
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        ThemeTokens::for_mode(visuals.dark_mode)
            .app_bg
            .to_normalized_gamma_f32()
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.ui_pass(ui);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> App {
        let (tx, rx) = std::sync::mpsc::channel();
        App::from_parts(None, Sender::new(tx), rx, vec![])
    }
    fn request(id: &str, mathml: Option<&str>) -> DesktopWindowRequest {
        DesktopWindowRequest {
            session_id: id.into(),
            mathml: mathml.map(str::to_owned),
            latex: None,
            font_pt: Some(14.0),
            tab: None,
            target_mode: Some("mathtype".into()),
        }
    }
    #[test]
    fn dirty_equation_is_queued_without_mutating_its_target_or_tree() {
        let mut a = app();
        a.editor.insert_text("x");
        let before = a.editor.math();
        a.window(request("", Some("<math><mi>y</mi></math>")));
        assert_eq!(a.editor.math(), before);
        assert_eq!(a.target, "omml");
        assert_eq!(a.queue.len(), 1);
        assert!(matches!(a.pending, Some(Intent::Next)));
    }
    #[test]
    fn omml_unsupported_warning_displays_in_status_bar() {
        let cases = [
            (
                r"\enclose{longdiv}{144}",
                "Long division is not supported in OMML",
            ),
            (
                r"\enclose{roundedbox}{x}",
                "Rounded box is not supported in OMML",
            ),
            (
                r"\enclose{verticalstrike}{x}",
                "Vertical strike is not supported in OMML",
            ),
            (
                r"\underddddot{x}",
                "Quadruple underdot is not supported in OMML",
            ),
        ];

        for (latex, expected_msg) in cases {
            let mut a = app();
            let math = eqntide::service::parse_latex_to_mathml(latex).unwrap();
            a.editor.replace(&math);
            a.changed();
            assert_eq!(a.target, "omml");

            let ctx = egui::Context::default();
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| a.status_bar(ui));
            output.textures_delta.clear();
            let has_warning = output.shapes.iter().any(|s| {
                matches!(&s.shape, egui::Shape::Text(t) if t.galley.text().contains(expected_msg))
            });
            assert!(
                has_warning,
                "Expected OMML warning in status bar for {latex}"
            );
        }

        for (latex, expected_msg) in [
            (
                r"\enclose{verticalstrike}{x}",
                "Vertical strike is not supported in MathType",
            ),
            (
                r"\enclose{circle}{x}",
                "Circle enclosure is not supported in MathType",
            ),
            (
                r"\enclose{phasorangle}{x}",
                "Phasor angle is not supported in MathType",
            ),
        ] {
            let mut a = app();
            a.target = "mathtype".into();
            let math = eqntide::service::parse_latex_to_mathml(latex).unwrap();
            a.editor.replace(&math);
            a.changed();

            let ctx = egui::Context::default();
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| a.status_bar(ui));
            output.textures_delta.clear();
            let has_warning = output.shapes.iter().any(|s| {
                matches!(&s.shape, egui::Shape::Text(t) if t.galley.text().contains(expected_msg))
            });
            assert!(
                has_warning,
                "Expected MathType warning in status bar for {latex}"
            );
        }

        // finish(true) in an Office session blocks unsupported constructs for the active target.
        let mut a = app();
        a.session = Some("sess-1".into());
        a.target = "omml".into();
        let math = eqntide::service::parse_latex_to_mathml(r"\enclose{longdiv}{144}").unwrap();
        a.editor.replace(&math);
        a.changed();
        a.finish(true);
        assert!(!a.busy);
        assert!(a.status.contains("not supported in OMML"));
    }
    #[test]
    fn startup_loading_never_opens_two_sessions_concurrently() {
        let mut a = app();
        a.busy = true;
        a.window(request("session-one", None));
        a.window(request("session-two", None));
        assert_eq!(a.queue.len(), 2);
        assert!(a.session.is_none());
    }
    fn draw_dialogs(a: &mut App) -> bool {
        let ctx = egui::Context::default();
        ctx.run_ui(egui::RawInput::default(), |ui| a.dialogs(ui.ctx()))
            .textures_delta
            .clear();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| a.dialogs(ui.ctx()));
        output.textures_delta.clear();
        output.shapes.iter().any(|s| {
            matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == "Finish current equation")
        })
    }
    #[test]
    fn untouched_office_session_does_not_prompt_even_while_busy() {
        for intent in [Intent::Close, Intent::Next] {
            let mut a = app();
            a.session = Some("office".into());
            a.pending = Some(intent);
            // Simulate loading/cancellation in flight; don't launch real HTTP.
            a.busy = true;
            assert!(!draw_dialogs(&mut a));
            assert!(a.pending.is_some());
            assert_eq!(a.session.as_deref(), Some("office"));
        }
    }
    #[test]
    fn clean_transition_finishes_without_prompt_and_opens_next_equation() {
        let mut a = app();
        a.pending = Some(Intent::Next);
        a.queue
            .push_back(request("", Some("<math><mi>y</mi></math>")));
        assert!(!draw_dialogs(&mut a));
        assert!(a.busy);
        let finished = a.rx.recv_timeout(Duration::from_secs(2)).unwrap();
        a.tx.send(finished).unwrap();
        a.events(&egui::Context::default());
        assert_eq!(a.editor.math().to_latex(), "y");
        assert!(a.pending.is_none());
        assert!(a.queue.is_empty());
        assert!(!a.dirty());
    }
    #[test]
    fn changed_office_equation_still_prompts_until_undone() {
        let mut a = app();
        a.session = Some("office".into());
        a.editor.insert_text("x");
        a.pending = Some(Intent::Close);
        assert!(draw_dialogs(&mut a));
        assert!(!a.busy);
        a.editor.undo();
        a.changed();
        a.busy = true;
        assert!(!draw_dialogs(&mut a));
    }
    #[test]
    fn invalid_source_or_changed_size_still_requires_confirmation() {
        for source_draft in [true, false] {
            let mut a = app();
            a.pending = Some(Intent::Close);
            if source_draft {
                a.latex = "\\frac{".into();
                a.source_changed(Instant::now());
            } else {
                a.font_pt += 2.0;
            }
            assert!(draw_dialogs(&mut a));
        }
    }
    #[test]
    fn running_batch_still_requires_confirmation_even_without_equation_edits() {
        let mut a = app();
        a.pending = Some(Intent::Close);
        a.batch_running = true;
        assert!(draw_dialogs(&mut a));
    }
    #[test]
    fn duplicate_requests_during_loading_do_not_prompt_or_queue() {
        let mut a = app();
        a.busy = true;
        a.loading_session = Some("office".into());
        a.window(request("office", None));
        // Both bridge callbacks and forwarded custom URIs reach window().
        a.launch(
            vec!["eqntide://open?session_id=office".into()],
            &egui::Context::default(),
        );
        assert!(a.queue.is_empty());
        assert!(a.pending.is_none());
        a.tx.send(Event::Session(
            "office".into(),
            Ok(serde_json::json!({
                "state": "waiting", "mathml": "<math><mi>x</mi></math>",
                "font_pt": 14, "target_mode": "mathtype"
            })),
        ))
        .unwrap();
        a.events(&egui::Context::default());
        assert_eq!(a.session.as_deref(), Some("office"));
        assert!(a.loading_session.is_none());
        assert!(!a.dirty());
        a.window(request("office", None));
        assert!(a.queue.is_empty());
        assert!(a.pending.is_none());
    }
    #[test]
    fn duplicate_queued_session_is_not_added_twice() {
        let mut a = app();
        a.session = Some("current".into());
        a.window(request("next", None));
        a.window(request("next", None));
        assert_eq!(a.queue.len(), 1);
        assert!(matches!(a.pending, Some(Intent::Next)));
        assert_eq!(a.session.as_deref(), Some("current"));
    }
    #[test]
    fn failed_session_load_clears_loading_marker_and_keeps_work() {
        let mut a = app();
        a.editor.insert_text("x");
        let before = a.editor.math();
        a.busy = true;
        a.loading_session = Some("office".into());
        a.tx.send(Event::Session("office".into(), Err("Unavailable".into())))
            .unwrap();
        a.events(&egui::Context::default());
        assert!(a.loading_session.is_none());
        assert!(!a.busy);
        assert!(a.session.is_none());
        assert_eq!(a.editor.math(), before);
        assert!(a.status.contains("Unavailable"));
    }
    #[test]
    fn successful_office_save_closes_the_window() {
        let mut a = app();
        a.editor.insert_text("x");
        a.session = Some("office".into());
        a.busy = true;
        a.tx.send(Event::Finished(Ok(()))).unwrap();
        a.events(&egui::Context::default());
        assert!(a.session.is_none());
        assert!(!a.dirty());
        assert!(a.close_deadline.is_some());
        a.close_deadline = Some(Instant::now());
        a.drain_before_close(&egui::Context::default());
        assert!(a.allow_close);
    }
    fn save_key(shift: bool) -> egui::Event {
        egui::Event::Key {
            key: egui::Key::S,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers {
                ctrl: true,
                command: true,
                shift,
                ..Default::default()
            },
        }
    }
    fn type_canvas(a: &mut App, text: &str) {
        egui::Context::default()
            .run_ui(
                egui::RawInput {
                    events: vec![egui::Event::Text(text.into())],
                    ..Default::default()
                },
                |ui| a.keyboard(ui),
            )
            .textures_delta
            .clear();
    }

    fn canvas_key(a: &mut App, key: egui::Key) {
        egui::Context::default()
            .run_ui(
                egui::RawInput {
                    events: vec![egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    }],
                    ..Default::default()
                },
                |ui| a.keyboard(ui),
            )
            .textures_delta
            .clear();
    }

    #[test]
    fn canvas_inserts_text_only_when_enter_confirms_a_complete_argument() {
        use egui::Key;
        let mut a = app();
        type_canvas(&mut a, "x + 1");
        let before = a.editor.math();

        type_canvas(&mut a, r"\text ");
        canvas_key(&mut a, Key::Enter);
        assert!(a.status.contains("Invalid LaTeX command"));
        assert_eq!(a.editor.math(), before);

        type_canvas(&mut a, "{5 cm ");
        assert_eq!(a.editor.math(), before);
        type_canvas(&mut a, "} ");
        assert_eq!(a.editor.math(), before);
        assert!(a.command_input.is_some());
        canvas_key(&mut a, Key::Enter);
        assert!(a.command_input.is_none());
        assert!(!a.canvas_in_text());
        assert_eq!(a.editor.math().find_all("mtext")[0].text, "5 cm ");

        let before = a.editor.math();
        type_canvas(&mut a, r"\text+");
        canvas_key(&mut a, Key::Enter);
        assert!(a.status.contains("Invalid LaTeX command"));
        assert_eq!(a.editor.math(), before);
    }

    #[test]
    fn canvas_arrows_and_space_distinguish_text_and_math_at_both_edges() {
        use egui::Key;
        let mut a = app();
        type_canvas(&mut a, r"\text{cm}");
        canvas_key(&mut a, Key::Enter);
        type_canvas(&mut a, "x");
        canvas_key(&mut a, Key::ArrowLeft);
        assert!(!a.canvas_in_text());
        type_canvas(&mut a, " ");
        assert_eq!(a.editor.math().to_latex(), r"\text{cm}x");
        canvas_key(&mut a, Key::ArrowLeft);
        assert!(a.canvas_in_text());
        assert_eq!(a.editor.caret().offset, 2);
        canvas_key(&mut a, Key::ArrowLeft);
        assert_eq!(a.editor.caret().offset, 1);
        canvas_key(&mut a, Key::ArrowRight);
        assert!(a.canvas_in_text());
        assert_eq!(a.editor.caret().offset, 2);
        canvas_key(&mut a, Key::ArrowRight);
        assert!(!a.canvas_in_text());
        canvas_key(&mut a, Key::ArrowLeft);
        type_canvas(&mut a, " ");
        assert_eq!(a.editor.math().to_latex(), r"\text{cm }x");

        canvas_key(&mut a, Key::Home);
        assert!(!a.canvas_in_text());
        canvas_key(&mut a, Key::ArrowRight);
        assert!(a.canvas_in_text());
        assert_eq!(a.editor.caret().offset, 0);
        canvas_key(&mut a, Key::ArrowLeft);
        assert!(!a.canvas_in_text());
        type_canvas(&mut a, " ");
        assert_eq!(a.editor.math().find_all("mtext")[0].text, "cm ");
        canvas_key(&mut a, Key::ArrowRight);
        type_canvas(&mut a, r" \/_^");
        assert!(a.command_input.is_none());
        assert_eq!(a.editor.math().find_all("mtext")[0].text, r" \/_^cm ");
    }

    #[test]
    fn enter_leaves_canvas_text_and_resumes_math_input() {
        use egui::Key;
        let mut a = app();
        type_canvas(&mut a, r"\text{ngày mà em đến}");
        canvas_key(&mut a, Key::Enter);
        canvas_key(&mut a, Key::ArrowLeft);
        assert!(a.canvas_in_text());
        let before = a.editor.math();

        canvas_key(&mut a, Key::Enter);
        assert_eq!(a.editor.math(), before);
        assert!(!a.canvas_in_text());
        type_canvas(&mut a, "x+1");
        assert_eq!(a.editor.math().to_latex(), r"\text{ngày mà em đến}x+1");
    }

    #[test]
    fn canvas_arrows_keep_decomposed_vietnamese_letters_together() {
        use egui::Key;
        let mut a = app();
        type_canvas(&mut a, "\\text{e\u{0302}\u{0301}x}");
        canvas_key(&mut a, Key::Enter);
        canvas_key(&mut a, Key::ArrowLeft);
        assert_eq!(a.editor.caret().offset, 4);
        canvas_key(&mut a, Key::ArrowLeft);
        assert_eq!(a.editor.caret().offset, 3);
        canvas_key(&mut a, Key::ArrowLeft);
        assert_eq!(a.editor.caret().offset, 0);
        assert!(a.canvas_in_text());
    }

    #[test]
    fn canvas_caret_switches_to_the_active_text_color_without_moving() {
        let mut a = app();
        a.canvas_light = CanvasPreset::WarmCream;
        a.canvas_dark = CanvasPreset::DeepInk;
        type_canvas(&mut a, r"\text{ngày đến}");
        canvas_key(&mut a, egui::Key::Enter);
        type_canvas(&mut a, "=x");
        let text = a.editor.root().children[0].id;
        let math_caret = Caret {
            node: a.editor.root().id,
            offset: 1,
        };
        for (theme, dark) in [(egui::Theme::Light, false), (egui::Theme::Dark, true)] {
            let ctx = egui::Context::default();
            ctx.set_theme(theme);
            let pal = a.active_canvas_palette(dark);
            let mut ends = Vec::new();
            for (caret, ink) in [
                (math_caret, pal.caret),
                (
                    Caret {
                        node: text,
                        offset: 8,
                    },
                    pal.text_caret,
                ),
            ] {
                a.editor.set_caret(caret, false);
                let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                    ui.memory_mut(|m| m.request_focus(App::editor_focus_id(StartupFocus::Canvas)));
                    a.canvas(ui);
                });
                output.textures_delta.clear();
                let caret_x = output.shapes.iter().find_map(|shape| match &shape.shape {
                    egui::Shape::LineSegment { points, stroke }
                        if stroke.color == ink && points[0].x == points[1].x =>
                    {
                        Some(points[0].x)
                    }
                    _ => None,
                });
                ends.push(caret_x.expect("a visible caret with the active palette color"));
            }
            assert_eq!(ends[0], ends[1]);
        }
    }

    #[test]
    fn canvas_exit_from_text_in_fraction_stays_in_the_same_slot() {
        let mut a = app();
        assert!(a.load_math(
            "<math><mfrac><mtext>cm</mtext><mn>2</mn></mfrac></math>",
            None,
            None
        ));
        let id = a.editor.root().children[0].children[0].children[0].id;
        a.editor.set_caret(
            Caret {
                node: id,
                offset: 2,
            },
            false,
        );
        canvas_key(&mut a, egui::Key::ArrowRight);
        assert!(!a.canvas_in_text());
        type_canvas(&mut a, "x");
        assert_eq!(a.editor.math().to_latex(), r"\frac{\text{cm}x}{2}");
    }

    #[test]
    fn typed_latex_commands_build_real_math_and_keep_source_in_sync() {
        let mut a = app();
        // Separate keystrokes exercise the buffered command input without early placeholder jumps.
        for ch in r"\lim_{x\rightarrow+\infty}".chars() {
            type_canvas(&mut a, &ch.to_string());
        }
        assert_eq!(
            a.command_input.as_ref().map(|i| i.name.as_str()),
            Some(r"lim_{x\rightarrow+\infty}")
        );
        assert!(a.command_key(egui::Key::Enter, egui::Modifiers::NONE));
        let xml = a.editor.math().to_xml();
        assert!(xml.contains("lim"), "{xml}");
        assert!(xml.contains('→') && xml.contains('∞'), "{xml}");
        assert!(
            !xml.contains('\\'),
            "commands must not become literal characters: {xml}"
        );
        assert!(a.command_input.is_none());
        assert_eq!(a.latex, a.editor.math().to_latex());
        assert!(eqntide::render::render_interactive(&a.editor.math(), 12.0).is_ok());
    }

    #[test]
    fn typed_fraction_groups_fill_slots_and_exit_back_to_the_outer_equation() {
        let mut a = app();
        type_canvas(&mut a, r"\frac{1}{\sqrt{2}}");
        assert!(a.command_input.is_some());
        assert!(a.command_key(egui::Key::Enter, egui::Modifiers::NONE));
        type_canvas(&mut a, "+x");
        assert_eq!(a.editor.math().to_latex(), r"\frac{1}{\sqrt{2}}+x");
        assert!(a.command_input.is_none());
        assert!(a.latex_groups.is_empty());
        type_canvas(&mut a, "{y}");
        assert!(
            a.editor.math().to_xml().contains('{'),
            "literal braces still work outside LaTeX groups"
        );
    }

    #[test]
    fn dfrac_completion_inserts_fraction_slots_and_accepts_typed_arguments() {
        let mut a = app();
        type_canvas(&mut a, r"\df");
        assert!(a.command_key(egui::Key::Enter, egui::Modifiers::NONE));
        assert!(a.editor.math().iter().iter().any(|n| n.tag == "mfrac"));
        type_canvas(&mut a, "{1}{2}");
        assert_eq!(a.editor.math().to_latex(), r"\dfrac{1}{2}");
        assert_eq!(a.latex, a.editor.math().to_latex());
    }

    #[test]
    fn completion_navigation_cancel_and_unknown_commands_keep_existing_work() {
        let mut a = app();
        type_canvas(&mut a, "x");
        let before = a.editor.math();
        type_canvas(&mut a, r"\l");
        a.command_key(egui::Key::ArrowDown, egui::Modifiers::NONE);
        assert_eq!(a.command_input.as_ref().unwrap().selected, 1);
        a.command_key(egui::Key::ArrowUp, egui::Modifiers::NONE);
        assert_eq!(a.command_input.as_ref().unwrap().selected, 0);
        a.command_key(egui::Key::Escape, egui::Modifiers::NONE);
        type_canvas(&mut a, r"\alp");
        assert_eq!(a.editor.math(), before);
        assert!(a.command_key(egui::Key::Tab, egui::Modifiers::NONE));
        assert_eq!(
            a.command_input.as_ref().map(|i| i.name.as_str()),
            Some("alpha"),
            "Tab only completes the command name in the buffer without committing"
        );
        assert_eq!(a.editor.math(), before);
        assert!(a.command_key(egui::Key::Enter, egui::Modifiers::NONE));
        assert!(a.editor.math().to_xml().contains('α'));
        let accepted = a.editor.math();
        type_canvas(&mut a, r"\nosuchcommand+1");
        assert_eq!(a.command_input.as_ref().unwrap().name, "nosuchcommand+1");
        a.finish(true);
        assert!(!a.busy);
        assert_eq!(a.editor.math(), accepted);
        a.command_key(egui::Key::Backspace, egui::Modifiers::NONE);
        assert_eq!(a.command_input.as_ref().unwrap().name, "nosuchcommand+");
        a.command_key(egui::Key::Escape, egui::Modifiers::NONE);
        assert!(a.command_input.is_none());
        assert!(
            a.pending.is_none(),
            "Esc must cancel completion without closing the app"
        );
        a.editor.undo();
        assert_eq!(
            a.editor.math(),
            before,
            "accepting a command is one undo transaction"
        );
    }

    #[test]
    fn completion_accept_keys_work_through_the_focused_canvas() {
        let setup_focused = |a: &mut App, ctx: &egui::Context| {
            let input = |events| egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 650.0))),
                events,
                ..Default::default()
            };
            ctx.run_ui(input(vec![]), |ui| {
                a.shell(ui);
                ui.memory_mut(|m| m.request_focus(egui::Id::unique("equation-canvas")));
            })
            .textures_delta
            .clear();
        };
        let send_events = |a: &mut App, ctx: &egui::Context, events: Vec<egui::Event>| {
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 650.0))),
                    events,
                    ..Default::default()
                },
                |ui| a.shell(ui),
            )
            .textures_delta
            .clear();
        };
        let key_event = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };

        // Enter commits buffered command.
        let mut a = app();
        let ctx = egui::Context::default();
        setup_focused(&mut a, &ctx);
        send_events(&mut a, &ctx, vec![egui::Event::Text(r"\alp".into())]);
        assert!(a.command_input.is_some());
        assert!(ctx.memory(|m| m.has_focus(egui::Id::unique("canvas-command-input"))));
        send_events(&mut a, &ctx, vec![key_event(egui::Key::Enter)]);
        assert!(a.editor.math().to_xml().contains('α'));
        assert!(a.command_input.is_none());
        assert!(ctx.memory(|m| m.has_focus(egui::Id::unique("equation-canvas"))));
        assert_eq!(a.latex, a.editor.math().to_latex());

        // Tab completes command name without committing; Enter commits afterwards.
        let mut a = app();
        let ctx = egui::Context::default();
        setup_focused(&mut a, &ctx);
        send_events(&mut a, &ctx, vec![egui::Event::Text(r"\math".into())]);
        send_events(&mut a, &ctx, vec![key_event(egui::Key::Tab)]);
        assert_eq!(
            a.command_input.as_ref().map(|i| i.name.as_str()),
            Some("mathbb")
        );
        assert!(ctx.memory(|m| m.has_focus(egui::Id::unique("canvas-command-input"))));
        send_events(&mut a, &ctx, vec![egui::Event::Text("{R}".into())]);
        assert_eq!(
            a.command_input.as_ref().map(|i| i.name.as_str()),
            Some("mathbb{R}")
        );
        send_events(&mut a, &ctx, vec![key_event(egui::Key::Enter)]);
        assert!(a.command_input.is_none());
        assert!(ctx.memory(|m| m.has_focus(egui::Id::unique("equation-canvas"))));
        assert!(a.editor.math().to_latex().contains(r"\mathbb{R}"));

        // Space remains literal inside the command buffer until Enter commits.
        let mut a = app();
        let ctx = egui::Context::default();
        setup_focused(&mut a, &ctx);
        send_events(&mut a, &ctx, vec![egui::Event::Text(r"\lim_{x".into())]);
        send_events(
            &mut a,
            &ctx,
            vec![key_event(egui::Key::Space), egui::Event::Text(" ".into())],
        );
        assert_eq!(
            a.command_input.as_ref().map(|i| i.name.as_str()),
            Some("lim_{x ")
        );
        send_events(&mut a, &ctx, vec![egui::Event::Text(r"\to 0}".into())]);
        send_events(&mut a, &ctx, vec![key_event(egui::Key::Enter)]);
        assert!(a.command_input.is_none());
        let xml = a.editor.math().to_xml();
        assert!(xml.contains("lim") && xml.contains('→'), "{xml}");
    }

    #[test]
    fn completion_buffered_latex_handles_mathbb_braces_and_integrals() {
        // \math + { stays buffered, Backspace + Tab completes \mathbb, then {R} + Enter commits.
        let mut a = app();
        for ch in r"\math{".chars() {
            type_canvas(&mut a, &ch.to_string());
        }
        assert_eq!(
            a.command_input.as_ref().map(|i| i.name.as_str()),
            Some("math{")
        );
        assert!(a.latex_groups.is_empty());
        assert!(a.command_key(egui::Key::Backspace, egui::Modifiers::NONE));
        assert!(a.command_key(egui::Key::Tab, egui::Modifiers::NONE));
        assert_eq!(
            a.command_input.as_ref().map(|i| i.name.as_str()),
            Some("mathbb")
        );
        for ch in "{R}".chars() {
            type_canvas(&mut a, &ch.to_string());
        }
        assert!(a.command_key(egui::Key::Enter, egui::Modifiers::NONE));
        assert!(a.command_input.is_none());
        assert!(a.editor.math().to_latex().contains(r"\mathbb{R}"));

        // \int_0^1 x dx stays buffered across '_', '^', and spaces until Enter commits once.
        let mut a = app();
        for ch in r"\int_0^1 x dx".chars() {
            type_canvas(&mut a, &ch.to_string());
        }
        assert_eq!(
            a.command_input.as_ref().map(|i| i.name.as_str()),
            Some("int_0^1 x dx")
        );
        assert!(a.command_key(egui::Key::Enter, egui::Modifiers::NONE));
        assert!(a.command_input.is_none());
        let xml = a.editor.math().to_xml();
        assert!(
            xml.contains('∫') && (xml.contains("msubsup") || xml.contains("munderover")),
            "{xml}"
        );
        a.editor.undo();
        assert!(a.editor.math().children.is_empty());
    }

    #[test]
    fn completion_buffer_supports_cursor_navigation_and_mid_string_editing() {
        let mut a = app();
        type_canvas(&mut a, r"\frac{1}{2}");
        assert!(a.command_key(egui::Key::ArrowLeft, egui::Modifiers::NONE));
        assert!(a.command_key(egui::Key::ArrowLeft, egui::Modifiers::NONE));
        assert!(a.command_key(egui::Key::ArrowLeft, egui::Modifiers::NONE));
        assert!(a.command_key(egui::Key::ArrowLeft, egui::Modifiers::NONE));
        type_canvas(&mut a, "0");
        assert_eq!(
            a.command_input.as_ref().map(|i| i.name.as_str()),
            Some("frac{10}{2}")
        );
        assert!(a.command_key(egui::Key::Backspace, egui::Modifiers::NONE));
        assert_eq!(
            a.command_input.as_ref().map(|i| i.name.as_str()),
            Some("frac{1}{2}")
        );

        assert!(a.command_key(egui::Key::Home, egui::Modifiers::NONE));
        assert!(a.command_key(egui::Key::Backspace, egui::Modifiers::NONE));
        assert_eq!(
            a.command_input.as_ref().map(|i| i.name.as_str()),
            Some("frac{1}{2}")
        );
        assert!(a.command_key(egui::Key::Delete, egui::Modifiers::NONE));
        type_canvas(&mut a, "d");
        assert!(a.command_key(egui::Key::ArrowRight, egui::Modifiers::NONE));
        assert!(a.command_key(egui::Key::End, egui::Modifiers::NONE));
        type_canvas(&mut a, "+x");
        assert_eq!(
            a.command_input.as_ref().map(|i| i.name.as_str()),
            Some("drac{1}{2}+x")
        );
        assert!(a.command_key(egui::Key::Home, egui::Modifiers::NONE));
        assert!(a.command_key(egui::Key::Delete, egui::Modifiers::NONE));
        type_canvas(&mut a, "f");
        assert!(a.command_key(egui::Key::Enter, egui::Modifiers::NONE));
        assert!(a.command_input.is_none());
        assert_eq!(a.editor.math().to_latex(), r"\frac{1}{2}+x");
    }

    #[test]
    fn completion_popup_textbox_supports_arrow_navigation_and_editing() {
        let send_events = |a: &mut App, ctx: &egui::Context, events: Vec<egui::Event>| {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 650.0))),
                    events,
                    ..Default::default()
                },
                |ui| a.shell(ui),
            );
            out.textures_delta.clear();
            out
        };
        let key_event = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let canvas_id = egui::Id::unique("equation-canvas");
        let cmd_id = egui::Id::unique("canvas-command-input");

        let mut a = app();
        let ctx = egui::Context::default();
        ThemeTokens::apply_style(&ctx);
        send_events(&mut a, &ctx, vec![]);
        assert!(ctx.memory(|m| m.has_focus(canvas_id)));

        // Open popup with \frac{1}{2}, move left 4 times, insert 0 -> frac{10}{2}, backspace -> frac{1}{2}.
        send_events(&mut a, &ctx, vec![egui::Event::Text(r"\frac{1}{2}".into())]);
        assert!(ctx.memory(|m| m.has_focus(cmd_id)));
        for _ in 0..4 {
            send_events(&mut a, &ctx, vec![key_event(egui::Key::ArrowLeft)]);
        }
        send_events(&mut a, &ctx, vec![egui::Event::Text("0".into())]);
        assert_eq!(
            a.command_input.as_ref().map(|i| i.name.as_str()),
            Some("frac{10}{2}")
        );
        send_events(&mut a, &ctx, vec![key_event(egui::Key::Backspace)]);
        assert_eq!(
            a.command_input.as_ref().map(|i| i.name.as_str()),
            Some("frac{1}{2}")
        );

        // Home, Delete, insert 'd', End, insert '+x', Home, Delete, insert 'f', Enter -> \frac{1}{2}+x.
        send_events(&mut a, &ctx, vec![key_event(egui::Key::Home)]);
        send_events(&mut a, &ctx, vec![key_event(egui::Key::Delete)]);
        send_events(&mut a, &ctx, vec![egui::Event::Text("d".into())]);
        send_events(&mut a, &ctx, vec![key_event(egui::Key::End)]);
        send_events(&mut a, &ctx, vec![egui::Event::Text("+x".into())]);
        assert_eq!(
            a.command_input.as_ref().map(|i| i.name.as_str()),
            Some("drac{1}{2}+x")
        );
        send_events(&mut a, &ctx, vec![key_event(egui::Key::Home)]);
        send_events(&mut a, &ctx, vec![key_event(egui::Key::Delete)]);
        send_events(&mut a, &ctx, vec![egui::Event::Text("f".into())]);
        send_events(&mut a, &ctx, vec![key_event(egui::Key::Enter)]);
        assert!(a.command_input.is_none());
        assert_eq!(a.editor.math().to_latex(), r"\frac{1}{2}+x");
        assert!(ctx.memory(|m| m.has_focus(canvas_id)));

        // Bare command name ignores Space; Backspace on empty closes popup and restores canvas focus.
        send_events(&mut a, &ctx, vec![egui::Event::Text(r"\text".into())]);
        send_events(
            &mut a,
            &ctx,
            vec![key_event(egui::Key::Space), egui::Event::Text(" ".into())],
        );
        assert_eq!(
            a.command_input.as_ref().map(|i| i.name.as_str()),
            Some("text")
        );
        send_events(&mut a, &ctx, vec![key_event(egui::Key::Escape)]);
        assert!(a.command_input.is_none());
        assert!(ctx.memory(|m| m.has_focus(canvas_id)));

        send_events(&mut a, &ctx, vec![egui::Event::Text(r"\".into())]);
        assert!(a.command_input.is_some());
        send_events(&mut a, &ctx, vec![key_event(egui::Key::Backspace)]);
        assert!(a.command_input.is_none());
        assert!(ctx.memory(|m| m.has_focus(canvas_id)));
    }

    #[test]
    fn completion_popup_textbox_pointer_click_positions_cursor() {
        let send_events = |a: &mut App, ctx: &egui::Context, events: Vec<egui::Event>| {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 650.0))),
                    events,
                    ..Default::default()
                },
                |ui| a.shell(ui),
            );
            out.textures_delta.clear();
            out
        };
        let canvas_id = egui::Id::unique("equation-canvas");
        let cmd_id = egui::Id::unique("canvas-command-input");
        let latex_id = egui::Id::unique("latex-source");

        let mut a = app();
        let ctx = egui::Context::default();
        ThemeTokens::apply_style(&ctx);
        send_events(&mut a, &ctx, vec![]);
        send_events(&mut a, &ctx, vec![egui::Event::Text(r"\int_0^1".into())]);
        let out = send_events(&mut a, &ctx, vec![]);
        assert!(ctx.memory(|m| m.has_focus(cmd_id)));

        fn find_galley_click(shape: &egui::Shape) -> Option<Pos2> {
            match shape {
                egui::Shape::Text(t) if t.galley.text() == "int_0^1" => {
                    let cpos = t.galley.pos_from_cursor(egui::text::CCursor::new(4));
                    Some(t.pos + Vec2::new(cpos.min.x, cpos.center().y))
                }
                egui::Shape::Vec(shapes) => shapes.iter().find_map(find_galley_click),
                _ => None,
            }
        }

        // Find the rendered galley for "int_0^1" inside the popup textbox and click between '_' and '0'.
        let click_pos = out
            .shapes
            .iter()
            .find_map(|s| find_galley_click(&s.shape))
            .expect("popup command textbox galley must be visible");

        send_events(&mut a, &ctx, click_events(click_pos));
        assert!(a.command_input.is_some());
        assert!(ctx.memory(|m| m.has_focus(cmd_id)));
        send_events(&mut a, &ctx, vec![egui::Event::Text("a".into())]);
        assert_eq!(
            a.command_input.as_ref().map(|i| i.name.as_str()),
            Some("int_a0^1")
        );

        // Clicking outside onto the Equation Canvas closes the popup without committing.
        let canvas_rect = ctx.read_response(canvas_id).unwrap().rect;
        send_events(
            &mut a,
            &ctx,
            click_events(Pos2::new(
                canvas_rect.right() - 20.0,
                canvas_rect.bottom() - 20.0,
            )),
        );
        assert!(a.command_input.is_none());
        assert!(a.editor.math().children.is_empty());

        // Clicking into LaTeX source while popup is open also dismisses the popup.
        send_events(&mut a, &ctx, vec![egui::Event::Text(r"\alp".into())]);
        assert!(a.command_input.is_some());
        let latex_rect = ctx.read_response(latex_id).unwrap().rect;
        send_events(&mut a, &ctx, click_events(latex_rect.center()));
        assert!(a.command_input.is_none());
        assert!(ctx.memory(|m| m.has_focus(latex_id)));
    }

    #[test]
    fn completion_popup_accepts_mouse_selection() {
        let mut a = app();
        type_canvas(&mut a, r"\al");
        let ctx = egui::Context::default();
        let mut choice = None;
        for _ in 0..2 {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 650.0))),
                    ..Default::default()
                },
                |ui| a.command_popup(ui, Pos2::new(100.0, 100.0)),
            );
            choice = output.shapes.iter().find_map(|s| {
                if let egui::Shape::Text(t) = &s.shape {
                    if t.galley.text().starts_with(r"\alpha ") {
                        return Some(s.shape.visual_bounding_rect().center());
                    }
                }
                None
            });
            output.textures_delta.clear();
        }
        let pos = choice.expect("completion item must be visible");
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 650.0))),
                events: vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                ..Default::default()
            },
            |ui| a.command_popup(ui, Pos2::new(100.0, 100.0)),
        )
        .textures_delta
        .clear();
        assert!(a.command_input.is_none());
        assert!(a.editor.math().to_xml().contains('α'));
        assert_eq!(a.latex, a.editor.math().to_latex());
    }

    #[test]
    fn argument_requiring_commands_block_bare_or_empty_enter_and_show_status_error() {
        use egui::Key;

        // Prefix auto-completes on Enter without committing and shows error in the bottom status bar.
        let mut a = app();
        let ctx = egui::Context::default();
        ThemeTokens::apply_style(&ctx);
        let setup_focused = |a: &mut App, ctx: &egui::Context| {
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 650.0))),
                    ..Default::default()
                },
                |ui| {
                    a.shell(ui);
                    ui.memory_mut(|m| m.request_focus(egui::Id::unique("equation-canvas")));
                },
            )
            .textures_delta
            .clear();
        };
        let send_events = |a: &mut App, ctx: &egui::Context, events: Vec<egui::Event>| {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 650.0))),
                    events,
                    ..Default::default()
                },
                |ui| a.shell(ui),
            );
            out.textures_delta.clear();
            out
        };
        let key_event = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let has_shape_text = |out: &egui::FullOutput, needle: &str| -> bool {
            fn visit(shape: &egui::Shape, needle: &str) -> bool {
                match shape {
                    egui::Shape::Text(t) => t.galley.text().contains(needle),
                    egui::Shape::Vec(shapes) => shapes.iter().any(|s| visit(s, needle)),
                    _ => false,
                }
            }
            out.shapes.iter().any(|s| visit(&s.shape, needle))
        };

        let popup_has_text = |a: &mut App, needle: &str| -> bool {
            let pctx = egui::Context::default();
            let mut found = false;
            for _ in 0..2 {
                let mut pout = pctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            Pos2::ZERO,
                            Vec2::new(1100.0, 650.0),
                        )),
                        ..Default::default()
                    },
                    |ui| a.command_popup(ui, Pos2::new(100.0, 100.0)),
                );
                found = has_shape_text(&pout, needle);
                pout.textures_delta.clear();
            }
            found
        };

        setup_focused(&mut a, &ctx);
        send_events(&mut a, &ctx, vec![egui::Event::Text(r"\math".into())]);
        send_events(&mut a, &ctx, vec![key_event(Key::Enter)]);
        let out = send_events(&mut a, &ctx, Vec::new());
        assert_eq!(
            a.command_input.as_ref().map(|i| i.name.as_str()),
            Some("mathbb")
        );
        assert!(a.editor.math().children.is_empty());
        assert!(a.status.contains("Invalid LaTeX command"));
        assert!(a.status.contains(r"\mathbb{...}"));
        assert!(a.command_error.as_deref() == Some(a.status.as_str()));
        assert!(has_shape_text(&out, "LaTeX error: Invalid LaTeX command"));
        assert!(has_shape_text(&out, r"\mathbb{...}"));
        assert!(!popup_has_text(&mut a, "Invalid LaTeX command"));

        // Editing clears command_error, and valid {R} + Enter commits cleanly.
        send_events(&mut a, &ctx, vec![egui::Event::Text("{R}".into())]);
        assert!(a.command_error.is_none());
        assert!(!popup_has_text(&mut a, "Invalid LaTeX command"));
        send_events(&mut a, &ctx, vec![key_event(Key::Enter)]);
        assert!(a.command_input.is_none());
        assert!(a.editor.math().to_latex().contains(r"\mathbb{R}"));

        // Bare and empty forms for all 14 argument-requiring commands block Enter.
        for bare in [
            r"\text",
            r"\mathbb",
            r"\mathfrak",
            r"\mathcal",
            r"\mathscr",
            r"\mathrm",
            r"\mathbf",
            r"\mathit",
            r"\boldsymbol",
            r"\operatorname",
            r"\hspace",
            r"\textstyle",
            r"\displaystyle",
            r"\enclose",
            r"\text{}",
            r"\mathbb{}",
            r"\mathbb{ }",
            r"\operatorname{}",
            r"\enclose{box}",
            r"\enclose{box}{}",
        ] {
            let mut b = app();
            type_canvas(&mut b, bare);
            let before = b.editor.math();
            b.command_key(Key::Enter, egui::Modifiers::NONE);
            assert_eq!(b.editor.math(), before, "should not commit `{bare}`");
            assert!(
                b.command_input.is_some(),
                "popup should stay open for `{bare}`"
            );
            assert!(
                b.status.contains("Invalid LaTeX command"),
                "unexpected status for `{bare}`: {}",
                b.status
            );
        }

        // \text{ } remains valid and commits a space-only mtext node.
        let mut space_text = app();
        type_canvas(&mut space_text, r"\text{ }");
        assert!(space_text.command_key(Key::Enter, egui::Modifiers::NONE));
        assert!(space_text.command_input.is_none());
        assert_eq!(space_text.editor.math().find_all("mtext")[0].text, " ");

        // Structural templates still support bare-Enter placeholder boxes.
        for structural in [r"\frac", r"\sqrt", r"\boxed", r"\overset", r"\pmatrix"] {
            let mut s = app();
            type_canvas(&mut s, structural);
            assert!(s.command_key(Key::Enter, egui::Modifiers::NONE));
            assert!(
                s.command_input.is_none(),
                "structural `{structural}` should commit"
            );
            assert!(!s.editor.math().children.is_empty(), "{structural}");
        }
    }

    #[test]
    fn specialized_alphabet_validation_and_general_style_modifiers_in_command_popup() {
        use egui::Key;

        for invalid in [
            r"\mathbb{1^2}",
            r"\mathbb{1}",
            r"\mathbb{a}",
            r"\mathcal{1}",
            r"\mathcal{a}",
            r"\mathscr{1}",
            r"\mathfrak{1+2}",
        ] {
            let mut a = app();
            type_canvas(&mut a, invalid);
            a.command_key(Key::Enter, egui::Modifiers::NONE);
            assert!(a.editor.math().children.is_empty(), "{invalid}");
            assert_eq!(
                a.command_input.as_ref().map(|i| i.name.as_str()),
                Some(&invalid[1..])
            );
            assert!(
                a.status.contains("Invalid LaTeX command"),
                "{invalid}: {}",
                a.status
            );
            assert!(a.status.contains("^2"), "{invalid}: {}", a.status);
            assert_eq!(a.command_error.as_deref(), Some(a.status.as_str()));

            // Verify Equation Canvas renders the exact same status bar error prefix and color as LaTeX source.
            let mut src_app = app();
            src_app.latex = invalid.into();
            src_app.source_changed(Instant::now() - Duration::from_secs(1));
            src_app.request_latex_parse(&egui::Context::default());
            let parsed_res = src_app.rx.recv_timeout(Duration::from_secs(2)).unwrap();
            src_app.tx.send(parsed_res).unwrap();
            src_app.events(&egui::Context::default());
            assert_eq!(a.command_error, src_app.latex_error, "{invalid}");

            let rctx = egui::Context::default();
            let mut canvas_out = rctx.run_ui(egui::RawInput::default(), |ui| a.status_bar(ui));
            canvas_out.textures_delta.clear();
            let mut source_out =
                rctx.run_ui(egui::RawInput::default(), |ui| src_app.status_bar(ui));
            source_out.textures_delta.clear();
            let status_text_and_color = |out: &egui::FullOutput| {
                out.shapes.iter().find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.text().contains("LaTeX error:") => {
                        let color = t
                            .galley
                            .job
                            .sections
                            .first()
                            .map(|sec| sec.format.color)
                            .unwrap_or(t.override_text_color.unwrap_or_default());
                        Some((t.galley.text().to_owned(), color))
                    }
                    _ => None,
                })
            };
            assert_eq!(
                status_text_and_color(&canvas_out),
                status_text_and_color(&source_out),
                "status bar output must match between canvas and latex box for `{invalid}`"
            );
        }

        // General style modifiers accept compound expressions and export to OMML and MTEF.
        for valid in [
            r"\mathbf{x^2}",
            r"\mathrm{H_2O}",
            r"\mathrm{m/s^2}",
            r"\mathit{AB}",
            r"\boldsymbol{\alpha}",
        ] {
            let mut a = app();
            type_canvas(&mut a, valid);
            assert!(a.command_key(Key::Enter, egui::Modifiers::NONE));
            assert!(a.command_input.is_none(), "{valid}");
            let math = a.editor.math();
            assert!(!math.children.is_empty(), "{valid}");
            assert!(
                eqntide::mathml_to_omml_xml(&math)
                    .unwrap()
                    .contains("oMath"),
                "omml failed for `{valid}`"
            );
            assert!(
                !eqntide::mathml_to_mtef_bytes(&math).unwrap().is_empty(),
                "mtef failed for `{valid}`"
            );
        }
    }

    #[test]
    fn standardized_latex_roundtrip_on_desktop_editor() {
        let mut a = app();
        assert!(a.load_math("<math><mi>A</mi><mo>\\</mo><mi>B</mi></math>", None, None));
        assert_eq!(a.latex, r"A \setminus B");

        let mut b = app();
        type_canvas(&mut b, r"90\degree");
        assert!(b.command_key(egui::Key::Enter, egui::Modifiers::NONE));
        assert_eq!(b.latex, r"90^\circ");
    }

    #[test]
    fn canvas_style_and_argument_commands_serialize_without_splitting_tokens() {
        for (typed, expected_latex) in [
            (r"\mathbf{x^2 + 2x - 1 = 0}", r"\mathbf{x^{2}+2x-1=0}"),
            (
                r"\boldsymbol{x^2 + 2x - 1 = 0}",
                r"\boldsymbol{x^{2}+2x-1=0}",
            ),
            (r"\mathrm{H_2O + m/s^2}", r"\mathrm{H_{2}O+m/s^{2}}"),
            (r"\mathit{x^2 + 2x}", r"\mathit{x^{2}+2x}"),
            (r"\mathbb{RZ}", r"\mathbb{RZ}"),
            (r"\mathfrak{Ag}", r"\mathfrak{Ag}"),
            (r"\mathcal{AL}", r"\mathcal{AL}"),
            (r"\mathscr{Aa}", r"\mathscr{Aa}"),
            (r"\text{hello world}", r"\text{hello world}"),
            (r"\operatorname{sgn}", r"\operatorname{sgn}"),
            (r"\textstyle{\frac{a}{b}}", r"{\textstyle \frac{a}{b}}"),
            (
                r"\displaystyle{\frac{a}{b}}",
                r"{\displaystyle \frac{a}{b}}",
            ),
            (r"\enclose{roundedbox}{x+y}", r"\enclose{roundedbox}{x+y}"),
            (r"\hspace{12pt}", r"\hspace{12pt}"),
        ] {
            let mut a = app();
            type_canvas(&mut a, typed);
            assert!(
                a.command_key(egui::Key::Enter, egui::Modifiers::NONE),
                "{typed}"
            );
            assert!(a.command_input.is_none(), "{typed}");
            assert_eq!(a.latex, expected_latex, "typed `{typed}`");
            assert!(
                commands::validate_desktop_latex(&a.latex).is_ok(),
                "invalid serialized latex for `{typed}`: {}",
                a.latex
            );
            assert!(
                eqntide::service::parse_latex_to_mathml(&a.latex).is_ok(),
                "re-parse failed for `{typed}`: {}",
                a.latex
            );
        }
    }

    #[test]
    fn command_template_wraps_selection_without_reparsing_existing_math() {
        let mut a = app();
        a.load_math("<math><mi mathvariant=\"bold\">x</mi></math>", None, None);
        let before = a.editor.math();
        a.editor.select_all();
        type_canvas(&mut a, r"\frac");
        a.command_key(egui::Key::Enter, egui::Modifiers::NONE);
        let xml = a.editor.math().to_xml();
        assert!(
            xml.contains("mfrac") && xml.contains("mathvariant=\"bold\""),
            "{xml}"
        );
        a.editor.undo();
        assert_eq!(a.editor.math(), before);
    }

    #[test]
    fn save_shortcut_is_global_and_consumed_once_without_stealing_sum() {
        for focus in [None, Some("equation-canvas"), Some("latex-source")] {
            let ctx = egui::Context::default();
            ctx.begin_pass(egui::RawInput {
                events: vec![save_key(false), save_key(false), save_key(true)],
                ..Default::default()
            });
            if let Some(id) = focus {
                ctx.memory_mut(|m| m.request_focus(egui::Id::unique(id)));
            }
            assert!(App::take_save_shortcut(&ctx));
            assert!(!App::take_save_shortcut(&ctx));
            assert_eq!(ctx.input(|i| i.events.len()), 1);
            assert!(ctx.input(|i| matches!(&i.events[0], egui::Event::Key {
                key: egui::Key::S, modifiers, ..
            } if modifiers.shift && modifiers.command)));
            ctx.end_pass().textures_delta.clear();
        }
    }
    #[test]
    fn save_shortcut_with_a_latex_draft_keeps_the_last_valid_canvas() {
        let mut a = app();
        a.load_math("<math><mi>x</mi></math>", None, None);
        let before = a.editor.math();
        let ctx = egui::Context::default();
        // Focus the real source widget, then type and save in the same frame.
        ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.memory_mut(|m| m.request_focus(egui::Id::unique("latex-source")));
            a.editor_ui(ui);
        })
        .textures_delta
        .clear();
        ctx.run_ui(
            egui::RawInput {
                events: vec![egui::Event::Text("y".into()), save_key(false)],
                ..Default::default()
            },
            |ui| {
                let save = App::take_save_shortcut(&ctx);
                a.editor_ui(ui);
                if save && a.save_available() {
                    a.finish(true);
                }
            },
        )
        .textures_delta
        .clear();
        assert_eq!(a.editor.math(), before);
        assert!(a.latex.contains('y'));
        assert!(a.draft_dirty);
        assert!(!a.busy);
        assert!(a.status.contains("valid LaTeX"));
    }
    #[test]
    fn fixed_layout_keeps_save_and_source_visible_across_sizes_and_themes() {
        for office in [false, true] {
            for size in [
                Vec2::from(MIN_WINDOW_SIZE),
                Vec2::new(1100.0, 650.0),
                Vec2::new(1440.0, 900.0),
            ] {
                for theme in [egui::Theme::Light, egui::Theme::Dark] {
                    let mut a = app();
                    a.bridge_ready = true;
                    a.latex = "source-marker".into();
                    if office {
                        a.session = Some("office".into());
                    }
                    let ctx = egui::Context::default();
                    ThemeTokens::apply_style(&ctx);
                    ctx.set_theme(theme);
                    let screen = Rect::from_min_size(Pos2::ZERO, size);
                    // Run twice to cover initial panel sizing and the settled layout.
                    for _ in 0..2 {
                        let mut output = ctx.run_ui(
                            egui::RawInput {
                                screen_rect: Some(screen),
                                ..Default::default()
                            },
                            |ui| a.shell(ui),
                        );
                        output.textures_delta.clear();
                        let label = if office {
                            "Save & Return to Document (Ctrl+S)"
                        } else {
                            "Save MathML (Ctrl+S)"
                        };
                        for text in [label, "source-marker", "MathType", "OMML", "Font size"] {
                            let shape = output.shapes.iter().find(|shape| {
                                matches!(&shape.shape, egui::Shape::Text(t) if t.galley.text() == text)
                            }).unwrap_or_else(|| panic!("missing {text} at {size:?}"));
                            let rect = shape.shape.visual_bounding_rect();
                            if text != "source-marker" {
                                let save = output
                                    .shapes
                                    .iter()
                                    .find(|s| {
                                        matches!(&s.shape,
                                    egui::Shape::Text(t) if t.galley.text() == label)
                                    })
                                    .unwrap();
                                assert!(
                                    (rect.center().y
                                        - save.shape.visual_bounding_rect().center().y)
                                        .abs()
                                        <= 3.0,
                                    "footer control {text} must share the save row"
                                );
                            }
                            if text == "source-marker" {
                                let pal = a.active_canvas_palette(theme == egui::Theme::Dark);
                                for painted in &output.shapes {
                                    if let egui::Shape::Rect(frame) = &painted.shape {
                                        if frame.fill == pal.bg
                                            && frame.stroke.color == pal.border
                                            && frame.corner_radius == theme::SURFACE_CORNER_RADIUS
                                            && frame.rect.width() > 100.0
                                        {
                                            assert!(
                                                (frame.rect.top()
                                                    - CONTENT_PADDING as f32
                                                    - PALETTE_HEIGHT)
                                                    .abs()
                                                    <= 1.0,
                                                "palette must reserve three rows at every size"
                                            );
                                            let source = ctx
                                                .read_response(egui::Id::unique("latex-source"))
                                                .unwrap()
                                                .rect;
                                            assert!(
                                                (frame.rect.left() - source.left()).abs() <= 1.0,
                                                "left edges: canvas {:?}, source {:?}",
                                                frame.rect,
                                                source
                                            );
                                            assert!(
                                                (frame.rect.right() - source.right()).abs() <= 1.0,
                                                "right edges: canvas {:?}, source {:?}",
                                                frame.rect,
                                                source
                                            );
                                            assert!(
                                                frame.rect.bottom() <= rect.top(),
                                                "canvas {:?} covers source {:?} at {:?}",
                                                frame.rect,
                                                rect,
                                                size
                                            );
                                        }
                                    }
                                }
                            }
                            assert!(screen.contains_rect(rect), "{text}: {rect:?} at {size:?}");
                            assert!(
                                shape.clip_rect.contains_rect(rect),
                                "{text} clipped: {rect:?} by {:?}",
                                shape.clip_rect
                            );
                        }
                        let save_center_y = find_text_center(&output, |t| t == label).y;
                        for icon_name in ["arrow-back-up", "arrow-forward-up"] {
                            let tex_id = a
                                .ui_icon_texture_id(icon_name)
                                .unwrap_or_else(|| panic!("missing footer icon {icon_name}"));
                            let icon_rect = output
                                .shapes
                                .iter()
                                .find_map(|s| match &s.shape {
                                    egui::Shape::Rect(r)
                                        if r.brush
                                            .as_ref()
                                            .is_some_and(|b| b.fill_texture_id == tex_id) =>
                                    {
                                        Some(r.rect)
                                    }
                                    _ => None,
                                })
                                .unwrap_or_else(|| {
                                    panic!("missing painted footer icon {icon_name}")
                                });
                            assert!(
                                (icon_rect.center().y - save_center_y).abs() <= 3.0,
                                "footer icon {icon_name} must share the save row"
                            );
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn symbol_buttons_use_at_most_three_rows_even_during_startup_resize() {
        for dpi in [1.0, 1.25, 1.5, 2.0] {
            let ctx = egui::Context::default();
            ctx.set_pixels_per_point(dpi);
            let mut a = app();
            a.category = "Symbols".into();
            for width in [960.0, MIN_WINDOW_SIZE[0], 1100.0] {
                for _ in 0..2 {
                    let mut output = ctx.run_ui(
                        egui::RawInput {
                            screen_rect: Some(Rect::from_min_size(
                                Pos2::ZERO,
                                Vec2::new(width, 650.0),
                            )),
                            ..Default::default()
                        },
                        |ui| a.shell(ui),
                    );
                    output.textures_delta.clear();
                    let textures = a.palette_texture_ids();
                    let images: Vec<_> = output
                        .shapes
                        .iter()
                        .filter_map(|shape| {
                            if let egui::Shape::Rect(r) = &shape.shape {
                                if r.brush
                                    .as_ref()
                                    .is_some_and(|b| textures.contains(&b.fill_texture_id))
                                {
                                    return Some(r.rect);
                                }
                            }
                            None
                        })
                        .collect();
                    let mut rows: Vec<f32> = vec![];
                    for image in &images {
                        if !rows.iter().any(|y| (y - image.center().y).abs() < 2.0) {
                            rows.push(image.center().y);
                        }
                    }
                    assert!(
                        rows.len() <= 3,
                        "{} rows at width {width}, DPI {dpi}",
                        rows.len()
                    );
                    if width >= MIN_WINDOW_SIZE[0] {
                        let count = palette::PALETTE
                            .iter()
                            .filter(|(g, _, _)| *g == "Symbols")
                            .count();
                        assert_eq!(
                            images.len(),
                            count,
                            "every symbol must fit at the minimum width"
                        );
                        assert!(images.iter().all(|r| output.shapes.iter().any(|s|
                            matches!(&s.shape, egui::Shape::Rect(p) if p.rect == *r && s.clip_rect.contains_rect(*r)))));
                    }
                }
            }
        }
    }

    #[test]
    fn every_category_keeps_three_palette_rows_at_compact_sizes() {
        for dpi in [1.0, 1.25, 1.5] {
            let ctx = egui::Context::default();
            ctx.set_pixels_per_point(dpi);
            let mut a = app();
            for group in palette::UI_GROUPS {
                if group.id == palette::RECENT_GROUP {
                    continue;
                }
                a.category = group.id.into();
                for width in [960.0, MIN_WINDOW_SIZE[0]] {
                    for _ in 0..2 {
                        let mut output = ctx.run_ui(
                            egui::RawInput {
                                screen_rect: Some(Rect::from_min_size(
                                    Pos2::ZERO,
                                    Vec2::new(width, 560.0),
                                )),
                                ..Default::default()
                            },
                            |ui| a.shell(ui),
                        );
                        output.textures_delta.clear();
                        let textures = a.palette_texture_ids();
                        let mut rows = Vec::<f32>::new();
                        for shape in &output.shapes {
                            if let egui::Shape::Rect(r) = &shape.shape {
                                if r.brush
                                    .as_ref()
                                    .is_some_and(|b| textures.contains(&b.fill_texture_id))
                                    && !rows.iter().any(|y| (y - r.rect.center().y).abs() < 2.0)
                                {
                                    rows.push(r.rect.center().y);
                                }
                            }
                        }
                        assert!(
                            !rows.is_empty(),
                            "{} must contain rendered buttons",
                            group.id
                        );
                        assert!(
                            rows.len() <= 3,
                            "{}: {} rows at width {width}, DPI {dpi}",
                            group.id,
                            rows.len()
                        );
                        let batch = output
                            .shapes
                            .iter()
                            .find(|s| {
                                matches!(&s.shape,
                            egui::Shape::Text(t) if t.galley.text() == "Batch DOCX")
                            })
                            .unwrap();
                        assert!(
                            batch
                                .clip_rect
                                .contains_rect(batch.shape.visual_bounding_rect()),
                            "{}",
                            group.id
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn all_sidebar_categories_and_batch_stay_visible_without_scrolling() {
        for dpi in [1.0, 1.25, 1.5] {
            let ctx = egui::Context::default();
            ctx.set_pixels_per_point(dpi);
            let mut a = app();
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::from(MIN_WINDOW_SIZE))),
                    ..Default::default()
                },
                |ui| a.shell(ui),
            );
            output.textures_delta.clear();
            let sidebar_tex_ids: std::collections::HashSet<_> = palette::UI_GROUPS
                .iter()
                .flat_map(|g| g.icons.iter().copied())
                .map(|icon_name| {
                    a.ui_icon_texture_id(icon_name)
                        .unwrap_or_else(|| panic!("missing sidebar icon {icon_name}"))
                })
                .collect();
            let expected_icon_count: usize = palette::UI_GROUPS.iter().map(|g| g.icons.len()).sum();
            let icon_rects: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|shape| {
                    if let egui::Shape::Rect(r) = &shape.shape {
                        if r.brush
                            .as_ref()
                            .is_some_and(|b| sidebar_tex_ids.contains(&b.fill_texture_id))
                        {
                            return Some((r.rect, shape.clip_rect));
                        }
                    }
                    None
                })
                .collect();
            assert_eq!(
                icon_rects.len(),
                expected_icon_count,
                "all {expected_icon_count} category icons must render at DPI {dpi}"
            );
            for (rect, clip) in icon_rects {
                assert!(
                    clip.contains_rect(rect),
                    "category icon {rect:?} clipped by {clip:?} at DPI {dpi}"
                );
            }
            for nav in ["Batch DOCX", "Settings", "About"] {
                let (rect, clip) = output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Text(t) if t.galley.text() == nav => {
                            Some((shape.shape.visual_bounding_rect(), shape.clip_rect))
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| panic!("missing sidebar button {nav}"));
                assert!(
                    clip.contains_rect(rect),
                    "{nav} clipped at minimum window size"
                );
            }
        }
    }

    #[test]
    fn custom_matrix_dialog_cancels_without_changes_and_inserts_one_transaction() {
        let ctx = egui::Context::default();
        let mut a = app();
        a.editor.insert_text("x");
        a.changed();
        let before = a.editor.math();
        a.matrix_dialog = Some(MatrixDialog::default());
        assert!(!a.save_available());
        for _ in 0..2 {
            ctx.run_ui(egui::RawInput::default(), |ui| a.matrix_dialog_ui(ui.ctx()))
                .textures_delta
                .clear();
        }
        ctx.run_ui(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..Default::default()
            },
            |ui| a.matrix_dialog_ui(ui.ctx()),
        )
        .textures_delta
        .clear();
        assert!(a.matrix_dialog.is_none());
        assert_eq!(a.editor.math(), before);
        assert!(a.save_available());
        a.insert_custom_matrix(&MatrixDialog {
            rows: 3,
            columns: 4,
            alignment: "right",
            fence: "brackets",
        });
        let math = a.editor.math();
        let table = math.iter().into_iter().find(|n| n.tag == "mtable").unwrap();
        assert_eq!(table.children.len(), 3);
        assert!(table.children.iter().all(|r| r.children.len() == 4));
        assert_eq!(table.attr("columnalign"), Some("right"));
        assert_eq!(a.latex, math.to_latex());
        a.editor.undo();
        assert_eq!(a.editor.math(), before);
        a.editor.redo();
        assert_eq!(a.editor.math(), math);

        for (fence, open_ch) in [("left-brace", "{"), ("left-bracket", "[")] {
            let mut b = app();
            b.matrix_dialog = Some(MatrixDialog {
                rows: 4,
                columns: 1,
                alignment: "left",
                fence,
            });
            for _ in 0..2 {
                ctx.run_ui(egui::RawInput::default(), |ui| b.matrix_dialog_ui(ui.ctx()))
                    .textures_delta
                    .clear();
            }
            assert!(
                b.matrix_dialog.is_some(),
                "dialog should remain open before clicking Insert matrix"
            );
            b.insert_custom_matrix(&MatrixDialog {
                rows: 4,
                columns: 1,
                alignment: "left",
                fence,
            });
            let sys = b.editor.math();
            let row = &sys.children[0].children;
            assert_eq!(row[0].text, open_ch);
            assert_eq!(row[1].tag, "mtable");
            assert_eq!(row[1].children.len(), 4);
            assert!(row[1].children.iter().all(|r| r.children.len() == 1));
            assert_eq!(row[1].attr("columnalign"), Some("left"));
            assert_eq!(row[2].text, "");
            assert_eq!(b.latex, sys.to_latex());
        }
    }

    #[test]
    fn enter_on_canvas_inserts_or_promotes_system_rows_and_preserves_command_commit() {
        let ctx = egui::Context::default();
        let enter_input = |modifiers: egui::Modifiers| egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            }],
            ..Default::default()
        };

        // 1. Inside a single-slot left brace `{□`, Enter promotes to a 2x1 left-aligned system.
        let mut a = app();
        a.insert_template(r"\left\{ #? \right.");
        a.canvas_text("x+2=1");
        a.changed();
        let before_promote = a.editor.math();
        ctx.run_ui(enter_input(egui::Modifiers::NONE), |ui| a.keyboard(ui))
            .textures_delta
            .clear();
        let promoted = a.editor.math();
        let table = promoted
            .iter()
            .into_iter()
            .find(|n| n.tag == "mtable")
            .unwrap();
        assert_eq!(table.children.len(), 2);
        assert_eq!(table.attr("columnalign"), Some("left"));
        assert_eq!(a.latex, promoted.to_latex());

        // 2. Pressing Enter inside row 2 inserts row 3 right below and moves the caret there.
        a.canvas_text("y=3");
        a.changed();
        ctx.run_ui(enter_input(egui::Modifiers::NONE), |ui| a.keyboard(ui))
            .textures_delta
            .clear();
        let grown = a.editor.math();
        let table = grown
            .iter()
            .into_iter()
            .find(|n| n.tag == "mtable")
            .unwrap();
        assert_eq!(table.children.len(), 3);
        assert_eq!(a.latex, grown.to_latex());

        // Undo steps back cleanly.
        a.editor.undo();
        assert_eq!(
            a.editor
                .math()
                .iter()
                .into_iter()
                .find(|n| n.tag == "mtable")
                .unwrap()
                .children
                .len(),
            2
        );
        // Undo typing `y=3`, then undo promotion.
        a.editor.undo();
        a.editor.undo();
        a.editor.undo();
        a.editor.undo();
        assert_eq!(a.editor.math(), before_promote);

        // 3. Active `\` command buffer still commits on Enter without adding a table row.
        let mut b = app();
        b.insert_template(r"\left\{ \begin{array}{l} #? \\ #? \end{array} \right.");
        b.canvas_text(r"\alpha");
        assert!(b.command_input.is_some());
        ctx.run_ui(enter_input(egui::Modifiers::NONE), |ui| b.keyboard(ui))
            .textures_delta
            .clear();
        assert!(b.command_input.is_none());
        let math_b = b.editor.math();
        assert_eq!(
            math_b
                .iter()
                .into_iter()
                .find(|n| n.tag == "mtable")
                .unwrap()
                .children
                .len(),
            2
        );
        assert!(math_b.to_xml().contains("α"));

        // 4. At the root row, Enter is a no-op.
        let mut c = app();
        c.canvas_text("x+1");
        c.changed();
        let before_root = c.editor.math();
        ctx.run_ui(enter_input(egui::Modifiers::NONE), |ui| c.keyboard(ui))
            .textures_delta
            .clear();
        assert_eq!(c.editor.math(), before_root);
    }

    #[test]
    fn backspace_and_delete_on_canvas_remove_empty_system_rows() {
        let ctx = egui::Context::default();
        let key_input = |key: egui::Key| egui::RawInput {
            events: vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };

        let mut a = app();
        a.insert_template(r"\left\{ \begin{array}{l} #? \\ #? \end{array} \right.");
        a.canvas_text("x=1");
        // Insert an extra empty row in the middle, then fill row 3.
        ctx.run_ui(key_input(egui::Key::Enter), |ui| a.keyboard(ui))
            .textures_delta
            .clear();
        a.editor
            .move_caret(eqntide_editor::Move::NextPlaceholder, false);
        a.canvas_text("y=2");
        a.changed();
        assert_eq!(
            a.editor
                .math()
                .iter()
                .into_iter()
                .find(|n| n.tag == "mtable")
                .unwrap()
                .children
                .len(),
            3
        );

        // Move caret into the empty middle row (row 1) and press Backspace.
        let row1_cell = a.editor.root().children[0].children[1].children[1].children[0].id;
        a.editor.set_caret(
            eqntide_editor::Caret {
                node: row1_cell,
                offset: 0,
            },
            false,
        );
        ctx.run_ui(key_input(egui::Key::Backspace), |ui| a.keyboard(ui))
            .textures_delta
            .clear();
        let after_bs = a.editor.math();
        assert_eq!(
            after_bs
                .iter()
                .into_iter()
                .find(|n| n.tag == "mtable")
                .unwrap()
                .children
                .len(),
            2
        );
        assert_eq!(a.latex, after_bs.to_latex());

        // Undo restores the empty middle row, then Delete removes it as well.
        a.editor.undo();
        a.changed();
        assert_eq!(
            a.editor
                .math()
                .iter()
                .into_iter()
                .find(|n| n.tag == "mtable")
                .unwrap()
                .children
                .len(),
            3
        );
        ctx.run_ui(key_input(egui::Key::Delete), |ui| a.keyboard(ui))
            .textures_delta
            .clear();
        let after_del = a.editor.math();
        assert_eq!(
            after_del
                .iter()
                .into_iter()
                .find(|n| n.tag == "mtable")
                .unwrap()
                .children
                .len(),
            2
        );
        assert_eq!(a.latex, after_del.to_latex());
    }

    #[test]
    fn canvas_arrow_navigation_a2_fraction_deletion_and_table_row_selection() {
        let ctx = egui::Context::default();
        let key_input = |key: egui::Key| egui::RawInput {
            events: vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };

        // 1) \frac{1}{} with caret in empty denominator: Backspace selects fraction, 2nd deletes it.
        let mut a = app();
        a.insert_template(r"\frac{#?}{#?}");
        a.canvas_text("1");
        a.editor
            .move_caret(eqntide_editor::Move::NextPlaceholder, false);
        a.changed();
        let frac_id = a.editor.root().children[0].id;
        ctx.run_ui(key_input(egui::Key::Backspace), |ui| a.keyboard(ui))
            .textures_delta
            .clear();
        assert_eq!(a.editor.selected_ids(), vec![frac_id]);
        assert_eq!(a.editor.math().children.len(), 1);
        ctx.run_ui(key_input(egui::Key::Backspace), |ui| a.keyboard(ui))
            .textures_delta
            .clear();
        assert!(a.editor.math().children.is_empty());
        assert_eq!(a.latex, "");

        // 2) Single-press ArrowRight traverses `\frac{1}{2}` and `(x)` without duplicate stalls.
        a.load_math(
            "<math><mfrac><mrow><mn>1</mn></mrow><mrow><mn>2</mn></mrow></mfrac><mrow><mo stretchy=\"true\" form=\"prefix\">(</mo><mrow><mi>x</mi></mrow><mo stretchy=\"true\" form=\"postfix\">)</mo></mrow></math>",
            None,
            None,
        );
        a.editor.move_caret(eqntide_editor::Move::Home, false);
        let num_mn = a.editor.root().children[0].children[0].children[0].id;
        let den_mn = a.editor.root().children[0].children[1].children[0].id;
        let fence_x = a.editor.root().children[1].children[1].children[0].id;
        let expected = [
            Caret {
                node: num_mn,
                offset: 0,
            },
            Caret {
                node: num_mn,
                offset: 1,
            },
            Caret {
                node: den_mn,
                offset: 0,
            },
            Caret {
                node: den_mn,
                offset: 1,
            },
            Caret {
                node: a.editor.root().id,
                offset: 1,
            },
            Caret {
                node: fence_x,
                offset: 0,
            },
            Caret {
                node: fence_x,
                offset: 1,
            },
            Caret {
                node: a.editor.root().id,
                offset: 2,
            },
        ];
        for caret in expected {
            ctx.run_ui(key_input(egui::Key::ArrowRight), |ui| a.keyboard(ui))
                .textures_delta
                .clear();
            assert_eq!(a.editor.caret(), caret);
        }

        // 3) Partially filled 2x2 matrix row selects row first, then deletes; drag-selecting an
        // empty 2x2 row removes the <mtr> cleanly and updates LaTeX.
        a.load_math(
            "<math><mtable><mtr><mtd><mi>x</mi></mtd><mtd><mi>y</mi></mtd></mtr><mtr><mtd/><mtd><mi>z</mi></mtd></mtr></mtable></math>",
            None,
            None,
        );
        let r1_c0 = a.editor.root().children[0].children[1].children[0].id;
        let r1_c1 = a.editor.root().children[0].children[1].children[1].id;
        a.editor.set_caret(
            Caret {
                node: r1_c0,
                offset: 0,
            },
            false,
        );
        ctx.run_ui(key_input(egui::Key::Backspace), |ui| a.keyboard(ui))
            .textures_delta
            .clear();
        assert_eq!(a.editor.selected_ids(), vec![r1_c0, r1_c1]);
        ctx.run_ui(key_input(egui::Key::Backspace), |ui| a.keyboard(ui))
            .textures_delta
            .clear();
        assert_eq!(a.editor.math().children[0].children.len(), 1);
        assert_eq!(a.latex, r"\begin{matrix} x & y \end{matrix}");

        a.load_math(
            "<math><mtable><mtr><mtd><mi>a</mi></mtd><mtd><mi>b</mi></mtd></mtr><mtr><mtd/><mtd/></mtr></mtable></math>",
            None,
            None,
        );
        let empty_c0 = a.editor.root().children[0].children[1].children[0].id;
        let empty_c1 = a.editor.root().children[0].children[1].children[1].id;
        a.editor.set_caret(
            Caret {
                node: empty_c0,
                offset: 0,
            },
            false,
        );
        a.editor.set_caret(
            Caret {
                node: empty_c1,
                offset: 0,
            },
            true,
        );
        ctx.run_ui(key_input(egui::Key::Delete), |ui| a.keyboard(ui))
            .textures_delta
            .clear();
        assert_eq!(a.editor.math().children[0].children.len(), 1);
        assert_eq!(a.latex, r"\begin{matrix} a & b \end{matrix}");
    }

    #[test]
    fn palette_accents_take_previous_expression_in_one_undo_step() {
        for source in [r"\hat{#?}", r"\underline{#?}", r"\boxed{#?}"] {
            let mut a = app();
            a.editor.insert_text("x");
            let before = a.editor.math();
            a.insert_template(source);
            let math = a.editor.math();
            assert_eq!(math.children.len(), 1, "{source}");
            assert!(math.to_xml().contains("<mi>x</mi>"));
            assert_eq!(a.latex, math.to_latex());
            a.editor.undo();
            assert_eq!(a.editor.math(), before, "{source}");
        }
    }

    #[test]
    fn startup_resize_corrects_undersized_native_and_wayland_viewports() {
        for wayland in [false, true] {
            for (size, maximized, monitor, expected) in [
                (
                    Vec2::new(960.0, 540.0),
                    false,
                    Vec2::new(1600.0, 900.0),
                    Some(Vec2::from(MIN_WINDOW_SIZE)),
                ),
                (
                    Vec2::new(1100.0, 650.0),
                    false,
                    Vec2::new(1600.0, 900.0),
                    None,
                ),
                (
                    Vec2::new(960.0, 540.0),
                    true,
                    Vec2::new(1600.0, 900.0),
                    None,
                ),
                (
                    Vec2::new(900.0, 600.0),
                    false,
                    Vec2::new(900.0, 600.0),
                    None,
                ),
            ] {
                let ctx = egui::Context::default();
                let rect = Rect::from_min_size(Pos2::ZERO, size);
                let mut input = egui::RawInput {
                    screen_rect: Some(rect),
                    ..Default::default()
                };
                let viewport = input.viewports.get_mut(&egui::ViewportId::ROOT).unwrap();
                viewport.inner_rect = (!wayland).then_some(rect);
                viewport.monitor_size = Some(monitor);
                viewport.maximized = Some(maximized);
                let mut output = ctx.run_ui(input, |ui| App::enforce_window_minimum(ui.ctx()));
                output.textures_delta.clear();
                let requested = output.viewport_output[&egui::ViewportId::ROOT]
                    .commands
                    .iter()
                    .find_map(|c| {
                        if let egui::ViewportCommand::InnerSize(size) = c {
                            Some(*size)
                        } else {
                            None
                        }
                    });
                assert_eq!(requested, expected, "Wayland fallback: {wayland}");
            }
        }
    }

    #[test]
    fn placeholder_decoration_uses_render_geometry_without_changing_equations() {
        for (xml, slots) in [
            ("<math/>", 1),
            ("<math><mfrac><mrow/><mrow/></mfrac></math>", 2),
            ("<math><msqrt><mrow/></msqrt></math>", 1),
            ("<math><mi>□</mi></math>", 0),
        ] {
            let mut a = app();
            a.load_math(xml, None, None);
            let before = a.editor.math();
            let ctx = egui::Context::default();
            a.request_render(&ctx);
            let Event::Render(_, result) = a.rx.recv_timeout(Duration::from_secs(5)).unwrap()
            else {
                panic!("expected equation render")
            };
            let rendered = result.unwrap();
            assert_eq!(rendered.placeholders.len(), slots, "{xml}");
            assert!(rendered
                .placeholders
                .iter()
                .all(|r| r.width > 0.0 && r.height > 0.0));
            assert_eq!(a.editor.math(), before);
            assert!(!a.editor.math().to_xml().contains(PLACEHOLDER_ATTR));
        }
    }

    #[test]
    fn canvas_does_not_underline_text_runs() {
        let mut a = app();
        let xml = r#"<math><mi>x</mi><mi mathvariant="normal">d</mi><mtext>với </mtext><mtext> </mtext><mtext/></math>"#;
        a.load_math(xml, None, None);
        let original = a.editor.math();
        let ctx = egui::Context::default();
        let mut color = Color32::TRANSPARENT;
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            color = a.active_canvas_palette(ui.visuals().dark_mode).text_caret;
            a.canvas(ui);
        });
        output.textures_delta.clear();
        let underlines = output.shapes.iter().filter(|shape| {
            matches!(&shape.shape, egui::Shape::LineSegment { stroke, .. } if stroke.color == color)
        });
        assert_eq!(underlines.count(), 0);
        assert_eq!(a.editor.math(), original);
    }

    #[test]
    fn canvas_scrolls_only_for_formula_overflow_across_dpi_changes() {
        for dpi in [1.0, 1.25, 1.5, 2.0] {
            let ctx = egui::Context::default();
            ctx.set_pixels_per_point(dpi);
            let mut a = app();
            let short = MathNode::with_children("math", vec![MathNode::leaf("mi", "x")]);
            let long =
                MathNode::with_children("math", vec![MathNode::leaf("mi", &"x".repeat(100))]);
            let mut tall = MathNode::leaf("mi", "x");
            for _ in 0..8 {
                tall = MathNode::with_children("mfrac", vec![tall, MathNode::leaf("mi", "x")]);
            }
            let tall = MathNode::with_children("math", vec![tall]);
            let mut frame = 0;
            // Returning to a short formula must remove overflow after either kind of scroll.
            for (math, horizontal, vertical) in [
                (&short, false, false),
                (&long, true, false),
                (&short, false, false),
                (&tall, false, true),
                (&short, false, false),
            ] {
                a.rendered = Some(Rendered {
                    placeholders: vec![],
                    image: egui::ColorImage::from_rgba_unmultiplied([1, 1], &[0, 0, 0, 255]),
                    preview: eqntide::render::render_interactive(math, a.font_pt).unwrap(),
                });
                for _ in 0..4 {
                    let mut scroll = None;
                    let mut output = ctx.run_ui(
                        egui::RawInput {
                            screen_rect: Some(Rect::from_min_size(
                                Pos2::ZERO,
                                Vec2::new(320.3, 200.7),
                            )),
                            time: Some(frame as f64 * 0.25),
                            ..Default::default()
                        },
                        |ui| {
                            scroll = Some(a.canvas(ui));
                        },
                    );
                    frame += 1;
                    output.textures_delta.clear();
                    let scroll = scroll.unwrap();
                    assert_eq!(
                        scroll.content_size.x > scroll.inner_rect.width(),
                        horizontal,
                        "horizontal overflow at DPI {dpi}: {:?} in {:?}",
                        scroll.content_size,
                        scroll.inner_rect
                    );
                    assert_eq!(
                        scroll.content_size.y > scroll.inner_rect.height(),
                        vertical,
                        "vertical overflow at DPI {dpi}: {:?} in {:?}",
                        scroll.content_size,
                        scroll.inner_rect
                    );
                    let pal = a.active_canvas_palette(ctx.theme() == egui::Theme::Dark);
                    let canvas = output
                        .shapes
                        .iter()
                        .find_map(|s| match &s.shape {
                            egui::Shape::Rect(r)
                                if r.fill == pal.bg && r.stroke.color == pal.border =>
                            {
                                Some(r)
                            }
                            _ => None,
                        })
                        .unwrap();
                    assert_eq!(canvas.corner_radius, theme::SURFACE_CORNER_RADIUS);
                }
            }
        }
    }

    #[test]
    fn invalid_source_keeps_white_canvas_and_reports_error_only_in_status() {
        let mut a = app();
        a.load_math("<math><mi>x</mi></math>", None, None);
        let before = a.editor.math();
        a.latex = "\\frac{1}".into();
        a.source_changed(Instant::now());
        a.latex_error = Some("incomplete fraction".into());
        let ctx = egui::Context::default();
        ctx.set_theme(egui::Theme::Dark);
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(940.0, 650.0))),
                ..Default::default()
            },
            |ui| a.shell(ui),
        );
        output.textures_delta.clear();
        let text_shapes: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|s| {
                if let egui::Shape::Text(t) = &s.shape {
                    Some(t)
                } else {
                    None
                }
            })
            .collect();
        assert!(!text_shapes
            .iter()
            .any(|t| t.galley.text() == "LaTeX" || t.galley.text() == a.status));
        let error = text_shapes
            .iter()
            .find(|t| t.galley.text().contains("incomplete fraction"))
            .unwrap();
        assert!(
            error.pos.y > 610.0,
            "error must be in the bottom status bar"
        );
        let pal = a.active_canvas_palette(true);
        assert!(output.shapes.iter().any(|s| matches!(&s.shape,
            egui::Shape::Rect(r) if r.fill == pal.bg && r.rect.width() > 200.0)));
        assert_eq!(a.editor.math(), before);
        assert!(a.draft_dirty);
    }

    #[test]
    fn sidebar_batch_and_back_preserve_unsaved_equation() {
        let mut a = app();
        a.editor.insert_text("x");
        a.changed();
        let before = a.editor.math();
        let source = a.latex.clone();
        let ctx = egui::Context::default();
        for (label, expected_tab) in [("Batch DOCX", 1), ("Back", 0)] {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| a.shell(ui));
            output.textures_delta.clear();
            let pos = output
                .shapes
                .iter()
                .find_map(|s| {
                    if let egui::Shape::Text(t) = &s.shape {
                        if t.galley.text() == label {
                            return Some(s.shape.visual_bounding_rect().center());
                        }
                    }
                    None
                })
                .unwrap();
            ctx.run_ui(
                egui::RawInput {
                    events: vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed: true,
                            modifiers: egui::Modifiers::NONE,
                        },
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed: false,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                    ..Default::default()
                },
                |ui| a.shell(ui),
            )
            .textures_delta
            .clear();
            assert_eq!(a.tab, expected_tab);
            assert_eq!(a.editor.math(), before);
            assert_eq!(a.latex, source);
            assert!(a.dirty());
        }
    }

    #[test]
    fn invalid_import_keeps_existing_work_and_source() {
        let mut a = app();
        a.editor.insert_text("x");
        a.changed();
        let before = a.editor.math();
        let source = a.latex.clone();
        assert!(!a.load_math("<math><bad>", None, None));
        assert_eq!(a.editor.math(), before);
        assert_eq!(a.latex, source);
    }
    #[test]
    fn session_failure_does_not_clear_dirty_work() {
        let mut a = app();
        a.editor.insert_text("x");
        a.session = Some("gone".into());
        a.busy = true;
        a.tx.send(Event::Finished(Err("Session expired".into())))
            .unwrap();
        a.events(&egui::Context::default());
        assert!(a.dirty());
        assert_eq!(a.session.as_deref(), Some("gone"));
        assert!(!a.busy);
        assert!(!a.allow_close);
    }
    #[test]
    fn a_latex_draft_prevents_accidental_save() {
        let mut a = app();
        a.draft_dirty = true;
        a.finish(true);
        assert!(!a.busy);
        assert!(a.status.contains("valid LaTeX"));
    }
    #[test]
    fn vertical_navigation_moves_between_fraction_slots_instead_of_the_root() {
        let mut a = app();
        a.load_math(
            "<math><mfrac><mi>a</mi><mi>b</mi></mfrac></math>",
            None,
            None,
        );
        let preview = eqntide::render::render_interactive(&a.editor.math(), 12.0).unwrap();
        a.rendered = Some(Rendered {
            placeholders: vec![],
            preview,
            image: egui::ColorImage::filled([1, 1], Color32::WHITE),
        });
        let numerator = a.editor.root().children[0].children[0].id;
        let denominator = a.editor.root().children[0].children[1].id;
        a.editor.set_caret(
            Caret {
                node: numerator,
                offset: 0,
            },
            false,
        );
        a.vertical(true, false);
        assert_eq!(a.editor.caret().node, denominator);
        a.vertical(false, false);
        assert_eq!(a.editor.caret().node, numerator);
    }
    #[test]
    fn selected_token_exports_as_a_renderable_math_equation() {
        let mut a = app();
        a.load_math("<math><mi>xyz</mi></math>", None, None);
        let id = a.editor.root().children[0].id;
        a.editor.set_caret(
            Caret {
                node: id,
                offset: 0,
            },
            false,
        );
        a.editor.set_caret(
            Caret {
                node: id,
                offset: 1,
            },
            true,
        );
        let math = a.selected_equation();
        assert_eq!(math.tag, "math");
        assert_eq!(math.children[0].text, "x");
        assert!(eqntide::render::measure(&math, 12.0).unwrap().0 > 0.0);
    }
    fn source_request(a: &mut App, source: &str, now: Instant) -> (u64, String) {
        a.latex = source.into();
        a.source_changed(now);
        a.take_latex_request(now + Duration::from_millis(180))
            .unwrap()
    }
    fn parsed(a: &mut App, generation: u64, result: Result<MathNode, String>) {
        a.tx.send(Event::Latex(generation, result)).unwrap();
        a.events(&egui::Context::default());
    }
    #[test]
    fn live_latex_updates_the_equation_without_rewriting_the_source() {
        let mut a = app();
        let source = r"  \frac{x + 1}{2}  ";
        let (generation, _) = source_request(&mut a, source, Instant::now());
        let key = a.render_key;
        parsed(
            &mut a,
            generation,
            eqntide::service::parse_latex_to_mathml(source).map_err(|e| e.to_string()),
        );
        assert_eq!(a.latex, source);
        assert!(!a.draft_dirty);
        assert!(a.latex_error.is_none());
        assert!(a.render_key > key);
        assert!(a.editor.math().iter().iter().any(|n| n.tag == "mfrac"));
    }
    #[test]
    fn live_latex_debounces_until_typing_pauses() {
        let mut a = app();
        let now = Instant::now();
        a.latex = "x".into();
        a.source_changed(now);
        assert!(a
            .take_latex_request(now + Duration::from_millis(100))
            .is_none());
        a.latex = "x+1".into();
        a.source_changed(now + Duration::from_millis(120));
        assert!(a
            .take_latex_request(now + Duration::from_millis(180))
            .is_none());
        assert_eq!(
            a.take_latex_request(now + Duration::from_millis(300))
                .unwrap()
                .1,
            "x+1"
        );
    }
    #[test]
    fn stale_latex_results_cannot_overwrite_newer_input() {
        let mut a = app();
        let now = Instant::now();
        let (old, _) = source_request(&mut a, "x", now);
        a.latex = "y".into();
        a.source_changed(now);
        parsed(
            &mut a,
            old,
            eqntide::service::parse_latex_to_mathml("x").map_err(|e| e.to_string()),
        );
        assert!(a.editor.math().children.is_empty());
        assert!(a.draft_dirty);
        assert_eq!(a.latex, "y");
        let (latest, source) = a
            .take_latex_request(now + Duration::from_millis(180))
            .unwrap();
        parsed(
            &mut a,
            latest,
            eqntide::service::parse_latex_to_mathml(&source).map_err(|e| e.to_string()),
        );
        assert_eq!(a.editor.math().to_latex(), "y");
        assert!(!a.draft_dirty);
    }
    #[test]
    fn invalid_latex_keeps_the_last_valid_equation_and_raw_draft() {
        let mut a = app();
        a.load_math("<math><mi>x</mi></math>", None, None);
        let before = a.editor.math();
        let key = a.render_key;
        let source = r"\unknowncommand{a}";
        let (generation, _) = source_request(&mut a, source, Instant::now());
        parsed(
            &mut a,
            generation,
            eqntide::service::parse_latex_to_mathml(source).map_err(|e| e.to_string()),
        );
        assert_eq!(a.editor.math(), before);
        assert_eq!(a.render_key, key);
        assert_eq!(a.latex, source);
        assert!(a.draft_dirty);
        assert!(a.latex_error.is_some());
    }
    #[test]
    fn canvas_change_invalidates_its_inflight_source_parse() {
        let mut a = app();
        a.load_math("<math><mi>x</mi></math>", None, None);
        let before = a.editor.math();
        let (generation, _) = source_request(&mut a, "y", Instant::now());
        a.changed();
        parsed(
            &mut a,
            generation,
            eqntide::service::parse_latex_to_mathml("y").map_err(|e| e.to_string()),
        );
        assert_eq!(a.editor.math(), before);
        assert_eq!(a.latex, "x");
        assert!(!a.draft_dirty);
    }
    #[test]
    fn live_source_replacement_can_undo_to_original_mathml_attributes() {
        let mut a = app();
        a.load_math("<math><mi mathvariant=\"bold\">x</mi></math>", None, None);
        let before = a.editor.math();
        let (generation, _) = source_request(&mut a, "y", Instant::now());
        parsed(
            &mut a,
            generation,
            eqntide::service::parse_latex_to_mathml("y").map_err(|e| e.to_string()),
        );
        a.editor.undo();
        a.changed();
        assert_eq!(a.editor.math(), before);
    }
    #[test]
    fn clearing_latex_clears_canvas_through_the_background_worker() {
        let mut a = app();
        a.load_math("<math><mi>x</mi></math>", None, None);
        a.latex = String::new();
        a.source_changed(Instant::now() - Duration::from_secs(1));
        a.request_latex_parse(&egui::Context::default());
        let result = a.rx.recv_timeout(Duration::from_secs(2)).unwrap();
        a.tx.send(result).unwrap();
        a.events(&egui::Context::default());
        assert!(a.editor.math().children.is_empty());
        assert!(!a.draft_dirty);
        assert!(a.latex.is_empty());
    }

    #[test]
    fn live_latex_source_blocks_invalid_commands_and_keeps_valid_style_groups() {
        for invalid in [r"\mathbb{1^2}", r"\text{}", r"\textstyle", r"\mathcal{1}"] {
            let mut a = app();
            a.load_math("<math><mi>x</mi></math>", None, None);
            a.session = Some("sess-1".into());
            let before = a.editor.math();
            a.latex = invalid.into();
            a.source_changed(Instant::now() - Duration::from_secs(1));
            a.request_latex_parse(&egui::Context::default());
            let result = a.rx.recv_timeout(Duration::from_secs(2)).unwrap();
            a.tx.send(result).unwrap();
            a.events(&egui::Context::default());

            assert_eq!(a.editor.math(), before, "{invalid}");
            assert!(a.draft_dirty, "{invalid}");
            assert!(
                a.latex_error
                    .as_deref()
                    .is_some_and(|e| e.contains("Invalid LaTeX command")),
                "expected latex_error for `{invalid}`, got {:?}",
                a.latex_error
            );
            a.finish(true);
            assert!(!a.busy, "saving must stay blocked for `{invalid}`");
            assert!(a.status.contains("Wait for valid LaTeX"), "{}", a.status);

            let ctx = egui::Context::default();
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| a.status_bar(ui));
            out.textures_delta.clear();
            assert!(
                out.shapes.iter().any(|s| match &s.shape {
                    egui::Shape::Text(t) => t.galley.text().contains("LaTeX error:"),
                    _ => false,
                }),
                "status bar should show LaTeX error for `{invalid}`"
            );
        }

        for valid in [r"\text{ }", r"{\textstyle \frac{1}{2}}", r"\mathbf{x^2}"] {
            let mut a = app();
            a.latex = valid.into();
            a.source_changed(Instant::now() - Duration::from_secs(1));
            a.request_latex_parse(&egui::Context::default());
            let result = a.rx.recv_timeout(Duration::from_secs(2)).unwrap();
            a.tx.send(result).unwrap();
            a.events(&egui::Context::default());

            assert!(!a.draft_dirty, "{valid}");
            assert!(a.latex_error.is_none(), "{valid}: {:?}", a.latex_error);
            assert!(!a.editor.math().children.is_empty(), "{valid}");
        }
    }

    #[test]
    fn palette_icons_use_theme_contrast_and_physical_dpi() {
        let mut a = app();
        for (theme, expected_tint) in [
            (egui::Theme::Dark, ThemeTokens::for_mode(true).text_primary),
            (
                egui::Theme::Light,
                ThemeTokens::for_mode(false).text_primary,
            ),
        ] {
            let ctx = egui::Context::default();
            ThemeTokens::apply_style(&ctx);
            ctx.set_theme(theme);
            ctx.set_pixels_per_point(1.5);
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| a.shell(ui));
            output.textures_delta.clear();
            let texture_ids = a.palette_texture_ids();
            assert!(!texture_ids.is_empty());
            assert!((a.palette_ppp - 1.5).abs() < f32::EPSILON);
            let tinted_match = output.shapes.iter().any(|s| match &s.shape {
                egui::Shape::Rect(r) => r.brush.as_ref().is_some_and(|b| {
                    texture_ids.contains(&b.fill_texture_id) && r.fill == expected_tint
                }),
                _ => false,
            });
            assert!(
                tinted_match,
                "expected palette icons to tint {expected_tint:?} in {theme:?}"
            );
        }
    }

    #[test]
    fn sidebar_settings_and_about_tabs_work_and_toggle_theme() {
        let mut a = app();
        let ctx = egui::Context::default();
        ctx.set_theme(egui::Theme::Dark);

        // Open Settings from sidebar
        for (label, expected_tab) in [("Settings", 2), ("Back", 0), ("About", 3), ("Back", 0)] {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| a.shell(ui));
            output.textures_delta.clear();
            let pos = output
                .shapes
                .iter()
                .find_map(|s| {
                    if let egui::Shape::Text(t) = &s.shape {
                        if t.galley.text() == label {
                            return Some(s.shape.visual_bounding_rect().center());
                        }
                    }
                    None
                })
                .unwrap_or_else(|| panic!("missing button {label}"));
            ctx.run_ui(
                egui::RawInput {
                    events: vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed: true,
                            modifiers: egui::Modifiers::NONE,
                        },
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed: false,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                    ..Default::default()
                },
                |ui| a.shell(ui),
            )
            .textures_delta
            .clear();
            assert_eq!(a.tab, expected_tab);
        }

        // Verify About displays logo, version, author, and GitHub link
        a.tab = 3;
        let mut about_out = ctx.run_ui(egui::RawInput::default(), |ui| a.shell(ui));
        about_out.textures_delta.clear();
        let texts: Vec<_> = about_out
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Text(t) => Some(t.galley.text().to_owned()),
                _ => None,
            })
            .collect();
        assert!(texts
            .iter()
            .any(|t| t == concat!("EqnTide ", env!("CARGO_PKG_VERSION"))));
        assert!(!texts.iter().any(|t| t == "About EqnTide"));
        assert!(!texts
            .iter()
            .any(|t| t == "Version:" || t.starts_with("Version ")));
        assert!(texts
            .iter()
            .any(|t| t.starts_with("Bidirectional math equation")));
        assert!(texts.iter().any(|t| t == env!("CARGO_PKG_AUTHORS")));
        assert!(texts.iter().any(|t| t == env!("CARGO_PKG_REPOSITORY")));
        assert!(a.logo_texture.is_some());

        // Verify Settings switches theme from Dark -> Light via selectable Light/Dark buttons
        a.tab = 2;
        let mut settings_out = ctx.run_ui(egui::RawInput::default(), |ui| a.shell(ui));
        settings_out.textures_delta.clear();
        let dark_tokens = ThemeTokens::for_mode(true);
        let dark_btn_pos = find_text_center(&settings_out, |t| t == "Dark");
        let light_btn_pos = find_text_center(&settings_out, |t| t == "Light");
        assert!(settings_out.shapes.iter().any(|s| matches!(&s.shape,
            egui::Shape::Rect(r) if r.rect.contains(dark_btn_pos) && r.fill == dark_tokens.selected_bg)));
        assert!(!settings_out.shapes.iter().any(|s| matches!(&s.shape,
            egui::Shape::Rect(r) if r.rect.contains(light_btn_pos) && r.fill == dark_tokens.selected_bg)));

        ctx.run_ui(
            egui::RawInput {
                events: click_events(light_btn_pos),
                ..Default::default()
            },
            |ui| a.shell(ui),
        )
        .textures_delta
        .clear();
        assert_eq!(ctx.theme(), egui::Theme::Light);

        let mut after_light = ctx.run_ui(egui::RawInput::default(), |ui| a.shell(ui));
        after_light.textures_delta.clear();

        let light_tokens = ThemeTokens::for_mode(false);
        let light_btn_pos = find_text_center(&after_light, |t| t == "Light");
        let dark_btn_pos = find_text_center(&after_light, |t| t == "Dark");
        assert!(after_light.shapes.iter().any(|s| matches!(&s.shape,
            egui::Shape::Rect(r)
                if r.rect.contains(light_btn_pos)
                    && (r.fill == light_tokens.selected_bg || r.fill == light_tokens.selected_hover_bg)
                    && r.stroke.color == light_tokens.selected_border)));
        assert!(!after_light.shapes.iter().any(|s| matches!(&s.shape,
            egui::Shape::Rect(r)
                if r.rect.contains(dark_btn_pos)
                    && (r.fill == light_tokens.selected_bg || r.fill == light_tokens.selected_hover_bg))));
        assert!(!after_light.shapes.iter().any(|s| matches!(&s.shape,
            egui::Shape::Text(t) if t.galley.text() == "Theme")));
        // Zoom slider rail in Light mode contrasts against card_bg (not white-on-white).
        let pct_pos = find_text_center(&after_light, |t| t == "200%");
        assert!(after_light.shapes.iter().any(|s| matches!(&s.shape,
            egui::Shape::Rect(r)
                if (r.rect.center().y - pct_pos.y).abs() < 12.0
                    && r.rect.width() >= 150.0
                    && r.rect.height() <= 24.0
                    && r.fill == light_tokens.border)));
        assert!(after_light.shapes.iter().any(|s| matches!(&s.shape,
            egui::Shape::Rect(r)
                if (r.rect.center().y - pct_pos.y).abs() < 12.0
                    && r.rect.height() <= 24.0
                    && r.fill == light_tokens.accent)));
    }

    #[test]
    fn sidebar_icons_load_48px_textures_and_tint_across_themes() {
        let mut a = app();
        let ctx = egui::Context::default();
        for dpi in [1.0, 2.0] {
            ctx.set_pixels_per_point(dpi);
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| a.shell(ui));
            out.textures_delta.clear();
            for group in palette::UI_GROUPS {
                for &icon_name in group.icons {
                    let handle = a
                        .ui_icons
                        .get(icon_name)
                        .unwrap_or_else(|| panic!("missing texture for {icon_name}"));
                    assert_eq!(handle.size(), [48, 48]);
                    let tex_id = handle.id();
                    let icon_rect = out
                        .shapes
                        .iter()
                        .find_map(|s| match &s.shape {
                            egui::Shape::Rect(r)
                                if r.brush
                                    .as_ref()
                                    .is_some_and(|b| b.fill_texture_id == tex_id) =>
                            {
                                Some(r.rect)
                            }
                            _ => None,
                        })
                        .unwrap_or_else(|| panic!("missing painted sidebar icon {icon_name}"));
                    assert!((icon_rect.width() - SIDEBAR_CATEGORY_ICON_SIZE).abs() <= 0.5);
                    assert!((icon_rect.height() - SIDEBAR_CATEGORY_ICON_SIZE).abs() <= 0.5);
                }
            }
        }

        for (theme, expected_tint) in [
            (egui::Theme::Dark, ThemeTokens::for_mode(true).text_primary),
            (
                egui::Theme::Light,
                ThemeTokens::for_mode(false).text_primary,
            ),
        ] {
            ThemeTokens::apply_style(&ctx);
            ctx.set_theme(theme);
            let mut tint = None;
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                tint = Some(App::icon_tint(ui));
            });
            out.textures_delta.clear();
            assert_eq!(tint, Some(expected_tint));
        }
    }

    #[test]
    fn recent_palette_tracks_mru_without_duplicates_and_renders() {
        let mut a = app();
        let ctx = egui::Context::default();
        assert_eq!(a.category, palette::DEFAULT_GROUP);
        assert!(a.recent_palette.is_empty());

        a.category = palette::RECENT_GROUP.into();
        let mut empty_out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::from(MIN_WINDOW_SIZE))),
                ..Default::default()
            },
            |ui| a.shell(ui),
        );
        empty_out.textures_delta.clear();
        assert!(empty_out.shapes.iter().any(|s| matches!(&s.shape,
            egui::Shape::Text(t) if t.galley.text().contains("Click any symbol"))));

        a.category = "Fractions".into();
        let mut frac_out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::from(MIN_WINDOW_SIZE))),
                ..Default::default()
            },
            |ui| a.shell(ui),
        );
        frac_out.textures_delta.clear();
        let frac_tex = a.palette_atlases["Fractions"].texture.id();
        let first_pos = frac_out
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Rect(r)
                    if r.brush
                        .as_ref()
                        .is_some_and(|b| b.fill_texture_id == frac_tex) =>
                {
                    Some(r.rect.center())
                }
                _ => None,
            })
            .expect("fraction palette button");

        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::from(MIN_WINDOW_SIZE))),
                events: vec![
                    egui::Event::PointerMoved(first_pos),
                    egui::Event::PointerButton {
                        pos: first_pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                    egui::Event::PointerButton {
                        pos: first_pos,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                ..Default::default()
            },
            |ui| a.shell(ui),
        )
        .textures_delta
        .clear();
        assert_eq!(a.recent_palette.len(), 1);
        let first_source = a.recent_palette[0];

        a.record_recent_palette(r"\alpha");
        a.record_recent_palette(first_source);
        assert_eq!(a.recent_palette, vec![first_source, r"\alpha"]);

        a.category = palette::RECENT_GROUP.into();
        let mut recent_out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::from(MIN_WINDOW_SIZE))),
                ..Default::default()
            },
            |ui| a.shell(ui),
        );
        recent_out.textures_delta.clear();
        let recent_tex = a.palette_atlases[palette::RECENT_GROUP].texture.id();
        let rendered_recent = recent_out
            .shapes
            .iter()
            .filter(|s| {
                matches!(&s.shape, egui::Shape::Rect(r) if r.brush.as_ref().is_some_and(|b| b.fill_texture_id == recent_tex))
            })
            .count();
        assert_eq!(rendered_recent, 2);
    }

    #[test]
    fn palette_scrolls_horizontally_on_vertical_mouse_wheel() {
        fn run_wheel_case(category: &str, wheel_delta: Vec2, expected_overflow: bool) {
            let ctx = egui::Context::default();
            let mut a = app();
            a.category = category.into();
            let hover_pos = Pos2::new(360.0, 48.0);
            let mut time = 0.0f64;

            // Prime layout and palette rect.
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::from(MIN_WINDOW_SIZE))),
                    time: Some(time),
                    predicted_dt: 1.0 / 60.0,
                    events: vec![egui::Event::PointerMoved(hover_pos)],
                    ..Default::default()
                },
                |ui| a.shell(ui),
            );
            out.textures_delta.clear();

            // Send mouse-wheel scroll while hovering over palette.
            time += 0.1;
            let mut wheel_out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::from(MIN_WINDOW_SIZE))),
                    time: Some(time),
                    predicted_dt: 1.0 / 60.0,
                    events: vec![
                        egui::Event::PointerMoved(hover_pos),
                        egui::Event::MouseWheel {
                            unit: egui::MouseWheelUnit::Point,
                            delta: wheel_delta,
                            phase: egui::TouchPhase::Move,
                            source: egui::MouseWheelSource::Unknown,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                    ..Default::default()
                },
                |ui| a.shell(ui),
            );
            wheel_out.textures_delta.clear();

            // Step settle frames so smooth/kinetic scroll finishes.
            let mut settle_out = wheel_out;
            for _ in 0..4 {
                time += 0.05;
                settle_out = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            Pos2::ZERO,
                            Vec2::from(MIN_WINDOW_SIZE),
                        )),
                        time: Some(time),
                        predicted_dt: 1.0 / 60.0,
                        events: vec![egui::Event::PointerMoved(hover_pos)],
                        ..Default::default()
                    },
                    |ui| a.shell(ui),
                );
                settle_out.textures_delta.clear();
            }

            let palette_tex = a.palette_atlases[category].texture.id();
            let first_x = |shapes: &[egui::epaint::ClippedShape]| -> f32 {
                shapes
                    .iter()
                    .filter_map(|s| match &s.shape {
                        egui::Shape::Rect(r)
                            if r.brush
                                .as_ref()
                                .is_some_and(|b| b.fill_texture_id == palette_tex) =>
                        {
                            Some(r.rect.left())
                        }
                        _ => None,
                    })
                    .fold(f32::INFINITY, f32::min)
            };
            let initial_x = first_x(&out.shapes);
            let final_x = first_x(&settle_out.shapes);
            if expected_overflow {
                assert!(
                    final_x < initial_x - 20.0,
                    "expected overflowing category {category} to pan left (scroll right) with {wheel_delta:?}: {initial_x} -> {final_x}"
                );
            } else {
                assert!(
                    (final_x - initial_x).abs() < 1.0,
                    "expected non-overflowing category {category} to stay stationary with {wheel_delta:?}: {initial_x} -> {final_x}"
                );
            }
        }

        run_wheel_case("LogicSetsSymbols", Vec2::new(0.0, -180.0), true);
        run_wheel_case("LogicSetsSymbols", Vec2::new(-180.0, 0.0), true);
        run_wheel_case("Fractions", Vec2::new(0.0, -180.0), false);
    }

    fn click_events(pos: Pos2) -> Vec<egui::Event> {
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ]
    }

    fn run_ui_pass_frame(
        ctx: &egui::Context,
        app: &mut App,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::from(MIN_WINDOW_SIZE))),
                events,
                ..Default::default()
            },
            |ui| app.ui_pass(ui),
        );
        output.textures_delta.clear();
        output
    }

    #[track_caller]
    fn find_text_center(output: &egui::FullOutput, predicate: impl Fn(&str) -> bool) -> Pos2 {
        output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if predicate(t.galley.text()) => {
                    Some(s.shape.visual_bounding_rect().center())
                }
                _ => None,
            })
            .expect("expected matching text shape")
    }

    #[test]
    fn with_settings_path_restores_all_persisted_preferences() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let saved = Settings {
            zoom: 3.0,
            theme: ThemeMode::Dark,
            startup_focus: StartupFocus::Latex,
            font_pt: 18.5,
            target: "mathtype".into(),
            category: palette::RECENT_GROUP.into(),
            recent_palette: vec![r"\alpha".into(), r"\frac{#?}{#?}".into()],
            batch_target: 2,
            batch_overwrite: true,
            ..Settings::default()
        };
        saved.save_to_path(&path).unwrap();

        let isolated = app();
        assert!(isolated.settings_path.is_none());
        assert_eq!(isolated.zoom, settings::DEFAULT_ZOOM);
        assert_eq!(isolated.font_pt, settings::DEFAULT_FONT_PT);

        let ctx = egui::Context::default();
        let restored = app().with_settings_path(path, Some(&ctx));
        assert_eq!(restored.zoom, 3.0);
        assert_eq!(ctx.theme(), egui::Theme::Dark);
        assert_eq!(restored.startup_focus, StartupFocus::Latex);
        assert_eq!(restored.font_pt, 18.5);
        assert_eq!(restored.saved_font, 18.5);
        assert_eq!(restored.target, "mathtype");
        assert_eq!(restored.saved_mode, "mathtype");
        assert_eq!(restored.category, palette::RECENT_GROUP);
        assert_eq!(restored.recent_palette, vec![r"\alpha", r"\frac{#?}{#?}"]);
        assert_eq!(restored.batch_target, 2);
        assert!(restored.batch_overwrite);
        assert!(!restored.dirty());
        assert!(!restored.settings_dirty);
    }

    #[test]
    fn office_session_overrides_do_not_overwrite_saved_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        Settings {
            font_pt: 16.0,
            target: "mathtype".into(),
            ..Settings::default()
        }
        .save_to_path(&path)
        .unwrap();

        let ctx = egui::Context::default();
        let mut a = app().with_settings_path(path.clone(), Some(&ctx));

        // Standalone load_math with font=None keeps the persisted font_pt.
        assert!(a.load_math("<math><mi>y</mi></math>", None, None));
        assert_eq!(a.font_pt, 16.0);

        // Office session overrides live font_pt and target without dirtying persisted settings.
        a.loading_session = Some("office-1".into());
        a.tx.send(Event::Session(
            "office-1".into(),
            Ok(serde_json::json!({
                "state": "waiting",
                "mathml": "<math><mi>x</mi></math>",
                "font_pt": 24.0,
                "target_mode": "omml"
            })),
        ))
        .unwrap();
        a.events(&ctx);

        assert_eq!(a.font_pt, 24.0);
        assert_eq!(a.target, "omml");
        assert_eq!(a.session.as_deref(), Some("office-1"));
        assert!(!a.settings_dirty);

        // Completing/closing the session leaves the saved defaults intact on disk.
        run_ui_pass_frame(&ctx, &mut a, vec![]);
        a.tx.send(Event::Finished(Ok(()))).unwrap();
        a.events(&ctx);

        let on_disk = Settings::load_from_path(&path);
        assert_eq!(on_disk.font_pt, 16.0);
        assert_eq!(on_disk.target, "mathtype");
    }

    #[test]
    fn user_ui_edits_persist_atomically_and_coalesce_pointer_drag() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let ctx = egui::Context::default();
        ctx.set_theme(egui::Theme::Dark);
        let mut a = app().with_settings_path(path.clone(), Some(&ctx));
        // Switch back to Dark explicitly for initial baseline via theme toggle test later.
        let out = run_ui_pass_frame(&ctx, &mut a, vec![]);

        // 1. Click sidebar category "Scripts".
        let scripts_tex = a
            .ui_icon_texture_id(palette::ui_group("Scripts").unwrap().icons[0])
            .expect("Scripts sidebar texture");
        let scripts_pos = out
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Rect(r)
                    if r.brush
                        .as_ref()
                        .is_some_and(|b| b.fill_texture_id == scripts_tex) =>
                {
                    Some(r.rect.center())
                }
                _ => None,
            })
            .expect("Scripts sidebar icon");
        let out = run_ui_pass_frame(&ctx, &mut a, click_events(scripts_pos));
        assert_eq!(a.category, "Scripts");
        assert_eq!(Settings::load_from_path(&path).category, "Scripts");

        // 2. Click a palette item in "Scripts".
        let scripts_tex = a.palette_atlases["Scripts"].texture.id();
        let item_pos = out
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Rect(r)
                    if r.brush
                        .as_ref()
                        .is_some_and(|b| b.fill_texture_id == scripts_tex) =>
                {
                    Some(r.rect.center())
                }
                _ => None,
            })
            .expect("Scripts palette button");
        let out = run_ui_pass_frame(&ctx, &mut a, click_events(item_pos));
        assert_eq!(a.recent_palette.len(), 1);
        let clicked_source = a.recent_palette[0];
        assert_eq!(
            Settings::load_from_path(&path).recent_palette,
            vec![clicked_source.to_string()]
        );

        // 3. Click footer "MathType" radio button.
        let mathtype_pos = find_text_center(&out, |t| t == "MathType");
        let out = run_ui_pass_frame(&ctx, &mut a, click_events(mathtype_pos));
        assert_eq!(a.target, "mathtype");
        assert_eq!(Settings::load_from_path(&path).target, "mathtype");

        // 4. Drag footer Font size DragValue while pointer is down, then release.
        let font_pos = find_text_center(&out, |t| t.ends_with(" pt"));
        let drag_to = Pos2::new(font_pos.x + 80.0, font_pos.y);
        run_ui_pass_frame(
            &ctx,
            &mut a,
            vec![
                egui::Event::PointerMoved(font_pos),
                egui::Event::PointerButton {
                    pos: font_pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        run_ui_pass_frame(&ctx, &mut a, vec![egui::Event::PointerMoved(drag_to)]);
        assert!(a.font_pt > settings::DEFAULT_FONT_PT);
        assert!(a.settings_dirty);
        // Disk write is deferred until pointer drag releases.
        assert_eq!(
            Settings::load_from_path(&path).font_pt,
            settings::DEFAULT_FONT_PT
        );
        run_ui_pass_frame(
            &ctx,
            &mut a,
            vec![egui::Event::PointerButton {
                pos: drag_to,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(!a.settings_dirty);
        let dragged_font = a.font_pt;
        assert_eq!(Settings::load_from_path(&path).font_pt, dragged_font);

        // 5. Open Settings tab, adjust zoom and toggle theme.
        let settings_pos = find_text_center(&out, |t| t == "Settings");
        run_ui_pass_frame(&ctx, &mut a, click_events(settings_pos));
        assert_eq!(a.tab, 2);
        let out = run_ui_pass_frame(&ctx, &mut a, vec![]);

        // Drag zoom slider right while pointer is down, then release.
        let pct_pos = find_text_center(&out, |t| t == "200%");
        let slider_center = out
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Rect(r)
                    if (r.rect.center().y - pct_pos.y).abs() < 12.0
                        && r.rect.width() >= 150.0
                        && r.rect.width() <= 260.0
                        && r.rect.height() <= 24.0 =>
                {
                    Some(r.rect.center())
                }
                _ => None,
            })
            .unwrap_or(pct_pos);
        let slider_drag_end = Pos2::new(slider_center.x + 60.0, slider_center.y);
        run_ui_pass_frame(
            &ctx,
            &mut a,
            vec![
                egui::Event::PointerMoved(slider_center),
                egui::Event::PointerButton {
                    pos: slider_center,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        run_ui_pass_frame(
            &ctx,
            &mut a,
            vec![egui::Event::PointerMoved(slider_drag_end)],
        );
        assert!(a.settings_dirty);
        assert_eq!(Settings::load_from_path(&path).zoom, settings::DEFAULT_ZOOM);
        let out = run_ui_pass_frame(
            &ctx,
            &mut a,
            vec![egui::Event::PointerButton {
                pos: slider_drag_end,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(!a.settings_dirty);
        let dragged_zoom = a.zoom;
        assert!((dragged_zoom - settings::DEFAULT_ZOOM).abs() > 0.05);
        assert_eq!(Settings::load_from_path(&path).zoom, dragged_zoom);

        // Select Light theme (Dark -> Light).
        let light_pos = find_text_center(&out, |t| t == "Light");
        let out = run_ui_pass_frame(&ctx, &mut a, click_events(light_pos));
        assert_eq!(ctx.theme(), egui::Theme::Light);
        assert_eq!(Settings::load_from_path(&path).theme, ThemeMode::Light);

        // 6. Open Batch DOCX tab, toggle overwrite and change batch target.
        let back_pos = find_text_center(&out, |t| t == "Back");
        run_ui_pass_frame(&ctx, &mut a, click_events(back_pos));
        let out = run_ui_pass_frame(&ctx, &mut a, vec![]);
        let batch_pos = find_text_center(&out, |t| t == "Batch DOCX");
        run_ui_pass_frame(&ctx, &mut a, click_events(batch_pos));
        assert_eq!(a.tab, 1);
        let out = run_ui_pass_frame(&ctx, &mut a, vec![]);

        let overwrite_pos = find_text_center(&out, |t| t.starts_with("Overwrite original"));
        let out = run_ui_pass_frame(&ctx, &mut a, click_events(overwrite_pos));
        assert!(a.batch_overwrite);
        assert!(Settings::load_from_path(&path).batch_overwrite);

        let combo_pos = find_text_center(&out, |t| t == batch::TARGETS[0].1);
        run_ui_pass_frame(&ctx, &mut a, click_events(combo_pos));
        let out = run_ui_pass_frame(&ctx, &mut a, vec![]);
        let option_pos = find_text_center(&out, |t| t == batch::TARGETS[1].1);
        run_ui_pass_frame(&ctx, &mut a, click_events(option_pos));
        assert_eq!(a.batch_target, 1);
        assert_eq!(Settings::load_from_path(&path).batch_target, 1);

        // 7. Reload in a brand new App instance and confirm full restoration.
        let fresh_ctx = egui::Context::default();
        let mut reloaded = app().with_settings_path(path.clone(), Some(&fresh_ctx));
        assert_eq!(reloaded.category, "Scripts");
        assert_eq!(reloaded.recent_palette, vec![clicked_source]);
        assert_eq!(reloaded.target, "mathtype");
        assert_eq!(reloaded.font_pt, dragged_font);
        assert_eq!(reloaded.zoom, dragged_zoom);
        assert_eq!(fresh_ctx.theme(), egui::Theme::Light);
        assert!(reloaded.batch_overwrite);
        assert_eq!(reloaded.batch_target, 1);

        // 8. Verify "Reset (200%)" persists default zoom back to disk.
        reloaded.tab = 2;
        let out = run_ui_pass_frame(&fresh_ctx, &mut reloaded, vec![]);
        let reset_pos = find_text_center(&out, |t| t == "Reset (200%)");
        run_ui_pass_frame(&fresh_ctx, &mut reloaded, click_events(reset_pos));
        assert_eq!(reloaded.zoom, settings::DEFAULT_ZOOM);
        assert_eq!(Settings::load_from_path(&path).zoom, settings::DEFAULT_ZOOM);
    }

    #[test]
    fn modern_theme_applies_zero_hover_expansion_radii_and_tinted_sidebar_selection() {
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            let dark_mode = theme == egui::Theme::Dark;
            let tokens = ThemeTokens::for_mode(dark_mode);
            let ctx = egui::Context::default();
            ThemeTokens::apply_style(&ctx);
            ctx.set_theme(theme);

            let mut a = app();
            let out = run_ui_pass_frame(&ctx, &mut a, vec![]);

            // Selected sidebar category ("Fractions") displays tinted-accent surface and matching icon tint.
            let fractions_tex = a
                .ui_icon_texture_id(palette::ui_group("Fractions").unwrap().icons[0])
                .expect("Fractions sidebar texture");
            assert!(out.shapes.iter().any(|s| matches!(&s.shape,
                egui::Shape::Rect(r) if r.fill == tokens.selected_bg && r.stroke.color == tokens.selected_border)));
            assert!(out.shapes.iter().any(|s| matches!(&s.shape,
                egui::Shape::Rect(r) if r.brush.as_ref().is_some_and(|b| b.fill_texture_id == fractions_tex)
                    && r.fill == tokens.selected_fg)));

            // Primary Save button uses primary_bg at rest, primary_hover_bg on hover without growing, and disabled fill when disabled.
            let save_pos = find_text_center(&out, |t| t == "Save MathML (Ctrl+S)");
            let save_rest = out
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Rect(r)
                        if r.fill == tokens.primary_bg && r.rect.contains(save_pos) =>
                    {
                        Some((r.rect, r.corner_radius))
                    }
                    _ => None,
                })
                .expect("Save button rest rect");
            assert_eq!(save_rest.1, theme::CONTROL_CORNER_RADIUS);

            let hover_out =
                run_ui_pass_frame(&ctx, &mut a, vec![egui::Event::PointerMoved(save_pos)]);
            let save_hover = hover_out
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Rect(r)
                        if r.fill == tokens.primary_hover_bg && r.rect.contains(save_pos) =>
                    {
                        Some(r.rect)
                    }
                    _ => None,
                })
                .expect("Save button hover rect");
            assert_eq!(save_hover, save_rest.0);

            // Disabled Save button suppresses primary/hover blue emphasis.
            a.draft_dirty = true;
            let disabled_out =
                run_ui_pass_frame(&ctx, &mut a, vec![egui::Event::PointerMoved(save_pos)]);
            assert!(disabled_out.shapes.iter().any(|s| matches!(&s.shape,
                egui::Shape::Rect(r)
                    if r.rect.contains(save_pos)
                        && r.fill != tokens.primary_bg
                        && r.fill != tokens.primary_hover_bg)));
            a.draft_dirty = false;

            // Settings zoom slider shares CONTROL_CORNER_RADIUS (no 4px override).
            a.tab = 2;
            let _ = run_ui_pass_frame(&ctx, &mut a, vec![]);
            let style = ctx.style_of(theme);
            assert_eq!(
                style.visuals.widgets.inactive.corner_radius,
                theme::CONTROL_CORNER_RADIUS
            );
            assert_eq!(style.visuals.widgets.hovered.expansion, 0.0);
        }
    }

    #[test]
    fn canvas_presets_switch_adapt_ink_and_persist_per_mode() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let ctx = egui::Context::default();
        ThemeTokens::apply_style(&ctx);
        let mut a = app().with_settings_path(path.clone(), Some(&ctx));
        a.load_math("<math><mi>x</mi></math>", None, None);

        // Render preview texture for ink verification.
        let (mask, preview) =
            eqntide::render::render_interactive_alpha_scaled(&a.editor.math(), a.font_pt, 1.0)
                .unwrap();
        let img = egui::ColorImage {
            size: [mask.width as usize, mask.height as usize],
            source_size: Vec2::new(mask.width as f32, mask.height as f32),
            pixels: mask
                .alpha
                .into_iter()
                .map(Color32::from_white_alpha)
                .collect(),
        };
        a.texture = Some(ctx.load_texture("eq-test", img, egui::TextureOptions::LINEAR));
        a.rendered = Some(Rendered {
            placeholders: Vec::new(),
            preview,
            image: egui::ColorImage::filled([1, 1], Color32::WHITE),
        });
        a.requested_key = a.render_key;

        let has_formula_ink = |output: &egui::FullOutput, tex_id: egui::TextureId, ink: Color32| {
            output.shapes.iter().any(|s| match &s.shape {
                egui::Shape::Rect(r) => {
                    r.fill == ink
                        && r.brush
                            .as_ref()
                            .is_some_and(|b| b.fill_texture_id == tex_id)
                }
                egui::Shape::Mesh(m) => {
                    m.texture_id == tex_id && m.vertices.iter().all(|v| v.color == ink)
                }
                _ => false,
            })
        };

        // Default in Dark mode still uses SoftPaper (light paper background + dark ink) on both canvas and LaTeX source.
        ctx.set_theme(egui::Theme::Dark);
        let out = run_ui_pass_frame(&ctx, &mut a, vec![]);
        let tex_id = a.texture.as_ref().unwrap().id();
        let latex_rect = ctx
            .read_response(egui::Id::unique("latex-source"))
            .unwrap()
            .rect;
        let soft_dark = CanvasPreset::SoftPaper.palette(true);
        assert!(out.shapes.iter().any(|s| matches!(&s.shape,
            egui::Shape::Rect(r) if r.fill == soft_dark.bg && r.stroke.color == soft_dark.border)));
        assert!(out.shapes.iter().any(|s| matches!(&s.shape,
            egui::Shape::Rect(r) if r.fill == soft_dark.bg && r.rect == latex_rect)));
        assert!(has_formula_ink(&out, tex_id, soft_dark.ink));

        // Open Settings and pick CoolSlate for Light mode and DimSlate for Dark mode.
        a.tab = 2;
        let out = run_ui_pass_frame(&ctx, &mut a, vec![]);
        let cool_pos = find_text_center(&out, |t| t == CanvasPreset::CoolSlate.label());
        let out = run_ui_pass_frame(&ctx, &mut a, click_events(cool_pos));
        let dim_pos = find_text_center(&out, |t| t == CanvasPreset::DimSlate.label());
        run_ui_pass_frame(&ctx, &mut a, click_events(dim_pos));

        let on_disk = Settings::load_from_path(&path);
        assert_eq!(on_disk.canvas_light, CanvasPreset::CoolSlate);
        assert_eq!(on_disk.canvas_dark, CanvasPreset::DimSlate);

        // Verify Dark mode now uses DimSlate surface + light ink, and Light mode uses CoolSlate surface + dark ink.
        a.tab = 0;
        let dark_out = run_ui_pass_frame(&ctx, &mut a, vec![]);
        let dim_pal = CanvasPreset::DimSlate.palette(true);
        assert!(dark_out.shapes.iter().any(|s| matches!(&s.shape,
            egui::Shape::Rect(r) if r.fill == dim_pal.bg && r.stroke.color == dim_pal.border)));
        assert!(dark_out.shapes.iter().any(|s| matches!(&s.shape,
            egui::Shape::Rect(r) if r.fill == dim_pal.bg && r.rect == latex_rect)));
        assert!(has_formula_ink(&dark_out, tex_id, dim_pal.ink));

        ctx.set_theme(egui::Theme::Light);
        let light_out = run_ui_pass_frame(&ctx, &mut a, vec![]);
        let cool_pal = CanvasPreset::CoolSlate.palette(false);
        assert!(light_out.shapes.iter().any(|s| matches!(&s.shape,
            egui::Shape::Rect(r) if r.fill == cool_pal.bg && r.stroke.color == cool_pal.border)));
        assert!(light_out.shapes.iter().any(|s| matches!(&s.shape,
            egui::Shape::Rect(r) if r.fill == cool_pal.bg && r.rect == latex_rect)));
        assert!(has_formula_ink(&light_out, tex_id, cool_pal.ink));

        // Reload in a fresh App instance and confirm presets restored.
        let fresh_ctx = egui::Context::default();
        let reloaded = app().with_settings_path(path, Some(&fresh_ctx));
        assert_eq!(reloaded.canvas_light, CanvasPreset::CoolSlate);
        assert_eq!(reloaded.canvas_dark, CanvasPreset::DimSlate);
    }

    #[test]
    fn sidebar_and_palette_icons_render_at_uniform_base_em_scale() {
        let ctx = egui::Context::default();
        ThemeTokens::apply_style(&ctx);
        let mut a = app();
        a.category = "Scripts".into();
        let out = run_ui_pass_frame(&ctx, &mut a, vec![]);

        for group in palette::UI_GROUPS {
            for &icon_name in group.icons {
                let tex = a
                    .ui_icon_texture_id(icon_name)
                    .unwrap_or_else(|| panic!("missing sidebar icon {icon_name}"));
                let rect = out
                    .shapes
                    .iter()
                    .find_map(|s| match &s.shape {
                        egui::Shape::Rect(r)
                            if r.brush.as_ref().is_some_and(|b| b.fill_texture_id == tex) =>
                        {
                            Some(r.rect)
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| panic!("missing painted sidebar icon {icon_name}"));
                assert!((rect.width() - SIDEBAR_CATEGORY_ICON_SIZE).abs() <= 0.5);
                assert!((rect.height() - SIDEBAR_CATEGORY_ICON_SIZE).abs() <= 0.5);
            }
        }

        let scripts_items = palette::items_for_ui_group("Scripts", &[]);
        let scripts_atlas = &a.palette_atlases["Scripts"];
        let first_h = scripts_atlas.entries[scripts_items[0].2].display_size.y;
        let second_h = scripts_atlas.entries[scripts_items[1].2].display_size.y;
        assert!(first_h <= 26.0 && second_h <= 26.0);
        assert!((first_h - second_h).abs() <= 2.0);

        // Verify palette button frames share uniform 46x32 dimensions.
        let scripts_tex = scripts_atlas.texture.id();
        let icon_rects: Vec<_> = out
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Rect(r)
                    if r.brush
                        .as_ref()
                        .is_some_and(|b| b.fill_texture_id == scripts_tex) =>
                {
                    Some(r.rect)
                }
                _ => None,
            })
            .collect();
        assert!(!icon_rects.is_empty());
        let first_icon = icon_rects[0];
        let button_rect = out
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Rect(r)
                    if r.brush.is_none()
                        && r.rect.contains(first_icon.center())
                        && (r.rect.width() - PALETTE_BUTTON_SIZE.x).abs() < 1.0 =>
                {
                    Some(r.rect)
                }
                _ => None,
            })
            .expect("palette tile rect");
        assert!((button_rect.width() - PALETTE_BUTTON_SIZE.x).abs() < 1.0);
        assert!((button_rect.height() - PALETTE_BUTTON_SIZE.y).abs() < 1.0);
    }

    #[test]
    fn button_height_tiers_align_footer_tabs_and_compact_controls() {
        let ctx = egui::Context::default();
        ThemeTokens::apply_style(&ctx);
        let mut a = app();
        a.category = "MatricesAndBoxes".into();
        let out = run_ui_pass_frame(&ctx, &mut a, vec![]);

        let button_rect_around = |label: &str| -> Rect {
            let pos = find_text_center(&out, |t| t == label);
            out.shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Rect(r)
                        if r.brush.is_none() && r.rect.contains(pos) && r.rect.height() <= 40.0 =>
                    {
                        Some(r.rect)
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("missing button rect for {label}"))
        };

        let icon_button_rect = |icon_name: &str| -> Rect {
            let tex = a
                .ui_icon_texture_id(icon_name)
                .unwrap_or_else(|| panic!("missing icon texture {icon_name}"));
            let pos = out
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Rect(r)
                        if r.brush.as_ref().is_some_and(|b| b.fill_texture_id == tex) =>
                    {
                        Some(r.rect.center())
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("missing painted icon {icon_name}"));
            out.shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Rect(r)
                        if r.brush.is_none() && r.rect.contains(pos) && r.rect.height() <= 40.0 =>
                    {
                        Some(r.rect)
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("missing button rect for icon {icon_name}"))
        };

        // Footer standard buttons and DragValue share BUTTON_HEIGHT (30px).
        let save_rect = button_rect_around("Save MathML (Ctrl+S)");
        let undo_rect = icon_button_rect("arrow-back-up");
        let redo_rect = icon_button_rect("arrow-forward-up");
        assert!((save_rect.height() - theme::BUTTON_HEIGHT).abs() <= 0.5);
        assert!((undo_rect.height() - theme::BUTTON_HEIGHT).abs() <= 0.5);
        assert!((redo_rect.height() - theme::BUTTON_HEIGHT).abs() <= 0.5);

        // Sidebar navigation tabs share SIDEBAR_TAB_HEIGHT (34px).
        for tab_label in ["Batch DOCX", "Settings", "About"] {
            let tab_rect = button_rect_around(tab_label);
            assert!((tab_rect.height() - theme::SIDEBAR_TAB_HEIGHT).abs() <= 0.5);
        }

        // Compact matrix controls share COMPACT_BUTTON_HEIGHT (24px).
        for matrix_label in ["+ row", "− row", "+ col", "− col"] {
            let rect = button_rect_around(matrix_label);
            assert!((rect.height() - theme::COMPACT_BUTTON_HEIGHT).abs() <= 0.5);
        }
    }

    #[test]
    fn settings_ui_startup_focus_selector_persists_preference() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let ctx = egui::Context::default();
        ThemeTokens::apply_style(&ctx);
        let mut a = app().with_settings_path(path.clone(), Some(&ctx));
        assert_eq!(a.startup_focus, StartupFocus::Canvas);

        a.tab = 2;
        let out = run_ui_pass_frame(&ctx, &mut a, vec![]);
        let latex_btn = find_text_center(&out, |t| t == "LaTeX source");
        run_ui_pass_frame(&ctx, &mut a, click_events(latex_btn));

        assert_eq!(a.startup_focus, StartupFocus::Latex);
        let on_disk = Settings::load_from_path(&path);
        assert_eq!(on_disk.startup_focus, StartupFocus::Latex);

        let fresh_ctx = egui::Context::default();
        let reloaded = app().with_settings_path(path, Some(&fresh_ctx));
        assert_eq!(reloaded.startup_focus, StartupFocus::Latex);
    }

    #[test]
    fn startup_focus_applies_on_launch_and_return_to_editor() {
        let canvas_id = egui::Id::unique("equation-canvas");
        let latex_id = egui::Id::unique("latex-source");

        // Default startup focus activates Equation Canvas immediately.
        let ctx = egui::Context::default();
        ThemeTokens::apply_style(&ctx);
        let mut a = app();
        run_ui_pass_frame(&ctx, &mut a, vec![]);
        assert!(ctx.memory(|m| m.has_focus(canvas_id)));
        assert!(!ctx.memory(|m| m.has_focus(latex_id)));

        // Configured StartupFocus::Latex activates LaTeX source on launch and when returning from Settings.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        Settings {
            startup_focus: StartupFocus::Latex,
            ..Settings::default()
        }
        .save_to_path(&path)
        .unwrap();

        let latex_ctx = egui::Context::default();
        ThemeTokens::apply_style(&latex_ctx);
        let mut b = app().with_settings_path(path, Some(&latex_ctx));
        run_ui_pass_frame(&latex_ctx, &mut b, vec![]);
        assert!(latex_ctx.memory(|m| m.has_focus(latex_id)));
        assert!(!latex_ctx.memory(|m| m.has_focus(canvas_id)));

        // Open Settings and click "Back" -> focuses LaTeX source again.
        let out = run_ui_pass_frame(&latex_ctx, &mut b, vec![]);
        let settings_btn = find_text_center(&out, |t| t == "Settings");
        run_ui_pass_frame(&latex_ctx, &mut b, click_events(settings_btn));
        assert_eq!(b.tab, 2);
        let out = run_ui_pass_frame(&latex_ctx, &mut b, vec![]);
        let back_btn = find_text_center(&out, |t| t == "Back");
        run_ui_pass_frame(&latex_ctx, &mut b, click_events(back_btn));
        assert_eq!(b.tab, 0);
        run_ui_pass_frame(&latex_ctx, &mut b, vec![]);
        assert!(latex_ctx.memory(|m| m.has_focus(latex_id)));
    }

    #[test]
    fn editor_controls_preserve_active_focus_for_canvas_and_latex() {
        let canvas_id = egui::Id::unique("equation-canvas");
        let latex_id = egui::Id::unique("latex-source");

        let ctx = egui::Context::default();
        ThemeTokens::apply_style(&ctx);
        let mut a = app();
        let out = run_ui_pass_frame(&ctx, &mut a, vec![]);
        assert!(ctx.memory(|m| m.has_focus(canvas_id)));

        // Click a sidebar category tile ("Symbols") -> Equation Canvas retains focus.
        let symbols_pos = out
            .shapes
            .iter()
            .find_map(|s| {
                let rect = s.shape.visual_bounding_rect();
                if rect.min.x < 174.0
                    && rect.min.y > 40.0
                    && rect.min.y < 360.0
                    && (rect.height() - 42.0).abs() <= 1.0
                {
                    Some(rect.center())
                } else {
                    None
                }
            })
            .expect("sidebar category rect");
        let out = run_ui_pass_frame(&ctx, &mut a, click_events(symbols_pos));
        assert!(ctx.memory(|m| m.has_focus(canvas_id)));

        // Click Undo and MathType radio -> Equation Canvas retains focus.
        let undo_tex = a.ui_icon_texture_id("arrow-back-up").expect("undo icon");
        let undo_pos = out
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Rect(r)
                    if r.brush
                        .as_ref()
                        .is_some_and(|b| b.fill_texture_id == undo_tex) =>
                {
                    Some(r.rect.center())
                }
                _ => None,
            })
            .expect("undo icon rect");
        let out = run_ui_pass_frame(&ctx, &mut a, click_events(undo_pos));
        assert!(ctx.memory(|m| m.has_focus(canvas_id)));
        let mt_pos = find_text_center(&out, |t| t == "MathType");
        let _out = run_ui_pass_frame(&ctx, &mut a, click_events(mt_pos));
        assert!(ctx.memory(|m| m.has_focus(canvas_id)));

        // Click into LaTeX source box to switch active focus there.
        let latex_rect = ctx.read_response(latex_id).unwrap().rect;
        let out = run_ui_pass_frame(&ctx, &mut a, click_events(latex_rect.center()));
        assert!(ctx.memory(|m| m.has_focus(latex_id)));

        // Click palette button, Undo, Redo, and OMML radio -> focus stays on LaTeX source.
        let palette_pos = out
            .shapes
            .iter()
            .find_map(|s| {
                let rect = s.shape.visual_bounding_rect();
                if rect.min.x > 174.0
                    && rect.min.y < 140.0
                    && (rect.width() - PALETTE_BUTTON_SIZE.x).abs() <= 1.0
                    && (rect.height() - PALETTE_BUTTON_SIZE.y).abs() <= 1.0
                {
                    Some(rect.center())
                } else {
                    None
                }
            })
            .expect("palette button rect");
        let out = run_ui_pass_frame(&ctx, &mut a, click_events(palette_pos));
        assert!(ctx.memory(|m| m.has_focus(latex_id)));

        let undo_pos = out
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Rect(r)
                    if r.brush
                        .as_ref()
                        .is_some_and(|b| b.fill_texture_id == undo_tex) =>
                {
                    Some(r.rect.center())
                }
                _ => None,
            })
            .expect("undo icon rect");
        let out = run_ui_pass_frame(&ctx, &mut a, click_events(undo_pos));
        assert!(ctx.memory(|m| m.has_focus(latex_id)));

        let omml_pos = find_text_center(&out, |t| t == "OMML");
        run_ui_pass_frame(&ctx, &mut a, click_events(omml_pos));
        assert!(ctx.memory(|m| m.has_focus(latex_id)));
    }

    #[test]
    fn canvas_caret_blinks_when_idle_and_resets_on_activity() {
        let ctx = egui::Context::default();
        ThemeTokens::apply_style(&ctx);
        let mut a = app();
        let caret_stroke = CanvasPreset::SoftPaper.palette(false).caret;
        let run_at = |a: &mut App, time: f64, events: Vec<egui::Event>| {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 650.0))),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |ui| a.ui_pass(ui),
            );
            out.textures_delta.clear();
            out.shapes.iter().any(|s| matches!(&s.shape,
                egui::Shape::LineSegment { stroke, .. } if stroke.color == caret_stroke && (stroke.width - 1.5).abs() <= 0.1
            ))
        };

        // Initial frame focuses canvas and shows caret ON.
        assert!(run_at(&mut a, 0.0, vec![]));
        let cursor_style = ctx.style_of(ctx.theme()).visuals.text_cursor.clone();
        let (on_dur, off_dur) = (
            f64::from(cursor_style.on_duration),
            f64::from(cursor_style.off_duration),
        );
        // Midway through OFF phase -> caret hidden.
        assert!(!run_at(&mut a, on_dur + off_dur * 0.5, vec![]));
        // Typing resets blink phase immediately to ON even inside what was OFF window.
        assert!(run_at(
            &mut a,
            on_dur + off_dur * 0.6,
            vec![egui::Event::Text("x".into())]
        ));
        // After full on + off period from the activity timestamp -> ON again.
        assert!(run_at(
            &mut a,
            on_dur + off_dur * 0.6 + on_dur + off_dur + 0.05,
            vec![]
        ));
    }

    #[test]
    fn caret_rect_aligns_consistently_across_tokens_and_row_stops() {
        let ctx = egui::Context::default();
        let mut a = app();
        let math = MathNode::with_children(
            "math",
            vec![
                MathNode::leaf("mi", "x"),
                MathNode::leaf("mo", "+"),
                MathNode::leaf("mn", "1"),
            ],
        );
        a.editor.replace(&math);
        a.changed();
        a.sync_render(&ctx);

        let origin = Pos2::new(20.0, 40.0);
        let scale = 2.0 * 96.0 / 72.0;

        let root_id = a.editor.root().id;
        let x_id = a.editor.root().children[0].id;
        let plus_id = a.editor.root().children[1].id;
        let one_id = a.editor.root().children[2].id;

        let mut rects = Vec::new();
        for caret in [
            Caret {
                node: root_id,
                offset: 0,
            },
            Caret {
                node: x_id,
                offset: 0,
            },
            Caret {
                node: x_id,
                offset: 1,
            },
            Caret {
                node: plus_id,
                offset: 0,
            },
            Caret {
                node: plus_id,
                offset: 1,
            },
            Caret {
                node: one_id,
                offset: 0,
            },
            Caret {
                node: one_id,
                offset: 1,
            },
            Caret {
                node: root_id,
                offset: 3,
            },
        ] {
            a.editor.set_caret(caret, false);
            rects.push(a.caret_rect(origin, scale).expect("caret rect"));
        }

        let first = rects[0];
        for rect in &rects[1..] {
            assert!((rect.top() - first.top()).abs() < 0.01);
            assert!((rect.bottom() - first.bottom()).abs() < 0.01);
        }
        // End of <mn>1</mn> and end of <math> share the exact same X and Y coordinates.
        let end_of_one = rects[6];
        let end_of_root = rects[7];
        assert!((end_of_one.left() - end_of_root.left()).abs() < 0.01);
        assert!((end_of_one.top() - end_of_root.top()).abs() < 0.01);
        assert!((end_of_one.bottom() - end_of_root.bottom()).abs() < 0.01);

        // Subscript caret scales down proportionally to script size.
        let sub_math = eqntide::service::parse_latex_to_mathml("x_2").unwrap();
        a.editor.replace(&sub_math);
        a.changed();
        a.sync_render(&ctx);
        a.editor.move_caret(Move::Left, false);
        let sub_rect = a.caret_rect(origin, scale).expect("subscript caret rect");
        assert!(sub_rect.height() < first.height() * 0.8);
    }

    #[test]
    fn multiline_latex_scrolls_within_bounds_without_canvas_overlap() {
        let ctx = egui::Context::default();
        ThemeTokens::apply_style(&ctx);
        let mut a = app();
        let pal = a.active_canvas_palette(ctx.theme() == egui::Theme::Dark);
        let latex_id = egui::Id::unique("latex-source");

        let find_canvas_and_latex_frames = |output: &egui::FullOutput| {
            let canvas = output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Rect(r)
                        if r.fill == pal.bg
                            && r.corner_radius == theme::SURFACE_CORNER_RADIUS
                            && r.rect.width() > 100.0 =>
                    {
                        Some(r.rect)
                    }
                    _ => None,
                })
                .expect("canvas frame");
            let latex = output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Rect(r)
                        if r.corner_radius == theme::CONTROL_CORNER_RADIUS
                            && r.stroke == Stroke::new(1.0, pal.border)
                            && r.rect.width() > 300.0 =>
                    {
                        Some(r.rect)
                    }
                    _ => None,
                })
                .expect("latex frame");
            (canvas, latex)
        };

        let out = run_ui_pass_frame(&ctx, &mut a, vec![]);
        let (canvas_single, latex_single) = find_canvas_and_latex_frames(&out);
        assert!(canvas_single.bottom() + CONTENT_PADDING as f32 <= latex_single.top() + 1.0);
        assert!((canvas_single.left() - latex_single.left()).abs() <= 1.0);
        assert!((canvas_single.right() - latex_single.right()).abs() <= 1.0);
        // No scrollbar handle/track appears inside the empty LaTeX box.
        assert!(!out.shapes.iter().any(|s| matches!(&s.shape,
            egui::Shape::Rect(r) if latex_single.contains_rect(r.rect) && r.fill == pal.scroll_track)));

        // Clicking inside the LaTeX box focuses it without adding a bright focus border.
        let out = run_ui_pass_frame(&ctx, &mut a, click_events(latex_single.center()));
        assert!(ctx.memory(|m| m.has_focus(latex_id)));
        assert!(!out.shapes.iter().any(|s| matches!(&s.shape,
            egui::Shape::Rect(r) if r.rect == latex_single && r.stroke.color == pal.caret)));
        assert!(out.shapes.iter().any(|s| matches!(&s.shape,
            egui::Shape::Rect(r) if r.rect == latex_single && r.stroke.color == pal.border)));

        // Pressing Enter repeatedly to produce many lines keeps the LaTeX box bounded and scrollable.
        let mut events = Vec::new();
        for i in 1..=8 {
            events.push(egui::Event::Text(format!("line{i}")));
            events.push(egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            });
        }
        run_ui_pass_frame(&ctx, &mut a, events);
        let out = run_ui_pass_frame(&ctx, &mut a, vec![]);
        let (canvas_multi, latex_multi) = find_canvas_and_latex_frames(&out);
        assert_eq!(canvas_multi, canvas_single);
        assert_eq!(latex_multi, latex_single);
        assert!(canvas_multi.bottom() + CONTENT_PADDING as f32 <= latex_multi.top() + 1.0);

        let save_center = find_text_center(&out, |t| t == "Save MathML (Ctrl+S)");
        assert!(latex_multi.bottom() < save_center.y);

        // Rendered LaTeX lines stay clipped inside the bounded ScrollArea, never leaking out.
        let line_shapes: Vec<_> = out
            .shapes
            .iter()
            .filter(
                |s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text().contains("line")),
            )
            .collect();
        assert!(!line_shapes.is_empty());
        for s in line_shapes {
            assert!(
                s.clip_rect.top() >= latex_multi.top() - 0.5
                    && s.clip_rect.bottom() <= latex_multi.bottom() + 0.5,
                "latex text clip {:?} escaped frame {:?}",
                s.clip_rect,
                latex_multi
            );
        }
    }

    fn run_ui_pass_sized(ctx: &egui::Context, app: &mut App, size: Vec2) -> egui::FullOutput {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                ..Default::default()
            },
            |ui| app.ui_pass(ui),
        );
        output.textures_delta.clear();
        output
    }

    fn card_rects_for_section_titles(
        output: &egui::FullOutput,
        tokens: ThemeTokens,
        titles: &[&str],
    ) -> Vec<Rect> {
        titles
            .iter()
            .map(|title| {
                let title_pos = find_text_center(output, |t| t == *title);
                output
                    .shapes
                    .iter()
                    .find_map(|s| match &s.shape {
                        egui::Shape::Rect(r)
                            if r.fill == tokens.card_bg
                                && r.corner_radius == theme::SURFACE_CORNER_RADIUS
                                && r.rect.contains(title_pos) =>
                        {
                            Some(r.rect)
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| panic!("missing card rect containing '{title}'"))
            })
            .collect()
    }

    #[test]
    fn settings_cards_span_full_central_panel_width() {
        let ctx = egui::Context::default();
        ThemeTokens::apply_style(&ctx);
        ctx.set_theme(egui::Theme::Dark);
        let tokens = ThemeTokens::for_mode(true);
        let mut a = app();
        a.tab = 2;

        for width in [MIN_WINDOW_SIZE[0], 1400.0] {
            let size = Vec2::new(width, MIN_WINDOW_SIZE[1]);
            let _ = run_ui_pass_sized(&ctx, &mut a, size);
            let out = run_ui_pass_sized(&ctx, &mut a, size);
            let expected_width = width - 2.0 * theme::SPACE_LG;
            let cards = card_rects_for_section_titles(
                &out,
                tokens,
                &["Canvas zoom", "Appearance", "Startup focus"],
            );
            for rect in cards {
                assert!(
                    rect.width() >= expected_width - 24.0 && rect.width() <= expected_width + 0.5,
                    "Settings card width {} did not match panel width {} (rect={rect:?})",
                    rect.width(),
                    expected_width
                );
                assert!(
                    (rect.left() - theme::SPACE_LG).abs() <= 1.0,
                    "Settings card left {} misaligned at width {width}",
                    rect.left()
                );
            }
        }
    }

    #[test]
    fn settings_zoom_row_aligns_vertically_and_matches_handle_border_to_accent() {
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            let dark_mode = theme == egui::Theme::Dark;
            let tokens = ThemeTokens::for_mode(dark_mode);
            let ctx = egui::Context::default();
            ThemeTokens::apply_style(&ctx);
            ctx.set_theme(theme);
            let mut a = app();
            a.tab = 2;

            let _ = run_ui_pass_frame(&ctx, &mut a, vec![]);
            let out = run_ui_pass_frame(&ctx, &mut a, vec![]);

            let pct_pos = find_text_center(&out, |t| t == "200%");
            let reset_pos = find_text_center(&out, |t| t == "Reset (200%)");
            let control_rect = |pos: Pos2, label: &str| {
                out.shapes
                    .iter()
                    .find_map(|s| match &s.shape {
                        egui::Shape::Rect(r)
                            if r.brush.is_none()
                                && r.rect.contains(pos)
                                && r.rect.height() <= 40.0 =>
                        {
                            Some(r.rect)
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| panic!("missing rect for {label} in {theme:?}"))
            };
            let pct_rect = control_rect(pct_pos, "200%");
            let reset_rect = control_rect(reset_pos, "Reset (200%)");
            assert!((pct_rect.height() - theme::BUTTON_HEIGHT).abs() <= 0.5);
            assert!((reset_rect.height() - theme::BUTTON_HEIGHT).abs() <= 0.5);
            assert!(
                (pct_rect.center().y - reset_rect.center().y).abs() <= 0.5,
                "DragValue center {} != Reset center {} in {theme:?}",
                pct_rect.center().y,
                reset_rect.center().y
            );

            let rail_rect = out
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Rect(r)
                        if r.fill == tokens.border
                            && r.rect.width() >= 150.0
                            && r.rect.height() <= 12.0
                            && (r.rect.center().y - pct_rect.center().y).abs() < 12.0 =>
                    {
                        Some(r.rect)
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("missing zoom slider rail in {theme:?}"));
            assert!(
                (rail_rect.center().y - pct_rect.center().y).abs() <= 0.5,
                "Slider rail center {} != DragValue center {} in {theme:?}",
                rail_rect.center().y,
                pct_rect.center().y
            );

            let find_handle = |frame: &egui::FullOutput, label: &str| {
                frame
                    .shapes
                    .iter()
                    .find_map(|s| match &s.shape {
                        egui::Shape::Rect(r)
                            if (r.rect.center().y - rail_rect.center().y).abs() <= 1.0
                                && (r.rect.width() - r.rect.height()).abs() <= 0.5
                                && r.rect.height() >= 10.0
                                && r.rect.height() <= 18.0
                                && r.rect.left() >= rail_rect.left() - 2.0
                                && r.rect.right() <= rail_rect.right() + 2.0 =>
                        {
                            Some((r.rect, r.stroke))
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| panic!("missing zoom slider handle ({label}) in {theme:?}"))
            };

            let (idle_handle, idle_stroke) = find_handle(&out, "idle");
            assert_eq!(idle_stroke.color, tokens.accent);
            assert!((idle_handle.center().y - pct_rect.center().y).abs() <= 0.5);

            // Hovered handle preserves accent border color.
            let hovered_out = run_ui_pass_frame(
                &ctx,
                &mut a,
                vec![egui::Event::PointerMoved(idle_handle.center())],
            );
            let (_, hovered_stroke) = find_handle(&hovered_out, "hovered");
            assert_eq!(hovered_stroke.color, tokens.accent);

            // Active/dragged handle preserves accent border color.
            let active_out = run_ui_pass_frame(
                &ctx,
                &mut a,
                vec![egui::Event::PointerButton {
                    pos: idle_handle.center(),
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            let (_, active_stroke) = find_handle(&active_out, "active");
            assert_eq!(active_stroke.color, tokens.accent);
        }
    }

    #[test]
    fn about_card_spans_full_central_panel_width() {
        let ctx = egui::Context::default();
        ThemeTokens::apply_style(&ctx);
        ctx.set_theme(egui::Theme::Dark);
        let tokens = ThemeTokens::for_mode(true);
        let mut a = app();
        a.tab = 3;

        for width in [MIN_WINDOW_SIZE[0], 1400.0] {
            let size = Vec2::new(width, MIN_WINDOW_SIZE[1]);
            let _ = run_ui_pass_sized(&ctx, &mut a, size);
            let out = run_ui_pass_sized(&ctx, &mut a, size);
            let expected_width = width - 2.0 * theme::SPACE_LG;
            let cards = card_rects_for_section_titles(
                &out,
                tokens,
                &[concat!("EqnTide ", env!("CARGO_PKG_VERSION"))],
            );
            let rect = cards[0];
            assert!(
                rect.width() >= expected_width - 24.0 && rect.width() <= expected_width + 0.5,
                "About card width {} did not match panel width {} (rect={rect:?})",
                rect.width(),
                expected_width
            );
            assert!(
                (rect.left() - theme::SPACE_LG).abs() <= 1.0,
                "About card left {} misaligned at width {width}",
                rect.left()
            );
            let github_pos = find_text_center(&out, |t| t == env!("CARGO_PKG_REPOSITORY"));
            assert!(
                rect.contains(github_pos),
                "About metadata grid should sit inside full-width card {rect:?}"
            );
            let logo_tex = a.logo_texture.as_ref().expect("about logo texture").id();
            let logo_rect = out
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Rect(r)
                        if r.brush
                            .as_ref()
                            .is_some_and(|b| b.fill_texture_id == logo_tex) =>
                    {
                        Some(r.rect)
                    }
                    _ => None,
                })
                .expect("about logo rect");
            let title_rect = out
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t)
                        if t.galley.text() == concat!("EqnTide ", env!("CARGO_PKG_VERSION")) =>
                    {
                        Some(s.shape.visual_bounding_rect())
                    }
                    _ => None,
                })
                .expect("about title rect");
            let desc_rect = out
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t)
                        if t.galley.text().starts_with("Bidirectional math equation") =>
                    {
                        Some(s.shape.visual_bounding_rect())
                    }
                    _ => None,
                })
                .expect("about desc rect");
            let text_mid_y = (title_rect.top() + desc_rect.bottom()) * 0.5;
            assert!(
                (text_mid_y - logo_rect.center().y).abs() <= 2.5,
                "about header text ({text_mid_y}) should align vertically with logo ({})",
                logo_rect.center().y
            );
        }
    }

    #[test]
    fn batch_cards_and_empty_state_span_full_central_panel_width() {
        let ctx = egui::Context::default();
        ThemeTokens::apply_style(&ctx);
        ctx.set_theme(egui::Theme::Dark);
        let tokens = ThemeTokens::for_mode(true);
        let mut a = app();
        a.tab = 1;

        for width in [MIN_WINDOW_SIZE[0], 1400.0] {
            let size = Vec2::new(width, MIN_WINDOW_SIZE[1]);
            let expected_width = width - 2.0 * theme::SPACE_LG;

            // Empty state: only controls card renders (no empty drop-zone frame).
            a.files.clear();
            a.batch_logs.clear();
            let _ = run_ui_pass_sized(&ctx, &mut a, size);
            let out = run_ui_pass_sized(&ctx, &mut a, size);
            assert!(!out.shapes.iter().any(|s| match &s.shape {
                egui::Shape::Text(t) => t.galley.text() == "No .docx files queued",
                _ => false,
            }));
            let cards = card_rects_for_section_titles(&out, tokens, &["Add DOCX files…"]);
            for rect in cards {
                assert!(
                    rect.width() >= expected_width - 24.0 && rect.width() <= expected_width + 0.5,
                    "Batch empty-state card width {} did not match panel width {} (rect={rect:?})",
                    rect.width(),
                    expected_width
                );
                assert!(
                    (rect.left() - theme::SPACE_LG).abs() <= 1.0,
                    "Batch empty-state card left {} misaligned at width {width}",
                    rect.left()
                );
            }

            // Populated state: controls card + queued file row card span the full panel width.
            a.files = vec![batch::BatchItem::file(PathBuf::from(
                "/tmp/sample_document.docx",
            ))];
            a.batch_logs = vec!["Ready".into()];
            let _ = run_ui_pass_sized(&ctx, &mut a, size);
            let out = run_ui_pass_sized(&ctx, &mut a, size);
            let cards = card_rects_for_section_titles(
                &out,
                tokens,
                &["Add DOCX files…", "/tmp/sample_document.docx"],
            );
            for rect in cards {
                assert!(
                    rect.width() >= expected_width - 24.0 && rect.width() <= expected_width + 0.5,
                    "Batch populated card width {} did not match panel width {} (rect={rect:?})",
                    rect.width(),
                    expected_width
                );
                assert!(
                    (rect.left() - theme::SPACE_LG).abs() <= 1.0,
                    "Batch populated card left {} misaligned at width {width}",
                    rect.left()
                );
            }
        }
    }

    #[test]
    fn batch_tab_discovers_nested_folder_docx_files_and_targets_output_dir() {
        use std::io::{Cursor, Write};
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let opts = zip::write::SimpleFileOptions::default();
        zip.start_file("[Content_Types].xml", opts).unwrap();
        zip.write_all(br#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/></Types>"#).unwrap();
        zip.start_file("word/document.xml", opts).unwrap();
        zip.write_all(br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"><w:body><w:p><m:oMath><m:r><m:t>x</m:t></m:r></m:oMath></w:p></w:body></w:document>"#).unwrap();
        let docx_bytes = zip.finish().unwrap().into_inner();

        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("course_docs");
        let sub = folder.join("chapter1");
        let hidden = folder.join(".hidden");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::create_dir_all(&hidden).unwrap();

        let root_doc = folder.join("intro.docx");
        let nested_doc = sub.join("lesson1.docx");
        let lock_doc = sub.join("~$lesson1.docx");
        let hidden_doc = hidden.join("secret.docx");
        let extra_doc = dir.path().join("standalone.docx");
        for (path, bytes) in [
            (&root_doc, &docx_bytes[..]),
            (&nested_doc, &docx_bytes[..]),
            (&lock_doc, b"lock"),
            (&hidden_doc, &docx_bytes[..]),
            (&extra_doc, &docx_bytes[..]),
        ] {
            std::fs::write(path, bytes).unwrap();
        }

        let ctx = egui::Context::default();
        ThemeTokens::apply_style(&ctx);
        let mut a = app();
        a.tab = 1;
        a.batch_target = 1; // "mathtype"
        a.add_files(vec![folder.clone(), nested_doc.clone(), extra_doc.clone()]);

        assert_eq!(a.files.len(), 3);
        assert_eq!(a.files[0].input, nested_doc);
        assert_eq!(a.files[0].source_root.as_deref(), Some(folder.as_path()));
        assert_eq!(a.files[1].input, root_doc);
        assert_eq!(a.files[1].source_root.as_deref(), Some(folder.as_path()));
        assert_eq!(a.files[2].input, extra_doc);
        assert_eq!(a.files[2].source_root, None);

        let out = run_ui_pass_frame(&ctx, &mut a, vec![]);
        let add_folder_pos = find_text_center(&out, |t| t == "Add folder…");
        assert!(add_folder_pos.x > 0.0);

        let convert_pos = find_text_center(&out, |t| t == "Convert");
        run_ui_pass_frame(&ctx, &mut a, click_events(convert_pos));

        let deadline = Instant::now() + Duration::from_secs(10);
        while a.batch_running && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
            run_ui_pass_frame(&ctx, &mut a, vec![]);
        }
        assert!(!a.batch_running);

        let nested_out = a.batch_rows[0]
            .last_output
            .as_ref()
            .expect("nested folder item should record output metadata");
        let expected_out_dir = nested_out
            .root_output_dir
            .clone()
            .expect("folder batch should set timestamped root output folder");
        let out_name = expected_out_dir
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert!(
            out_name.starts_with("course_docs_mathtype_"),
            "unexpected output folder name: {out_name}"
        );
        assert_eq!(expected_out_dir.parent(), Some(dir.path()));
        assert_eq!(nested_out.file_dir, expected_out_dir.join("chapter1"));
        assert!(expected_out_dir.join("intro.docx").is_file());
        assert!(expected_out_dir
            .join("chapter1")
            .join("lesson1.docx")
            .is_file());
        assert!(dir.path().join("standalone_mathtype.docx").is_file());
        assert_eq!(
            a.batch_rows[2]
                .last_output
                .as_ref()
                .unwrap()
                .root_output_dir,
            None
        );
        assert_eq!(
            a.batch_rows[2].last_output.as_ref().unwrap().file_dir,
            dir.path()
        );

        // Default mode is Folder tree: root folder, subfolder, and standalone file show "Open folder";
        // folder-derived file rows (both under root and under subfolders) and Individual files header do not.
        let tree_out = run_ui_pass_sized(&ctx, &mut a, Vec2::new(1056.0, 900.0));
        let has_label_near_y = |frame: &egui::FullOutput, label: &str, y: f32| {
            frame.shapes.iter().any(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == label => {
                    (s.shape.visual_bounding_rect().center().y - y).abs() <= 14.0
                }
                _ => false,
            })
        };
        assert!(!tree_out.shapes.iter().any(|s| match &s.shape {
            egui::Shape::Text(t) => {
                t.galley.text() == "Open output folder"
                    || t.galley.text() == "Open root output folder"
            }
            _ => false,
        }));
        let root_group_y = find_text_center(&tree_out, |t| t == "course_docs").y;
        let sub_group_y = find_text_center(&tree_out, |t| t == "chapter1").y;
        let tree_nested_y = find_text_center(&tree_out, |t| t == "lesson1.docx").y;
        let tree_intro_y = find_text_center(&tree_out, |t| t == "intro.docx").y;
        let individual_group_y = find_text_center(&tree_out, |t| t == "Individual files").y;
        let tree_solo_y = find_text_center(&tree_out, |t| t.ends_with("standalone.docx")).y;
        assert!(has_label_near_y(&tree_out, "Open folder", root_group_y));
        assert!(has_label_near_y(&tree_out, "Open folder", sub_group_y));
        assert!(!has_label_near_y(&tree_out, "Open folder", tree_nested_y));
        assert!(!has_label_near_y(&tree_out, "Open folder", tree_intro_y));
        assert!(!has_label_near_y(
            &tree_out,
            "Open folder",
            individual_group_y
        ));
        assert!(has_label_near_y(&tree_out, "Open folder", tree_solo_y));

        // Switch to File tree: every converted file row shows one "Open folder" button.
        a.batch_selection_mode = batch::BatchSelectionMode::FlatList;
        let post_out = run_ui_pass_sized(&ctx, &mut a, Vec2::new(1056.0, 900.0));
        let nested_y = find_text_center(&post_out, |t| t.ends_with("lesson1.docx")).y;
        let root_file_y = find_text_center(&post_out, |t| t.ends_with("intro.docx")).y;
        let solo_y = find_text_center(&post_out, |t| t.ends_with("standalone.docx")).y;
        assert!(has_label_near_y(&post_out, "Open folder", nested_y));
        assert!(has_label_near_y(&post_out, "Open folder", root_file_y));
        assert!(has_label_near_y(&post_out, "Open folder", solo_y));
    }

    #[test]
    fn desktop_ui_views_render_assigned_tabler_icons() {
        let assert_painted = |a: &App, out: &egui::FullOutput, icon: &str, tint: Color32| {
            let tex = a
                .ui_icon_texture_id(icon)
                .unwrap_or_else(|| panic!("missing icon texture {icon}"));
            assert!(
                out.shapes.iter().any(|s| match &s.shape {
                    egui::Shape::Rect(r) => {
                        r.brush.as_ref().is_some_and(|b| b.fill_texture_id == tex) && r.fill == tint
                    }
                    _ => false,
                }),
                "expected painted icon {icon} with tint {tint:?}"
            );
        };

        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            let tokens = ThemeTokens::for_mode(theme == egui::Theme::Dark);
            let ctx = egui::Context::default();
            ThemeTokens::apply_style(&ctx);
            ctx.set_theme(theme);
            let mut a = app();

            // Editor view: sidebar tabs + Undo/Redo.
            let editor_out = run_ui_pass_frame(&ctx, &mut a, vec![]);
            for icon in [
                "file-type-docx",
                "settings",
                "info-circle",
                "arrow-back-up",
                "arrow-forward-up",
            ] {
                assert_painted(&a, &editor_out, icon, tokens.text_primary);
            }

            // Batch DOCX view (empty drop-zone + buttons).
            a.tab = 1;
            let batch_out = run_ui_pass_frame(&ctx, &mut a, vec![]);
            for icon in ["chevron-left", "file", "folder"] {
                assert_painted(&a, &batch_out, icon, tokens.text_primary);
            }
            assert_painted(
                &a,
                &batch_out,
                "arrows-left-right",
                tokens
                    .text_primary
                    .gamma_multiply(ctx.style_of(theme).visuals.disabled_alpha()),
            );

            // Settings view.
            a.tab = 2;
            let settings_out = run_ui_pass_frame(&ctx, &mut a, vec![]);
            assert_painted(&a, &settings_out, "chevron-left", tokens.text_primary);

            // About view.
            a.tab = 3;
            let about_out = run_ui_pass_frame(&ctx, &mut a, vec![]);
            assert_painted(&a, &about_out, "chevron-left", tokens.text_primary);
            for icon in ["user", "license", "brand-github"] {
                assert_painted(&a, &about_out, icon, tokens.text_muted);
            }
        }
    }

    #[test]
    fn batch_subset_execution_upgrades_source_root_and_tracks_multi_folder_outputs() {
        use std::io::{Cursor, Write};
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let opts = zip::write::SimpleFileOptions::default();
        zip.start_file("[Content_Types].xml", opts).unwrap();
        zip.write_all(br#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/></Types>"#).unwrap();
        zip.start_file("word/document.xml", opts).unwrap();
        zip.write_all(br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"><w:body><w:p><m:oMath><m:r><m:t>x</m:t></m:r></m:oMath></w:p></w:body></w:document>"#).unwrap();
        let docx_bytes = zip.finish().unwrap().into_inner();

        let dir_a = tempfile::tempdir().unwrap();
        let dir_b = tempfile::tempdir().unwrap();
        let folder_a = dir_a.path().join("folder_a");
        let sub_a = folder_a.join("ch1");
        let folder_b = dir_b.path().join("folder_b");
        std::fs::create_dir_all(&sub_a).unwrap();
        std::fs::create_dir_all(&folder_b).unwrap();

        let doc_a1 = folder_a.join("a1.docx");
        let doc_a2 = sub_a.join("a2.docx");
        let doc_b1 = folder_b.join("b1.docx");
        for path in [&doc_a1, &doc_a2, &doc_b1] {
            std::fs::write(path, &docx_bytes).unwrap();
        }

        let ctx = egui::Context::default();
        ThemeTokens::apply_style(&ctx);
        let mut a = app();
        a.tab = 1;
        a.batch_target = 1; // "mathtype"
                            // Add doc_a1 individually first, then add folder_a (should upgrade source_root), then folder_b.
        a.add_files(vec![doc_a1.clone()]);
        assert_eq!(a.files[0].source_root, None);
        a.add_files(vec![folder_a.clone()]);
        assert_eq!(a.files.len(), 2);
        assert_eq!(a.files[0].source_root.as_deref(), Some(folder_a.as_path()));
        a.add_files(vec![folder_b.clone()]);
        assert_eq!(a.files.len(), 3);

        // Uncheck doc_a1 (index 0) in default FolderTree mode; run only doc_a2 (1) and doc_b1 (2).
        a.batch_rows[0].set_selected(batch::BatchSelectionMode::FolderTree, false);
        assert_eq!(a.selected_batch_count(), 2);

        let out = run_ui_pass_frame(&ctx, &mut a, vec![]);
        let convert_pos = find_text_center(&out, |t| t == "Convert");
        run_ui_pass_frame(&ctx, &mut a, click_events(convert_pos));

        let deadline = Instant::now() + Duration::from_secs(10);
        while a.batch_running && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
            run_ui_pass_frame(&ctx, &mut a, vec![]);
        }
        assert!(!a.batch_running);
        assert_eq!(a.batch_run_total, 2);
        assert_eq!(a.batch_completed, 2);
        assert_eq!(a.batch_rows[0].last_output, None);
        assert_eq!(a.batch_logs[0], "Ready");

        let out_a2 = a.batch_rows[1].last_output.as_ref().expect("a2 output");
        assert_eq!(
            out_a2.file_dir,
            out_a2.root_output_dir.as_ref().unwrap().join("ch1")
        );
        assert_eq!(
            out_a2.root_output_dir.as_ref().unwrap().parent(),
            Some(dir_a.path())
        );

        let out_b1 = a.batch_rows[2].last_output.as_ref().expect("b1 output");
        assert_eq!(
            out_b1.file_dir,
            out_b1.root_output_dir.as_ref().unwrap().to_path_buf()
        );
        assert_eq!(
            out_b1.root_output_dir.as_ref().unwrap().parent(),
            Some(dir_b.path())
        );
    }

    #[test]
    fn batch_toolbar_groups_and_disabled_states_are_consistent() {
        let ctx = egui::Context::default();
        ThemeTokens::apply_style(&ctx);
        ctx.set_theme(egui::Theme::Dark);
        let tokens = ThemeTokens::for_mode(true);
        let disabled_alpha = ctx.style_of(egui::Theme::Dark).visuals.disabled_alpha();
        let expected_disabled_text = tokens.text_primary.gamma_multiply(disabled_alpha);

        let mut a = app();
        a.tab = 1;
        let _ = run_ui_pass_frame(&ctx, &mut a, vec![]);
        let empty_out = run_ui_pass_frame(&ctx, &mut a, vec![]);

        // Toolbar sections exist; global "Open output folder" button is removed.
        assert!(find_text_center(&empty_out, |t| t == "Input").x > 0.0);
        assert!(find_text_center(&empty_out, |t| t == "Options & Actions").x > 0.0);
        assert!(!empty_out.shapes.iter().any(|s| match &s.shape {
            egui::Shape::Text(t) => t.galley.text() == "Open output folder",
            _ => false,
        }));

        // Disabled Convert, Stop, and Clear list share identical text color.
        let text_color_for = |out: &egui::FullOutput, label: &str| {
            out.shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.text() == label => {
                        Some(t.override_text_color.unwrap_or(t.fallback_color))
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("missing rendered text {label:?}"))
        };
        assert_eq!(
            text_color_for(&empty_out, "Convert"),
            expected_disabled_text
        );
        assert_eq!(text_color_for(&empty_out, "Stop"), expected_disabled_text);
        assert_eq!(
            text_color_for(&empty_out, "Clear list"),
            expected_disabled_text
        );

        // Populated queue with all items unchecked: Clear list enabled, Convert disabled.
        a.files = vec![batch::BatchItem::file(PathBuf::from("/tmp/doc1.docx"))];
        a.sync_batch_row_state();
        a.batch_rows[0].set_selected(batch::BatchSelectionMode::FolderTree, false);
        let unchecked_out = run_ui_pass_frame(&ctx, &mut a, vec![]);
        assert_eq!(
            text_color_for(&unchecked_out, "Clear list"),
            tokens.text_primary
        );
        assert_eq!(
            text_color_for(&unchecked_out, "Convert"),
            expected_disabled_text
        );

        // Clicking Clear list clears everything; re-selecting enables Convert.
        a.batch_rows[0].set_selected(batch::BatchSelectionMode::FolderTree, true);
        let ready_out = run_ui_pass_frame(&ctx, &mut a, vec![]);
        assert_eq!(text_color_for(&ready_out, "Convert"), tokens.primary_fg);
        let clear_pos = find_text_center(&ready_out, |t| t == "Clear list");
        let _ = run_ui_pass_frame(&ctx, &mut a, click_events(clear_pos));
        assert!(a.files.is_empty());
        assert!(a.batch_rows.is_empty());
    }

    #[test]
    fn batch_dual_selection_modes_preserve_independent_state_and_tree_controls() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("course");
        let sub = folder.join("ch1");
        std::fs::create_dir_all(&sub).unwrap();
        let f_root = folder.join("intro.docx");
        let f_sub1 = sub.join("a.docx");
        let f_sub2 = sub.join("b.docx");
        let f_solo = dir.path().join("solo.docx");
        for p in [&f_root, &f_sub1, &f_sub2, &f_solo] {
            std::fs::write(p, b"placeholder").unwrap();
        }

        let ctx = egui::Context::default();
        ThemeTokens::apply_style(&ctx);
        let mut a = app();
        a.tab = 1;
        a.add_files(vec![folder.clone(), f_solo.clone()]);
        assert_eq!(a.files.len(), 4);
        assert_eq!(
            a.batch_selection_mode,
            batch::BatchSelectionMode::FolderTree
        );
        assert_eq!(a.selected_batch_count(), 4);

        // Default Folder tree layout & header assertions.
        let tree_out = run_ui_pass_sized(&ctx, &mut a, Vec2::new(1056.0, 900.0));
        assert!(!tree_out.shapes.iter().any(|s| match &s.shape {
            egui::Shape::Text(t) => {
                let txt = t.galley.text();
                txt == "Ready" || txt == "Waiting" || txt == "View:" || txt.starts_with("Selected:")
            }
            _ => false,
        }));
        let progress_pos = find_text_center(&tree_out, |t| t == "0/4 files");
        let deselect_pos = find_text_center(&tree_out, |t| t == "Deselect all");
        let found_pos = find_text_center(&tree_out, |t| t.starts_with("Found:"));
        let flat_pos = find_text_center(&tree_out, |t| t == "File tree");
        assert!((progress_pos.y - deselect_pos.y).abs() <= 2.0);
        assert!(deselect_pos.x > progress_pos.x);
        assert!((found_pos.y - flat_pos.y).abs() <= 2.0);
        assert!(flat_pos.x > found_pos.x);
        assert!(found_pos.y > progress_pos.y);

        // Folder tree renders root, subfolder, and Individual files headers with clear indentation.
        let text_left = |frame: &egui::FullOutput, label: &str| {
            frame
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.text() == label => {
                        Some(s.shape.visual_bounding_rect().left())
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("missing label {label:?}"))
        };
        let course_left = text_left(&tree_out, "course");
        let ch1_left = text_left(&tree_out, "ch1");
        let leaf_left = text_left(&tree_out, "a.docx");
        assert!(ch1_left > course_left + 16.0);
        assert!(leaf_left > ch1_left);
        assert!(find_text_center(&tree_out, |t| t == "Individual files").x > 0.0);

        // Collapse ch1 by clicking its "-" button.
        let ch1_y = find_text_center(&tree_out, |t| t == "ch1").y;
        let ch1_toggle_pos = tree_out
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t)
                    if t.galley.text() == "-"
                        && (s.shape.visual_bounding_rect().center().y - ch1_y).abs() <= 14.0 =>
                {
                    Some(s.shape.visual_bounding_rect().center())
                }
                _ => None,
            })
            .expect("ch1 collapse toggle");
        let _ = run_ui_pass_frame(&ctx, &mut a, click_events(ch1_toggle_pos));
        let collapsed_out = run_ui_pass_sized(&ctx, &mut a, Vec2::new(1056.0, 900.0));
        assert!(!collapsed_out.shapes.iter().any(|s| match &s.shape {
            egui::Shape::Text(t) => t.galley.text() == "a.docx" || t.galley.text() == "b.docx",
            _ => false,
        }));

        // Click ch1's parent checkbox to uncheck both files inside ch1 -> course shows 1/3 selected.
        let ch1_row_y = find_text_center(&collapsed_out, |t| t == "ch1").y;
        let toggle_x = ch1_toggle_pos.x;
        // Checkbox sits immediately to the right of the +/- compact button on the ch1 row.
        let ch1_checkbox_pos = Pos2::new(toggle_x + 22.0, ch1_row_y);
        let _ = run_ui_pass_frame(&ctx, &mut a, click_events(ch1_checkbox_pos));
        let after_ch1_uncheck = run_ui_pass_sized(&ctx, &mut a, Vec2::new(1056.0, 900.0));
        assert_eq!(a.selected_batch_count(), 2); // intro.docx + solo.docx
        assert!(find_text_center(&after_ch1_uncheck, |t| t == "1/3 selected").x > 0.0);

        // Click course's indeterminate parent checkbox -> selects all 3 files in course (4 total).
        let course_row_y = find_text_center(&after_ch1_uncheck, |t| t == "course").y;
        let course_toggle_x = after_ch1_uncheck
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t)
                    if t.galley.text() == "-"
                        && (s.shape.visual_bounding_rect().center().y - course_row_y).abs()
                            <= 14.0 =>
                {
                    Some(s.shape.visual_bounding_rect().center().x)
                }
                _ => None,
            })
            .expect("course collapse toggle");
        let _ = run_ui_pass_frame(
            &ctx,
            &mut a,
            click_events(Pos2::new(course_toggle_x + 22.0, course_row_y)),
        );
        assert_eq!(a.selected_batch_count(), 4);

        // Uncheck one in Folder tree, switch to File tree -> all 4 still checked in File tree.
        a.batch_rows[1].tree_selected = false;
        assert_eq!(a.selected_batch_count(), 3);
        let flat_btn = find_text_center(&after_ch1_uncheck, |t| t == "File tree");
        let _ = run_ui_pass_frame(&ctx, &mut a, click_events(flat_btn));
        assert_eq!(a.batch_selection_mode, batch::BatchSelectionMode::FlatList);
        assert_eq!(a.selected_batch_count(), 4);

        // Uncheck in File tree via Deselect all, re-enable 2 items, then switch back to Folder tree ->
        // Folder tree still retains its 3 selections!
        let flat_out = run_ui_pass_sized(&ctx, &mut a, Vec2::new(1056.0, 900.0));
        let flat_deselect = find_text_center(&flat_out, |t| t == "Deselect all");
        let _ = run_ui_pass_frame(&ctx, &mut a, click_events(flat_deselect));
        assert_eq!(a.selected_batch_count(), 0);
        assert_eq!(a.files.len(), 4);
        a.batch_rows[0].flat_selected = true;
        a.batch_rows[3].flat_selected = true;
        assert_eq!(a.selected_batch_count(), 2);
        let two_selected_out = run_ui_pass_sized(&ctx, &mut a, Vec2::new(1056.0, 900.0));
        assert!(find_text_center(&two_selected_out, |t| t == "0/2 files").x > 0.0);

        let tree_btn = find_text_center(&two_selected_out, |t| t == "Folder tree");
        let _ = run_ui_pass_frame(&ctx, &mut a, click_events(tree_btn));
        assert_eq!(
            a.batch_selection_mode,
            batch::BatchSelectionMode::FolderTree
        );
        assert_eq!(a.selected_batch_count(), 3);
        assert!(a.batch_rows[0].flat_selected);
        assert!(!a.batch_rows[1].flat_selected);
        assert!(!a.batch_rows[2].flat_selected);
        assert!(a.batch_rows[3].flat_selected);
    }

    #[test]
    fn panel_dividers_draw_over_opaque_fills_and_clear_color_matches_theme() {
        let a = app();
        let light_tokens = ThemeTokens::for_mode(false);
        let dark_tokens = ThemeTokens::for_mode(true);
        assert_eq!(
            eframe::App::clear_color(&a, &egui::Visuals::light()),
            light_tokens.app_bg.to_normalized_gamma_f32()
        );
        assert_eq!(
            eframe::App::clear_color(&a, &egui::Visuals::dark()),
            dark_tokens.app_bg.to_normalized_gamma_f32()
        );

        let ctx = egui::Context::default();
        ThemeTokens::apply_style(&ctx);
        let mut a = app();
        a.persisted.theme = ThemeMode::Light;
        ctx.set_theme(egui::Theme::Light);
        let out = run_ui_pass_sized(&ctx, &mut a, Vec2::new(1100.0, 650.0));

        // Both CentralPanel (app_bg) and app-footer (panel_bg) meet flush with no 1px gap.
        let mut central_max_y = None;
        let mut footer_min_y = None;
        let mut central_idx = None;
        let mut footer_divider_idx = None;
        let mut sidebar_divider_idx = None;
        for (idx, clipped) in out.shapes.iter().enumerate() {
            match &clipped.shape {
                egui::Shape::Rect(r) if r.fill == light_tokens.app_bg && r.rect.width() > 800.0 => {
                    central_max_y = Some(r.rect.max.y);
                    central_idx = Some(idx);
                }
                egui::Shape::Rect(r)
                    if r.fill == light_tokens.panel_bg && (r.rect.height() - 56.0).abs() < 0.01 =>
                {
                    footer_min_y = Some(r.rect.min.y);
                }
                egui::Shape::LineSegment { points, stroke }
                    if stroke.color == light_tokens.border && stroke.width == 1.0 =>
                {
                    if (points[0].y - points[1].y).abs() < 0.01
                        && (points[1].x - points[0].x) > 800.0
                    {
                        footer_divider_idx = Some(idx);
                    } else if (points[0].x - points[1].x).abs() < 0.01
                        && (points[1].y - points[0].y) > 500.0
                    {
                        sidebar_divider_idx = Some(idx);
                    }
                }
                _ => {}
            }
        }
        let central_max_y = central_max_y.expect("central panel fill");
        let footer_min_y = footer_min_y.expect("footer panel fill");
        assert!((central_max_y - footer_min_y).abs() < 0.01);
        let central_idx = central_idx.expect("central panel index");
        assert!(footer_divider_idx.expect("footer divider") > central_idx);
        assert!(sidebar_divider_idx.expect("sidebar divider") > central_idx);
    }
}
