use super::super::*;
pub(super) use eframe::egui::Pos2;
use eqntide::MathNode;
use std::time::Duration;

pub(super) fn app() -> App {
    let (tx, rx) = std::sync::mpsc::channel();
    App::from_parts(None, Sender::new(tx), rx, vec![])
}

pub(super) fn request(id: &str, mathml: Option<&str>) -> DesktopWindowRequest {
    DesktopWindowRequest {
        session_id: id.into(),
        mathml: mathml.map(str::to_owned),
        latex: None,
        font_pt: Some(14.0),
        tab: None,
        target_mode: Some("mathtype".into()),
    }
}

pub(super) fn draw_dialogs(app: &mut App) -> bool {
    let ctx = egui::Context::default();
    ctx.run_ui(egui::RawInput::default(), |ui| app.draw_dialogs(ui.ctx()))
        .textures_delta
        .clear();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| app.draw_dialogs(ui.ctx()));
    output.textures_delta.clear();
    output.shapes.iter().any(|clipped| {
        matches!(&clipped.shape, egui::Shape::Text(text_shape) if text_shape.galley.text() == "Finish current equation")
    })
}

pub(super) fn save_key(shift: bool) -> egui::Event {
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

pub(super) fn type_canvas(app: &mut App, text: &str) {
    egui::Context::default()
        .run_ui(
            egui::RawInput {
                events: vec![egui::Event::Text(text.into())],
                ..Default::default()
            },
            |ui| app.handle_canvas_keyboard(ui),
        )
        .textures_delta
        .clear();
}

pub(super) fn canvas_key(app: &mut App, key: egui::Key) {
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
            |ui| app.handle_canvas_keyboard(ui),
        )
        .textures_delta
        .clear();
}

pub(super) fn source_request(app: &mut App, source: &str, now: Instant) -> (u64, String) {
    app.latex = source.into();
    app.mark_source_changed(now);
    app.take_latex_request(now + Duration::from_millis(180))
        .unwrap()
}

pub(super) fn parsed(app: &mut App, generation: u64, result: Result<MathNode, String>) {
    app.tx.send(Event::Latex(generation, result)).unwrap();
    app.process_events(&egui::Context::default());
}

pub(super) fn click_events(pos: Pos2) -> Vec<egui::Event> {
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

pub(super) fn run_ui_pass_frame(
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
        |ui| app.draw_ui(ui),
    );
    output.textures_delta.clear();
    output
}

#[track_caller]
pub(super) fn find_text_center(
    output: &egui::FullOutput,
    predicate: impl Fn(&str) -> bool,
) -> Pos2 {
    output
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text_shape) if predicate(text_shape.galley.text()) => {
                Some(clipped.shape.visual_bounding_rect().center())
            }
            _ => None,
        })
        .expect("expected matching text shape")
}

pub(super) fn run_ui_pass_sized(
    ctx: &egui::Context,
    app: &mut App,
    size: Vec2,
) -> egui::FullOutput {
    run_ui_pass_sized_events(ctx, app, size, vec![])
}

pub(super) fn run_ui_pass_sized_events(
    ctx: &egui::Context,
    app: &mut App,
    size: Vec2,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
            events,
            ..Default::default()
        },
        |ui| app.draw_ui(ui),
    );
    output.textures_delta.clear();
    output
}

pub(super) fn card_rects_for_section_titles(
    output: &egui::FullOutput,
    tokens: ThemeTokens,
    titles: &[&str],
) -> Vec<Rect> {
    titles
        .iter()
        .map(|title| {
            let title_pos = find_text_center(output, |text| text == *title);
            output
                .shapes
                .iter()
                .find_map(|clipped| match &clipped.shape {
                    egui::Shape::Rect(rect_shape)
                        if rect_shape.fill == tokens.card_bg
                            && rect_shape.corner_radius == theme::SURFACE_CORNER_RADIUS
                            && rect_shape.rect.contains(title_pos) =>
                    {
                        Some(rect_shape.rect)
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("missing card rect containing '{title}'"))
        })
        .collect()
}

pub(super) fn has_rendered_text(output: &egui::FullOutput, needle: &str) -> bool {
    output.shapes.iter().any(|clipped| match &clipped.shape {
        egui::Shape::Text(text_shape) => text_shape.galley.text().contains(needle),
        _ => false,
    })
}
