use crate::EventSender as Sender;
use crate::{batch, commands, instance::Instance, palette, runtime, Event, Rendered};
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use eqntide::{
    bridge::{DesktopWindowRequest, ServerHandle},
    MathNode,
};
use eqntide_editor::{Caret, Editor, Move};
use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Receiver,
        Arc,
    },
    time::{Duration, Instant},
};

const CONTENT_PADDING: i8 = 10;
const UI_CORNER_RADIUS: egui::CornerRadius = egui::CornerRadius::same(6);
const FONT_SIZE_CONTROL_WIDTH: f32 = 96.0;
const FOOTER_CONTROL_HEIGHT: f32 = 30.0;
pub(crate) const MIN_WINDOW_SIZE: [f32; 2] = [1040.0, 560.0];
const PALETTE_BUTTON_SIZE: Vec2 = Vec2::new(44.0, 31.0);
const PALETTE_GAP: f32 = 6.0;
const PALETTE_HEIGHT: f32 =
    3.0 * PALETTE_BUTTON_SIZE.y + 2.0 * PALETTE_GAP + CONTENT_PADDING as f32;
const ZOOM_RANGE: std::ops::RangeInclusive<f32> = 1.5..=4.0;
const PLACEHOLDER_ATTR: &str = "data-eqntide-placeholder";

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
    palette_textures: HashMap<&'static str, Option<egui::TextureHandle>>,
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
    files: Vec<PathBuf>,
    batch_logs: Vec<String>,
    batch_target: usize,
    batch_overwrite: bool,
    batch_totals: [usize; 4],
    batch_completed: usize,
    batch_running: bool,
    stop: Arc<AtomicBool>,
}

impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        instance: Instance,
        tx: Sender<Event>,
        rx: Receiver<Event>,
        args: Vec<String>,
    ) -> Self {
        cc.egui_ctx.set_theme(egui::ThemePreference::System);
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            cc.egui_ctx.style_mut_of(theme, |style| {
                style.spacing.item_spacing = Vec2::new(8.0, 8.0);
                style.spacing.button_padding = Vec2::new(10.0, 7.0);
                style.visuals.selection.bg_fill = Color32::from_rgb(40, 157, 207);
                style.visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);
                let widgets = &mut style.visuals.widgets;
                for state in [
                    &mut widgets.noninteractive,
                    &mut widgets.inactive,
                    &mut widgets.hovered,
                    &mut widgets.active,
                    &mut widgets.open,
                ] {
                    state.corner_radius = UI_CORNER_RADIUS;
                }
                style.visuals.window_corner_radius = UI_CORNER_RADIUS;
                style.visuals.menu_corner_radius = UI_CORNER_RADIUS;
            });
        }
        tx.attach(cc.egui_ctx.clone());
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
        cc.egui_ctx.set_fonts(fonts);
        runtime::start_bridge(tx.clone());
        Self::from_parts(Some(instance), tx, rx, args)
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
            category: "Fractions".into(),
            matrix_dialog: None,
            palette_textures: HashMap::new(),
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
            batch_logs: vec![],
            batch_target: 0,
            batch_overwrite: false,
            batch_totals: [0; 4],
            batch_completed: 0,
            batch_running: false,
            stop: Arc::new(AtomicBool::new(false)),
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
    fn request_latex_parse(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        if let Some((generation, source)) = self.take_latex_request(now) {
            let tx = self.tx.clone();
            std::thread::spawn(move || {
                let result = if source.trim().is_empty() {
                    Ok(MathNode::new("math"))
                } else {
                    eqntide::service::parse_latex_to_mathml(&source).map_err(|e| e.to_string())
                }
                .and_then(|math| Editor::try_new(&math).map(|_| math).map_err(str::to_owned));
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
                    }
                    Err(e) => {
                        self.status = e.into();
                        return false;
                    }
                }
                self.font_pt = font.filter(|f| f.is_finite() && *f > 0.0).unwrap_or(12.0);
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
                        self.launch(args, ctx);
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
                            Ok(rendered) => {
                                self.texture = Some(ctx.load_texture(
                                    "equation",
                                    rendered.image.clone(),
                                    egui::TextureOptions::LINEAR,
                                ));
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
                            format!(
                                "{} converted, {} skipped, {} errors → {}",
                                r.converted, r.skipped, r.errors, r.output_path
                            )
                        }
                        Err(e) => {
                            self.batch_totals[3] += 1;
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
                        "Batch stopped between files."
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
        let tx = self.tx.clone();
        let context = ctx.clone();
        std::thread::spawn(move || {
            let result = eqntide::render::render_interactive(&math, font)
                .map_err(|e| e.to_string())
                .and_then(|preview| {
                    let image = image::load_from_memory(&preview.preview.png)
                        .map_err(|e| e.to_string())?
                        .to_rgba8();
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
                    Ok(Rendered {
                        placeholders,
                        image: egui::ColorImage::from_rgba_unmultiplied(
                            [image.width() as usize, image.height() as usize],
                            image.as_raw(),
                        ),
                        preview,
                    })
                });
            let _ = tx.send(Event::Render(key, result));
            context.request_repaint();
        });
    }
    fn add_files(&mut self, paths: Vec<PathBuf>) {
        if self.batch_running {
            return;
        }
        for path in paths {
            if path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("docx"))
                && !self.files.contains(&path)
            {
                self.files.push(path);
                self.batch_logs.push("Waiting".into());
            }
        }
    }
    fn finish(&mut self, save: bool) {
        if self.busy {
            return;
        }
        if save && self.command_input.is_some() {
            self.status =
                "Hoàn tất lệnh LaTeX bằng Tab/Enter hoặc hủy bằng Esc trước khi lưu.".into();
            return;
        }
        if save && self.draft_dirty {
            self.status = "Wait for valid LaTeX before saving.".into();
            return;
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
            "Lưu và trở về tài liệu (Ctrl+S)"
        } else {
            "Lưu MathML (Ctrl+S)"
        };
        if ui
            .add_enabled(
                self.save_available() && !self.draft_dirty && self.command_input.is_none(),
                egui::Button::new(egui::RichText::new(label).color(Color32::WHITE))
                    .fill(Color32::from_rgb(25, 135, 190)),
            )
            .clicked()
        {
            self.save_clicked = true;
        }
    }

    fn footer(&mut self, ui: &mut egui::Ui) {
        ui.add_enabled_ui(
            !self.busy && self.pending.is_none() && self.matrix_dialog.is_none(),
            |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 5.0;
                    ui.add_enabled_ui(!self.draft_dirty, |ui| {
                        let font = egui::TextStyle::Body.resolve(ui.style());
                        let color = ui.visuals().widgets.inactive.fg_stroke.color;
                        let caption =
                            |text: &str| egui::RichText::new(text).font(font.clone()).color(color);
                        ui.radio_value(&mut self.target, "mathtype".into(), caption("MathType"));
                        ui.add_space(12.0);
                        ui.radio_value(&mut self.target, "omml".into(), caption("OMML"));
                        ui.separator();
                        ui.label(caption("Cỡ chữ"));
                        let size_changed = ui
                            .scope(|ui| {
                                ui.style_mut().explanation_tooltips = false;
                                ui.add(
                                    egui::DragValue::from_get_set(|value| {
                                        if let Some(value) = value {
                                            self.font_pt = ((value * 2.0).round() / 2.0) as f32;
                                        }
                                        f64::from(self.font_pt)
                                    })
                                    .range(6.0..=72.0)
                                    .min_size(Vec2::new(
                                        FONT_SIZE_CONTROL_WIDTH,
                                        FOOTER_CONTROL_HEIGHT,
                                    ))
                                    // A quarter of the control's width changes the size by 1 pt.
                                    .speed(4.0 / FONT_SIZE_CONTROL_WIDTH)
                                    .max_decimals(1)
                                    .suffix(" pt"),
                                )
                                .on_hover_text("Nhấn giữ và kéo trái/phải để điều chỉnh")
                                .changed()
                            })
                            .inner;
                        if size_changed {
                            self.render_key += 1;
                        }
                        if ui
                            .add(
                                egui::Button::new("↶")
                                    .min_size(Vec2::new(32.0, FOOTER_CONTROL_HEIGHT)),
                            )
                            .on_hover_text("Hoàn tác (Ctrl+Z)")
                            .clicked()
                        {
                            self.command_input = None;
                            self.latex_groups.clear();
                            self.expects_latex_group = false;
                            self.editor.undo();
                            self.changed();
                        }
                        if ui
                            .add(
                                egui::Button::new("↷")
                                    .min_size(Vec2::new(32.0, FOOTER_CONTROL_HEIGHT)),
                            )
                            .on_hover_text("Làm lại (Ctrl+Y)")
                            .clicked()
                        {
                            self.command_input = None;
                            self.latex_groups.clear();
                            self.expects_latex_group = false;
                            self.editor.redo();
                            self.changed();
                        }
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        self.save_button(ui);
                        ui.scope(|ui| {
                            ui.spacing_mut().slider_width = 120.0;
                            let widgets = &mut ui.visuals_mut().widgets;
                            for state in [
                                &mut widgets.noninteractive,
                                &mut widgets.inactive,
                                &mut widgets.hovered,
                                &mut widgets.active,
                                &mut widgets.open,
                            ] {
                                state.corner_radius = egui::CornerRadius::same(4);
                            }
                            ui.add(
                                egui::Slider::new(&mut self.zoom, ZOOM_RANGE)
                                    .handle_shape(egui::style::HandleShape::Rect {
                                        aspect_ratio: 1.0,
                                    })
                                    .show_value(false),
                            )
                            .on_hover_text("Thu phóng hiển thị");
                        });
                    });
                });
            },
        );
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) -> egui::scroll_area::ScrollAreaOutput<()> {
        egui::Panel::bottom("sidebar-batch")
            .exact_size(60.0)
            .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(0, 8)))
            .show(ui, |ui| {
                if ui
                    .add_sized(
                        [ui.available_width(), 42.0],
                        egui::Button::selectable(false, "Batch DOCX"),
                    )
                    .clicked()
                {
                    self.tab = 1;
                }
            });
        ui.add_space(8.0);
        egui::ScrollArea::vertical()
            .id_salt("categories")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (group, label) in palette::GROUPS {
                    if ui
                        .add_sized(
                            [ui.available_width(), 42.0],
                            egui::Button::selectable(self.category == *group, *label),
                        )
                        .clicked()
                    {
                        self.category = (*group).into();
                    }
                }
            })
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
        self.status = format!("Đã chèn ma trận {} × {}.", settings.rows, settings.columns);
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
            ui.heading("Ma trận tuỳ chỉnh");
            ui.add_space(6.0);
            egui::Grid::new("matrix-settings")
                .spacing([16.0, 10.0])
                .show(ui, |ui| {
                    ui.label("Số hàng");
                    ui.add(egui::DragValue::new(&mut settings.rows).range(1..=20));
                    ui.end_row();
                    ui.label("Số cột");
                    ui.add(egui::DragValue::new(&mut settings.columns).range(1..=20));
                    ui.end_row();
                    ui.label("Căn lề");
                    egui::ComboBox::from_id_salt("matrix-alignment")
                        .selected_text(match settings.alignment {
                            "left" => "Trái",
                            "right" => "Phải",
                            _ => "Giữa",
                        })
                        .show_ui(ui, |ui| {
                            for (value, label) in
                                [("left", "Trái"), ("center", "Giữa"), ("right", "Phải")]
                            {
                                ui.selectable_value(&mut settings.alignment, value, label);
                            }
                        });
                    ui.end_row();
                    ui.label("Kiểu ngoặc");
                    const FENCES: &[(&str, &str)] = &[
                        ("none", "Không ngoặc"),
                        ("parentheses", "Ngoặc tròn ( )"),
                        ("brackets", "Ngoặc vuông [ ]"),
                        ("braces", "Ngoặc nhọn { }"),
                        ("bars", "Định thức | |"),
                        ("double-bars", "Chuẩn ‖ ‖"),
                    ];
                    egui::ComboBox::from_id_salt("matrix-fence")
                        .selected_text(
                            FENCES
                                .iter()
                                .find(|(value, _)| *value == settings.fence)
                                .unwrap()
                                .1,
                        )
                        .show_ui(ui, |ui| {
                            for &(value, label) in FENCES {
                                ui.selectable_value(&mut settings.fence, value, label);
                            }
                        });
                    ui.end_row();
                });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                insert = ui.button("Chèn ma trận").clicked();
                if ui.button("Huỷ").clicked() {
                    ui.close();
                }
            });
        });
        if insert {
            self.insert_custom_matrix(&settings);
            ctx.memory_mut(|m| m.request_focus(egui::Id::unique("equation-canvas")));
        } else if !response.should_close() {
            self.matrix_dialog = Some(settings);
        } else {
            ctx.memory_mut(|m| m.request_focus(egui::Id::unique("equation-canvas")));
        }
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if self.busy || self.render_busy {
                ui.spinner();
            }
            let ready = if self.bridge_ready { "●" } else { "○" };
            ui.label(ready).on_hover_text(&self.bridge_status);
            if !self.bridge_ready && ui.small_button("Thử lại").clicked() {
                runtime::start_bridge(self.tx.clone());
            }
            if !self.queue.is_empty()
                && ui
                    .small_button(format!("Phiên tiếp ({})", self.queue.len()))
                    .clicked()
            {
                self.pending = Some(Intent::Next);
            }
            if self.tab == 0 && self.latex_error.is_some() {
                let error = format!("Lỗi LaTeX: {}", self.latex_error.as_deref().unwrap());
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(&error).color(ui.visuals().error_fg_color),
                    )
                    .truncate(),
                )
                .on_hover_text(error);
            } else if self.tab == 0 && self.draft_dirty {
                ui.label("Đang cập nhật công thức…");
            } else {
                ui.add(egui::Label::new(&self.status).truncate())
                    .on_hover_text(&self.status);
            }
        });
    }

    fn palette_ui(&mut self, ui: &mut egui::Ui) {
        let height = ui.available_height();
        let items: Vec<_> = palette::PALETTE
            .iter()
            .filter(|(group, _, _)| *group == self.category)
            .collect();
        let fitting_columns = ((ui.available_width() + PALETTE_GAP)
            / (PALETTE_BUTTON_SIZE.x + PALETTE_GAP))
            .floor() as usize;
        // Native DPI/startup adjustments may briefly give us less than the minimum width.
        // Keep three rows in that frame too, scrolling horizontally if necessary.
        let item_count = items.len() + usize::from(self.category == "Matrices");
        let columns = fitting_columns.max(item_count.div_ceil(3)).max(1);
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
                        ui.spacing_mut().button_padding = Vec2::new(5.0, 4.0);
                        ui.spacing_mut().item_spacing = Vec2::splat(PALETTE_GAP);
                        for (index, (_, label, source)) in items.iter().enumerate() {
                            let texture =
                                self.palette_textures.entry(source).or_insert_with(|| {
                                    let math = palette::preview_template(source).ok()?;
                                    let preview =
                                        eqntide::render::render_preview(&math, 12.0).ok()?;
                                    let mut pixels =
                                        image::load_from_memory(&preview.png).ok()?.to_rgba8();
                                    // White alpha masks can be tinted for both system themes.
                                    for pixel in pixels.pixels_mut() {
                                        pixel.0[0..3].fill(255);
                                    }
                                    let image = egui::ColorImage::from_rgba_unmultiplied(
                                        [pixels.width() as usize, pixels.height() as usize],
                                        pixels.as_raw(),
                                    );
                                    Some(ui.ctx().load_texture(
                                        *source,
                                        image,
                                        egui::TextureOptions::LINEAR,
                                    ))
                                });
                            let response = if let Some(texture) = texture {
                                let size = texture.size_vec2();
                                let ratio = (34.0 / size.x).min(22.0 / size.y);
                                let image = egui::Image::new((texture.id(), size * ratio))
                                    .tint(ui.visuals().text_color());
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
                                .on_hover_text(format!("{label}\n{source}"))
                                .clicked()
                            {
                                self.command_input = None;
                                self.latex_groups.clear();
                                self.expects_latex_group = false;
                                self.insert_template(source);
                                ui.memory_mut(|m| {
                                    m.request_focus(egui::Id::unique("equation-canvas"))
                                });
                            }
                            if (index + 1) % columns == 0 {
                                ui.end_row();
                            }
                        }
                        if self.category == "Matrices"
                            && ui
                                .add_sized(PALETTE_BUTTON_SIZE, egui::Button::new("⋯"))
                                .on_hover_text("Ma trận tuỳ chỉnh")
                                .clicked()
                        {
                            self.command_input = None;
                            self.matrix_dialog = Some(MatrixDialog::default());
                            ui.memory_mut(|m| {
                                m.surrender_focus(egui::Id::unique("equation-canvas"))
                            });
                        }
                    });
            });
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
        egui::Panel::bottom("app-status")
            .exact_size(32.0)
            .show(ui, |ui| self.status_bar(ui));
        if self.tab == 0 {
            egui::Panel::left("app-sidebar")
                .exact_size(150.0)
                .resizable(false)
                .show(ui, |ui| {
                    ui.add_enabled_ui(
                        !self.busy && self.pending.is_none() && self.matrix_dialog.is_none(),
                        |ui| self.sidebar(ui),
                    );
                });
        }
        if self.tab == 0 || self.session.is_some() {
            egui::Panel::bottom("app-footer")
                .exact_size(56.0)
                .frame(
                    egui::Frame::NONE
                        .fill(ui.visuals().panel_fill)
                        .inner_margin(egui::Margin {
                            left: CONTENT_PADDING,
                            right: CONTENT_PADDING,
                            top: CONTENT_PADDING,
                            bottom: CONTENT_PADDING,
                        }),
                )
                .show(ui, |ui| self.footer(ui));
        }
        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(ui.visuals().panel_fill)
                    .inner_margin(CONTENT_PADDING),
            )
            .show(ui, |ui| {
                ui.add_enabled_ui(
                    !self.busy && self.pending.is_none() && self.matrix_dialog.is_none(),
                    |ui| {
                        if self.tab == 0 {
                            self.editor_ui(ui);
                        } else {
                            self.batch_ui(ui);
                        }
                    },
                );
            });
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
            .exact_size(60.0)
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
                    if self.category == "Matrices" {
                        ui.horizontal(|ui| {
                            for (label, column, grow) in [
                                ("+ hàng", false, true),
                                ("− hàng", false, false),
                                ("+ cột", true, true),
                                ("− cột", true, false),
                            ] {
                                if ui.small_button(label).clicked() {
                                    let changed = if grow {
                                        self.editor.grow_matrix(column)
                                    } else {
                                        self.editor.shrink_matrix(column)
                                    };
                                    if changed {
                                        self.changed();
                                    } else {
                                        self.status = "Đặt con trỏ trong ma trận trước.".into();
                                    }
                                }
                            }
                        });
                    }
                    self.canvas(ui);
                });
            });
    }

    fn latex_ui(&mut self, ui: &mut egui::Ui) {
        if ui
            .add_sized(
                [ui.available_width(), 48.0],
                egui::TextEdit::multiline(&mut self.latex)
                    .desired_rows(2)
                    .id(egui::Id::unique("latex-source"))
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Monospace),
            )
            .changed()
        {
            self.source_changed(Instant::now());
        }
    }

    fn canvas(&mut self, ui: &mut egui::Ui) -> egui::scroll_area::ScrollAreaOutput<()> {
        // A pending/invalid source disables editing, but keeps the last preview legible.
        ui.set_opacity(1.0);
        let scale = self.zoom * 96.0 / 72.0;
        let height = (ui.available_height() - 26.0).max(12.0);
        egui::Frame::new()
            .fill(Color32::WHITE)
            .corner_radius(UI_CORNER_RADIUS)
            .stroke(Stroke::new(1.0, Color32::LIGHT_GRAY))
            .inner_margin(12.0)
            .show(ui, |ui| {
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
                                Color32::BLACK,
                            );
                        }
                        if let Some(rendered) = &self.rendered {
                            for slot in &rendered.placeholders {
                                let rect = Rect::from_min_size(
                                    origin + Vec2::new(slot.x, slot.y) * scale,
                                    Vec2::new(slot.width, slot.height) * scale,
                                );
                                Self::paint_placeholder(ui.painter(), rect);
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
                                    UI_CORNER_RADIUS,
                                    Color32::from_rgba_unmultiplied(60, 120, 230, 65),
                                );
                            }
                        }
                        if response.has_focus() && ui.is_enabled() {
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
                            let cursor = self
                                .caret_rect(origin, scale)
                                .unwrap_or(Rect::from_min_size(origin, Vec2::new(1.0, 24.0)));
                            ui.painter().line_segment(
                                [cursor.left_top(), cursor.left_bottom()],
                                Stroke::new(1.5, Color32::from_rgb(30, 90, 180)),
                            );
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
                                    Color32::DARK_BLUE,
                                );
                            }
                            self.keyboard(ui);
                        }
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
    fn paint_placeholder(painter: &egui::Painter, rect: Rect) {
        // Cover only the synthetic square; the formula texture and hit regions stay unchanged.
        painter.rect_filled(rect.expand(0.5), 0.0, Color32::WHITE);
        let rect = rect.shrink(1.0);
        let radius = UI_CORNER_RADIUS;
        painter.rect_filled(rect, radius, Color32::from_rgb(239, 248, 255));
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
            Stroke::new(1.2, Color32::from_rgb(133, 192, 230)),
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
    fn caret_rect(&self, origin: Pos2, scale: f32) -> Option<Rect> {
        let caret = self.editor.caret();
        let node = self.editor.node(caret.node)?;
        let region = self.region(caret.node)?;
        let x = if node.token() {
            if region.carets.len() == node.text.chars().count() + 1 {
                region.carets[caret.offset.min(region.carets.len() - 1)]
            } else {
                region.x
                    + region.width * caret.offset as f32 / node.text.chars().count().max(1) as f32
            }
        } else if let Some(child) = node.children.get(caret.offset) {
            self.region(child.id).map_or(region.x, |r| r.x)
        } else if let Some(child) = node.children.last() {
            self.region(child.id)
                .map_or(region.x + region.width, |r| r.x + r.width)
        } else {
            region.x
        };
        Some(Rect::from_min_size(
            origin + Vec2::new(x * scale, region.y * scale),
            Vec2::new(1.0, region.height.max(8.0) * scale),
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
                let parent = self.editor.node(parent_id).unwrap();
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
        let command = if choose_suggestion {
            commands::suggestions(&input.name)
                .get(input.selected)
                .copied()
        } else {
            commands::exact(&input.name)
        };
        let result = if let Some(command) = command {
            palette::template(command.source).map(|math| (math, command.source.contains("#?")))
        } else {
            eqntide::service::parse_latex_to_mathml(&format!("\\{}", input.name))
                .map(|math| (math, false))
                .map_err(|e| e.to_string())
        };
        match result {
            Ok((math, template)) => {
                self.command_input = None;
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
                self.status = "Đã chèn lệnh LaTeX.".into();
                true
            }
            Err(_) => {
                self.status = format!("Lệnh \\{} chưa hoàn chỉnh hoặc chưa được hỗ trợ; Backspace để sửa, Esc để hủy.", input.name);
                false
            }
        }
    }

    fn canvas_text(&mut self, text: &str) {
        for ch in text.chars() {
            if let Some(input) = &mut self.command_input {
                if ch.is_ascii_alphabetic() || input.name.chars().any(|c| !c.is_ascii_alphabetic())
                {
                    input.name.push(ch);
                    input.selected = 0;
                    continue;
                }
                if input.name.is_empty() && matches!(ch, '{' | '}' | ' ' | ',' | ';' | '!') {
                    self.command_input = None;
                    let text = if matches!(ch, ',' | ';' | '!') {
                        " ".into()
                    } else {
                        ch.to_string()
                    };
                    self.editor.insert_text(&text);
                    continue;
                }
                if !self.commit_command(false) {
                    self.command_input.as_mut().unwrap().name.push(ch);
                    continue;
                }
                if ch.is_whitespace() {
                    continue;
                }
            }
            match ch {
                '\\' => {
                    self.expects_latex_group = false;
                    self.command_input = Some(commands::Input::default());
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
                _ => {
                    self.expects_latex_group = false;
                    self.editor.insert_text(&ch.to_string());
                }
            }
        }
    }

    fn close_latex_group(&mut self) {
        let Some(start) = self.latex_groups.pop() else {
            return;
        };
        let Some(mut path) = self.editor.path(start) else {
            return;
        };
        while let Some(index) = path.pop() {
            let Some(parent_id) = self.editor.id_at_path(&path) else {
                return;
            };
            let parent = self.editor.node(parent_id).unwrap();
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
                self.expects_latex_group = true;
                self.editor.set_caret(
                    Caret {
                        node: parent.children[1].id,
                        offset: 0,
                    },
                    false,
                );
                return;
            }
            while let Some(index) = path.pop() {
                let id = self.editor.id_at_path(&path).unwrap();
                if self.editor.node(id).unwrap().row() {
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
                let count = commands::suggestions(&input.name).len();
                if count > 0 {
                    input.selected = if key == Key::ArrowDown {
                        (input.selected + 1) % count
                    } else {
                        (input.selected + count - 1) % count
                    };
                }
                true
            }
            Key::Tab | Key::Enter => {
                self.commit_command(true);
                true
            }
            Key::Escape => {
                self.command_input = None;
                self.status = "Đã hủy lệnh LaTeX.".into();
                true
            }
            Key::Backspace => {
                if input.name.pop().is_none() {
                    self.command_input = None;
                } else {
                    input.selected = 0;
                }
                true
            }
            Key::Delete => true,
            Key::ArrowLeft | Key::ArrowRight | Key::Home | Key::End => !self.commit_command(false),
            _ => false,
        }
    }

    fn command_popup(&mut self, ui: &mut egui::Ui, anchor: Pos2) {
        let Some(input) = &self.command_input else {
            return;
        };
        let name = input.name.clone();
        let selected = input.selected;
        let candidates = commands::suggestions(&name);
        let mut accept = None;
        egui::Area::new(egui::Id::unique("canvas-command-popup"))
            .order(egui::Order::Foreground)
            .fixed_pos(anchor)
            .constrain(true)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_width(230.0);
                    ui.monospace(format!("\\{name}"));
                    if candidates.is_empty() {
                        ui.label("Chưa có gợi ý — Backspace để sửa, Esc để hủy");
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
                    ui.weak("↑↓ chọn · Tab/Enter chèn · Esc hủy");
                });
            });
        if let Some(index) = accept {
            if let Some(input) = &mut self.command_input {
                input.selected = index;
            }
            self.commit_command(true);
            ui.memory_mut(|m| m.request_focus(egui::Id::unique("equation-canvas")));
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
                    if text.trim_start().starts_with('<') {
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
    fn batch_ui(&mut self, ui: &mut egui::Ui) {
        if ui.button("← Quay lại soạn công thức").clicked() {
            self.tab = 0;
        }
        ui.heading("Batch document conversion");
        ui.label("Drop .docx files here. By default, a new copy is saved beside each input.");
        ui.add_enabled_ui(!self.batch_running, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Add DOCX files…").clicked() {
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
                if ui.button("Clear list").clicked() {
                    self.files.clear();
                    self.batch_logs.clear();
                }
                egui::ComboBox::from_id_salt("batch-target")
                    .selected_text(batch::TARGETS[self.batch_target].1)
                    .show_ui(ui, |ui| {
                        for (i, (_, label)) in batch::TARGETS.iter().enumerate() {
                            ui.selectable_value(&mut self.batch_target, i, *label);
                        }
                    });
            });
        });
        ui.add_enabled_ui(!self.batch_running, |ui| {
            ui.checkbox(
                &mut self.batch_overwrite,
                "Overwrite original documents (replace each selected input)",
            );
        });
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    !self.batch_running && !self.files.is_empty(),
                    egui::Button::new("Convert"),
                )
                .clicked()
            {
                self.batch_running = true;
                self.batch_totals = [0; 4];
                self.batch_completed = 0;
                self.stop.store(false, Ordering::Relaxed);
                self.batch_logs.fill("Waiting".into());
                batch::run(
                    self.files.clone(),
                    batch::TARGETS[self.batch_target].0.into(),
                    self.font_pt,
                    self.batch_overwrite,
                    Arc::clone(&self.stop),
                    self.tx.clone(),
                );
            }
            if ui
                .add_enabled(
                    self.batch_running,
                    egui::Button::new("Stop after current file"),
                )
                .clicked()
            {
                self.stop.store(true, Ordering::Relaxed);
            }
            if ui.button("Open output folder").clicked() {
                if let Some(path) = self.files.first() {
                    if let Err(e) = runtime::open_folder(path) {
                        self.status = e;
                    }
                }
            }
        });
        if !self.files.is_empty() {
            ui.add(
                egui::ProgressBar::new(self.batch_completed as f32 / self.files.len() as f32).text(
                    format!("{}/{} files", self.batch_completed, self.files.len()),
                ),
            );
            ui.label(format!(
                "Found: {}   Converted: {}   Skipped: {}   Errors: {}",
                self.batch_totals[0],
                self.batch_totals[1],
                self.batch_totals[2],
                self.batch_totals[3]
            ));
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (path, log) in self.files.iter().zip(&self.batch_logs) {
                ui.group(|ui| {
                    ui.label(path.display().to_string());
                    ui.label(log);
                });
            }
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
                        if ui.button("Stop batch").clicked() {
                            self.stop.store(true, Ordering::Relaxed);
                        }
                    }
                    ui.add_enabled_ui(!self.busy && !self.batch_running, |ui| {
                        ui.horizontal(|ui| {
                            if ui.button("Save").clicked() { self.finish(true); }
                            if ui.button("Discard / cancel session").clicked() { self.finish(false); }
                            if ui.button("Continue editing").clicked() { self.pending = None; }
                        });
                    });
                });
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        Self::enforce_window_minimum(&ctx);
        self.events(&ctx);
        let save_requested = Self::take_save_shortcut(&ctx);
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close {
            if !self.dirty()
                && self.session.is_none()
                && self.queue.is_empty()
                && !self.batch_running
                && !self.busy
            {
                self.allow_close = true;
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.pending = Some(Intent::Close);
            }
        }
        let dropped = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .collect::<Vec<_>>()
        });
        if !dropped.is_empty() {
            self.add_files(dropped);
            self.tab = 1;
        }
        self.request_render(&ctx);
        self.shell(ui);
        if (save_requested || self.save_clicked) && self.save_available() {
            self.finish(true);
        }
        self.matrix_dialog_ui(&ctx);
        self.dialogs(&ctx);
        self.request_latex_parse(&ctx);
        self.drain_before_close(&ctx);
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

    #[test]
    fn typed_latex_commands_build_real_math_and_keep_source_in_sync() {
        let mut a = app();
        // Separate keystrokes exercise the command buffer rather than the paste parser.
        for ch in r"\lim_{x\rightarrow+\infty}".chars() {
            type_canvas(&mut a, &ch.to_string());
        }
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
        type_canvas(&mut a, r"\frac{1}{\sqrt{2}}+x");
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
        for (key, prefix) in [
            (egui::Key::Enter, "alp"),
            (egui::Key::Tab, "alp"),
            (egui::Key::Space, "alpha"),
        ] {
            let mut a = app();
            let ctx = egui::Context::default();
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
            ctx.run_ui(
                input(vec![egui::Event::Text(format!("\\{prefix}"))]),
                |ui| a.shell(ui),
            )
            .textures_delta
            .clear();
            assert!(a.command_input.is_some());
            assert!(ctx.memory(|m| m.has_focus(egui::Id::unique("equation-canvas"))));
            let mut events = vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }];
            if key == egui::Key::Space {
                events.push(egui::Event::Text(" ".into()));
            }
            ctx.run_ui(input(events), |ui| a.shell(ui))
                .textures_delta
                .clear();
            assert!(
                a.editor.math().to_xml().contains('α'),
                "{key:?} must insert the completion"
            );
            assert!(a.command_input.is_none());
            assert_eq!(a.latex, a.editor.math().to_latex());
        }
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
                    ctx.set_theme(theme);
                    // Match the desktop's padding: taller buttons used to misalign earlier labels.
                    ctx.style_mut_of(theme, |style| {
                        style.spacing.item_spacing = Vec2::new(8.0, 8.0);
                        style.spacing.button_padding = Vec2::new(10.0, 7.0);
                    });
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
                            "Lưu và trở về tài liệu (Ctrl+S)"
                        } else {
                            "Lưu MathML (Ctrl+S)"
                        };
                        for text in [
                            label,
                            "source-marker",
                            "MathType",
                            "OMML",
                            "Cỡ chữ",
                            "↶",
                            "↷",
                        ] {
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
                                for painted in &output.shapes {
                                    if let egui::Shape::Rect(frame) = &painted.shape {
                                        if frame.fill == Color32::WHITE
                                            && frame.stroke.color == Color32::LIGHT_GRAY
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
                    let textures: std::collections::HashSet<_> = a
                        .palette_textures
                        .values()
                        .filter_map(|t| t.as_ref().map(egui::TextureHandle::id))
                        .collect();
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
                    output.textures_delta.clear();
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
            for &(group, _) in palette::GROUPS {
                a.category = group.into();
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
                        let textures: std::collections::HashSet<_> = a
                            .palette_textures
                            .values()
                            .filter_map(|t| t.as_ref().map(egui::TextureHandle::id))
                            .collect();
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
                        assert!(!rows.is_empty(), "{group} must contain rendered buttons");
                        assert!(
                            rows.len() <= 3,
                            "{group}: {} rows at width {width}, DPI {dpi}",
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
                            "{group}"
                        );
                        output.textures_delta.clear();
                    }
                }
            }
        }
    }

    #[test]
    fn batch_stays_visible_while_sidebar_groups_scroll() {
        let ctx = egui::Context::default();
        let mut a = app();
        let mut last_position = None;
        let mut last_scroll = -1.0f32;
        let mut time = 0.0f64;
        for step in [0.0, 300.0, 500.0] {
            time += 0.1;
            let mut events = vec![egui::Event::PointerMoved(Pos2::new(75.0, 200.0))];
            if step > 0.0 {
                events.push(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: Vec2::new(0.0, -step),
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Unknown,
                    modifiers: egui::Modifiers::NONE,
                });
            }
            let mut scroll_y = 0.0f32;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::from(MIN_WINDOW_SIZE))),
                    time: Some(time),
                    predicted_dt: 1.0 / 60.0,
                    events,
                    ..Default::default()
                },
                |ui| {
                    egui::CentralPanel::default().show(ui, |ui| {
                        ui.set_width(150.0);
                        let sa = a.sidebar(ui);
                        scroll_y = sa.state.clamped_offset().y;
                    });
                },
            );
            if step > 0.0 {
                // Step one settle frame so kinetic/smooth scroll applies.
                time += 0.05;
                output.textures_delta.clear();
                output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            Pos2::ZERO,
                            Vec2::from(MIN_WINDOW_SIZE),
                        )),
                        time: Some(time),
                        predicted_dt: 1.0 / 60.0,
                        events: vec![egui::Event::PointerMoved(Pos2::new(75.0, 200.0))],
                        ..Default::default()
                    },
                    |ui| {
                        egui::CentralPanel::default().show(ui, |ui| {
                            ui.set_width(150.0);
                            let sa = a.sidebar(ui);
                            scroll_y = sa.state.clamped_offset().y;
                        });
                    },
                );
                assert!(scroll_y > last_scroll, "expected categories to scroll");
            }
            last_scroll = scroll_y;
            let (rect, clip) = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(t) if t.galley.text() == "Batch DOCX" => {
                        Some((shape.shape.visual_bounding_rect(), shape.clip_rect))
                    }
                    _ => None,
                })
                .expect("Batch must be visible even with many groups");
            assert!(clip.contains_rect(rect));
            if let Some(previous) = last_position {
                assert_eq!(rect, previous);
            }
            last_position = Some(rect);
            output.textures_delta.clear();
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
                output.textures_delta.clear();
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
                    let canvas = output
                        .shapes
                        .iter()
                        .find_map(|s| match &s.shape {
                            egui::Shape::Rect(r)
                                if r.fill == Color32::WHITE
                                    && r.stroke.color == Color32::LIGHT_GRAY =>
                            {
                                Some(r)
                            }
                            _ => None,
                        })
                        .unwrap();
                    assert_eq!(
                        canvas.corner_radius,
                        ctx.style_of(ctx.theme())
                            .visuals
                            .widgets
                            .inactive
                            .corner_radius
                    );
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
        assert!(output.shapes.iter().any(|s| matches!(&s.shape,
            egui::Shape::Rect(r) if r.fill == Color32::WHITE && r.rect.width() > 200.0)));
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
        for (label, expected_tab) in [("Batch DOCX", 1), ("← Quay lại soạn công thức", 0)]
        {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| a.shell(ui));
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
            output.textures_delta.clear();
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
}
