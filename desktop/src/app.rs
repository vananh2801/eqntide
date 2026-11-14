use crate::EventSender as Sender;
use crate::{batch, instance::Instance, palette, runtime, Event, Rendered};
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use eqntide::{
    bridge::{DesktopWindowRequest, ServerHandle},
    MathNode,
};
use eqntide_editor::{Caret, Editor, Move};
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Receiver,
        Arc,
    },
    time::{Duration, Instant},
};

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
    pending: Option<Intent>,
    allow_close: bool,
    close_deadline: Option<Instant>,
    font_pt: f32,
    target: String,
    zoom: f32,
    tab: usize,
    category: String,
    status: String,
    texture: Option<egui::TextureHandle>,
    rendered: Option<Rendered>,
    render_key: u64,
    requested_key: u64,
    render_busy: bool,
    ime: String,
    files: Vec<PathBuf>,
    batch_logs: Vec<String>,
    batch_target: usize,
    batch_overwrite: bool,
    batch_totals: [usize; 4],
    batch_completed: usize,
    batch_running: bool,
    stop: Arc<AtomicBool>,
    copy_format: usize,
    clipboard: Option<arboard::Clipboard>,
}

impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        instance: Instance,
        tx: Sender<Event>,
        rx: Receiver<Event>,
        args: Vec<String>,
    ) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::light());
        tx.attach(cc.egui_ctx.clone());
        let mut fonts = egui::FontDefinitions::default();
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
            pending: None,
            allow_close: false,
            close_deadline: None,
            font_pt: 12.0,
            target: "omml".into(),
            zoom: 2.0,
            tab: 0,
            category: "Fractions".into(),
            status: "Click the canvas to type. Use the palette or Ctrl+F for a fraction.".into(),
            texture: None,
            rendered: None,
            render_key: 1,
            requested_key: 0,
            render_busy: false,
            ime: String::new(),
            files: vec![],
            batch_logs: vec![],
            batch_target: 0,
            batch_overwrite: false,
            batch_totals: [0; 4],
            batch_completed: 0,
            batch_running: false,
            stop: Arc::new(AtomicBool::new(false)),
            copy_format: 0,
            clipboard: None,
        }
    }
    fn dirty(&self) -> bool {
        self.draft_dirty
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
                    Ok(editor) => self.editor = editor,
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
                Event::Copied(result) => match result {
                    Ok(text) => {
                        if !text.is_empty() {
                            ctx.copy_text(text);
                        }
                        self.status = "Done.".into();
                    }
                    Err(e) => self.status = e,
                },
                Event::Image(image) => {
                    let result = (|| {
                        if self.clipboard.is_none() {
                            self.clipboard =
                                Some(arboard::Clipboard::new().map_err(|e| e.to_string())?);
                        }
                        self.clipboard
                            .as_mut()
                            .unwrap()
                            .set_image(image)
                            .map_err(|e| e.to_string())
                    })();
                    self.status = match result {
                        Ok(()) => "PNG copied.".into(),
                        Err(e) => e,
                    };
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
                n.children.push(MathNode::leaf("mi", "□"));
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
                    Ok(Rendered {
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
        if save && self.draft_dirty {
            self.status =
                "Wait for valid LaTeX, or revert to the current canvas before saving.".into();
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
                self.editor.insert_template(&template);
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
    fn copy(&mut self, format: usize, ctx: &egui::Context) {
        let math = self.selected_equation();
        if format == 0 {
            ctx.copy_text(math.to_latex());
            return;
        }
        if format == 1 {
            ctx.copy_text(math.to_xml());
            return;
        }
        let font = self.font_pt;
        let mode = self.target.clone();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            if format == 3 {
                let result = (|| {
                    let png = eqntide::render_preview(&math, font).map_err(|e| e.to_string())?;
                    let image = image::load_from_memory(&png.png)
                        .map_err(|e| e.to_string())?
                        .to_rgba8();
                    tx.send(Event::Image(arboard::ImageData {
                        width: image.width() as usize,
                        height: image.height() as usize,
                        bytes: std::borrow::Cow::Owned(image.into_raw()),
                    }))
                    .map_err(|e| e.to_string())?;
                    Ok(String::new())
                })();
                let _ = tx.send(Event::Copied(result));
            } else {
                let target = if mode == "mathtype" {
                    "flat_opc_mathtype"
                } else {
                    "flat_opc_omml"
                };
                let result =
                    eqntide::service::convert_snippet(&eqntide::bridge::ConvertSnippetRequest {
                        data: math.to_xml(),
                        from: "mathml".into(),
                        to: target.into(),
                        font_pt: font,
                    })
                    .map(|r| r.result)
                    .map_err(|e| e.to_string());
                let _ = tx.send(Event::Copied(result));
            }
        });
    }
    fn export_emf(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("EMF", &["emf"])
            .set_file_name("equation.emf")
            .save_file()
        {
            let math = self.editor.math();
            let font = self.font_pt;
            let tx = self.tx.clone();
            std::thread::spawn(move || {
                let result = eqntide::render_emf(&math, font)
                    .map_err(|e| e.to_string())
                    .and_then(|p| std::fs::write(path, p.emf).map_err(|e| e.to_string()));
                let _ = tx.send(Event::Copied(result.map(|_| String::new())));
            });
        }
    }
    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("EqnTide");
            ui.separator();
            ui.selectable_value(&mut self.tab, 0, "Equation editor");
            ui.selectable_value(&mut self.tab, 1, "Batch DOCX");
            if self.dirty() {
                ui.label(egui::RichText::new("● Unsaved").color(Color32::from_rgb(180, 90, 0)));
            }
        });
        ui.horizontal(|ui| {
            ui.label(&self.bridge_status);
            if !self.bridge_ready && ui.button("Retry").clicked() {
                runtime::start_bridge(self.tx.clone());
            }
            if !self.queue.is_empty() {
                ui.label(format!("{} session(s) queued", self.queue.len()));
                if ui.button("Next session").clicked() {
                    self.pending = Some(Intent::Next);
                }
            }
        });
        if self.tab == 0 || self.session.is_some() {
            ui.horizontal_wrapped(|ui| {
                let label = if self.session.is_some() {
                    "Lưu và trở về tài liệu (Ctrl+S)"
                } else {
                    "Lưu MathML (Ctrl+S)"
                };
                if ui
                    .add_enabled(
                        self.save_available() && !self.draft_dirty,
                        egui::Button::new(label),
                    )
                    .clicked()
                {
                    self.finish(true);
                }
                if self.draft_dirty {
                    if self.latex_error.is_some() {
                        ui.colored_label(
                            ui.visuals().error_fg_color,
                            "Sửa lỗi LaTeX trước khi lưu.",
                        );
                    } else {
                        ui.label("Đang cập nhật LaTeX, vui lòng chờ trước khi lưu…");
                    }
                }
            });
        }
        ui.separator();
    }
    fn save_available(&self) -> bool {
        !self.busy && !self.batch_running && self.pending.is_none() && !self.allow_close
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
        let ctx = ui.ctx().clone();
        ui.add_enabled_ui(!self.draft_dirty, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("Size (pt)");
                if ui
                    .add(
                        egui::DragValue::new(&mut self.font_pt)
                            .range(6.0..=72.0)
                            .speed(0.25),
                    )
                    .changed()
                {
                    self.render_key += 1;
                }
                ui.selectable_value(&mut self.target, "omml".into(), "OMML");
                ui.selectable_value(&mut self.target, "mathtype".into(), "MathType");
                ui.label("Zoom");
                ui.add(egui::Slider::new(&mut self.zoom, 1.0..=5.0).show_value(false));
                if ui.button("Undo").clicked() {
                    self.editor.undo();
                    self.changed();
                }
                if ui.button("Redo").clicked() {
                    self.editor.redo();
                    self.changed();
                }
            });
            ui.horizontal_wrapped(|ui| {
                for group in [
                    "Fractions",
                    "Scripts",
                    "Integrals",
                    "Fences",
                    "Matrices",
                    "Symbols",
                ] {
                    ui.selectable_value(&mut self.category, group.into(), group);
                }
            });
            egui::ScrollArea::vertical()
                .id_salt("palette")
                .max_height(92.0)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for (group, label, source) in palette::PALETTE {
                            if *group == self.category {
                                let short = label.split(" (").next().unwrap_or(label);
                                if ui.small_button(short).on_hover_text(*source).clicked() {
                                    self.insert_template(source);
                                    ui.memory_mut(|m| {
                                        m.request_focus(egui::Id::unique("equation-canvas"))
                                    });
                                }
                            }
                        }
                    });
                });
            ui.horizontal(|ui| {
                if ui.small_button("Matrix + row").clicked() {
                    if !self.editor.grow_matrix(false) {
                        self.status = "Place the caret inside a matrix first.".into();
                    } else {
                        self.changed();
                    }
                }
                if ui.small_button("Matrix − row").clicked() && self.editor.shrink_matrix(false) {
                    self.changed();
                }
                if ui.small_button("Matrix − column").clicked() && self.editor.shrink_matrix(true)
                {
                    self.changed();
                }
                if ui.small_button("Matrix + column").clicked() {
                    if !self.editor.grow_matrix(true) {
                        self.status = "Place the caret inside a matrix first.".into();
                    } else {
                        self.changed();
                    }
                }
            });
            self.canvas(ui);
        });
        ui.add_enabled_ui(!self.draft_dirty, |ui| {
            ui.horizontal(|ui| {
                let labels = ["LaTeX", "MathML", "Flat OPC", "PNG image"];
                egui::ComboBox::from_id_salt("copy-format")
                    .selected_text(labels[self.copy_format])
                    .show_ui(ui, |ui| {
                        for (i, label) in labels.iter().enumerate() {
                            ui.selectable_value(&mut self.copy_format, i, *label);
                        }
                    });
                if ui.button("Copy").clicked() {
                    self.copy(self.copy_format, &ctx);
                }
                if ui.button("Export EMF…").clicked() {
                    self.export_emf();
                }
            });
        });
        ui.label("LaTeX source — live preview");
        if ui
            .add(
                egui::TextEdit::multiline(&mut self.latex)
                    .id(egui::Id::unique("latex-source"))
                    .desired_rows(2)
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Monospace),
            )
            .changed()
        {
            self.source_changed(Instant::now());
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(self.draft_dirty, egui::Button::new("Revert to canvas"))
                .clicked()
            {
                self.changed();
            }
            if let Some(error) = &self.latex_error {
                ui.colored_label(ui.visuals().error_fg_color, format!("LaTeX error: {error}"));
            } else if self.draft_dirty {
                ui.label("Updating preview…");
            }
        });
    }

    fn canvas(&mut self, ui: &mut egui::Ui) {
        let scale = self.zoom * 96.0 / 72.0;
        egui::Frame::new()
            .fill(Color32::WHITE)
            .stroke(Stroke::new(1.0, Color32::LIGHT_GRAY))
            .inner_margin(12.0)
            .show(ui, |ui| {
                egui::ScrollArea::both()
                    .id_salt("formula-scroll")
                    .max_height(220.0)
                    .min_scrolled_height(135.0)
                    .show(ui, |ui| {
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
                        let available = ui.available_width().max(size.x + 12.0);
                        let (_, rect) =
                            ui.allocate_space(Vec2::new(available, 135.0_f32.max(size.y + 20.0)));
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
                        if response.clicked() || response.drag_started() {
                            response.request_focus();
                        }
                        if response.drag_started() {
                            if let Some(pos) = ui.input(|i| i.pointer.press_origin()) {
                                if let Some(caret) = self.hit(pos, origin, scale) {
                                    self.editor.set_caret(caret, false);
                                }
                            }
                        }
                        if (response.clicked() || response.dragged()) && !self.render_busy {
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
                                    2.0,
                                    Color32::from_rgba_unmultiplied(60, 120, 230, 65),
                                );
                            }
                        }
                        if response.has_focus() {
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
                    });
            });
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
    fn keyboard(&mut self, ui: &mut egui::Ui) {
        let events = ui.input(|i| i.events.clone());
        let before = self.editor.revision;
        for event in events {
            match event {
                egui::Event::Copy => self.copy(0, ui.ctx()),
                egui::Event::Cut => {
                    if self.editor.selected_math().is_some() {
                        self.copy(0, ui.ctx());
                        self.editor.delete(true);
                    }
                }
                egui::Event::Paste(text) => {
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
                        match text.as_str() {
                            "/" => {
                                if let Ok(template) = palette::template("\\frac{#?}{#?}") {
                                    self.editor.insert_template_wrapping_previous(&template);
                                }
                            }
                            "^" => self.insert_template("^{#?}"),
                            "_" => self.insert_template("_{#?}"),
                            _ => self.editor.insert_text(&text),
                        }
                    }
                }
                egui::Event::Ime(egui::ImeEvent::Preedit { text, .. }) => self.ime = text,
                egui::Event::Ime(egui::ImeEvent::Commit(text)) => {
                    self.ime.clear();
                    self.editor.insert_text(&text);
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
        if self.pending.is_some() {
            egui::Window::new("Finish current equation")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(if self.dirty() {
                        "Save your changes, discard them, or continue editing."
                    } else {
                        "Finish or cancel the current Office session before continuing."
                    });
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
        egui::CentralPanel::default().show(ui, |ui| {
            self.toolbar(ui);
            ui.add_enabled_ui(!self.busy && self.pending.is_none(), |ui| {
                if self.tab == 0 {
                    egui::ScrollArea::vertical()
                        .id_salt("editor-layout")
                        .show(ui, |ui| self.editor_ui(ui));
                } else {
                    self.batch_ui(ui);
                }
            });
            ui.separator();
            ui.label(&self.status);
            if self.busy || self.render_busy {
                ui.spinner();
            }
        });
        if save_requested && self.save_available() {
            self.finish(true);
        }
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
    fn save_button_stays_visible_above_scrolled_editor_at_minimum_size() {
        for office in [false, true] {
            let mut previous_rect = None;
            for offset in [0.0, 600.0] {
                let mut a = app();
                a.bridge_ready = true;
                if office {
                    a.session = Some("office".into());
                }
                let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(600.0, 450.0));
                let mut output = egui::Context::default().run_ui(
                    egui::RawInput {
                        screen_rect: Some(screen),
                        ..Default::default()
                    },
                    |ui| {
                        a.toolbar(ui);
                        egui::ScrollArea::vertical()
                            .id_salt("editor-layout")
                            .vertical_scroll_offset(offset)
                            .show(ui, |ui| a.editor_ui(ui));
                    },
                );
                output.textures_delta.clear();
                let label = if office {
                    "Lưu và trở về tài liệu (Ctrl+S)"
                } else {
                    "Lưu MathML (Ctrl+S)"
                };
                let shape = output.shapes.iter().find(|shape| {
                    matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == label)
                }).expect("fixed save button must be rendered");
                let rect = shape.shape.visual_bounding_rect();
                assert!(screen.contains_rect(rect));
                assert!(shape.clip_rect.contains_rect(rect));
                if let Some(previous) = previous_rect {
                    assert_eq!(rect, previous);
                }
                previous_rect = Some(rect);
            }
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
    fn reverting_source_invalidates_its_inflight_parse() {
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
