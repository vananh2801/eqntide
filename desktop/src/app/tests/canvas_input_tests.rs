use super::super::*;
use super::helpers::*;
use eframe::egui::Color32;
use eqntide::MathNode;
use eqntide_editor::Move;
use std::time::Duration;

#[test]
fn canvas_inserts_text_only_when_enter_confirms_a_complete_argument() {
    use egui::Key;
    let mut app = super::helpers::app();
    type_canvas(&mut app, "x + 1");
    let before = app.editor.math();

    type_canvas(&mut app, r"\text ");
    canvas_key(&mut app, Key::Enter);
    assert!(app.status.contains("Invalid LaTeX command"));
    assert_eq!(app.editor.math(), before);

    type_canvas(&mut app, "{5 cm ");
    assert_eq!(app.editor.math(), before);
    type_canvas(&mut app, "} ");
    assert_eq!(app.editor.math(), before);
    assert!(app.command_input.is_some());
    canvas_key(&mut app, Key::Enter);
    assert!(app.command_input.is_none());
    assert!(!app.is_caret_in_text());
    assert_eq!(app.editor.math().find_all("mtext")[0].text, "5 cm ");

    let before = app.editor.math();
    type_canvas(&mut app, r"\text+");
    canvas_key(&mut app, Key::Enter);
    assert!(app.status.contains("Invalid LaTeX command"));
    assert_eq!(app.editor.math(), before);
}

#[test]
fn canvas_arrows_and_space_distinguish_text_and_math_at_both_edges() {
    use egui::Key;
    let mut app = super::helpers::app();
    type_canvas(&mut app, r"\text{cm}");
    canvas_key(&mut app, Key::Enter);
    type_canvas(&mut app, "x");
    canvas_key(&mut app, Key::ArrowLeft);
    assert!(!app.is_caret_in_text());
    type_canvas(&mut app, " ");
    assert_eq!(app.editor.math().to_latex(), r"\text{cm}x");
    canvas_key(&mut app, Key::ArrowLeft);
    assert!(app.is_caret_in_text());
    assert_eq!(app.editor.caret().offset, 2);
    canvas_key(&mut app, Key::ArrowLeft);
    assert_eq!(app.editor.caret().offset, 1);
    canvas_key(&mut app, Key::ArrowRight);
    assert!(app.is_caret_in_text());
    assert_eq!(app.editor.caret().offset, 2);
    canvas_key(&mut app, Key::ArrowRight);
    assert!(!app.is_caret_in_text());
    canvas_key(&mut app, Key::ArrowLeft);
    type_canvas(&mut app, " ");
    assert_eq!(app.editor.math().to_latex(), r"\text{cm }x");

    canvas_key(&mut app, Key::Home);
    assert!(!app.is_caret_in_text());
    canvas_key(&mut app, Key::ArrowRight);
    assert!(app.is_caret_in_text());
    assert_eq!(app.editor.caret().offset, 0);
    canvas_key(&mut app, Key::ArrowLeft);
    assert!(!app.is_caret_in_text());
    type_canvas(&mut app, " ");
    assert_eq!(app.editor.math().find_all("mtext")[0].text, "cm ");
    canvas_key(&mut app, Key::ArrowRight);
    type_canvas(&mut app, r" \/_^");
    assert!(app.command_input.is_none());
    assert_eq!(app.editor.math().find_all("mtext")[0].text, r" \/_^cm ");
}

#[test]
fn enter_leaves_canvas_text_and_resumes_math_input() {
    use egui::Key;
    let mut app = super::helpers::app();
    type_canvas(&mut app, r"\text{ngày mà em đến}");
    canvas_key(&mut app, Key::Enter);
    canvas_key(&mut app, Key::ArrowLeft);
    assert!(app.is_caret_in_text());
    let before = app.editor.math();

    canvas_key(&mut app, Key::Enter);
    assert_eq!(app.editor.math(), before);
    assert!(!app.is_caret_in_text());
    type_canvas(&mut app, "x+1");
    assert_eq!(app.editor.math().to_latex(), r"\text{ngày mà em đến}x+1");
}

#[test]
fn canvas_arrows_keep_decomposed_vietnamese_letters_together() {
    use egui::Key;
    let mut app = super::helpers::app();
    type_canvas(&mut app, "\\text{e\u{0302}\u{0301}x}");
    canvas_key(&mut app, Key::Enter);
    canvas_key(&mut app, Key::ArrowLeft);
    assert_eq!(app.editor.caret().offset, 4);
    canvas_key(&mut app, Key::ArrowLeft);
    assert_eq!(app.editor.caret().offset, 3);
    canvas_key(&mut app, Key::ArrowLeft);
    assert_eq!(app.editor.caret().offset, 0);
    assert!(app.is_caret_in_text());
}

#[test]
fn canvas_caret_switches_to_the_active_text_color_without_moving() {
    let mut app = super::helpers::app();
    type_canvas(&mut app, r"\text{ngày đến}");
    canvas_key(&mut app, egui::Key::Enter);
    type_canvas(&mut app, "=x");
    let text = app.editor.root().children[0].id;
    let math_caret = Caret {
        node: app.editor.root().id,
        offset: 1,
    };
    for (theme, dark) in [(egui::Theme::Light, false), (egui::Theme::Dark, true)] {
        let ctx = egui::Context::default();
        ctx.set_theme(theme);
        let pal = app.active_canvas_palette(dark);
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
            app.editor.set_caret(caret, false);
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.memory_mut(|mem| mem.request_focus(App::editor_focus_id(StartupFocus::Canvas)));
                app.draw_canvas(ui);
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
    let mut app = super::helpers::app();
    assert!(app.load_math(
        "<math><mfrac><mtext>cm</mtext><mn>2</mn></mfrac></math>",
        None,
        None
    ));
    let id = app.editor.root().children[0].children[0].children[0].id;
    app.editor.set_caret(
        Caret {
            node: id,
            offset: 2,
        },
        false,
    );
    canvas_key(&mut app, egui::Key::ArrowRight);
    assert!(!app.is_caret_in_text());
    type_canvas(&mut app, "x");
    assert_eq!(app.editor.math().to_latex(), r"\frac{\text{cm}x}{2}");
}

#[test]
fn typed_latex_commands_build_real_math_and_keep_source_in_sync() {
    let mut app = super::helpers::app();
    // Separate keystrokes exercise the buffered command input without early placeholder jumps.
    for ch in r"\lim_{x\rightarrow+\infty}".chars() {
        type_canvas(&mut app, &ch.to_string());
    }
    assert_eq!(
        app.command_input.as_ref().map(|cmd| cmd.name.as_str()),
        Some(r"lim_{x\rightarrow+\infty}")
    );
    assert!(app.handle_command_key(egui::Key::Enter, egui::Modifiers::NONE));
    let xml = app.editor.math().to_xml();
    assert!(xml.contains("lim"), "{xml}");
    assert!(xml.contains('→') && xml.contains('∞'), "{xml}");
    assert!(
        !xml.contains('\\'),
        "commands must not become literal characters: {xml}"
    );
    assert!(app.command_input.is_none());
    assert_eq!(app.latex, app.editor.math().to_latex());
    assert!(eqntide::render::render_interactive(&app.editor.math(), 12.0).is_ok());
}

#[test]
fn typed_fraction_groups_fill_slots_and_exit_back_to_the_outer_equation() {
    let mut app = super::helpers::app();
    type_canvas(&mut app, r"\frac{1}{\sqrt{2}}");
    assert!(app.command_input.is_some());
    assert!(app.handle_command_key(egui::Key::Enter, egui::Modifiers::NONE));
    type_canvas(&mut app, "+x");
    assert_eq!(app.editor.math().to_latex(), r"\frac{1}{\sqrt{2}}+x");
    assert!(app.command_input.is_none());
    assert!(app.latex_groups.is_empty());
    type_canvas(&mut app, "{y}");
    assert!(
        app.editor.math().to_xml().contains('{'),
        "literal braces still work outside LaTeX groups"
    );
}

#[test]
fn dfrac_completion_inserts_fraction_slots_and_accepts_typed_arguments() {
    let mut app = super::helpers::app();
    type_canvas(&mut app, r"\df");
    assert!(app.handle_command_key(egui::Key::Enter, egui::Modifiers::NONE));
    assert!(app
        .editor
        .math()
        .iter()
        .iter()
        .any(|node| node.tag == "mfrac"));
    type_canvas(&mut app, "{1}{2}");
    assert_eq!(app.editor.math().to_latex(), r"\dfrac{1}{2}");
    assert_eq!(app.latex, app.editor.math().to_latex());
}

#[test]
fn completion_navigation_cancel_and_unknown_commands_keep_existing_work() {
    let mut app = super::helpers::app();
    type_canvas(&mut app, "x");
    let before = app.editor.math();
    type_canvas(&mut app, r"\l");
    app.handle_command_key(egui::Key::ArrowDown, egui::Modifiers::NONE);
    assert_eq!(app.command_input.as_ref().unwrap().selected, 1);
    app.handle_command_key(egui::Key::ArrowUp, egui::Modifiers::NONE);
    assert_eq!(app.command_input.as_ref().unwrap().selected, 0);
    app.handle_command_key(egui::Key::Escape, egui::Modifiers::NONE);
    type_canvas(&mut app, r"\alp");
    assert_eq!(app.editor.math(), before);
    assert!(app.handle_command_key(egui::Key::Tab, egui::Modifiers::NONE));
    assert_eq!(
        app.command_input.as_ref().map(|cmd| cmd.name.as_str()),
        Some("alpha"),
        "Tab only completes the command name in the buffer without committing"
    );
    assert_eq!(app.editor.math(), before);
    assert!(app.handle_command_key(egui::Key::Enter, egui::Modifiers::NONE));
    assert!(app.editor.math().to_xml().contains('α'));
    let accepted = app.editor.math();
    type_canvas(&mut app, r"\nosuchcommand+1");
    assert_eq!(app.command_input.as_ref().unwrap().name, "nosuchcommand+1");
    app.finish_session(true);
    assert!(!app.busy);
    assert_eq!(app.editor.math(), accepted);
    app.handle_command_key(egui::Key::Backspace, egui::Modifiers::NONE);
    assert_eq!(app.command_input.as_ref().unwrap().name, "nosuchcommand+");
    app.handle_command_key(egui::Key::Escape, egui::Modifiers::NONE);
    assert!(app.command_input.is_none());
    assert!(
        app.pending.is_none(),
        "Esc must cancel completion without closing the app"
    );
    app.editor.undo();
    assert_eq!(
        app.editor.math(),
        before,
        "accepting a command is one undo transaction"
    );
}

#[test]
fn completion_accept_keys_work_through_the_focused_canvas() {
    let setup_focused = |app: &mut App, ctx: &egui::Context| {
        let input = |events| egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 650.0))),
            events,
            ..Default::default()
        };
        ctx.run_ui(input(vec![]), |ui| {
            app.draw_shell(ui);
            ui.memory_mut(|mem| mem.request_focus(egui::Id::unique("equation-canvas")));
        })
        .textures_delta
        .clear();
    };
    let send_events = |app: &mut App, ctx: &egui::Context, events: Vec<egui::Event>| {
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 650.0))),
                events,
                ..Default::default()
            },
            |ui| app.draw_shell(ui),
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
    let mut enter_app = super::helpers::app();
    let ctx = egui::Context::default();
    setup_focused(&mut enter_app, &ctx);
    send_events(
        &mut enter_app,
        &ctx,
        vec![egui::Event::Text(r"\alp".into())],
    );
    assert!(enter_app.command_input.is_some());
    assert!(ctx.memory(|mem| mem.has_focus(egui::Id::unique("canvas-command-input"))));
    send_events(&mut enter_app, &ctx, vec![key_event(egui::Key::Enter)]);
    assert!(enter_app.editor.math().to_xml().contains('α'));
    assert!(enter_app.command_input.is_none());
    assert!(ctx.memory(|mem| mem.has_focus(egui::Id::unique("equation-canvas"))));
    assert_eq!(enter_app.latex, enter_app.editor.math().to_latex());

    // Tab completes command name without committing; Enter commits afterwards.
    let mut tab_app = super::helpers::app();
    let ctx = egui::Context::default();
    setup_focused(&mut tab_app, &ctx);
    send_events(&mut tab_app, &ctx, vec![egui::Event::Text(r"\math".into())]);
    send_events(&mut tab_app, &ctx, vec![key_event(egui::Key::Tab)]);
    assert_eq!(
        tab_app.command_input.as_ref().map(|cmd| cmd.name.as_str()),
        Some("mathbb")
    );
    assert!(ctx.memory(|mem| mem.has_focus(egui::Id::unique("canvas-command-input"))));
    send_events(&mut tab_app, &ctx, vec![egui::Event::Text("{R}".into())]);
    assert_eq!(
        tab_app.command_input.as_ref().map(|cmd| cmd.name.as_str()),
        Some("mathbb{R}")
    );
    send_events(&mut tab_app, &ctx, vec![key_event(egui::Key::Enter)]);
    assert!(tab_app.command_input.is_none());
    assert!(ctx.memory(|mem| mem.has_focus(egui::Id::unique("equation-canvas"))));
    assert!(tab_app.editor.math().to_latex().contains(r"\mathbb{R}"));

    // Space remains literal inside the command buffer until Enter commits.
    let mut space_app = super::helpers::app();
    let ctx = egui::Context::default();
    setup_focused(&mut space_app, &ctx);
    send_events(
        &mut space_app,
        &ctx,
        vec![egui::Event::Text(r"\lim_{x".into())],
    );
    send_events(
        &mut space_app,
        &ctx,
        vec![key_event(egui::Key::Space), egui::Event::Text(" ".into())],
    );
    assert_eq!(
        space_app
            .command_input
            .as_ref()
            .map(|cmd| cmd.name.as_str()),
        Some("lim_{x ")
    );
    send_events(
        &mut space_app,
        &ctx,
        vec![egui::Event::Text(r"\to 0}".into())],
    );
    send_events(&mut space_app, &ctx, vec![key_event(egui::Key::Enter)]);
    assert!(space_app.command_input.is_none());
    let xml = space_app.editor.math().to_xml();
    assert!(xml.contains("lim") && xml.contains('→'), "{xml}");
}

#[test]
fn completion_buffered_latex_handles_mathbb_braces_and_integrals() {
    // \math + { stays buffered, Backspace + Tab completes \mathbb, then {R} + Enter commits.
    let mut mathbb_app = super::helpers::app();
    for ch in r"\math{".chars() {
        type_canvas(&mut mathbb_app, &ch.to_string());
    }
    assert_eq!(
        mathbb_app
            .command_input
            .as_ref()
            .map(|cmd| cmd.name.as_str()),
        Some("math{")
    );
    assert!(mathbb_app.latex_groups.is_empty());
    assert!(mathbb_app.handle_command_key(egui::Key::Backspace, egui::Modifiers::NONE));
    assert!(mathbb_app.handle_command_key(egui::Key::Tab, egui::Modifiers::NONE));
    assert_eq!(
        mathbb_app
            .command_input
            .as_ref()
            .map(|cmd| cmd.name.as_str()),
        Some("mathbb")
    );
    for ch in "{R}".chars() {
        type_canvas(&mut mathbb_app, &ch.to_string());
    }
    assert!(mathbb_app.handle_command_key(egui::Key::Enter, egui::Modifiers::NONE));
    assert!(mathbb_app.command_input.is_none());
    assert!(mathbb_app.editor.math().to_latex().contains(r"\mathbb{R}"));

    // \int_0^1 x dx stays buffered across '_', '^', and spaces until Enter commits once.
    let mut integral_app = super::helpers::app();
    for ch in r"\int_0^1 x dx".chars() {
        type_canvas(&mut integral_app, &ch.to_string());
    }
    assert_eq!(
        integral_app
            .command_input
            .as_ref()
            .map(|cmd| cmd.name.as_str()),
        Some("int_0^1 x dx")
    );
    assert!(integral_app.handle_command_key(egui::Key::Enter, egui::Modifiers::NONE));
    assert!(integral_app.command_input.is_none());
    let xml = integral_app.editor.math().to_xml();
    assert!(
        xml.contains('∫') && (xml.contains("msubsup") || xml.contains("munderover")),
        "{xml}"
    );
    integral_app.editor.undo();
    assert!(integral_app.editor.math().children.is_empty());
}

#[test]
fn completion_buffer_supports_cursor_navigation_and_mid_string_editing() {
    let mut app = super::helpers::app();
    type_canvas(&mut app, r"\frac{1}{2}");
    assert!(app.handle_command_key(egui::Key::ArrowLeft, egui::Modifiers::NONE));
    assert!(app.handle_command_key(egui::Key::ArrowLeft, egui::Modifiers::NONE));
    assert!(app.handle_command_key(egui::Key::ArrowLeft, egui::Modifiers::NONE));
    assert!(app.handle_command_key(egui::Key::ArrowLeft, egui::Modifiers::NONE));
    type_canvas(&mut app, "0");
    assert_eq!(
        app.command_input.as_ref().map(|cmd| cmd.name.as_str()),
        Some("frac{10}{2}")
    );
    assert!(app.handle_command_key(egui::Key::Backspace, egui::Modifiers::NONE));
    assert_eq!(
        app.command_input.as_ref().map(|cmd| cmd.name.as_str()),
        Some("frac{1}{2}")
    );

    assert!(app.handle_command_key(egui::Key::Home, egui::Modifiers::NONE));
    assert!(app.handle_command_key(egui::Key::Backspace, egui::Modifiers::NONE));
    assert_eq!(
        app.command_input.as_ref().map(|cmd| cmd.name.as_str()),
        Some("frac{1}{2}")
    );
    assert!(app.handle_command_key(egui::Key::Delete, egui::Modifiers::NONE));
    type_canvas(&mut app, "d");
    assert!(app.handle_command_key(egui::Key::ArrowRight, egui::Modifiers::NONE));
    assert!(app.handle_command_key(egui::Key::End, egui::Modifiers::NONE));
    type_canvas(&mut app, "+x");
    assert_eq!(
        app.command_input.as_ref().map(|cmd| cmd.name.as_str()),
        Some("drac{1}{2}+x")
    );
    assert!(app.handle_command_key(egui::Key::Home, egui::Modifiers::NONE));
    assert!(app.handle_command_key(egui::Key::Delete, egui::Modifiers::NONE));
    type_canvas(&mut app, "f");
    assert!(app.handle_command_key(egui::Key::Enter, egui::Modifiers::NONE));
    assert!(app.command_input.is_none());
    assert_eq!(app.editor.math().to_latex(), r"\frac{1}{2}+x");
}

#[test]
fn completion_popup_textbox_supports_arrow_navigation_and_editing() {
    let send_events = |app: &mut App, ctx: &egui::Context, events: Vec<egui::Event>| {
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 650.0))),
                events,
                ..Default::default()
            },
            |ui| app.draw_shell(ui),
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

    let mut app = super::helpers::app();
    let ctx = egui::Context::default();
    ThemeTokens::apply_style(&ctx);
    send_events(&mut app, &ctx, vec![]);
    assert!(ctx.memory(|mem| mem.has_focus(canvas_id)));

    // Open popup with \frac{1}{2}, move left 4 times, insert 0 -> frac{10}{2}, backspace -> frac{1}{2}.
    send_events(
        &mut app,
        &ctx,
        vec![egui::Event::Text(r"\frac{1}{2}".into())],
    );
    assert!(ctx.memory(|mem| mem.has_focus(cmd_id)));
    for _ in 0..4 {
        send_events(&mut app, &ctx, vec![key_event(egui::Key::ArrowLeft)]);
    }
    send_events(&mut app, &ctx, vec![egui::Event::Text("0".into())]);
    assert_eq!(
        app.command_input.as_ref().map(|cmd| cmd.name.as_str()),
        Some("frac{10}{2}")
    );
    send_events(&mut app, &ctx, vec![key_event(egui::Key::Backspace)]);
    assert_eq!(
        app.command_input.as_ref().map(|cmd| cmd.name.as_str()),
        Some("frac{1}{2}")
    );

    // Home, Delete, insert 'd', End, insert '+x', Home, Delete, insert 'f', Enter -> \frac{1}{2}+x.
    send_events(&mut app, &ctx, vec![key_event(egui::Key::Home)]);
    send_events(&mut app, &ctx, vec![key_event(egui::Key::Delete)]);
    send_events(&mut app, &ctx, vec![egui::Event::Text("d".into())]);
    send_events(&mut app, &ctx, vec![key_event(egui::Key::End)]);
    send_events(&mut app, &ctx, vec![egui::Event::Text("+x".into())]);
    assert_eq!(
        app.command_input.as_ref().map(|cmd| cmd.name.as_str()),
        Some("drac{1}{2}+x")
    );
    send_events(&mut app, &ctx, vec![key_event(egui::Key::Home)]);
    send_events(&mut app, &ctx, vec![key_event(egui::Key::Delete)]);
    send_events(&mut app, &ctx, vec![egui::Event::Text("f".into())]);
    send_events(&mut app, &ctx, vec![key_event(egui::Key::Enter)]);
    assert!(app.command_input.is_none());
    assert_eq!(app.editor.math().to_latex(), r"\frac{1}{2}+x");
    assert!(ctx.memory(|mem| mem.has_focus(canvas_id)));

    // Bare command name ignores Space; Backspace on empty closes popup and restores canvas focus.
    send_events(&mut app, &ctx, vec![egui::Event::Text(r"\text".into())]);
    send_events(
        &mut app,
        &ctx,
        vec![key_event(egui::Key::Space), egui::Event::Text(" ".into())],
    );
    assert_eq!(
        app.command_input.as_ref().map(|cmd| cmd.name.as_str()),
        Some("text")
    );
    send_events(&mut app, &ctx, vec![key_event(egui::Key::Escape)]);
    assert!(app.command_input.is_none());
    assert!(ctx.memory(|mem| mem.has_focus(canvas_id)));

    send_events(&mut app, &ctx, vec![egui::Event::Text(r"\".into())]);
    assert!(app.command_input.is_some());
    send_events(&mut app, &ctx, vec![key_event(egui::Key::Backspace)]);
    assert!(app.command_input.is_none());
    assert!(ctx.memory(|mem| mem.has_focus(canvas_id)));
}

#[test]
fn completion_popup_textbox_pointer_click_positions_cursor() {
    let send_events = |app: &mut App, ctx: &egui::Context, events: Vec<egui::Event>| {
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 650.0))),
                events,
                ..Default::default()
            },
            |ui| app.draw_shell(ui),
        );
        out.textures_delta.clear();
        out
    };
    let canvas_id = egui::Id::unique("equation-canvas");
    let cmd_id = egui::Id::unique("canvas-command-input");
    let latex_id = egui::Id::unique("latex-source");

    let mut app = super::helpers::app();
    let ctx = egui::Context::default();
    ThemeTokens::apply_style(&ctx);
    send_events(&mut app, &ctx, vec![]);
    send_events(&mut app, &ctx, vec![egui::Event::Text(r"\int_0^1".into())]);
    let out = send_events(&mut app, &ctx, vec![]);
    assert!(ctx.memory(|mem| mem.has_focus(cmd_id)));

    fn find_galley_click(shape: &egui::Shape) -> Option<Pos2> {
        match shape {
            egui::Shape::Text(text_shape) if text_shape.galley.text() == "int_0^1" => {
                let cpos = text_shape
                    .galley
                    .pos_from_cursor(egui::text::CCursor::new(4));
                Some(text_shape.pos + Vec2::new(cpos.min.x, cpos.center().y))
            }
            egui::Shape::Vec(shapes) => shapes.iter().find_map(find_galley_click),
            _ => None,
        }
    }

    // Find the rendered galley for "int_0^1" inside the popup textbox and click between '_' and '0'.
    let click_pos = out
        .shapes
        .iter()
        .find_map(|clipped| find_galley_click(&clipped.shape))
        .expect("popup command textbox galley must be visible");

    send_events(&mut app, &ctx, click_events(click_pos));
    assert!(app.command_input.is_some());
    assert!(ctx.memory(|mem| mem.has_focus(cmd_id)));
    send_events(&mut app, &ctx, vec![egui::Event::Text("a".into())]);
    assert_eq!(
        app.command_input.as_ref().map(|cmd| cmd.name.as_str()),
        Some("int_a0^1")
    );

    // Clicking outside onto the Equation Canvas closes the popup without committing.
    let canvas_rect = ctx.read_response(canvas_id).unwrap().rect;
    send_events(
        &mut app,
        &ctx,
        click_events(Pos2::new(
            canvas_rect.right() - 20.0,
            canvas_rect.bottom() - 20.0,
        )),
    );
    assert!(app.command_input.is_none());
    assert!(app.editor.math().children.is_empty());

    // Clicking into LaTeX source while popup is open also dismisses the popup.
    send_events(&mut app, &ctx, vec![egui::Event::Text(r"\alp".into())]);
    assert!(app.command_input.is_some());
    let latex_rect = ctx.read_response(latex_id).unwrap().rect;
    send_events(&mut app, &ctx, click_events(latex_rect.center()));
    assert!(app.command_input.is_none());
    assert!(ctx.memory(|mem| mem.has_focus(latex_id)));
}

#[test]
fn completion_popup_accepts_mouse_selection() {
    let mut app = super::helpers::app();
    type_canvas(&mut app, r"\al");
    let ctx = egui::Context::default();
    let mut choice = None;
    for _ in 0..2 {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 650.0))),
                ..Default::default()
            },
            |ui| app.draw_command_popup(ui, Pos2::new(100.0, 100.0)),
        );
        choice = output.shapes.iter().find_map(|clipped| {
            if let egui::Shape::Text(text_shape) = &clipped.shape {
                if text_shape.galley.text().starts_with(r"\alpha ") {
                    return Some(clipped.shape.visual_bounding_rect().center());
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
        |ui| app.draw_command_popup(ui, Pos2::new(100.0, 100.0)),
    )
    .textures_delta
    .clear();
    assert!(app.command_input.is_none());
    assert!(app.editor.math().to_xml().contains('α'));
    assert_eq!(app.latex, app.editor.math().to_latex());
}

#[test]
fn argument_requiring_commands_block_bare_or_empty_enter_and_show_status_error() {
    use egui::Key;

    // Prefix auto-completes on Enter without committing and shows error in the bottom status bar.
    let mut app = super::helpers::app();
    let ctx = egui::Context::default();
    ThemeTokens::apply_style(&ctx);
    let setup_focused = |app: &mut App, ctx: &egui::Context| {
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 650.0))),
                ..Default::default()
            },
            |ui| {
                app.draw_shell(ui);
                ui.memory_mut(|mem| mem.request_focus(egui::Id::unique("equation-canvas")));
            },
        )
        .textures_delta
        .clear();
    };
    let send_events = |app: &mut App, ctx: &egui::Context, events: Vec<egui::Event>| {
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 650.0))),
                events,
                ..Default::default()
            },
            |ui| app.draw_shell(ui),
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
                egui::Shape::Text(text_shape) => text_shape.galley.text().contains(needle),
                egui::Shape::Vec(shapes) => shapes.iter().any(|child| visit(child, needle)),
                _ => false,
            }
        }
        out.shapes
            .iter()
            .any(|clipped| visit(&clipped.shape, needle))
    };

    let popup_has_text = |app: &mut App, needle: &str| -> bool {
        let pctx = egui::Context::default();
        let mut found = false;
        for _ in 0..2 {
            let mut pout = pctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 650.0))),
                    ..Default::default()
                },
                |ui| app.draw_command_popup(ui, Pos2::new(100.0, 100.0)),
            );
            found = has_shape_text(&pout, needle);
            pout.textures_delta.clear();
        }
        found
    };

    setup_focused(&mut app, &ctx);
    send_events(&mut app, &ctx, vec![egui::Event::Text(r"\math".into())]);
    send_events(&mut app, &ctx, vec![key_event(Key::Enter)]);
    let out = send_events(&mut app, &ctx, Vec::new());
    assert_eq!(
        app.command_input.as_ref().map(|cmd| cmd.name.as_str()),
        Some("mathbb")
    );
    assert!(app.editor.math().children.is_empty());
    assert!(app.status.contains("Invalid LaTeX command"));
    assert!(app.status.contains(r"\mathbb{...}"));
    assert!(app.command_error.as_deref() == Some(app.status.as_str()));
    assert!(has_shape_text(&out, "LaTeX error: Invalid LaTeX command"));
    assert!(has_shape_text(&out, r"\mathbb{...}"));
    assert!(!popup_has_text(&mut app, "Invalid LaTeX command"));

    // Editing clears command_error, and valid {R} + Enter commits cleanly.
    send_events(&mut app, &ctx, vec![egui::Event::Text("{R}".into())]);
    assert!(app.command_error.is_none());
    assert!(!popup_has_text(&mut app, "Invalid LaTeX command"));
    send_events(&mut app, &ctx, vec![key_event(Key::Enter)]);
    assert!(app.command_input.is_none());
    assert!(app.editor.math().to_latex().contains(r"\mathbb{R}"));

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
        let mut case_app = super::helpers::app();
        type_canvas(&mut case_app, bare);
        let before = case_app.editor.math();
        case_app.handle_command_key(Key::Enter, egui::Modifiers::NONE);
        assert_eq!(case_app.editor.math(), before, "should not commit `{bare}`");
        assert!(
            case_app.command_input.is_some(),
            "popup should stay open for `{bare}`"
        );
        assert!(
            case_app.status.contains("Invalid LaTeX command"),
            "unexpected status for `{bare}`: {}",
            case_app.status
        );
    }

    // \text{ } remains valid and commits a space-only mtext node.
    let mut space_text = super::helpers::app();
    type_canvas(&mut space_text, r"\text{ }");
    assert!(space_text.handle_command_key(Key::Enter, egui::Modifiers::NONE));
    assert!(space_text.command_input.is_none());
    assert_eq!(space_text.editor.math().find_all("mtext")[0].text, " ");

    // Structural templates still support bare-Enter placeholder boxes.
    for structural in [r"\frac", r"\sqrt", r"\boxed", r"\overset", r"\pmatrix"] {
        let mut structural_app = super::helpers::app();
        type_canvas(&mut structural_app, structural);
        assert!(structural_app.handle_command_key(Key::Enter, egui::Modifiers::NONE));
        assert!(
            structural_app.command_input.is_none(),
            "structural `{structural}` should commit"
        );
        assert!(
            !structural_app.editor.math().children.is_empty(),
            "{structural}"
        );
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
        let mut canvas_app = super::helpers::app();
        type_canvas(&mut canvas_app, invalid);
        canvas_app.handle_command_key(Key::Enter, egui::Modifiers::NONE);
        assert!(canvas_app.editor.math().children.is_empty(), "{invalid}");
        assert_eq!(
            canvas_app
                .command_input
                .as_ref()
                .map(|cmd| cmd.name.as_str()),
            Some(&invalid[1..])
        );
        assert!(
            canvas_app.status.contains("Invalid LaTeX command"),
            "{invalid}: {}",
            canvas_app.status
        );
        assert!(
            canvas_app.status.contains("^2"),
            "{invalid}: {}",
            canvas_app.status
        );
        assert_eq!(
            canvas_app.command_error.as_deref(),
            Some(canvas_app.status.as_str())
        );

        // Verify Equation Canvas renders the exact same status bar error prefix and color as LaTeX source.
        let mut src_app = super::helpers::app();
        src_app.latex = invalid.into();
        src_app.mark_source_changed(Instant::now() - Duration::from_secs(1));
        src_app.request_latex_parse(&egui::Context::default());
        let parsed_res = src_app.rx.recv_timeout(Duration::from_secs(2)).unwrap();
        src_app.tx.send(parsed_res).unwrap();
        src_app.process_events(&egui::Context::default());
        assert_eq!(canvas_app.command_error, src_app.latex_error, "{invalid}");

        let rctx = egui::Context::default();
        let mut canvas_out = rctx.run_ui(egui::RawInput::default(), |ui| {
            canvas_app.draw_status_bar(ui)
        });
        canvas_out.textures_delta.clear();
        let mut source_out =
            rctx.run_ui(egui::RawInput::default(), |ui| src_app.draw_status_bar(ui));
        source_out.textures_delta.clear();
        let status_text_and_color = |out: &egui::FullOutput| {
            out.shapes.iter().find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text_shape)
                    if text_shape.galley.text().contains("LaTeX error:") =>
                {
                    let color = text_shape
                        .galley
                        .job
                        .sections
                        .first()
                        .map(|sec| sec.format.color)
                        .unwrap_or(text_shape.override_text_color.unwrap_or_default());
                    Some((text_shape.galley.text().to_owned(), color))
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
        let mut valid_app = super::helpers::app();
        type_canvas(&mut valid_app, valid);
        assert!(valid_app.handle_command_key(Key::Enter, egui::Modifiers::NONE));
        assert!(valid_app.command_input.is_none(), "{valid}");
        let math = valid_app.editor.math();
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
    let mut app = super::helpers::app();
    assert!(app.load_math("<math><mi>A</mi><mo>\\</mo><mi>B</mi></math>", None, None));
    assert_eq!(app.latex, r"A \setminus B");

    let mut second_app = super::helpers::app();
    type_canvas(&mut second_app, r"90\degree");
    assert!(second_app.handle_command_key(egui::Key::Enter, egui::Modifiers::NONE));
    assert_eq!(second_app.latex, r"90^\circ");
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
        let mut app = super::helpers::app();
        type_canvas(&mut app, typed);
        assert!(
            app.handle_command_key(egui::Key::Enter, egui::Modifiers::NONE),
            "{typed}"
        );
        assert!(app.command_input.is_none(), "{typed}");
        assert_eq!(app.latex, expected_latex, "typed `{typed}`");
        assert!(
            commands::validate_desktop_latex(&app.latex).is_ok(),
            "invalid serialized latex for `{typed}`: {}",
            app.latex
        );
        assert!(
            eqntide::service::parse_latex_to_mathml(&app.latex).is_ok(),
            "re-parse failed for `{typed}`: {}",
            app.latex
        );
    }
}

#[test]
fn command_template_wraps_selection_without_reparsing_existing_math() {
    let mut app = super::helpers::app();
    app.load_math("<math><mi mathvariant=\"bold\">x</mi></math>", None, None);
    let before = app.editor.math();
    app.editor.select_all();
    type_canvas(&mut app, r"\frac");
    app.handle_command_key(egui::Key::Enter, egui::Modifiers::NONE);
    let xml = app.editor.math().to_xml();
    assert!(
        xml.contains("mfrac") && xml.contains("mathvariant=\"bold\""),
        "{xml}"
    );
    app.editor.undo();
    assert_eq!(app.editor.math(), before);
}

#[test]
fn custom_matrix_dialog_cancels_without_changes_and_inserts_one_transaction() {
    let ctx = egui::Context::default();
    let mut app = super::helpers::app();
    app.editor.insert_text("x");
    app.mark_canvas_changed();
    let before = app.editor.math();
    app.matrix_dialog = Some(MatrixDialog::default());
    assert!(!app.can_save());
    for _ in 0..2 {
        ctx.run_ui(egui::RawInput::default(), |ui| {
            app.draw_matrix_dialog(ui.ctx())
        })
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
        |ui| app.draw_matrix_dialog(ui.ctx()),
    )
    .textures_delta
    .clear();
    assert!(app.matrix_dialog.is_none());
    assert_eq!(app.editor.math(), before);
    assert!(app.can_save());
    app.insert_custom_matrix(&MatrixDialog {
        rows: 3,
        columns: 4,
        alignment: "right",
        fence: "brackets",
    });
    let math = app.editor.math();
    let table = math
        .iter()
        .into_iter()
        .find(|node| node.tag == "mtable")
        .unwrap();
    assert_eq!(table.children.len(), 3);
    assert!(table.children.iter().all(|row| row.children.len() == 4));
    assert_eq!(table.attr("columnalign"), Some("right"));
    assert_eq!(app.latex, math.to_latex());
    app.editor.undo();
    assert_eq!(app.editor.math(), before);
    app.editor.redo();
    assert_eq!(app.editor.math(), math);

    for (fence, open_ch) in [("left-brace", "{"), ("left-bracket", "[")] {
        let mut fence_app = super::helpers::app();
        fence_app.matrix_dialog = Some(MatrixDialog {
            rows: 4,
            columns: 1,
            alignment: "left",
            fence,
        });
        for _ in 0..2 {
            ctx.run_ui(egui::RawInput::default(), |ui| {
                fence_app.draw_matrix_dialog(ui.ctx())
            })
            .textures_delta
            .clear();
        }
        assert!(
            fence_app.matrix_dialog.is_some(),
            "dialog should remain open before clicking Insert matrix"
        );
        fence_app.insert_custom_matrix(&MatrixDialog {
            rows: 4,
            columns: 1,
            alignment: "left",
            fence,
        });
        let sys = fence_app.editor.math();
        let row = &sys.children[0].children;
        assert_eq!(row[0].text, open_ch);
        assert_eq!(row[1].tag, "mtable");
        assert_eq!(row[1].children.len(), 4);
        assert!(row[1].children.iter().all(|cell| cell.children.len() == 1));
        assert_eq!(row[1].attr("columnalign"), Some("left"));
        assert_eq!(row[2].text, "");
        assert_eq!(fence_app.latex, sys.to_latex());
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
    let mut app = super::helpers::app();
    app.insert_template(r"\left\{ #? \right.");
    app.handle_canvas_text("x+2=1");
    app.mark_canvas_changed();
    let before_promote = app.editor.math();
    ctx.run_ui(enter_input(egui::Modifiers::NONE), |ui| {
        app.handle_canvas_keyboard(ui)
    })
    .textures_delta
    .clear();
    let promoted = app.editor.math();
    let table = promoted
        .iter()
        .into_iter()
        .find(|node| node.tag == "mtable")
        .unwrap();
    assert_eq!(table.children.len(), 2);
    assert_eq!(table.attr("columnalign"), Some("left"));
    assert_eq!(app.latex, promoted.to_latex());

    // 2. Pressing Enter inside row 2 inserts row 3 right below and moves the caret there.
    app.handle_canvas_text("y=3");
    app.mark_canvas_changed();
    ctx.run_ui(enter_input(egui::Modifiers::NONE), |ui| {
        app.handle_canvas_keyboard(ui)
    })
    .textures_delta
    .clear();
    let grown = app.editor.math();
    let table = grown
        .iter()
        .into_iter()
        .find(|node| node.tag == "mtable")
        .unwrap();
    assert_eq!(table.children.len(), 3);
    assert_eq!(app.latex, grown.to_latex());

    // Undo steps back cleanly.
    app.editor.undo();
    assert_eq!(
        app.editor
            .math()
            .iter()
            .into_iter()
            .find(|node| node.tag == "mtable")
            .unwrap()
            .children
            .len(),
        2
    );
    // Undo typing `y=3`, then undo promotion.
    app.editor.undo();
    app.editor.undo();
    app.editor.undo();
    app.editor.undo();
    assert_eq!(app.editor.math(), before_promote);

    // 3. Active `\` command buffer still commits on Enter without adding a table row.
    let mut cmd_app = super::helpers::app();
    cmd_app.insert_template(r"\left\{ \begin{array}{l} #? \\ #? \end{array} \right.");
    cmd_app.handle_canvas_text(r"\alpha");
    assert!(cmd_app.command_input.is_some());
    ctx.run_ui(enter_input(egui::Modifiers::NONE), |ui| {
        cmd_app.handle_canvas_keyboard(ui)
    })
    .textures_delta
    .clear();
    assert!(cmd_app.command_input.is_none());
    let math_b = cmd_app.editor.math();
    assert_eq!(
        math_b
            .iter()
            .into_iter()
            .find(|node| node.tag == "mtable")
            .unwrap()
            .children
            .len(),
        2
    );
    assert!(math_b.to_xml().contains("α"));

    // 4. At the root row, Enter is a no-op.
    let mut root_app = super::helpers::app();
    root_app.handle_canvas_text("x+1");
    root_app.mark_canvas_changed();
    let before_root = root_app.editor.math();
    ctx.run_ui(enter_input(egui::Modifiers::NONE), |ui| {
        root_app.handle_canvas_keyboard(ui)
    })
    .textures_delta
    .clear();
    assert_eq!(root_app.editor.math(), before_root);
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

    let mut app = super::helpers::app();
    app.insert_template(r"\left\{ \begin{array}{l} #? \\ #? \end{array} \right.");
    app.handle_canvas_text("x=1");
    // Insert an extra empty row in the middle, then fill row 3.
    ctx.run_ui(key_input(egui::Key::Enter), |ui| {
        app.handle_canvas_keyboard(ui)
    })
    .textures_delta
    .clear();
    app.editor
        .move_caret(eqntide_editor::Move::NextPlaceholder, false);
    app.handle_canvas_text("y=2");
    app.mark_canvas_changed();
    assert_eq!(
        app.editor
            .math()
            .iter()
            .into_iter()
            .find(|node| node.tag == "mtable")
            .unwrap()
            .children
            .len(),
        3
    );

    // Move caret into the empty middle row (row 1) and press Backspace.
    let row1_cell = app.editor.root().children[0].children[1].children[1].children[0].id;
    app.editor.set_caret(
        eqntide_editor::Caret {
            node: row1_cell,
            offset: 0,
        },
        false,
    );
    ctx.run_ui(key_input(egui::Key::Backspace), |ui| {
        app.handle_canvas_keyboard(ui)
    })
    .textures_delta
    .clear();
    let after_bs = app.editor.math();
    assert_eq!(
        after_bs
            .iter()
            .into_iter()
            .find(|node| node.tag == "mtable")
            .unwrap()
            .children
            .len(),
        2
    );
    assert_eq!(app.latex, after_bs.to_latex());

    // Undo restores the empty middle row, then Delete removes it as well.
    app.editor.undo();
    app.mark_canvas_changed();
    assert_eq!(
        app.editor
            .math()
            .iter()
            .into_iter()
            .find(|node| node.tag == "mtable")
            .unwrap()
            .children
            .len(),
        3
    );
    ctx.run_ui(key_input(egui::Key::Delete), |ui| {
        app.handle_canvas_keyboard(ui)
    })
    .textures_delta
    .clear();
    let after_del = app.editor.math();
    assert_eq!(
        after_del
            .iter()
            .into_iter()
            .find(|node| node.tag == "mtable")
            .unwrap()
            .children
            .len(),
        2
    );
    assert_eq!(app.latex, after_del.to_latex());
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
    let mut app = super::helpers::app();
    app.insert_template(r"\frac{#?}{#?}");
    app.handle_canvas_text("1");
    app.editor
        .move_caret(eqntide_editor::Move::NextPlaceholder, false);
    app.mark_canvas_changed();
    let frac_id = app.editor.root().children[0].id;
    ctx.run_ui(key_input(egui::Key::Backspace), |ui| {
        app.handle_canvas_keyboard(ui)
    })
    .textures_delta
    .clear();
    assert_eq!(app.editor.selected_ids(), vec![frac_id]);
    assert_eq!(app.editor.math().children.len(), 1);
    ctx.run_ui(key_input(egui::Key::Backspace), |ui| {
        app.handle_canvas_keyboard(ui)
    })
    .textures_delta
    .clear();
    assert!(app.editor.math().children.is_empty());
    assert_eq!(app.latex, "");

    // 2) Single-press ArrowRight traverses `\frac{1}{2}` and `(x)` without duplicate stalls.
    app.load_math(
        "<math><mfrac><mrow><mn>1</mn></mrow><mrow><mn>2</mn></mrow></mfrac><mrow><mo stretchy=\"true\" form=\"prefix\">(</mo><mrow><mi>x</mi></mrow><mo stretchy=\"true\" form=\"postfix\">)</mo></mrow></math>",
        None,
        None,
    );
    app.editor.move_caret(eqntide_editor::Move::Home, false);
    let num_mn = app.editor.root().children[0].children[0].children[0].id;
    let den_mn = app.editor.root().children[0].children[1].children[0].id;
    let fence_x = app.editor.root().children[1].children[1].children[0].id;
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
            node: app.editor.root().id,
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
            node: app.editor.root().id,
            offset: 2,
        },
    ];
    for caret in expected {
        ctx.run_ui(key_input(egui::Key::ArrowRight), |ui| {
            app.handle_canvas_keyboard(ui)
        })
        .textures_delta
        .clear();
        assert_eq!(app.editor.caret(), caret);
    }

    // 3) Partially filled 2x2 matrix row selects row first, then deletes; drag-selecting an
    // empty 2x2 row removes the <mtr> cleanly and updates LaTeX.
    app.load_math(
        "<math><mtable><mtr><mtd><mi>x</mi></mtd><mtd><mi>y</mi></mtd></mtr><mtr><mtd/><mtd><mi>z</mi></mtd></mtr></mtable></math>",
        None,
        None,
    );
    let r1_c0 = app.editor.root().children[0].children[1].children[0].id;
    let r1_c1 = app.editor.root().children[0].children[1].children[1].id;
    app.editor.set_caret(
        Caret {
            node: r1_c0,
            offset: 0,
        },
        false,
    );
    ctx.run_ui(key_input(egui::Key::Backspace), |ui| {
        app.handle_canvas_keyboard(ui)
    })
    .textures_delta
    .clear();
    assert_eq!(app.editor.selected_ids(), vec![r1_c0, r1_c1]);
    ctx.run_ui(key_input(egui::Key::Backspace), |ui| {
        app.handle_canvas_keyboard(ui)
    })
    .textures_delta
    .clear();
    assert_eq!(app.editor.math().children[0].children.len(), 1);
    assert_eq!(app.latex, r"\begin{matrix} x & y \end{matrix}");

    app.load_math(
        "<math><mtable><mtr><mtd><mi>a</mi></mtd><mtd><mi>b</mi></mtd></mtr><mtr><mtd/><mtd/></mtr></mtable></math>",
        None,
        None,
    );
    let empty_c0 = app.editor.root().children[0].children[1].children[0].id;
    let empty_c1 = app.editor.root().children[0].children[1].children[1].id;
    app.editor.set_caret(
        Caret {
            node: empty_c0,
            offset: 0,
        },
        false,
    );
    app.editor.set_caret(
        Caret {
            node: empty_c1,
            offset: 0,
        },
        true,
    );
    ctx.run_ui(key_input(egui::Key::Delete), |ui| {
        app.handle_canvas_keyboard(ui)
    })
    .textures_delta
    .clear();
    assert_eq!(app.editor.math().children[0].children.len(), 1);
    assert_eq!(app.latex, r"\begin{matrix} a & b \end{matrix}");
}

#[test]
fn palette_accents_take_previous_expression_in_one_undo_step() {
    for source in [r"\hat{#?}", r"\underline{#?}", r"\boxed{#?}"] {
        let mut app = super::helpers::app();
        app.editor.insert_text("x");
        let before = app.editor.math();
        app.insert_template(source);
        let math = app.editor.math();
        assert_eq!(math.children.len(), 1, "{source}");
        assert!(math.to_xml().contains("<mi>x</mi>"));
        assert_eq!(app.latex, math.to_latex());
        app.editor.undo();
        assert_eq!(app.editor.math(), before, "{source}");
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
        let mut app = super::helpers::app();
        app.load_math(xml, None, None);
        let before = app.editor.math();
        let ctx = egui::Context::default();
        app.request_render(&ctx);
        let Event::Render(_, result) = app.rx.recv_timeout(Duration::from_secs(5)).unwrap() else {
            panic!("expected equation render")
        };
        let rendered = result.unwrap();
        assert_eq!(rendered.placeholders.len(), slots, "{xml}");
        assert!(rendered
            .placeholders
            .iter()
            .all(|rect| rect.width > 0.0 && rect.height > 0.0));
        assert_eq!(app.editor.math(), before);
        assert!(!app.editor.math().to_xml().contains(PLACEHOLDER_ATTR));
    }
}

#[test]
fn canvas_does_not_underline_text_runs() {
    let mut app = super::helpers::app();
    let xml = r#"<math><mi>x</mi><mi mathvariant="normal">d</mi><mtext>với </mtext><mtext> </mtext><mtext/></math>"#;
    app.load_math(xml, None, None);
    let original = app.editor.math();
    let ctx = egui::Context::default();
    let mut color = Color32::TRANSPARENT;
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        color = app.active_canvas_palette(ui.visuals().dark_mode).text_caret;
        app.draw_canvas(ui);
    });
    output.textures_delta.clear();
    let underlines = output.shapes.iter().filter(|shape| {
        matches!(&shape.shape, egui::Shape::LineSegment { stroke, .. } if stroke.color == color)
    });
    assert_eq!(underlines.count(), 0);
    assert_eq!(app.editor.math(), original);
}

#[test]
fn vertical_navigation_moves_between_fraction_slots_instead_of_the_root() {
    let mut app = super::helpers::app();
    app.load_math(
        "<math><mfrac><mi>a</mi><mi>b</mi></mfrac></math>",
        None,
        None,
    );
    let preview = eqntide::render::render_interactive(&app.editor.math(), 12.0).unwrap();
    app.rendered = Some(Rendered {
        placeholders: vec![],
        preview,
        image: egui::ColorImage::filled([1, 1], Color32::WHITE),
    });
    let numerator = app.editor.root().children[0].children[0].id;
    let denominator = app.editor.root().children[0].children[1].id;
    app.editor.set_caret(
        Caret {
            node: numerator,
            offset: 0,
        },
        false,
    );
    app.move_caret_vertically(true, false);
    assert_eq!(app.editor.caret().node, denominator);
    app.move_caret_vertically(false, false);
    assert_eq!(app.editor.caret().node, numerator);
}

#[test]
fn selected_token_exports_as_a_renderable_math_equation() {
    let mut app = super::helpers::app();
    app.load_math("<math><mi>xyz</mi></math>", None, None);
    let id = app.editor.root().children[0].id;
    app.editor.set_caret(
        Caret {
            node: id,
            offset: 0,
        },
        false,
    );
    app.editor.set_caret(
        Caret {
            node: id,
            offset: 1,
        },
        true,
    );
    let math = app.selected_equation();
    assert_eq!(math.tag, "math");
    assert_eq!(math.children[0].text, "x");
    assert!(eqntide::render::measure(&math, 12.0).unwrap().0 > 0.0);
}

#[test]
fn editor_controls_preserve_active_focus_for_canvas_and_latex() {
    let canvas_id = egui::Id::unique("equation-canvas");
    let latex_id = egui::Id::unique("latex-source");

    let ctx = egui::Context::default();
    ThemeTokens::apply_style(&ctx);
    let mut app = super::helpers::app();
    let out = run_ui_pass_frame(&ctx, &mut app, vec![]);
    assert!(ctx.memory(|mem| mem.has_focus(canvas_id)));

    // Click a sidebar category tile ("Symbols") -> Equation Canvas retains focus.
    let symbols_pos = out
        .shapes
        .iter()
        .find_map(|clipped| {
            let rect = clipped.shape.visual_bounding_rect();
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
    let out = run_ui_pass_frame(&ctx, &mut app, click_events(symbols_pos));
    assert!(ctx.memory(|mem| mem.has_focus(canvas_id)));

    // Click Undo and MathType radio -> Equation Canvas retains focus.
    let undo_tex = app.ui_icon_texture_id("arrow-back-up").expect("undo icon");
    let undo_pos = out
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect_shape)
                if rect_shape
                    .brush
                    .as_ref()
                    .is_some_and(|brush| brush.fill_texture_id == undo_tex) =>
            {
                Some(rect_shape.rect.center())
            }
            _ => None,
        })
        .expect("undo icon rect");
    let out = run_ui_pass_frame(&ctx, &mut app, click_events(undo_pos));
    assert!(ctx.memory(|mem| mem.has_focus(canvas_id)));
    let mt_pos = find_text_center(&out, |text| text == "MathType");
    let _out = run_ui_pass_frame(&ctx, &mut app, click_events(mt_pos));
    assert!(ctx.memory(|mem| mem.has_focus(canvas_id)));

    // Click into LaTeX source box to switch active focus there.
    let latex_rect = ctx.read_response(latex_id).unwrap().rect;
    let out = run_ui_pass_frame(&ctx, &mut app, click_events(latex_rect.center()));
    assert!(ctx.memory(|mem| mem.has_focus(latex_id)));

    // Click palette button, Undo, Redo, and OMML radio -> focus stays on LaTeX source.
    let palette_pos = out
        .shapes
        .iter()
        .find_map(|clipped| {
            let rect = clipped.shape.visual_bounding_rect();
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
    let out = run_ui_pass_frame(&ctx, &mut app, click_events(palette_pos));
    assert!(ctx.memory(|mem| mem.has_focus(latex_id)));

    let undo_pos = out
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect_shape)
                if rect_shape
                    .brush
                    .as_ref()
                    .is_some_and(|brush| brush.fill_texture_id == undo_tex) =>
            {
                Some(rect_shape.rect.center())
            }
            _ => None,
        })
        .expect("undo icon rect");
    let out = run_ui_pass_frame(&ctx, &mut app, click_events(undo_pos));
    assert!(ctx.memory(|mem| mem.has_focus(latex_id)));

    let omml_pos = find_text_center(&out, |text| text == "OMML");
    run_ui_pass_frame(&ctx, &mut app, click_events(omml_pos));
    assert!(ctx.memory(|mem| mem.has_focus(latex_id)));
}

#[test]
fn canvas_caret_blinks_when_idle_and_resets_on_activity() {
    let ctx = egui::Context::default();
    ThemeTokens::apply_style(&ctx);
    let mut app = super::helpers::app();
    let caret_stroke = theme::CanvasPalette::for_mode(ctx.theme() == egui::Theme::Dark).caret;
    let run_at = |app: &mut App, time: f64, events: Vec<egui::Event>| {
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 650.0))),
                time: Some(time),
                events,
                ..Default::default()
            },
            |ui| app.draw_ui(ui),
        );
        out.textures_delta.clear();
        out.shapes.iter().any(|clipped| {
            matches!(&clipped.shape, egui::Shape::LineSegment { stroke, .. }
                if stroke.color == caret_stroke && (stroke.width - 1.5).abs() <= 0.1)
        })
    };

    // Initial frame focuses canvas and shows caret ON.
    assert!(run_at(&mut app, 0.0, vec![]));
    let cursor_style = ctx.style_of(ctx.theme()).visuals.text_cursor.clone();
    let (on_dur, off_dur) = (
        f64::from(cursor_style.on_duration),
        f64::from(cursor_style.off_duration),
    );
    // Midway through OFF phase -> caret hidden.
    assert!(!run_at(&mut app, on_dur + off_dur * 0.5, vec![]));
    // Typing resets blink phase immediately to ON even inside what was OFF window.
    assert!(run_at(
        &mut app,
        on_dur + off_dur * 0.6,
        vec![egui::Event::Text("x".into())]
    ));
    // After full on + off period from the activity timestamp -> ON again.
    assert!(run_at(
        &mut app,
        on_dur + off_dur * 0.6 + on_dur + off_dur + 0.05,
        vec![]
    ));
}

#[test]
fn caret_rect_aligns_consistently_across_tokens_and_row_stops() {
    let ctx = egui::Context::default();
    let mut app = super::helpers::app();
    let math = MathNode::with_children(
        "math",
        vec![
            MathNode::leaf("mi", "x"),
            MathNode::leaf("mo", "+"),
            MathNode::leaf("mn", "1"),
        ],
    );
    app.editor.replace(&math);
    app.mark_canvas_changed();
    app.sync_render(&ctx);

    let origin = Pos2::new(20.0, 40.0);
    let scale = 2.0 * 96.0 / 72.0;

    let root_id = app.editor.root().id;
    let x_id = app.editor.root().children[0].id;
    let plus_id = app.editor.root().children[1].id;
    let one_id = app.editor.root().children[2].id;

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
        app.editor.set_caret(caret, false);
        rects.push(app.caret_rect(origin, scale).expect("caret rect"));
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
    app.editor.replace(&sub_math);
    app.mark_canvas_changed();
    app.sync_render(&ctx);
    app.editor.move_caret(Move::Left, false);
    let sub_rect = app.caret_rect(origin, scale).expect("subscript caret rect");
    assert!(sub_rect.height() < first.height() * 0.8);
}

#[test]
fn command_error_clears_when_backspacing_or_canceling_out_of_popup() {
    use egui::Key;

    // 1) Focused command popup TextEdit flow: \text + Enter -> error, then Backspace to empty -> clean.
    let ctx = egui::Context::default();
    ThemeTokens::apply_style(&ctx);
    let mut app = super::helpers::app();
    app.bridge_ready = true;
    ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 650.0))),
            ..Default::default()
        },
        |ui| {
            app.draw_shell(ui);
            ui.memory_mut(|mem| mem.request_focus(egui::Id::unique("equation-canvas")));
        },
    )
    .textures_delta
    .clear();

    let send = |app: &mut App, events: Vec<egui::Event>| {
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 650.0))),
                events,
                ..Default::default()
            },
            |ui| app.draw_shell(ui),
        );
        out.textures_delta.clear();
        out
    };
    let key_ev = |key| egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };

    send(&mut app, vec![egui::Event::Text(r"\text".into())]);
    send(&mut app, Vec::new());
    send(&mut app, vec![key_ev(Key::Enter)]);
    assert!(app.command_input.is_some());
    assert!(app.command_error.is_some());
    assert!(app.status.contains("Invalid LaTeX command"));

    // First Backspace inside popup clears both command_error and status immediately.
    send(&mut app, vec![key_ev(Key::Backspace)]);
    assert!(app.command_input.is_some());
    assert!(app.command_error.is_none());
    assert!(app.status.is_empty(), "status leaked: {:?}", app.status);

    // Remaining Backspaces close the popup and keep status empty.
    for _ in 0..4 {
        send(&mut app, vec![key_ev(Key::Backspace)]);
    }
    assert!(app.command_input.is_none());
    assert!(app.command_error.is_none());
    assert!(
        app.status.is_empty(),
        "status leaked after close: {:?}",
        app.status
    );

    // 2) Pressing Escape right after an invalid command error also clears status.
    send(&mut app, vec![egui::Event::Text(r"\text".into())]);
    send(&mut app, Vec::new());
    send(&mut app, vec![key_ev(Key::Enter)]);
    assert!(app.status.contains("Invalid LaTeX command"));
    send(&mut app, vec![key_ev(Key::Escape)]);
    assert!(app.command_input.is_none());
    assert!(app.command_error.is_none());
    assert!(
        app.status.is_empty(),
        "status leaked after Esc: {:?}",
        app.status
    );

    // 3) Direct handle_command_key Backspace flow also clears status.
    let mut direct_app = super::helpers::app();
    type_canvas(&mut direct_app, r"\text");
    canvas_key(&mut direct_app, Key::Enter);
    assert!(direct_app.status.contains("Invalid LaTeX command"));
    for _ in 0..5 {
        canvas_key(&mut direct_app, Key::Backspace);
    }
    assert!(direct_app.command_input.is_none());
    assert!(direct_app.command_error.is_none());
    assert!(
        direct_app.status.is_empty(),
        "status leaked in handle_command_key: {:?}",
        direct_app.status
    );
}

#[test]
fn placeholder_paint_uses_light_radius_dashed_stroke_and_transparent_interior() {
    let ctx = egui::Context::default();
    let mut app = super::helpers::app();
    app.load_math("<math><mfrac><mrow/><mrow/></mfrac></math>", None, None);

    for (theme, dark) in [(egui::Theme::Light, false), (egui::Theme::Dark, true)] {
        ctx.set_theme(theme);
        let pal = app.active_canvas_palette(dark);
        assert_eq!(pal.placeholder_bg, Color32::TRANSPARENT);

        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(600.0, 300.0))),
                ..Default::default()
            },
            |ui| {
                app.draw_canvas(ui);
            },
        );
        output.textures_delta.clear();

        let dashed_strokes = output
            .shapes
            .iter()
            .filter(|clipped| {
                matches!(
                    &clipped.shape,
                    egui::Shape::LineSegment { stroke, .. }
                        if stroke.color == pal.placeholder_stroke
                )
            })
            .count();
        assert!(
            dashed_strokes >= 8,
            "expected dashed placeholder segments, got {dashed_strokes}"
        );

        let rounded_rects_with_fill = output
            .shapes
            .iter()
            .filter(|clipped| {
                matches!(
                    &clipped.shape,
                    egui::Shape::Rect(rect_shape)
                        if rect_shape.corner_radius == theme::CONTROL_CORNER_RADIUS
                            && rect_shape.rect.width() < 100.0
                            && rect_shape.rect.height() < 100.0
                )
            })
            .count();
        assert_eq!(
            rounded_rects_with_fill, 0,
            "placeholders must not paint 6px-rounded filled boxes"
        );
    }
}

#[test]
fn canvas_selection_renders_contiguous_lightly_rounded_highlight() {
    let ctx = egui::Context::default();
    let mut app = super::helpers::app();
    app.load_math(
        "<math><mfrac><mn>1</mn><mn>2</mn></mfrac><msup><mi>x</mi><mn>2</mn></msup><mo>+</mo><mn>2</mn><mi>x</mi><mo>-</mo><mn>1</mn><mo>=</mo><mn>0</mn></math>",
        None,
        None,
    );
    app.sync_render(&ctx);
    let pal = app.active_canvas_palette(ctx.theme() == egui::Theme::Dark);

    // 1) Selecting the full multi-term expression renders one contiguous 2px-rounded selection rect.
    app.editor.select_all();
    let mut all_out = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 300.0))),
            ..Default::default()
        },
        |ui| {
            app.draw_canvas(ui);
        },
    );
    all_out.textures_delta.clear();

    let sel_rects: Vec<_> = all_out
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect_shape) if rect_shape.fill == pal.selection_bg => {
                Some(rect_shape.clone())
            }
            _ => None,
        })
        .collect();
    assert_eq!(sel_rects.len(), 1, "expected 1 contiguous selection rect");
    assert_eq!(
        sel_rects[0].corner_radius,
        theme::CANVAS_OVERLAY_CORNER_RADIUS
    );

    // 2) Selecting `-` alone uses baseline line-height bounds instead of hanging downward.
    let minus_id = app.editor.root().children[5].id;
    let minus_region = app.hit_region(minus_id).unwrap();
    let scale = app.zoom * 96.0 / 72.0;
    app.editor.select_node(minus_id);
    let minus_rect = app.selection_rect(Pos2::ZERO, scale).unwrap();
    let expected_top = (minus_region.baseline - 0.72 * app.font_pt) * scale;
    let expected_bottom = (minus_region.baseline + 0.22 * app.font_pt) * scale;
    assert!((minus_rect.top() - expected_top).abs() < 0.1);
    assert!((minus_rect.bottom() - expected_bottom).abs() < 0.1);
    assert!(minus_rect.top() < minus_region.y * scale);

    // 3) Selecting a substring inside a token highlights only the [start..end] caret span.
    app.load_math("<math><mi>abc</mi></math>", None, None);
    app.sync_render(&ctx);
    let token_id = app.editor.root().children[0].id;
    let token_region = app.hit_region(token_id).unwrap();
    app.editor.set_caret(
        Caret {
            node: token_id,
            offset: 0,
        },
        false,
    );
    app.editor.set_caret(
        Caret {
            node: token_id,
            offset: 2,
        },
        true,
    );
    let sub_rect = app.selection_rect(Pos2::ZERO, scale).unwrap();
    let expected_width = (token_region.carets[2] - token_region.carets[0]) * scale;
    assert!((sub_rect.width() - expected_width).abs() < 0.1);
    assert!(sub_rect.width() < token_region.width * scale);
}

#[test]
fn canvas_drag_selection_excludes_character_before_midpoint() {
    let ctx = egui::Context::default();
    let mut app = super::helpers::app();
    app.load_math(
        "<math><mfrac><mn>1</mn><mn>2</mn></mfrac><msup><mi>x</mi><mn>2</mn></msup><mo>-</mo><mn>2</mn><mi>x</mi><mo>+</mo><mn>1</mn><mo>=</mo><mn>0</mn></math>",
        None,
        None,
    );
    app.sync_render(&ctx);

    let msup_id = app.editor.root().children[1].id;
    let base_x_id = app.editor.root().children[1].children[0].id;
    let minus_id = app.editor.root().children[2].id;
    let two_id = app.editor.root().children[3].id;

    let x_region = app.hit_region(base_x_id).unwrap();
    let two_region = app.hit_region(two_id).unwrap();
    let scale = app.zoom * 96.0 / 72.0;

    // Initial pass registers the canvas widget and its origin.
    ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 300.0))),
            ..Default::default()
        },
        |ui| {
            app.draw_canvas(ui);
        },
    )
    .textures_delta
    .clear();
    let canvas_rect = ctx
        .read_response(egui::Id::unique("equation-canvas"))
        .unwrap()
        .rect;
    let origin = canvas_rect.min + Vec2::new(4.0, 8.0);

    let press_pos = origin
        + Vec2::new(
            (x_region.x + x_region.width * 0.2) * scale,
            (x_region.y + x_region.height * 0.5) * scale,
        );
    let drag_before_half = origin
        + Vec2::new(
            (two_region.x + two_region.width * 0.25) * scale,
            (two_region.y + two_region.height * 0.5) * scale,
        );
    let drag_after_half = origin
        + Vec2::new(
            (two_region.x + two_region.width * 0.75) * scale,
            (two_region.y + two_region.height * 0.5) * scale,
        );

    let run_pointer = |app: &mut App, events: Vec<egui::Event>| {
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 300.0))),
                events,
                ..Default::default()
            },
            |ui| {
                app.draw_canvas(ui);
            },
        )
        .textures_delta
        .clear();
    };

    // Press in left half of `x` and drag into left half of `2` -> caret sits before `2`,
    // and `2` is NOT included in the selection highlight.
    run_pointer(
        &mut app,
        vec![
            egui::Event::PointerMoved(press_pos),
            egui::Event::PointerButton {
                pos: press_pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    run_pointer(&mut app, vec![egui::Event::PointerMoved(drag_before_half)]);
    assert_eq!(
        app.editor.caret(),
        Caret {
            node: two_id,
            offset: 0,
        }
    );
    assert_eq!(app.editor.selected_ids(), vec![msup_id, minus_id]);
    let selection_before = app.selection_rect(origin, scale).unwrap();
    assert!(selection_before.right() < origin.x + two_region.x * scale);

    // Continue dragging past the midpoint of `2` -> caret moves after `2` and `2` is included.
    run_pointer(&mut app, vec![egui::Event::PointerMoved(drag_after_half)]);
    assert_eq!(
        app.editor.caret(),
        Caret {
            node: two_id,
            offset: 1,
        }
    );
    assert_eq!(app.editor.selected_ids(), vec![msup_id, minus_id, two_id]);
    let selection_after = app.selection_rect(origin, scale).unwrap();
    assert!(selection_after.right() >= origin.x + (two_region.x + two_region.width) * scale - 0.5);
}

#[test]
fn canvas_drag_selection_caret_stays_locked_to_selection_edge_across_operator_gap() {
    let ctx = egui::Context::default();
    let mut app = super::helpers::app();
    app.load_math(
        "<math><mfrac><mn>1</mn><mn>2</mn></mfrac><msup><mi>x</mi><mn>2</mn></msup><mo>-</mo><mn>2</mn><mi>x</mi><mo>+</mo><mn>1</mn><mo>=</mo><mn>0</mn></math>",
        None,
        None,
    );
    app.sync_render(&ctx);

    let msup_id = app.editor.root().children[1].id;
    let base_x_id = app.editor.root().children[1].children[0].id;
    let minus_id = app.editor.root().children[2].id;
    let two_id = app.editor.root().children[3].id;
    let var_x_id = app.editor.root().children[4].id;
    let plus_id = app.editor.root().children[5].id;
    let one_id = app.editor.root().children[6].id;

    let base_x_region = app.hit_region(base_x_id).unwrap();
    let var_x_region = app.hit_region(var_x_id).unwrap();
    let plus_region = app.hit_region(plus_id).unwrap();
    let one_region = app.hit_region(one_id).unwrap();
    let scale = app.zoom * 96.0 / 72.0;

    let run_pointer = |app: &mut App, events: Vec<egui::Event>| {
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 300.0))),
                events,
                ..Default::default()
            },
            |ui| {
                app.draw_canvas(ui);
            },
        )
        .textures_delta
        .clear();
    };
    run_pointer(&mut app, vec![]);

    let canvas_rect = ctx
        .read_response(egui::Id::unique("equation-canvas"))
        .unwrap()
        .rect;
    let origin = canvas_rect.min + Vec2::new(4.0, 8.0);
    let midpoint_y = (var_x_region.y + var_x_region.height * 0.5) * scale;
    let var_x_right_pt = var_x_region.x + var_x_region.width;
    assert!(plus_region.x - var_x_right_pt > 2.5);

    // Clicking inside the left half of the inter-token gap keeps the caret on `x` instead of jumping to `+`.
    let gap_left_pos = origin + Vec2::new((var_x_right_pt + 1.2) * scale, midpoint_y);
    run_pointer(
        &mut app,
        vec![
            egui::Event::PointerMoved(gap_left_pos),
            egui::Event::PointerButton {
                pos: gap_left_pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerButton {
                pos: gap_left_pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    assert_eq!(
        app.editor.caret(),
        Caret {
            node: var_x_id,
            offset: 1,
        }
    );

    // Dragging across `x^2 - 2x` into the gap and left quarter of `+` keeps both the selection
    // and the caret bar locked to the right edge of `x`.
    let press_pos = origin
        + Vec2::new(
            (base_x_region.x + base_x_region.width * 0.2) * scale,
            (base_x_region.y + base_x_region.height * 0.5) * scale,
        );
    run_pointer(
        &mut app,
        vec![
            egui::Event::PointerMoved(press_pos),
            egui::Event::PointerButton {
                pos: press_pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    let mut locked_right: Option<f32> = None;
    for point_x in [
        var_x_region.x + var_x_region.width * 0.85,
        var_x_right_pt + 1.2,
        (var_x_right_pt + plus_region.x) * 0.5 + 0.3,
        plus_region.x + plus_region.width * 0.25,
    ] {
        let drag_pos = origin + Vec2::new(point_x * scale, midpoint_y);
        run_pointer(&mut app, vec![egui::Event::PointerMoved(drag_pos)]);
        assert_eq!(
            app.editor.selected_ids(),
            vec![msup_id, minus_id, two_id, var_x_id]
        );
        let selection_rect = app.selection_rect(origin, scale).unwrap();
        let caret_rect = app.caret_rect(origin, scale).unwrap();
        assert!((caret_rect.left() - selection_rect.right()).abs() < 0.01);
        if let Some(expected) = locked_right {
            assert!((caret_rect.left() - expected).abs() < 0.01);
        } else {
            locked_right = Some(caret_rect.left());
        }
    }
    run_pointer(
        &mut app,
        vec![egui::Event::PointerButton {
            pos: origin
                + Vec2::new(
                    (plus_region.x + plus_region.width * 0.25) * scale,
                    midpoint_y,
                ),
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );

    // Dragging backward from `1` across `+` locks the caret bar to the left edge of `+`.
    let back_press =
        origin + Vec2::new((one_region.x + one_region.width * 0.8) * scale, midpoint_y);
    run_pointer(
        &mut app,
        vec![
            egui::Event::PointerMoved(back_press),
            egui::Event::PointerButton {
                pos: back_press,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    let mut locked_left: Option<f32> = None;
    for point_x in [
        plus_region.x + plus_region.width * 0.2,
        var_x_right_pt + 1.2,
        var_x_region.x + var_x_region.width * 0.85,
    ] {
        let drag_pos = origin + Vec2::new(point_x * scale, midpoint_y);
        run_pointer(&mut app, vec![egui::Event::PointerMoved(drag_pos)]);
        assert_eq!(app.editor.selected_ids(), vec![plus_id, one_id]);
        let selection_rect = app.selection_rect(origin, scale).unwrap();
        let caret_rect = app.caret_rect(origin, scale).unwrap();
        assert!((caret_rect.left() - selection_rect.left()).abs() < 0.01);
        if let Some(expected) = locked_left {
            assert!((caret_rect.left() - expected).abs() < 0.01);
        } else {
            locked_left = Some(caret_rect.left());
        }
    }
}

#[test]
fn command_popup_backslash_prefix_aligns_with_input_and_uses_active_accent() {
    fn find_text_shape(shape: &egui::Shape, target: &str) -> Option<(Pos2, Rect, Color32)> {
        match shape {
            egui::Shape::Text(text_shape) if text_shape.galley.text() == target => {
                let color = text_shape
                    .override_text_color
                    .filter(|col| *col != Color32::PLACEHOLDER)
                    .or_else(|| {
                        text_shape
                            .galley
                            .job
                            .sections
                            .first()
                            .map(|sec| sec.format.color)
                    })
                    .unwrap_or_default();
                Some((text_shape.pos, shape.visual_bounding_rect(), color))
            }
            egui::Shape::Vec(shapes) => shapes
                .iter()
                .find_map(|child| find_text_shape(child, target)),
            _ => None,
        }
    }

    fn has_borderless_input_box(
        shape: &egui::Shape,
        control_bg: Color32,
        prefix_center: Pos2,
        input_center: Pos2,
    ) -> bool {
        match shape {
            egui::Shape::Rect(rect_shape) => {
                rect_shape.fill == control_bg
                    && rect_shape.stroke == Stroke::NONE
                    && rect_shape.rect.contains(prefix_center)
                    && rect_shape.rect.contains(input_center)
            }
            egui::Shape::Vec(shapes) => shapes.iter().any(|child| {
                has_borderless_input_box(child, control_bg, prefix_center, input_center)
            }),
            _ => false,
        }
    }

    let dir = tempfile::tempdir().unwrap();
    let settings_path = dir.path().join("settings.json");
    let img_path = dir.path().join("cyan-bg.png");
    let img = image::RgbaImage::from_pixel(48, 48, image::Rgba([20, 140, 220, 255]));
    image::DynamicImage::ImageRgba8(img)
        .save_with_format(&img_path, image::ImageFormat::Png)
        .unwrap();

    let ctx = egui::Context::default();
    ThemeTokens::apply_style(&ctx);
    let mut app = super::helpers::app().with_settings_path(settings_path, Some(&ctx));
    app.set_background_image_from_path(&img_path, &ctx);
    let seed = app.background_accent.expect("cyan accent seed");
    let canvas_id = egui::Id::unique("equation-canvas");

    let mut frame_time = 0.0_f64;
    let mut send = |app: &mut App, events: Vec<egui::Event>| {
        frame_time += 1.0;
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 650.0))),
                time: Some(frame_time),
                events,
                ..Default::default()
            },
            |ui| app.draw_shell(ui),
        );
        out.textures_delta.clear();
        out
    };

    for (mode, use_adaptive) in [
        (crate::settings::BackgroundMode::Solid, false),
        (crate::settings::BackgroundMode::Image, true),
    ] {
        app.background_mode = mode;
        app.background_accent_enabled = use_adaptive;
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            let dark_mode = theme == egui::Theme::Dark;
            ctx.set_theme(theme);
            let expected_tokens = ThemeTokens::for_mode_with_accent(
                dark_mode,
                if use_adaptive { Some(seed) } else { None },
            );

            app.dismiss_command_input();
            ctx.memory_mut(|mem| mem.request_focus(canvas_id));
            send(&mut app, vec![]);
            send(&mut app, vec![egui::Event::Text(r"\int_a^b".into())]);
            let out = send(&mut app, vec![]);

            let (prefix_pos, prefix_rect, prefix_color) = out
                .shapes
                .iter()
                .find_map(|clipped| find_text_shape(&clipped.shape, r"\"))
                .expect("leading backslash shape must be visible");
            let (input_pos, input_rect, _) = out
                .shapes
                .iter()
                .find_map(|clipped| find_text_shape(&clipped.shape, "int_a^b"))
                .expect("command input text shape must be visible");

            assert_eq!(
                prefix_color, expected_tokens.accent,
                "leading backslash must use active accent color"
            );
            assert!(
                (prefix_pos.y - input_pos.y).abs() <= 0.1,
                "backslash y ({}) must align with input y ({})",
                prefix_pos.y,
                input_pos.y
            );
            assert!(
                (prefix_rect.center().y - input_rect.center().y).abs() <= 0.5,
                "backslash center y ({}) must align with input center y ({})",
                prefix_rect.center().y,
                input_rect.center().y
            );
            assert!(
                out.shapes.iter().any(|clipped| has_borderless_input_box(
                    &clipped.shape,
                    expected_tokens.control_bg,
                    prefix_rect.center(),
                    input_rect.center()
                )),
                "command popup input box must render borderless with control_bg surface"
            );
        }
    }
}

#[test]
fn command_popup_selected_suggestion_uses_active_selection_tokens() {
    fn find_prefix_text(shape: &egui::Shape, prefix: &str) -> Option<(Rect, Color32)> {
        match shape {
            egui::Shape::Text(text_shape) if text_shape.galley.text().starts_with(prefix) => {
                let color = text_shape
                    .override_text_color
                    .filter(|col| *col != Color32::PLACEHOLDER)
                    .or_else(|| {
                        text_shape
                            .galley
                            .job
                            .sections
                            .first()
                            .map(|sec| sec.format.color)
                            .filter(|col| *col != Color32::PLACEHOLDER)
                    })
                    .unwrap_or(text_shape.fallback_color);
                Some((shape.visual_bounding_rect(), color))
            }
            egui::Shape::Vec(shapes) => shapes
                .iter()
                .find_map(|child| find_prefix_text(child, prefix)),
            _ => None,
        }
    }

    fn has_selected_row_rect(
        shape: &egui::Shape,
        point: Pos2,
        expected: crate::theme::ThemeTokens,
    ) -> bool {
        match shape {
            egui::Shape::Rect(rect_shape) => {
                rect_shape.rect.contains(point)
                    && rect_shape.fill == expected.selected_bg
                    && rect_shape.stroke == Stroke::NONE
            }
            egui::Shape::Vec(shapes) => shapes
                .iter()
                .any(|child| has_selected_row_rect(child, point, expected)),
            _ => false,
        }
    }

    let suggestions = commands::suggestions("al");
    assert!(suggestions.len() >= 2);
    let first_prefix = format!(r"\{}   ", suggestions[0].name);
    let second_prefix = format!(r"\{}   ", suggestions[1].name);

    let dir = tempfile::tempdir().unwrap();
    let settings_path = dir.path().join("settings.json");
    let img_path = dir.path().join("cyan-bg.png");
    let img = image::RgbaImage::from_pixel(48, 48, image::Rgba([20, 140, 220, 255]));
    image::DynamicImage::ImageRgba8(img)
        .save_with_format(&img_path, image::ImageFormat::Png)
        .unwrap();

    let ctx = egui::Context::default();
    ThemeTokens::apply_style(&ctx);
    let mut app = super::helpers::app().with_settings_path(settings_path, Some(&ctx));
    app.set_background_image_from_path(&img_path, &ctx);
    let seed = app.background_accent.expect("cyan accent seed");
    let canvas_id = egui::Id::unique("equation-canvas");

    let mut frame_time = 0.0_f64;
    let mut send = |app: &mut App, events: Vec<egui::Event>| {
        frame_time += 1.0;
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 650.0))),
                time: Some(frame_time),
                events,
                ..Default::default()
            },
            |ui| app.draw_shell(ui),
        );
        out.textures_delta.clear();
        out
    };

    for (mode, use_adaptive) in [
        (crate::settings::BackgroundMode::Solid, false),
        (crate::settings::BackgroundMode::Image, true),
    ] {
        app.background_mode = mode;
        app.background_accent_enabled = use_adaptive;
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            let dark_mode = theme == egui::Theme::Dark;
            ctx.set_theme(theme);
            let expected = ThemeTokens::for_mode_with_background_and_accent(
                dark_mode,
                mode == crate::settings::BackgroundMode::Image,
                app.background_opacity,
                if use_adaptive { Some(seed) } else { None },
            );

            app.dismiss_command_input();
            ctx.memory_mut(|mem| mem.request_focus(canvas_id));
            send(&mut app, vec![]);
            send(&mut app, vec![egui::Event::Text(r"\al".into())]);
            let first_out = send(&mut app, vec![]);

            let (first_rect, first_color) = first_out
                .shapes
                .iter()
                .find_map(|clipped| find_prefix_text(&clipped.shape, &first_prefix))
                .expect("first command suggestion row");
            assert_eq!(first_color, expected.selected_fg);
            assert!(first_color != expected.accent);
            assert!(first_out.shapes.iter().any(|clipped| {
                has_selected_row_rect(&clipped.shape, first_rect.center(), expected)
            }));

            // ArrowDown moves selection highlight to the second suggestion row.
            let down_out = send(
                &mut app,
                vec![egui::Event::Key {
                    key: egui::Key::ArrowDown,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            let (second_rect, second_color) = down_out
                .shapes
                .iter()
                .find_map(|clipped| find_prefix_text(&clipped.shape, &second_prefix))
                .expect("second command suggestion row");
            assert_eq!(second_color, expected.selected_fg);
            assert!(down_out.shapes.iter().any(|clipped| {
                has_selected_row_rect(&clipped.shape, second_rect.center(), expected)
            }));
        }
    }
}

#[test]
fn translucent_canvas_renders_placeholders_without_opaque_cover_or_baked_square() {
    let ctx = egui::Context::default();
    let mut app = super::helpers::app();
    let bg_img = egui::ColorImage::filled([32, 32], Color32::from_rgb(50, 90, 140));
    app.background_texture =
        Some(ctx.load_texture("bg-ph-test", bg_img, egui::TextureOptions::LINEAR));
    app.background_mode = crate::settings::BackgroundMode::Image;
    app.canvas_opacity = 0.25;
    app.load_math("<math><mfrac><mrow/><mrow/></mfrac></math>", None, None);
    app.sync_render(&ctx);

    let rendered = app
        .rendered
        .as_ref()
        .expect("rendered fraction placeholders");
    assert_eq!(rendered.placeholders.len(), 2);
    let img_w = rendered.image.width();
    let img_h = rendered.image.height();
    let scale_x = img_w as f32 / rendered.preview.preview.width_pt;
    let scale_y = img_h as f32 / rendered.preview.preview.height_pt;
    for slot in &rendered.placeholders {
        let min_x = ((slot.x * scale_x).floor().max(0.0) as usize).min(img_w);
        let max_x = ((slot.x + slot.width) * scale_x)
            .ceil()
            .clamp(0.0, img_w as f32) as usize;
        let min_y = ((slot.y * scale_y).floor().max(0.0) as usize).min(img_h);
        let max_y = ((slot.y + slot.height) * scale_y)
            .ceil()
            .clamp(0.0, img_h as f32) as usize;
        for y in min_y..max_y {
            for x in min_x..max_x {
                assert_eq!(
                    rendered.image.pixels[y * img_w + x].a(),
                    0,
                    "placeholder slot pixel ({x}, {y}) must be cleared from alpha texture"
                );
            }
        }
    }

    for (theme, dark) in [(egui::Theme::Light, false), (egui::Theme::Dark, true)] {
        ctx.set_theme(theme);
        let pal = app.active_canvas_palette(dark);
        assert!(pal.bg.a() < 255);

        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(600.0, 300.0))),
                ..Default::default()
            },
            |ui| {
                app.draw_canvas(ui);
            },
        );
        output.textures_delta.clear();

        let pal_bg_rects: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(rect_shape) if rect_shape.fill == pal.bg => Some(rect_shape),
                _ => None,
            })
            .collect();
        assert_eq!(
            pal_bg_rects.len(),
            1,
            "only the outer canvas Frame should fill with pal.bg (no placeholder cover rects)"
        );
    }
}

fn matrix_modal_key_event(key: egui::Key, shift: bool) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers {
            shift,
            ..Default::default()
        },
    }
}

#[test]
fn custom_matrix_dialog_ignores_backdrop_clicks_and_supports_keyboard_navigation() {
    // 1. Backdrop click does nothing; ArrowRight -> Cancel + Enter closes without changes.
    {
        let ctx = egui::Context::default();
        ThemeTokens::apply_style(&ctx);
        let mut app = super::helpers::app();
        app.editor.insert_text("x");
        app.mark_canvas_changed();
        let before = app.editor.math();
        app.matrix_dialog = Some(MatrixDialog::default());

        let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
        let matrix_out = run_ui_pass_frame(&ctx, &mut app, vec![]);
        let heading_center = find_text_center(&matrix_out, |text| text == "Custom matrix");
        let cancel_center = find_text_center(&matrix_out, |text| text == "Cancel");
        let insert_center = find_text_center(&matrix_out, |text| text == "Insert matrix");
        assert!(insert_center.x < cancel_center.x);
        fn find_matrix_modal_rect(
            shape: &egui::Shape,
            heading: Pos2,
            button: Pos2,
            max_width: f32,
        ) -> Option<Rect> {
            match shape {
                egui::Shape::Rect(rect_shape)
                    if rect_shape.rect.width() < max_width
                        && rect_shape.rect.width() < 500.0
                        && rect_shape.blur_width == 0.0
                        && rect_shape.corner_radius == theme::SURFACE_CORNER_RADIUS
                        && rect_shape.stroke == Stroke::NONE
                        && rect_shape.rect.contains(heading)
                        && rect_shape.rect.contains(button) =>
                {
                    Some(rect_shape.rect)
                }
                egui::Shape::Vec(inner) => inner
                    .iter()
                    .find_map(|child| find_matrix_modal_rect(child, heading, button, max_width)),
                _ => None,
            }
        }
        let matrix_rect = matrix_out
            .shapes
            .iter()
            .find_map(|clipped| {
                find_matrix_modal_rect(
                    &clipped.shape,
                    heading_center,
                    cancel_center,
                    MIN_WINDOW_SIZE[0],
                )
            })
            .unwrap_or_else(|| panic!("Custom matrix modal rect missing: heading={heading_center:?}, insert={insert_center:?}, cancel={cancel_center:?}"));
        assert!(
            (heading_center.x - matrix_rect.center().x).abs() < 4.0,
            "expected centered Custom matrix heading near {}, got {} (rect={matrix_rect:?})",
            matrix_rect.center().x,
            heading_center.x
        );
        fn find_matrix_button_rect(shape: &egui::Shape, point: Pos2) -> Option<Rect> {
            match shape {
                egui::Shape::Rect(rect_shape)
                    if rect_shape.rect.height() < 45.0
                        && rect_shape.rect.width() < 200.0
                        && rect_shape.rect.contains(point) =>
                {
                    Some(rect_shape.rect)
                }
                egui::Shape::Vec(inner) => inner
                    .iter()
                    .find_map(|child| find_matrix_button_rect(child, point)),
                _ => None,
            }
        }
        let insert_rect = matrix_out
            .shapes
            .iter()
            .find_map(|clipped| find_matrix_button_rect(&clipped.shape, insert_center))
            .expect("Insert matrix button rect");
        let cancel_rect = matrix_out
            .shapes
            .iter()
            .find_map(|clipped| find_matrix_button_rect(&clipped.shape, cancel_center))
            .expect("Cancel button rect");
        assert_eq!(
            (insert_rect.height(), cancel_rect.height()),
            (theme::BUTTON_HEIGHT, theme::BUTTON_HEIGHT)
        );
        assert_eq!(insert_rect.min.y, cancel_rect.min.y);
        assert!(
            matrix_rect.width() >= 270.0 && matrix_rect.width() < 330.0,
            "expected balanced Custom matrix modal width (270..330px), got {}",
            matrix_rect.width()
        );
        let rows_rect = matrix_out
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text_shape) if text_shape.galley.text() == "Rows" => {
                    Some(clipped.shape.visual_bounding_rect())
                }
                _ => None,
            })
            .expect("Rows label rect");
        let matrix_heading_rect = matrix_out
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text_shape) if text_shape.galley.text() == "Custom matrix" => {
                    Some(clipped.shape.visual_bounding_rect())
                }
                _ => None,
            })
            .expect("Custom matrix heading rect");
        let left_pad = rows_rect.left() - matrix_rect.left();
        let right_pad = matrix_rect.right() - cancel_rect.right();
        let bottom_pad = matrix_rect.bottom() - cancel_rect.bottom();
        let top_pad = matrix_heading_rect.top() - matrix_rect.top();
        assert!(
            (left_pad - right_pad).abs() <= 2.0,
            "left={left_pad}, right={right_pad}"
        );
        assert!(
            (right_pad - bottom_pad).abs() <= 1.0,
            "right={right_pad}, bottom={bottom_pad}"
        );
        assert!(
            right_pad >= 18.0 && top_pad >= 18.0,
            "right={right_pad}, top={top_pad}"
        );
        let _ = run_ui_pass_frame(&ctx, &mut app, click_events(Pos2::new(10.0, 10.0)));
        assert!(
            app.matrix_dialog.is_some(),
            "clicking outside Custom matrix backdrop must do nothing"
        );

        let insert_id = ctx
            .memory(|mem| mem.focused())
            .expect("Insert matrix initial focus");
        let _ = run_ui_pass_frame(
            &ctx,
            &mut app,
            vec![matrix_modal_key_event(egui::Key::ArrowRight, false)],
        );
        let cancel_id = ctx.memory(|mem| mem.focused()).expect("Cancel focus");
        assert_ne!(cancel_id, insert_id);

        let _ = run_ui_pass_frame(
            &ctx,
            &mut app,
            vec![matrix_modal_key_event(egui::Key::Enter, false)],
        );
        assert!(app.matrix_dialog.is_none());
        assert_eq!(app.editor.math(), before);
    }

    // 2. Pressing Enter right after opening inserts the default 2x2 matrix.
    {
        let ctx = egui::Context::default();
        let mut app = super::helpers::app();
        app.matrix_dialog = Some(MatrixDialog::default());

        let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
        let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
        let _ = run_ui_pass_frame(
            &ctx,
            &mut app,
            vec![matrix_modal_key_event(egui::Key::Enter, false)],
        );
        assert!(app.matrix_dialog.is_none());
        let math = app.editor.math();
        let table = math
            .iter()
            .into_iter()
            .find(|node| node.tag == "mtable")
            .unwrap();
        assert_eq!(table.children.len(), 2);
        assert_eq!(table.children[0].children.len(), 2);
    }

    // 3. Tab cycles into DragValue where ArrowUp adjusts rows, then Shift+Tab + ArrowLeft + Enter inserts 3x2.
    {
        let ctx = egui::Context::default();
        let mut app = super::helpers::app();
        app.matrix_dialog = Some(MatrixDialog::default());

        let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
        let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
        // Tab from Insert matrix -> Cancel -> Rows
        let _ = run_ui_pass_frame(
            &ctx,
            &mut app,
            vec![matrix_modal_key_event(egui::Key::Tab, false)],
        );
        let _ = run_ui_pass_frame(
            &ctx,
            &mut app,
            vec![matrix_modal_key_event(egui::Key::Tab, false)],
        );
        let _ = run_ui_pass_frame(&ctx, &mut app, vec![egui::Event::Text("3".into())]);
        // Moving focus out of DragValue via Shift+Tab commits the edited value.
        let _ = run_ui_pass_frame(
            &ctx,
            &mut app,
            vec![matrix_modal_key_event(egui::Key::Tab, true)],
        );
        assert_eq!(app.matrix_dialog.as_ref().unwrap().rows, 3);

        // ArrowLeft moves from Cancel to Insert matrix, Enter inserts 3x2.
        let _ = run_ui_pass_frame(
            &ctx,
            &mut app,
            vec![matrix_modal_key_event(egui::Key::ArrowLeft, false)],
        );
        let _ = run_ui_pass_frame(
            &ctx,
            &mut app,
            vec![matrix_modal_key_event(egui::Key::Enter, false)],
        );
        assert!(app.matrix_dialog.is_none());
        let math = app.editor.math();
        let table = math
            .iter()
            .into_iter()
            .find(|node| node.tag == "mtable")
            .unwrap();
        assert_eq!(table.children.len(), 3);
        assert_eq!(table.children[0].children.len(), 2);
    }
}
