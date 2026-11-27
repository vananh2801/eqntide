//! Native Rust desktop application state, frame orchestration, and main window chrome.
//!
//! # Lifecycle & Async Event Flow
//!
//! ```text
//!  main / wgpu_runner / eframe::App
//!        │
//!        ▼
//!  App::new_with_state() ──► runtime::start_bridge() / IPC Instance listener
//!        │
//!        ▼
//!  EventSender<Event> ──► Receiver<Event> (tx / rx)
//!        │                     ▲
//!        ▼                     │ Launch, Window, Bridge, Render, Latex,
//!  App::draw_ui(ui)            │ Session, Finished, Paths, Background*, Batch*, UpdateCheck
//!    ├── process_events() ─────┘
//!    ├── take_save_shortcut() / close guard
//!    ├── sync_render() ──────► eqntide::render::render_interactive_alpha_scaled
//!    ├── draw_shell(ui) ─────► draw_status_bar / draw_sidebar / draw_footer / tab:
//!    │                           ├── draw_editor()       (palette + canvas + latex + keyboard)
//!    │                           ├── draw_batch_tab()    (folder tree / batch worker progress)
//!    │                           ├── draw_settings_tab() (theme, background, startup focus, shortcuts)
//!    │                           └── draw_about_tab()    (version, links, manual update check)
//!    ├── draw_dialogs() / draw_matrix_dialog()
//!    ├── request_latex_parse() ──► debounced background parse -> Event::Latex
//!    └── drain_before_close() / flush_settings()
//! ```

mod about_ui;
mod batch_ui;
mod canvas;
mod dialogs;
mod events;
mod keyboard;
mod palette_ui;
mod session;
mod settings_ui;
mod shortcuts_ui;

use crate::EventSender as Sender;
use crate::{
    batch, commands,
    instance::Instance,
    palette, runtime,
    settings::{self, BackgroundMode, Settings, StartupFocus},
    shortcuts::{ShortcutChord, ShortcutConflict, ShortcutMap},
    theme::{self, ThemeTokens},
    update::UpdateState,
    Event, Rendered,
};
use eframe::egui::{self, Rect, Stroke, Vec2};
use eqntide::bridge::{DesktopWindowRequest, ServerHandle};
use eqntide_editor::{Caret, Editor};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::PathBuf,
    sync::{atomic::AtomicBool, mpsc::Receiver, Arc},
    time::Instant,
};

pub(super) const CONTENT_PADDING: i8 = theme::SPACE_MD as i8;
const FONT_SIZE_CONTROL_WIDTH: f32 = 96.0;
const ZOOM_CONTROL_WIDTH: f32 = 96.0;
const FOOTER_CONTROL_HEIGHT: f32 = theme::BUTTON_HEIGHT;
pub(crate) const MIN_WINDOW_SIZE: [f32; 2] = [1056.0, 560.0];
pub(super) const PALETTE_BUTTON_SIZE: Vec2 = Vec2::new(46.0, 32.0);
pub(super) const PALETTE_GAP: f32 = theme::SPACE_XS;
const SIDEBAR_CATEGORY_ICON_SIZE: f32 = 22.0;
pub(super) const UI_ICON_SIZE: f32 = 16.0;
pub(super) const PALETTE_HEIGHT: f32 =
    3.0 * PALETTE_BUTTON_SIZE.y + 2.0 * PALETTE_GAP + CONTENT_PADDING as f32;
pub(super) const LATEX_PANEL_HEIGHT: f32 = 88.0;
pub(super) const LATEX_INPUT_HEIGHT: f32 = LATEX_PANEL_HEIGHT - CONTENT_PADDING as f32;
pub(super) const PLACEHOLDER_ATTR: &str = "data-eqntide-placeholder";

/// Rendering hardware capability tier controlling animation and raster scaling choices.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum GpuProfile {
    Native,
    VirtualGpu,
    Software,
}

/// Modal form state for inserting a custom `mtable` with chosen dimensions and delimiters.
#[derive(Clone)]
pub(super) struct MatrixDialog {
    pub(super) rows: usize,
    pub(super) columns: usize,
    pub(super) alignment: &'static str,
    pub(super) fence: &'static str,
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

/// Category filter applied to the customizable palette shortcuts table in Settings.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum ShortcutCategoryFilter {
    #[default]
    All,
    AssignedOnly,
    Group(&'static str),
}

/// Modal confirmation state when assigning a palette shortcut collides with an existing binding.
#[derive(Clone)]
pub(super) struct ShortcutConflictDialog {
    pub(super) target_source: &'static str,
    pub(super) attempted_chord: ShortcutChord,
    pub(super) target_previous: Option<ShortcutChord>,
    pub(super) conflict: ShortcutConflict,
}

/// Pending window or session transition waiting on unsaved changes or active batch work.
#[derive(Clone, Copy)]
pub(super) enum Intent {
    Close,
    Next,
}

/// UV bounds and DIP display dimensions for a single icon in [`PaletteAtlas`].
pub(super) struct PaletteAtlasEntry {
    /// Normalized texture UV rectangle inside the sprite sheet.
    pub(super) uv: Rect,
    /// Logical display size in DIP (`40x26`).
    pub(super) display_size: Vec2,
}

/// Shared pre-rasterized sprite atlas and lookup map for all palette entries.
pub(super) struct PaletteAtlas {
    /// Uploaded GPU texture handle backing all palette icons.
    pub(super) texture: egui::TextureHandle,
    /// Lookup table keyed by canonical `PALETTE` LaTeX template source.
    pub(super) entries: HashMap<&'static str, PaletteAtlasEntry>,
}

/// Top-level native desktop application state shared across editor, batch, and settings views.
pub struct App {
    // ── Instance & Event Channel ─────────────────────────────────────────────
    /// Single-instance lock and IPC listener handle.
    pub(super) _instance: Option<Instance>,
    /// Sender used by background workers and UI actions to queue [`Event`] messages.
    pub(super) tx: Sender<Event>,
    /// Receiver drained each frame by `process_events()`.
    pub(super) rx: Receiver<Event>,

    // ── Office Bridge & Session Queue ────────────────────────────────────────
    /// Hosted HTTP bridge handle when desktop owns port `17890`.
    pub(super) bridge: Option<ServerHandle>,
    /// Whether the Office bridge is reachable (hosted or external).
    pub(super) bridge_ready: bool,
    /// Human-readable bridge status shown in the status-bar indicator tooltip.
    pub(super) bridge_status: String,
    /// Active Office editing session ID, if opened from Word/PowerPoint.
    pub(super) session: Option<String>,
    /// Session ID currently being fetched in the background.
    pub(super) loading_session: Option<String>,
    /// Pending window/session requests waiting behind an active or dirty equation.
    pub(super) queue: VecDeque<DesktopWindowRequest>,
    /// Launch arguments buffered before `bridge_ready` becomes true.
    pub(super) startup: Vec<String>,
    /// True while a session fetch, save/cancel post, or close drain is in flight.
    pub(super) busy: bool,
    /// Set by clicking Save; processed after widgets update in `draw_ui()`.
    pub(super) save_clicked: bool,
    /// Pending transition (`Close` or `Next`) awaiting unsaved-changes confirmation.
    pub(super) pending: Option<Intent>,
    /// Set once clean/drained so the viewport may close without prompting again.
    pub(super) allow_close: bool,
    /// Deadline bounding how long close waits for pending bridge long-polls.
    pub(super) close_deadline: Option<Instant>,

    // ── Editor & Document Snapshot ───────────────────────────────────────────
    /// Structural MathML equation editor state and undo/redo history.
    pub(super) editor: Editor,
    /// Clean MathML XML baseline used for dirty detection.
    pub(super) saved_xml: String,
    /// True when the clean baseline equation started empty (`Edit` vs `Insert`).
    pub(super) saved_empty: bool,
    /// Clean font size (pt) baseline used for dirty detection.
    pub(super) saved_font: f32,
    /// Clean export target (`"omml"` or `"mathtype"`) baseline.
    pub(super) saved_mode: String,
    /// Active equation font size in points.
    pub(super) font_pt: f32,
    /// Active Office output target (`"omml"` or `"mathtype"`).
    pub(super) target: String,
    /// Canvas preview zoom factor.
    pub(super) zoom: f32,

    // ── LaTeX Source & Command Buffer ────────────────────────────────────────
    /// Live LaTeX source buffer synchronized with `editor`.
    pub(super) latex: String,
    /// True while `latex` has unparsed user edits not yet reflected in `editor`.
    pub(super) draft_dirty: bool,
    /// Monotonic counter invalidating stale background LaTeX parse responses.
    pub(super) latex_generation: u64,
    /// Debounce deadline after typing in the LaTeX box before spawning a parse.
    pub(super) latex_deadline: Option<Instant>,
    /// Generation currently being parsed by a background thread.
    pub(super) latex_parsing: Option<u64>,
    /// Validation/parse error from the LaTeX source editor.
    pub(super) latex_error: Option<String>,
    /// Validation/parse error from the canvas inline `\command` popup.
    pub(super) command_error: Option<String>,
    /// Active IME preedit composition string on the equation canvas.
    pub(super) ime: String,
    /// State for the inline canvas `\command` autocomplete popup when open.
    pub(super) command_input: Option<commands::Input>,
    /// Stack of open `{...}` placeholder scopes tracked during fast canvas typing.
    pub(super) latex_groups: Vec<u64>,
    /// True right after inserting a template whose next `{` should scope its slot.
    pub(super) expects_latex_group: bool,
    /// Prevents running `handle_canvas_keyboard()` twice between `draw_shell()` and `draw_canvas()`.
    pub(super) keyboard_handled: bool,
    /// Blink timer state `(caret, revision, cmd_len, phase_start)`.
    pub(super) caret_blink: Option<(Caret, u64, usize, f64)>,

    // ── Navigation, Palette & Icons ──────────────────────────────────────────
    /// Active main view tab (`0` Editor, `1` Batch DOCX, `2` Settings, `3` About).
    pub(super) tab: usize,
    /// Selected palette category ID in the sidebar.
    pub(super) category: String,
    /// State for the modal custom matrix dimensions/fence dialog.
    pub(super) matrix_dialog: Option<MatrixDialog>,
    /// Shared pre-rasterized sprite atlas covering all palette symbols and templates.
    pub(super) palette_atlas: Option<PaletteAtlas>,
    /// Loaded Tabler SVG/PNG icon textures keyed by icon name.
    pub(super) ui_icons: HashMap<&'static str, egui::TextureHandle>,
    /// Most-recently-used palette template sources.
    pub(super) recent_palette: Vec<&'static str>,
    /// Cached application logo texture for the About view.
    pub(super) logo_texture: Option<egui::TextureHandle>,
    /// Current state of the manual About-tab GitHub Release update check.
    pub(super) update_state: UpdateState,
    /// Monotonic sequence ID discarding stale background update check responses.
    pub(super) update_check_seq: u64,
    /// Transient informational or error message displayed in the status bar.
    pub(super) status: String,
    /// Cached compatibility warning `(render_key, latex_gen, target, message)`.
    pub(super) warning_cache: Option<(u64, u64, String, Option<String>)>,

    // ── Canvas Render & Display ──────────────────────────────────────────────
    /// Detected GPU tier controlling animation and raster scaling choices.
    pub(super) gpu_profile: GpuProfile,
    /// Last enforced `(viewport_size, fullscreen_or_max)` to avoid redundant commands.
    pub(super) last_enforced_viewport: Option<(Vec2, bool)>,
    /// Uploaded alpha-mask texture for the current equation preview.
    pub(super) texture: Option<egui::TextureHandle>,
    /// Latest interactive preview hit regions and placeholder bounding boxes.
    pub(super) rendered: Option<Rendered>,
    /// Monotonic revision bumped whenever the equation or font size changes.
    pub(super) render_key: u64,
    /// Last `render_key` submitted to the renderer.
    pub(super) requested_key: u64,
    /// True while an equation render pass is active.
    pub(super) render_busy: bool,

    // ── Batch DOCX Conversion ────────────────────────────────────────────────
    /// Queued `.docx` files and folder items in the Batch tab.
    pub(super) files: Vec<batch::BatchItem>,
    /// Per-item selection flags and last output paths parallel to `files`.
    pub(super) batch_rows: Vec<batch::BatchRowState>,
    /// Current Batch list presentation (`FlatList` or `FolderTree`).
    pub(super) batch_selection_mode: batch::BatchSelectionMode,
    /// Set of collapsed directory keys in `FolderTree` mode.
    pub(super) batch_collapsed_dirs: HashSet<String>,
    /// Per-item conversion status strings parallel to `files`.
    pub(super) batch_logs: Vec<String>,
    /// Index into `batch::TARGETS` selected in the Batch tab.
    pub(super) batch_target: usize,
    /// Whether Batch conversion overwrites source `.docx` files in place.
    pub(super) batch_overwrite: bool,
    /// Aggregate batch counters `[found, converted, skipped, errors]`.
    pub(super) batch_totals: [usize; 4],
    /// Number of batch items completed in the current run.
    pub(super) batch_completed: usize,
    /// Total items scheduled in the current or most recent batch run.
    pub(super) batch_run_total: usize,
    /// Conversion target string captured when the batch run started.
    pub(super) batch_run_target: String,
    /// Overwrite flag captured when the batch run started.
    pub(super) batch_run_overwrite: bool,
    /// Timestamp directory suffix captured when the batch run started.
    pub(super) batch_run_timestamp: String,
    /// True while a background batch conversion job is executing.
    pub(super) batch_running: bool,
    /// Cooperative cancellation flag shared with the batch worker.
    pub(super) stop: Arc<AtomicBool>,

    // ── Persisted Settings, Appearance & Focus ───────────────────────────────
    /// Resolved on-disk path to `settings.json` (None in isolated unit tests).
    pub(super) settings_path: Option<PathBuf>,
    /// Last applied settings snapshot written to `settings.json` when dirty.
    pub(super) persisted: Settings,
    /// True when `persisted` has unsaved changes waiting for flush.
    pub(super) settings_dirty: bool,
    /// Selected background display mode (`Solid` or `Image`).
    pub(super) background_mode: BackgroundMode,
    /// Managed filename of the custom app background image, if any.
    pub(super) background_image: Option<String>,
    /// Opacity (`0.0..=1.0`) of the frosted UI panels/scrim over the background.
    pub(super) background_opacity: f32,
    /// Opacity (`0.0..=1.0`) of the Equation Canvas and LaTeX source surfaces over the background.
    pub(super) canvas_opacity: f32,
    /// Whether adaptive accent derivation from the background image is enabled.
    pub(super) background_accent_enabled: bool,
    /// Dominant chromatic RGB accent seed extracted from the saved background image.
    pub(super) background_accent: Option<[u8; 3]>,
    /// Last accent seed synced into `egui::Context` global styles.
    pub(super) applied_accent_seed: Option<[u8; 3]>,
    /// GPU texture handle for the custom app background image.
    pub(super) background_texture: Option<egui::TextureHandle>,
    /// Deferred load flag so startup disk image decoding runs on first UI frame.
    pub(super) background_load_pending: bool,
    /// True while importing/resizing a user-chosen background image off-thread.
    pub(super) background_importing: bool,
    /// Sequence ID discarding superseded background image imports.
    pub(super) background_import_seq: u64,
    /// Last error from loading or importing a custom background image.
    pub(super) background_error: Option<String>,
    /// True when running with the software backbuffer painter instead of wgpu/glow.
    pub(super) native_software_background: bool,
    /// Monotonic revision for invalidating software-painter cached background bitmaps.
    pub(super) software_bg_seq: u64,
    /// Shared CPU bitmap passed to [`crate::software_painter::SoftwarePainter`].
    pub(super) software_bg: Option<Arc<crate::software_painter::SoftwareBackgroundImage>>,
    /// Configured initial focus target when opening the Editor tab.
    pub(super) startup_focus: StartupFocus,
    /// Most recently focused editor surface (`Canvas` or `Latex`) for focus restore.
    pub(super) active_editor_focus: StartupFocus,
    /// One-shot focus restoration request consumed on the next editor frame.
    pub(super) pending_focus_restore: Option<StartupFocus>,
    /// Resolved bidirectional table of customizable palette shortcuts.
    pub(super) shortcuts: ShortcutMap,
    /// Canonical `PALETTE` source currently recording a new shortcut in Settings, if any.
    pub(super) shortcut_capture: Option<&'static str>,
    /// Inline validation hint shown when an invalid key is pressed during shortcut capture.
    pub(super) shortcut_capture_error: Option<String>,
    /// Search query filtering the customizable palette shortcuts list in Settings.
    pub(super) shortcut_search: String,
    /// Active category/assignment filter for the customizable palette shortcuts list.
    pub(super) shortcut_filter: ShortcutCategoryFilter,
    /// Open shortcut conflict-resolution modal state, if any.
    pub(super) shortcut_conflict: Option<ShortcutConflictDialog>,
}

impl App {
    /// Creates the desktop application from an `eframe` creation context.
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
            false,
            instance,
            tx,
            rx,
            args,
        )
    }

    /// Initializes fonts, GPU profile adaptations, bridge startup, and persisted settings.
    pub(crate) fn new_with_state(
        egui_ctx: &egui::Context,
        wgpu_render_state: Option<&eframe::egui_wgpu::RenderState>,
        is_software_backend: bool,
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
            egui_ctx.tessellation_options_mut(|options| options.feathering = false);
        }
        let force_scale_1 = args.iter().any(|arg| arg == "--scale1")
            || std::env::var("EQNTIDE_SOFTWARE_SCALE")
                .is_ok_and(|val| val.trim() == "1" || val.trim() == "1.0");
        if gpu_profile == GpuProfile::Software && force_scale_1 {
            egui_ctx.set_pixels_per_point(1.0);
        }
        egui_ctx.set_fonts(fonts);
        runtime::start_bridge(tx.clone());
        let mut app = Self::from_parts(Some(instance), tx, rx, args);
        app.gpu_profile = gpu_profile;
        app.native_software_background = is_software_backend;
        if let Some(path) = Settings::default_path() {
            app = app.with_settings_path(path, Some(egui_ctx));
        }
        app
    }

    /// Attaches a `settings.json` path and immediately loads and applies any stored preferences.
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

    /// Builds a clean [`App`] state from explicit IPC and event channels without starting I/O.
    pub(super) fn from_parts(
        instance: Option<Instance>,
        tx: Sender<Event>,
        rx: Receiver<Event>,
        args: Vec<String>,
    ) -> Self {
        let editor = Editor::default();
        let math = editor.math();
        let xml = math.to_xml();
        let saved_empty = math.children.is_empty();
        Self {
            _instance: instance,
            tx,
            rx,
            bridge: None,
            bridge_ready: false,
            bridge_status: "Connecting bridge…".into(),
            editor,
            saved_xml: xml,
            saved_empty,
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
            palette_atlas: None,
            ui_icons: HashMap::new(),
            recent_palette: Vec::new(),
            gpu_profile: GpuProfile::Native,
            last_enforced_viewport: None,
            logo_texture: None,
            update_state: UpdateState::Idle,
            update_check_seq: 0,
            status: String::new(),
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
            background_mode: BackgroundMode::Solid,
            background_image: None,
            background_opacity: settings::DEFAULT_BACKGROUND_OPACITY,
            canvas_opacity: settings::DEFAULT_CANVAS_OPACITY,
            background_accent_enabled: true,
            background_accent: None,
            applied_accent_seed: None,
            background_texture: None,
            background_load_pending: false,
            background_importing: false,
            background_import_seq: 0,
            background_error: None,
            native_software_background: false,
            software_bg_seq: 0,
            software_bg: None,
            startup_focus: StartupFocus::Canvas,
            active_editor_focus: StartupFocus::Canvas,
            pending_focus_restore: Some(StartupFocus::Canvas),
            caret_blink: None,
            shortcuts: ShortcutMap::default_map(),
            shortcut_capture: None,
            shortcut_capture_error: None,
            shortcut_search: String::new(),
            shortcut_filter: ShortcutCategoryFilter::default(),
            shortcut_conflict: None,
        }
    }

    /// Executes one full desktop frame: drains events, guards window close, paints the UI, and flushes state.
    pub(crate) fn draw_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.enforce_window_minimum_if_needed(&ctx);
        self.process_events(&ctx);
        if self.tab != 2 || self.pending.is_some() || self.busy {
            self.shortcut_capture = None;
            self.shortcut_capture_error = None;
        }
        let save_requested = if self.shortcut_capture.is_some() || self.shortcut_conflict.is_some()
        {
            false
        } else {
            Self::take_save_shortcut(&ctx)
        };
        if ctx.input(|input| input.viewport().close_requested()) && !self.allow_close {
            if !self.is_dirty()
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
        self.draw_shell(ui);
        if (save_requested || self.save_clicked) && self.can_save() {
            self.finish_session(true);
        }
        self.draw_matrix_dialog(&ctx);
        self.draw_shortcut_conflict_dialog(&ctx);
        self.sync_render(&ctx);
        self.draw_dialogs(&ctx);
        self.request_latex_parse(&ctx);
        self.drain_before_close(&ctx);
        if self.settings_dirty && !ctx.input(|input| input.pointer.any_down()) {
            self.flush_settings();
        }
    }

    /// Renders the window chrome (status bar, sidebar, footer) and active tab content.
    pub(super) fn draw_shell(&mut self, ui: &mut egui::Ui) {
        self.save_clicked = false;
        self.keyboard_handled = false;
        if self.tab == 0
            && !self.draft_dirty
            && !self.busy
            && self.pending.is_none()
            && self.matrix_dialog.is_none()
            && self.shortcut_conflict.is_none()
            && ui.is_enabled()
            && ui.memory(|mem| mem.has_focus(egui::Id::unique("equation-canvas")))
            && !ui.input(|input| {
                input.pointer.any_pressed()
                    || input.pointer.any_released()
                    || input.pointer.any_down()
            })
        {
            self.handle_canvas_keyboard(ui);
            self.keyboard_handled = true;
        }
        self.sync_render(ui.ctx());
        self.ensure_background_texture(ui.ctx());
        self.sync_accent_style(ui.ctx());
        self.paint_app_background(ui);
        let tokens = self.theme_tokens(ui);
        let status_rect =
            egui::Panel::bottom("app-status")
                .exact_size(32.0)
                .frame(egui::Frame::NONE.fill(tokens.panel_bg).inner_margin(
                    egui::Margin::symmetric(theme::SPACE_LG as i8, theme::SPACE_XS as i8),
                ))
                .show_separator_line(false)
                .show(ui, |ui| self.draw_status_bar(ui))
                .response
                .rect;
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
                            !self.busy
                                && self.pending.is_none()
                                && self.matrix_dialog.is_none()
                                && self.shortcut_conflict.is_none(),
                            |ui| self.draw_sidebar(ui),
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
                    .show(ui, |ui| self.draw_footer(ui))
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
                    !self.busy
                        && self.pending.is_none()
                        && self.matrix_dialog.is_none()
                        && self.shortcut_conflict.is_none(),
                    |ui| match self.tab {
                        0 => self.draw_editor(ui),
                        1 => self.draw_batch_tab(ui),
                        2 => self.draw_settings_tab(ui),
                        3 => self.draw_about_tab(ui),
                        _ => self.draw_editor(ui),
                    },
                );
            });
        let divider_stroke = Stroke::new(1.0, tokens.border);
        ui.painter().hline(
            status_rect.x_range(),
            status_rect.min.y + 0.5,
            divider_stroke,
        );
        if let Some(rect) = sidebar_rect {
            ui.painter()
                .vline(rect.max.x - 0.5, rect.y_range(), divider_stroke);
        }
        if let Some(rect) = footer_rect {
            ui.painter()
                .hline(rect.x_range(), rect.min.y + 0.5, divider_stroke);
        }
    }

    /// Renders the left navigation sidebar with palette categories and secondary view tabs.
    pub(super) fn draw_sidebar(&mut self, ui: &mut egui::Ui) {
        const SIDEBAR_GAP: f32 = theme::SPACE_SM;
        const SIDEBAR_BUTTON_HEIGHT: f32 = 42.0;
        let tokens = self.theme_tokens(ui);
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
                    if Self::with_selectable_style(ui, tokens, selected, |ui| {
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
                    let response = Self::with_selectable_style(ui, tokens, selected, |ui| {
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

    /// Renders the bottom action bar with target format radio buttons, font size, zoom, undo/redo, and save.
    pub(super) fn draw_footer(&mut self, ui: &mut egui::Ui) {
        let tokens = self.theme_tokens(ui);
        ui.add_enabled_ui(
            !self.busy
                && self.pending.is_none()
                && self.matrix_dialog.is_none()
                && self.shortcut_conflict.is_none(),
            |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = theme::SPACE_MD;
                    ui.add_enabled_ui(!self.draft_dirty, |ui| {
                        let font = egui::TextStyle::Body.resolve(ui.style());
                        let color = ui.visuals().widgets.inactive.fg_stroke.color;
                        let caption =
                            |text: &str| egui::RichText::new(text).font(font.clone()).color(color);
                        let mt_resp = Self::accent_radio_value(
                            ui,
                            tokens,
                            &mut self.target,
                            "mathtype".into(),
                            "MathType",
                        );
                        ui.add_space(theme::SPACE_MD);
                        let omml_resp = Self::accent_radio_value(
                            ui,
                            tokens,
                            &mut self.target,
                            "omml".into(),
                            "OMML",
                        );
                        if mt_resp.changed() || omml_resp.changed() {
                            self.persisted.target = self.target.clone();
                            self.settings_dirty = true;
                        }
                        if mt_resp.clicked() || omml_resp.clicked() {
                            self.pending_focus_restore = Some(self.active_editor_focus);
                        }
                        Self::draw_section_separator(ui, tokens);
                        ui.label(caption("Font size"));
                        let (font_resp, font_wheel_changed) = Self::draw_level_drag_value(
                            ui,
                            tokens,
                            "footer-font-size",
                            f64::from(4.0 / FONT_SIZE_CONTROL_WIDTH),
                            1.0,
                            settings::FONT_PT_RANGE,
                            &mut self.font_pt,
                            "Click and drag left/right, or scroll to adjust",
                            |ui, font_pt, speed| {
                                ui.add(
                                    egui::DragValue::from_get_set(|value| {
                                        if let Some(value) = value {
                                            *font_pt = value.round() as f32;
                                        }
                                        f64::from(*font_pt)
                                    })
                                    .range(settings::FONT_PT_RANGE)
                                    .min_size(Vec2::new(
                                        FONT_SIZE_CONTROL_WIDTH,
                                        FOOTER_CONTROL_HEIGHT,
                                    ))
                                    .speed(speed)
                                    .max_decimals(1)
                                    .suffix(" pt"),
                                )
                            },
                        );
                        if font_resp.changed() || font_wheel_changed {
                            self.render_key += 1;
                            self.persisted.font_pt = self.font_pt;
                            self.settings_dirty = true;
                        }
                        if font_resp.drag_stopped() && !font_resp.has_focus() {
                            self.pending_focus_restore = Some(self.active_editor_focus);
                        }
                        ui.label(caption("Zoom"));
                        let (zoom_resp, zoom_wheel_changed) = Self::draw_level_drag_value(
                            ui,
                            tokens,
                            "footer-zoom",
                            f64::from(1.0 / ZOOM_CONTROL_WIDTH),
                            10.0,
                            settings::ZOOM_RANGE,
                            &mut self.zoom,
                            "Click and drag left/right, or scroll to adjust",
                            |ui, zoom, speed| {
                                ui.add(
                                    egui::DragValue::from_get_set(|value| {
                                        if let Some(value) = value {
                                            *zoom = ((value * 10.0).round() / 10.0) as f32;
                                        }
                                        f64::from(*zoom)
                                    })
                                    .range(settings::ZOOM_RANGE)
                                    .min_size(Vec2::new(ZOOM_CONTROL_WIDTH, FOOTER_CONTROL_HEIGHT))
                                    .speed(speed)
                                    .custom_formatter(|value, _| {
                                        format!("{:.0}%", (value * 100.0).round())
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
                            },
                        );
                        if zoom_resp.changed() || zoom_wheel_changed {
                            self.persisted.zoom = self.zoom;
                            self.settings_dirty = true;
                        }
                        if zoom_resp.drag_stopped() && !zoom_resp.has_focus() {
                            self.pending_focus_restore = Some(self.active_editor_focus);
                        }
                        Self::draw_section_separator(ui, tokens);
                        let history_tint = self.theme_tokens(ui).icon_tint(false, ui.is_enabled());
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
                            self.mark_canvas_changed();
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
                            self.mark_canvas_changed();
                            self.pending_focus_restore = Some(self.active_editor_focus);
                        }
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        self.draw_save_button(ui);
                    });
                });
            },
        );
    }

    /// Renders a Krita-style left-filled `DragValue` supporting hover mouse-wheel stepping.
    #[allow(clippy::too_many_arguments)]
    fn draw_level_drag_value(
        ui: &mut egui::Ui,
        tokens: ThemeTokens,
        id_salt: &str,
        speed: f64,
        steps_per_unit: f64,
        range: std::ops::RangeInclusive<f32>,
        value: &mut f32,
        hover_text: &str,
        add_drag: impl FnOnce(&mut egui::Ui, &mut f32, f64) -> egui::Response,
    ) -> (egui::Response, bool) {
        let enabled = ui.is_enabled();
        let mut wheel_changed = false;
        let response = ui
            .push_id(id_salt, |ui| {
                if !enabled {
                    ui.set_opacity(1.0);
                }
                ui.style_mut().explanation_tooltips = false;
                let meta_id = ui.scope_id().with("level-meta");
                if enabled {
                    if let Some((widget_id, widget_rect)) = ui
                        .ctx()
                        .data(|data| data.get_temp::<(egui::Id, Rect)>(meta_id))
                    {
                        let editing_text = ui.memory(|mem| mem.has_focus(widget_id));
                        if !editing_text && ui.rect_contains_pointer(widget_rect) {
                            let wheel_y: f32 = ui.ctx().input(|input| {
                                input
                                    .raw
                                    .events
                                    .iter()
                                    .filter_map(|event| match event {
                                        egui::Event::MouseWheel { delta, .. } => Some(delta.y),
                                        _ => None,
                                    })
                                    .sum()
                            });
                            if wheel_y.abs() > f32::EPSILON {
                                let step = 1.0 / steps_per_unit;
                                let next = f64::from(*value) + f64::from(wheel_y.signum()) * step;
                                let snapped =
                                    ((next * steps_per_unit).round() / steps_per_unit) as f32;
                                *value = snapped.clamp(*range.start(), *range.end());
                                wheel_changed = true;
                                ui.ctx().input_mut(|input| {
                                    input.smooth_scroll_delta.y = 0.0;
                                });
                            }
                        }
                    }
                }

                let track_idx = ui.painter().add(egui::Shape::Noop);
                let fill_idx = ui.painter().add(egui::Shape::Noop);
                {
                    let visuals = ui.visuals_mut();
                    let widgets = &mut visuals.widgets;
                    widgets.noninteractive.bg_fill = egui::Color32::TRANSPARENT;
                    widgets.noninteractive.weak_bg_fill = egui::Color32::TRANSPARENT;
                    widgets.noninteractive.fg_stroke.color = tokens.text_disabled;
                    widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                    widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
                    widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
                    widgets.hovered.weak_bg_fill = egui::Color32::TRANSPARENT;
                    widgets.active.bg_fill = egui::Color32::TRANSPARENT;
                    widgets.active.weak_bg_fill = egui::Color32::TRANSPARENT;
                    visuals.extreme_bg_color = tokens.control_bg;
                }

                let drag_resp = add_drag(ui, value, speed).on_hover_text(hover_text);
                let rect = drag_resp.rect;
                let hovered = enabled && drag_resp.hovered();
                let fraction = if *range.end() > *range.start() {
                    ((*value - *range.start()) / (*range.end() - *range.start())).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let track_fill = if !enabled {
                    tokens.control_disabled_bg
                } else if hovered {
                    tokens.control_hover_bg
                } else {
                    tokens.control_bg
                };
                ui.painter().set(
                    track_idx,
                    egui::Shape::rect_filled(rect, theme::CONTROL_CORNER_RADIUS, track_fill),
                );
                let fill_width = rect.width() * fraction;
                if fill_width > 0.5 {
                    let radius = theme::RADIUS_CONTROL;
                    let radius_f = f32::from(radius);
                    let vertical_inset = if fill_width < radius_f {
                        let dx = radius_f - fill_width;
                        radius_f - (radius_f * radius_f - dx * dx).max(0.0).sqrt()
                    } else {
                        0.0
                    };
                    let fill_rect = Rect::from_min_max(
                        egui::Pos2::new(rect.left(), rect.top() + vertical_inset),
                        egui::Pos2::new(rect.left() + fill_width, rect.bottom() - vertical_inset),
                    );
                    let corner_radius = if fraction >= 0.999 {
                        theme::CONTROL_CORNER_RADIUS
                    } else {
                        egui::CornerRadius {
                            nw: radius,
                            sw: radius,
                            ne: 0,
                            se: 0,
                        }
                    };
                    ui.painter().set(
                        fill_idx,
                        egui::Shape::rect_filled(
                            fill_rect,
                            corner_radius,
                            tokens.soft_accent_fill(hovered, enabled),
                        ),
                    );
                }
                ui.ctx()
                    .data_mut(|data| data.insert_temp(meta_id, (drag_resp.id, rect)));
                drag_resp
            })
            .inner;
        (response, wheel_changed)
    }

    /// Renders a section separator line using the active theme border token without adding borders to disabled widgets.
    pub(super) fn draw_section_separator(ui: &mut egui::Ui, tokens: ThemeTokens) {
        let prev_stroke = ui.visuals().widgets.noninteractive.bg_stroke;
        ui.visuals_mut().widgets.noninteractive.bg_stroke = Stroke::new(1.0, tokens.border);
        ui.separator();
        ui.visuals_mut().widgets.noninteractive.bg_stroke = prev_stroke;
    }

    /// Renders the bottom status strip showing bridge state, queued sessions, errors, and warnings.
    pub(super) fn draw_status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            let dot_color = self.theme_tokens(ui).accent;
            let (dot_rect, dot_response) =
                ui.allocate_exact_size(Vec2::splat(12.0), egui::Sense::hover());
            if ui.is_rect_visible(dot_rect) {
                let center = dot_rect.center();
                if self.bridge_ready {
                    ui.painter().circle_filled(center, 4.0, dot_color);
                } else {
                    ui.painter()
                        .circle_stroke(center, 3.5, Stroke::new(1.5, dot_color));
                }
            }
            dot_response.on_hover_text(&self.bridge_status);
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
                    |(cached_key, cached_gen, cached_target, _)| {
                        *cached_key == self.render_key
                            && *cached_gen == self.latex_generation
                            && *cached_target == self.target
                    },
                );
                if !cache_hit {
                    let math = self.editor.math();
                    let computed = if self.target == "mathtype" {
                        eqntide::check_mathtype_unsupported(&math)
                            .or_else(|| eqntide::check_mathtype_unsupported_latex(&self.latex))
                            .map(|unsupported| {
                                format!("Warning: {unsupported} is not supported in MathType.")
                            })
                            .or_else(|| {
                                eqntide::check_omml_unsupported(&math)
                                    .or_else(|| eqntide::check_omml_unsupported_latex(&self.latex))
                                    .map(|unsupported| {
                                        format!(
                                            "Warning: {unsupported} is not supported in OMML (MathType target active)."
                                        )
                                    })
                            })
                    } else {
                        eqntide::check_omml_unsupported(&math)
                            .or_else(|| eqntide::check_omml_unsupported_latex(&self.latex))
                            .map(|unsupported| {
                                if eqntide::check_mathtype_unsupported(&math)
                                    .or_else(|| eqntide::check_mathtype_unsupported_latex(&self.latex))
                                    .is_some()
                                {
                                    format!("Warning: {unsupported} is not supported in OMML or MathType.")
                                } else {
                                    format!(
                                        "Warning: {unsupported} is not supported in OMML. Switch target to MathType to insert."
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
                    .and_then(|(_, _, _, cached_msg)| cached_msg.clone())
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
            } else if self.background_importing {
                ui.label("Importing background image…");
            } else if let Some(warning) = target_warning {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(&warning).color(ui.visuals().error_fg_color),
                    )
                    .truncate(),
                )
                .on_hover_text(warning);
            } else if !self.status.is_empty() {
                ui.add(egui::Label::new(&self.status).truncate())
                    .on_hover_text(&self.status);
            }
        });
    }

    pub(super) fn enforce_window_minimum_if_needed(&mut self, ctx: &egui::Context) {
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

    pub(super) fn enforce_window_minimum(ctx: &egui::Context) {
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
}

impl eframe::App for App {
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        ThemeTokens::for_mode(visuals.dark_mode)
            .app_bg
            .to_normalized_gamma_f32()
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.draw_ui(ui);
    }
}

/// Returns embedded PNG bytes for the named bundled Tabler UI icon.
pub(super) fn tabler_icon_bytes(name: &str) -> &'static [u8] {
    match name {
        "alpha" => include_bytes!("../../../assets/icons/generated/desktop/alpha.png"),
        "arrow-autofit-right" => {
            include_bytes!("../../../assets/icons/generated/desktop/arrow-autofit-right.png")
        }
        "arrow-back-up" => {
            include_bytes!("../../../assets/icons/generated/desktop/arrow-back-up.png")
        }
        "arrow-forward-up" => {
            include_bytes!("../../../assets/icons/generated/desktop/arrow-forward-up.png")
        }
        "arrows-left-right" => {
            include_bytes!("../../../assets/icons/generated/desktop/arrows-left-right.png")
        }
        "brand-github" => {
            include_bytes!("../../../assets/icons/generated/desktop/brand-github.png")
        }
        "chevron-left" => {
            include_bytes!("../../../assets/icons/generated/desktop/chevron-left.png")
        }
        "dots" => include_bytes!("../../../assets/icons/generated/desktop/dots.png"),
        "file" => include_bytes!("../../../assets/icons/generated/desktop/file.png"),
        "file-type-docx" => {
            include_bytes!("../../../assets/icons/generated/desktop/file-type-docx.png")
        }
        "file-upload" => include_bytes!("../../../assets/icons/generated/desktop/file-upload.png"),
        "folder" => include_bytes!("../../../assets/icons/generated/desktop/folder.png"),
        "folder-open" => include_bytes!("../../../assets/icons/generated/desktop/folder-open.png"),
        "history" => include_bytes!("../../../assets/icons/generated/desktop/history.png"),
        "info-circle" => include_bytes!("../../../assets/icons/generated/desktop/info-circle.png"),
        "license" => include_bytes!("../../../assets/icons/generated/desktop/license.png"),
        "math-avg" => include_bytes!("../../../assets/icons/generated/desktop/math-avg.png"),
        "math-equal-lower" => {
            include_bytes!("../../../assets/icons/generated/desktop/math-equal-lower.png")
        }
        "math-integral" => {
            include_bytes!("../../../assets/icons/generated/desktop/math-integral.png")
        }
        "math-sin" => include_bytes!("../../../assets/icons/generated/desktop/math-sin.png"),
        "math-x-divide-y" => {
            include_bytes!("../../../assets/icons/generated/desktop/math-x-divide-y.png")
        }
        "matrix" => include_bytes!("../../../assets/icons/generated/desktop/matrix.png"),
        "omega" => include_bytes!("../../../assets/icons/generated/desktop/omega.png"),
        "overline" => include_bytes!("../../../assets/icons/generated/desktop/overline.png"),
        "parentheses" => include_bytes!("../../../assets/icons/generated/desktop/parentheses.png"),
        "settings" => include_bytes!("../../../assets/icons/generated/desktop/settings.png"),
        "square-root-2" => {
            include_bytes!("../../../assets/icons/generated/desktop/square-root-2.png")
        }
        "sum" => include_bytes!("../../../assets/icons/generated/desktop/sum.png"),
        "underline" => include_bytes!("../../../assets/icons/generated/desktop/underline.png"),
        "user" => include_bytes!("../../../assets/icons/generated/desktop/user.png"),
        "x-power-y" => include_bytes!("../../../assets/icons/generated/desktop/x-power-y.png"),
        other => panic!("unknown tabler icon: {other}"),
    }
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

#[cfg(test)]
mod tests;
