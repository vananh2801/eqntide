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
mod background;
mod batch_ui;
mod canvas;
mod dialogs;
mod events;
mod gpu;
mod keyboard;
mod palette_ribbon;
mod palette_ui;
mod session;
mod settings_ui;
mod shell;
mod shortcuts_ui;

pub(crate) use gpu::{is_software_adapter_name, GpuProfile};
pub(crate) use shell::MIN_WINDOW_SIZE;
pub(in crate::app) use shell::{CONTENT_PADDING, PALETTE_BUTTON_SIZE, PALETTE_GAP, PALETTE_ROWS};
#[cfg(test)]
pub(in crate::app) use shell::{PALETTE_HEIGHT, SIDEBAR_CATEGORY_ICON_SIZE};

use crate::EventSender as Sender;
use crate::{
    batch, commands,
    instance::Instance,
    palette, runtime,
    settings::{self, BackgroundMode, Settings, StartupFocus},
    shortcuts::{ShortcutChord, ShortcutConflict, ShortcutMap},
    theme::ThemeTokens,
    ui::icons::IconCache,
    update::UpdateState,
    Event, Rendered,
};
use eframe::egui::{self, Rect, Vec2};
use eqntide::bridge::{DesktopWindowRequest, ServerHandle};
use eqntide_editor::{Caret, Editor};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::PathBuf,
    sync::{atomic::AtomicBool, mpsc::Receiver, Arc},
    time::Instant,
};

/// Fixed height of the bottom LaTeX source panel in DIP.
pub(super) const LATEX_PANEL_HEIGHT: f32 = 88.0;
/// Inner height available to the multiline LaTeX source editor box.
pub(super) const LATEX_INPUT_HEIGHT: f32 = LATEX_PANEL_HEIGHT - CONTENT_PADDING as f32;
/// MathML marker attribute identifying placeholder slots during rendering.
pub(super) const PLACEHOLDER_ATTR: &str = "data-eqntide-placeholder";

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
    /// Monospace font size (pt) used in the `LaTeX source` box.
    pub(super) latex_font_pt: f32,
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
    pub(super) ui_icons: IconCache,
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
        Self::configure_egui_options(egui_ctx);
        tx.attach(egui_ctx.clone());
        let gpu_profile = gpu::detect_and_apply_gpu_profile(
            egui_ctx,
            wgpu_render_state,
            is_software_backend,
            &args,
        );
        gpu::register_fonts(egui_ctx);
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
            latex_font_pt: settings::DEFAULT_LATEX_FONT_PT,
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
            ui_icons: IconCache::default(),
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
        Self::configure_egui_options(&ctx);
        self.enforce_window_minimum_if_needed(&ctx);
        self.process_events(&ctx);
        if self.tab != 2 || self.pending.is_some() || self.busy {
            self.shortcut_capture = None;
            self.shortcut_capture_error = None;
        }
        self.suppress_text_edit_readline_chords(&ctx);
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

#[cfg(test)]
mod tests;
