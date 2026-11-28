use super::super::*;
use super::helpers::*;
use crate::theme;
use eframe::egui::Stroke;
use std::time::Duration;

#[test]
fn dirty_equation_is_queued_without_mutating_its_target_or_tree() {
    let mut app = super::helpers::app();
    app.editor.insert_text("x");
    let before = app.editor.math();
    app.handle_window_request(request("", Some("<math><mi>y</mi></math>")));
    assert_eq!(app.editor.math(), before);
    assert_eq!(app.target, "omml");
    assert_eq!(app.queue.len(), 1);
    assert!(matches!(app.pending, Some(Intent::Next)));
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
        let mut app = super::helpers::app();
        let math = eqntide::service::parse_latex_to_mathml(latex).unwrap();
        app.editor.replace(&math);
        app.mark_canvas_changed();
        assert_eq!(app.target, "omml");

        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| app.draw_status_bar(ui));
        output.textures_delta.clear();
        let has_warning = output.shapes.iter().any(|clipped| {
            matches!(&clipped.shape, egui::Shape::Text(text_shape) if text_shape.galley.text().contains(expected_msg))
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
        let mut app = super::helpers::app();
        app.target = "mathtype".into();
        let math = eqntide::service::parse_latex_to_mathml(latex).unwrap();
        app.editor.replace(&math);
        app.mark_canvas_changed();

        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| app.draw_status_bar(ui));
        output.textures_delta.clear();
        let has_warning = output.shapes.iter().any(|clipped| {
            matches!(&clipped.shape, egui::Shape::Text(text_shape) if text_shape.galley.text().contains(expected_msg))
        });
        assert!(
            has_warning,
            "Expected MathType warning in status bar for {latex}"
        );
    }

    // finish_session(true) in an Office session blocks unsupported constructs for the active target.
    let mut app = super::helpers::app();
    app.session = Some("sess-1".into());
    app.target = "omml".into();
    let math = eqntide::service::parse_latex_to_mathml(r"\enclose{longdiv}{144}").unwrap();
    app.editor.replace(&math);
    app.mark_canvas_changed();
    app.finish_session(true);
    assert!(!app.busy);
    assert!(app.status.contains("not supported in OMML"));
}

#[test]
fn startup_loading_never_opens_two_sessions_concurrently() {
    let mut app = super::helpers::app();
    app.busy = true;
    app.handle_window_request(request("session-one", None));
    app.handle_window_request(request("session-two", None));
    assert_eq!(app.queue.len(), 2);
    assert!(app.session.is_none());
}

#[test]
fn untouched_office_session_does_not_prompt_even_while_busy() {
    for intent in [Intent::Close, Intent::Next] {
        let mut app = super::helpers::app();
        app.session = Some("office".into());
        app.pending = Some(intent);
        // Simulate loading/cancellation in flight; don't launch real HTTP.
        app.busy = true;
        assert!(!draw_dialogs(&mut app));
        assert!(app.pending.is_some());
        assert_eq!(app.session.as_deref(), Some("office"));
    }
}

#[test]
fn clean_transition_finishes_without_prompt_and_opens_next_equation() {
    let mut app = super::helpers::app();
    app.pending = Some(Intent::Next);
    app.queue
        .push_back(request("", Some("<math><mi>y</mi></math>")));
    assert!(!draw_dialogs(&mut app));
    assert!(app.busy);
    let finished = app.rx.recv_timeout(Duration::from_secs(2)).unwrap();
    app.tx.send(finished).unwrap();
    app.process_events(&egui::Context::default());
    assert_eq!(app.editor.math().to_latex(), "y");
    assert!(app.pending.is_none());
    assert!(app.queue.is_empty());
    assert!(!app.is_dirty());
}

#[test]
fn changed_office_equation_still_prompts_until_undone() {
    let mut app = super::helpers::app();
    app.session = Some("office".into());
    app.editor.insert_text("x");
    app.pending = Some(Intent::Close);
    assert!(draw_dialogs(&mut app));
    assert!(!app.busy);
    app.editor.undo();
    app.mark_canvas_changed();
    app.busy = true;
    assert!(!draw_dialogs(&mut app));
}

#[test]
fn invalid_source_or_changed_size_still_requires_confirmation() {
    for source_draft in [true, false] {
        let mut app = super::helpers::app();
        app.pending = Some(Intent::Close);
        if source_draft {
            app.latex = "\\frac{".into();
            app.mark_source_changed(Instant::now());
        } else {
            assert!(app.load_math("<math><mi>x</mi></math>", None, None));
            app.pending = Some(Intent::Close);
            app.font_pt += 2.0;
        }
        assert!(draw_dialogs(&mut app));
    }
}

#[test]
fn close_confirmation_respects_initial_edit_vs_insert_baseline() {
    fn close_raw_input() -> egui::RawInput {
        let mut input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::from(MIN_WINDOW_SIZE))),
            ..Default::default()
        };
        input
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .events
            .push(egui::ViewportEvent::Close);
        input
    }

    fn has_finish_modal(output: &egui::FullOutput) -> bool {
        output.shapes.iter().any(|clipped| {
            matches!(
                &clipped.shape,
                egui::Shape::Text(text_shape)
                    if text_shape.galley.text() == "Finish current equation"
            )
        })
    }

    // 1. Insert / standalone empty closes immediately without popup even if font size or target changed.
    {
        let ctx = egui::Context::default();
        let mut app = super::helpers::app();
        app.font_pt += 2.0;
        app.target = "mathtype".into();
        let mut out = ctx.run_ui(close_raw_input(), |ui| app.draw_ui(ui));
        out.textures_delta.clear();
        assert!(!has_finish_modal(&out));
        assert!(app.allow_close);
        assert!(app.pending.is_none());
    }

    // 2. Insert session typed then cleared back to empty closes without popup (even during debounced LaTeX clear).
    {
        let ctx = egui::Context::default();
        let mut app = super::helpers::app();
        app.editor.insert_text("x");
        app.mark_canvas_changed();
        assert!(app.is_dirty());
        app.latex.clear();
        app.mark_source_changed(Instant::now() + Duration::from_secs(10));
        assert!(app.draft_dirty);
        assert!(!app.is_dirty());
        let mut out = ctx.run_ui(close_raw_input(), |ui| app.draw_ui(ui));
        out.textures_delta.clear();
        assert!(!has_finish_modal(&out));
        assert!(app.allow_close);
        assert!(app.pending.is_none());
    }

    {
        let mut app = super::helpers::app();
        assert!(app.load_math("", None, None));
        app.session = Some("insert-session".into());
        app.editor.insert_text("x");
        app.mark_canvas_changed();
        assert!(app.is_dirty());
        app.editor.delete(true);
        app.mark_canvas_changed();
        assert!(!app.is_dirty());
        app.pending = Some(Intent::Close);
        assert!(!draw_dialogs(&mut app));
        assert!(app.busy);
    }

    // 3. Edit session left untouched cancels/closes without popup.
    {
        let mut app = super::helpers::app();
        assert!(app.load_math("<math><mi>x</mi></math>", None, None));
        app.session = Some("edit-session".into());
        app.pending = Some(Intent::Close);
        assert!(!draw_dialogs(&mut app));
        assert!(app.busy);
    }

    // 4. Edit session whose pre-existing equation is deleted on canvas MUST prompt.
    {
        let ctx = egui::Context::default();
        let mut app = super::helpers::app();
        assert!(app.load_math("<math><mi>x</mi></math>", None, None));
        app.session = Some("edit-session".into());
        app.editor.select_all();
        app.editor.delete(true);
        app.mark_canvas_changed();
        assert!(app.editor.math().children.is_empty());
        assert!(app.latex.is_empty());
        assert!(app.is_dirty());

        let mut out = ctx.run_ui(close_raw_input(), |ui| app.draw_ui(ui));
        out.textures_delta.clear();
        assert!(!app.allow_close);
        assert!(matches!(app.pending, Some(Intent::Close)));
        assert!(has_finish_modal(&out));
    }

    // 5. Edit session cleared in LaTeX box MUST prompt both during draft_dirty and after parse completes.
    {
        let ctx = egui::Context::default();
        let mut app = super::helpers::app();
        assert!(app.load_math("<math><mi>x</mi></math>", None, None));
        app.session = Some("edit-session".into());
        app.latex.clear();
        app.mark_source_changed(Instant::now() + Duration::from_secs(10));
        let generation = app.latex_generation;
        assert!(app.draft_dirty);
        assert!(app.is_dirty());

        let mut out = ctx.run_ui(close_raw_input(), |ui| app.draw_ui(ui));
        out.textures_delta.clear();
        assert!(!app.allow_close);
        assert!(matches!(app.pending, Some(Intent::Close)));
        assert!(has_finish_modal(&out));

        app.pending = None;
        parsed(&mut app, generation, Ok(eqntide::MathNode::new("math")));
        assert!(app.editor.math().children.is_empty());
        assert!(!app.draft_dirty);
        assert!(app.is_dirty());

        let mut out = ctx.run_ui(close_raw_input(), |ui| app.draw_ui(ui));
        out.textures_delta.clear();
        assert!(!app.allow_close);
        assert!(matches!(app.pending, Some(Intent::Close)));
        assert!(has_finish_modal(&out));
    }
}

#[test]
fn running_batch_still_requires_confirmation_even_without_equation_edits() {
    let mut app = super::helpers::app();
    app.pending = Some(Intent::Close);
    app.batch_running = true;
    assert!(draw_dialogs(&mut app));
}

#[test]
fn duplicate_requests_during_loading_do_not_prompt_or_queue() {
    let mut app = super::helpers::app();
    app.busy = true;
    app.loading_session = Some("office".into());
    app.handle_window_request(request("office", None));
    // Both bridge callbacks and forwarded custom URIs reach handle_window_request().
    app.handle_launch_args(
        vec!["eqntide://open?session_id=office".into()],
        &egui::Context::default(),
    );
    assert!(app.queue.is_empty());
    assert!(app.pending.is_none());
    app.tx
        .send(Event::Session(
            "office".into(),
            Ok(serde_json::json!({
                "state": "waiting", "mathml": "<math><mi>x</mi></math>",
                "font_pt": 14, "target_mode": "mathtype"
            })),
        ))
        .unwrap();
    app.process_events(&egui::Context::default());
    assert_eq!(app.session.as_deref(), Some("office"));
    assert!(app.loading_session.is_none());
    assert!(!app.is_dirty());
    app.handle_window_request(request("office", None));
    assert!(app.queue.is_empty());
    assert!(app.pending.is_none());
}

#[test]
fn duplicate_queued_session_is_not_added_twice() {
    let mut app = super::helpers::app();
    app.session = Some("current".into());
    app.handle_window_request(request("next", None));
    app.handle_window_request(request("next", None));
    assert_eq!(app.queue.len(), 1);
    assert!(matches!(app.pending, Some(Intent::Next)));
    assert_eq!(app.session.as_deref(), Some("current"));
}

#[test]
fn failed_session_load_clears_loading_marker_and_keeps_work() {
    let mut app = super::helpers::app();
    app.editor.insert_text("x");
    let before = app.editor.math();
    app.busy = true;
    app.loading_session = Some("office".into());
    app.tx
        .send(Event::Session("office".into(), Err("Unavailable".into())))
        .unwrap();
    app.process_events(&egui::Context::default());
    assert!(app.loading_session.is_none());
    assert!(!app.busy);
    assert!(app.session.is_none());
    assert_eq!(app.editor.math(), before);
    assert!(app.status.contains("Unavailable"));
}

#[test]
fn successful_office_save_closes_the_window() {
    let mut app = super::helpers::app();
    app.editor.insert_text("x");
    app.session = Some("office".into());
    app.busy = true;
    app.tx.send(Event::Finished(Ok(()))).unwrap();
    app.process_events(&egui::Context::default());
    assert!(app.session.is_none());
    assert!(!app.is_dirty());
    assert!(app.close_deadline.is_some());
    app.close_deadline = Some(Instant::now());
    app.drain_before_close(&egui::Context::default());
    assert!(app.allow_close);
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
            ctx.memory_mut(|mem| mem.request_focus(egui::Id::unique(id)));
        }
        assert!(App::take_save_shortcut(&ctx));
        assert!(!App::take_save_shortcut(&ctx));
        assert_eq!(ctx.input(|input| input.events.len()), 1);
        assert!(
            ctx.input(|input| matches!(&input.events[0], egui::Event::Key {
            key: egui::Key::S, modifiers, ..
        } if modifiers.shift && modifiers.command))
        );
        ctx.end_pass().textures_delta.clear();
    }
}

#[test]
fn save_shortcut_with_a_latex_draft_keeps_the_last_valid_canvas() {
    let mut app = super::helpers::app();
    app.load_math("<math><mi>x</mi></math>", None, None);
    let before = app.editor.math();
    let ctx = egui::Context::default();
    // Focus the real source widget, then type and save in the same frame.
    ctx.run_ui(egui::RawInput::default(), |ui| {
        ui.memory_mut(|mem| mem.request_focus(egui::Id::unique("latex-source")));
        app.draw_editor(ui);
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
            app.draw_editor(ui);
            if save && app.can_save() {
                app.finish_session(true);
            }
        },
    )
    .textures_delta
    .clear();
    assert_eq!(app.editor.math(), before);
    assert!(app.latex.contains('y'));
    assert!(app.draft_dirty);
    assert!(!app.busy);
    assert!(app.status.contains("valid LaTeX"));
}

#[test]
fn invalid_source_keeps_white_canvas_and_reports_error_only_in_status() {
    let mut app = super::helpers::app();
    app.load_math("<math><mi>x</mi></math>", None, None);
    let before = app.editor.math();
    app.latex = "\\frac{1}".into();
    app.mark_source_changed(Instant::now());
    app.latex_error = Some("incomplete fraction".into());
    let ctx = egui::Context::default();
    ctx.set_theme(egui::Theme::Dark);
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(940.0, 650.0))),
            ..Default::default()
        },
        |ui| app.draw_shell(ui),
    );
    output.textures_delta.clear();
    let text_shapes: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|clipped| {
            if let egui::Shape::Text(text_shape) = &clipped.shape {
                Some(text_shape)
            } else {
                None
            }
        })
        .collect();
    assert!(!text_shapes
        .iter()
        .any(|text_shape| text_shape.galley.text() == "LaTeX"
            || text_shape.galley.text() == app.status));
    let error = text_shapes
        .iter()
        .find(|text_shape| text_shape.galley.text().contains("incomplete fraction"))
        .unwrap();
    assert!(
        error.pos.y > 610.0,
        "error must be in the bottom status bar"
    );
    let pal = app.active_canvas_palette(true);
    assert!(output.shapes.iter().any(|clipped| matches!(&clipped.shape,
        egui::Shape::Rect(rect_shape) if rect_shape.fill == pal.bg && rect_shape.rect.width() > 200.0)));
    assert_eq!(app.editor.math(), before);
    assert!(app.draft_dirty);
}

#[test]
fn invalid_import_keeps_existing_work_and_source() {
    let mut app = super::helpers::app();
    app.editor.insert_text("x");
    app.mark_canvas_changed();
    let before = app.editor.math();
    let source = app.latex.clone();
    assert!(!app.load_math("<math><bad>", None, None));
    assert_eq!(app.editor.math(), before);
    assert_eq!(app.latex, source);
}

#[test]
fn session_failure_does_not_clear_dirty_work() {
    let mut app = super::helpers::app();
    app.editor.insert_text("x");
    app.session = Some("gone".into());
    app.busy = true;
    app.tx
        .send(Event::Finished(Err("Session expired".into())))
        .unwrap();
    app.process_events(&egui::Context::default());
    assert!(app.is_dirty());
    assert_eq!(app.session.as_deref(), Some("gone"));
    assert!(!app.busy);
    assert!(!app.allow_close);
}

#[test]
fn a_latex_draft_prevents_accidental_save() {
    let mut app = super::helpers::app();
    app.draft_dirty = true;
    app.finish_session(true);
    assert!(!app.busy);
    assert!(app.status.contains("valid LaTeX"));
}

#[test]
fn live_latex_updates_the_equation_without_rewriting_the_source() {
    let mut app = super::helpers::app();
    let source = r"  \frac{x + 1}{2}  ";
    let (generation, _) = source_request(&mut app, source, Instant::now());
    let key = app.render_key;
    parsed(
        &mut app,
        generation,
        eqntide::service::parse_latex_to_mathml(source).map_err(|err| err.to_string()),
    );
    assert_eq!(app.latex, source);
    assert!(!app.draft_dirty);
    assert!(app.latex_error.is_none());
    assert!(app.render_key > key);
    assert!(app
        .editor
        .math()
        .iter()
        .iter()
        .any(|node| node.tag == "mfrac"));
}

#[test]
fn live_latex_debounces_until_typing_pauses() {
    let mut app = super::helpers::app();
    let now = Instant::now();
    app.latex = "x".into();
    app.mark_source_changed(now);
    assert!(app
        .take_latex_request(now + Duration::from_millis(100))
        .is_none());
    app.latex = "x+1".into();
    app.mark_source_changed(now + Duration::from_millis(120));
    assert!(app
        .take_latex_request(now + Duration::from_millis(180))
        .is_none());
    assert_eq!(
        app.take_latex_request(now + Duration::from_millis(300))
            .unwrap()
            .1,
        "x+1"
    );
}

#[test]
fn stale_latex_results_cannot_overwrite_newer_input() {
    let mut app = super::helpers::app();
    let now = Instant::now();
    let (old, _) = source_request(&mut app, "x", now);
    app.latex = "y".into();
    app.mark_source_changed(now);
    parsed(
        &mut app,
        old,
        eqntide::service::parse_latex_to_mathml("x").map_err(|err| err.to_string()),
    );
    assert!(app.editor.math().children.is_empty());
    assert!(app.draft_dirty);
    assert_eq!(app.latex, "y");
    let (latest, source) = app
        .take_latex_request(now + Duration::from_millis(180))
        .unwrap();
    parsed(
        &mut app,
        latest,
        eqntide::service::parse_latex_to_mathml(&source).map_err(|err| err.to_string()),
    );
    assert_eq!(app.editor.math().to_latex(), "y");
    assert!(!app.draft_dirty);
}

#[test]
fn invalid_latex_keeps_the_last_valid_equation_and_raw_draft() {
    let mut app = super::helpers::app();
    app.load_math("<math><mi>x</mi></math>", None, None);
    let before = app.editor.math();
    let key = app.render_key;
    let source = r"\unknowncommand{a}";
    let (generation, _) = source_request(&mut app, source, Instant::now());
    parsed(
        &mut app,
        generation,
        eqntide::service::parse_latex_to_mathml(source).map_err(|err| err.to_string()),
    );
    assert_eq!(app.editor.math(), before);
    assert_eq!(app.render_key, key);
    assert_eq!(app.latex, source);
    assert!(app.draft_dirty);
    assert!(app.latex_error.is_some());
}

#[test]
fn canvas_change_invalidates_its_inflight_source_parse() {
    let mut app = super::helpers::app();
    app.load_math("<math><mi>x</mi></math>", None, None);
    let before = app.editor.math();
    let (generation, _) = source_request(&mut app, "y", Instant::now());
    app.mark_canvas_changed();
    parsed(
        &mut app,
        generation,
        eqntide::service::parse_latex_to_mathml("y").map_err(|err| err.to_string()),
    );
    assert_eq!(app.editor.math(), before);
    assert_eq!(app.latex, "x");
    assert!(!app.draft_dirty);
}

#[test]
fn live_source_replacement_can_undo_to_original_mathml_attributes() {
    let mut app = super::helpers::app();
    app.load_math("<math><mi mathvariant=\"bold\">x</mi></math>", None, None);
    let before = app.editor.math();
    let (generation, _) = source_request(&mut app, "y", Instant::now());
    parsed(
        &mut app,
        generation,
        eqntide::service::parse_latex_to_mathml("y").map_err(|err| err.to_string()),
    );
    app.editor.undo();
    app.mark_canvas_changed();
    assert_eq!(app.editor.math(), before);
}

#[test]
fn clearing_latex_clears_canvas_through_the_background_worker() {
    let mut app = super::helpers::app();
    app.load_math("<math><mi>x</mi></math>", None, None);
    app.latex = String::new();
    app.mark_source_changed(Instant::now() - Duration::from_secs(1));
    app.request_latex_parse(&egui::Context::default());
    let result = app.rx.recv_timeout(Duration::from_secs(2)).unwrap();
    app.tx.send(result).unwrap();
    app.process_events(&egui::Context::default());
    assert!(app.editor.math().children.is_empty());
    assert!(!app.draft_dirty);
    assert!(app.latex.is_empty());
}

#[test]
fn live_latex_source_blocks_invalid_commands_and_keeps_valid_style_groups() {
    for invalid in [r"\mathbb{1^2}", r"\text{}", r"\textstyle", r"\mathcal{1}"] {
        let mut app = super::helpers::app();
        app.load_math("<math><mi>x</mi></math>", None, None);
        app.session = Some("sess-1".into());
        let before = app.editor.math();
        app.latex = invalid.into();
        app.mark_source_changed(Instant::now() - Duration::from_secs(1));
        app.request_latex_parse(&egui::Context::default());
        let result = app.rx.recv_timeout(Duration::from_secs(2)).unwrap();
        app.tx.send(result).unwrap();
        app.process_events(&egui::Context::default());

        assert_eq!(app.editor.math(), before, "{invalid}");
        assert!(app.draft_dirty, "{invalid}");
        assert!(
            app.latex_error
                .as_deref()
                .is_some_and(|err| err.contains("Invalid LaTeX command")),
            "expected latex_error for `{invalid}`, got {:?}",
            app.latex_error
        );
        app.finish_session(true);
        assert!(!app.busy, "saving must stay blocked for `{invalid}`");
        assert!(
            app.status.contains("Wait for valid LaTeX"),
            "{}",
            app.status
        );

        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| app.draw_status_bar(ui));
        out.textures_delta.clear();
        assert!(
            out.shapes.iter().any(|clipped| match &clipped.shape {
                egui::Shape::Text(text_shape) => text_shape.galley.text().contains("LaTeX error:"),
                _ => false,
            }),
            "status bar should show LaTeX error for `{invalid}`"
        );
    }

    for valid in [r"\text{ }", r"{\textstyle \frac{1}{2}}", r"\mathbf{x^2}"] {
        let mut app = super::helpers::app();
        app.latex = valid.into();
        app.mark_source_changed(Instant::now() - Duration::from_secs(1));
        app.request_latex_parse(&egui::Context::default());
        let result = app.rx.recv_timeout(Duration::from_secs(2)).unwrap();
        app.tx.send(result).unwrap();
        app.process_events(&egui::Context::default());

        assert!(!app.draft_dirty, "{valid}");
        assert!(app.latex_error.is_none(), "{valid}: {:?}", app.latex_error);
        assert!(!app.editor.math().children.is_empty(), "{valid}");
    }
}

fn modal_key_event(key: egui::Key, shift: bool) -> egui::Event {
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
fn finish_equation_modal_centers_over_backdrop_and_ignores_outside_clicks() {
    let ctx = egui::Context::default();
    ThemeTokens::apply_style(&ctx);
    let mut app = super::helpers::app();
    app.editor.insert_text("x");
    app.mark_canvas_changed();
    app.pending = Some(Intent::Close);

    let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
    let out = run_ui_pass_frame(&ctx, &mut app, vec![]);
    let window_size = Vec2::from(MIN_WINDOW_SIZE);
    let window_center = Pos2::new(window_size.x / 2.0, window_size.y / 2.0);
    let heading_center = find_text_center(&out, |text| text == "Finish current equation");
    let save_center = find_text_center(&out, |text| text == "Save");
    let discard_center = find_text_center(&out, |text| text == "Discard / cancel session");
    let continue_center = find_text_center(&out, |text| text == "Continue editing");
    fn find_button_rect(shape: &egui::Shape, point: Pos2) -> Option<Rect> {
        match shape {
            egui::Shape::Rect(rect_shape)
                if rect_shape.rect.height() < 45.0
                    && rect_shape.rect.width() < 240.0
                    && rect_shape.rect.contains(point) =>
            {
                Some(rect_shape.rect)
            }
            egui::Shape::Vec(inner) => inner
                .iter()
                .find_map(|child| find_button_rect(child, point)),
            _ => None,
        }
    }
    let save_rect = out
        .shapes
        .iter()
        .find_map(|clipped| find_button_rect(&clipped.shape, save_center))
        .expect("Save button rect");
    let discard_rect = out
        .shapes
        .iter()
        .find_map(|clipped| find_button_rect(&clipped.shape, discard_center))
        .expect("Discard button rect");
    let continue_rect = out
        .shapes
        .iter()
        .find_map(|clipped| find_button_rect(&clipped.shape, continue_center))
        .expect("Continue button rect");
    assert_eq!(
        (
            save_rect.height(),
            discard_rect.height(),
            continue_rect.height()
        ),
        (
            theme::BUTTON_HEIGHT,
            theme::BUTTON_HEIGHT,
            theme::BUTTON_HEIGHT
        ),
        "save={save_rect:?}, discard={discard_rect:?}, continue={continue_rect:?}"
    );
    assert_eq!(save_rect.min.y, discard_rect.min.y);
    assert_eq!(save_rect.max.y, discard_rect.max.y);
    assert_eq!(continue_rect.min.y, discard_rect.min.y);
    fn find_modal_rect(
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
                .find_map(|child| find_modal_rect(child, heading, button, max_width)),
            _ => None,
        }
    }
    let modal_rect = out
        .shapes
        .iter()
        .find_map(|clipped| {
            find_modal_rect(
                &clipped.shape,
                heading_center,
                continue_center,
                window_size.x,
            )
        })
        .expect("centered Finish current equation modal rect");
    assert!(
        (modal_rect.center().x - window_center.x).abs() < 4.0
            && (modal_rect.center().y - window_center.y).abs() < 4.0,
        "expected centered modal rect near {window_center:?}, got {:?}",
        modal_rect.center()
    );
    assert!(
        (heading_center.x - modal_rect.center().x).abs() < 4.0,
        "expected centered heading near modal center {}, got {}",
        modal_rect.center().x,
        heading_center.x
    );
    assert!(
        !out.shapes.iter().any(|clipped| matches!(
            &clipped.shape,
            egui::Shape::Text(text_shape)
                if text_shape.galley.text() == "Save your changes, discard them, or continue editing."
        )),
        "subtitle should be omitted from Finish current equation modal"
    );
    assert!(
        out.shapes.iter().any(|clipped| matches!(
            &clipped.shape,
            egui::Shape::Rect(rect_shape)
                if rect_shape.rect == Rect::from_min_size(Pos2::ZERO, window_size)
                    && rect_shape.fill.a() > 0
        )),
        "expected full-window modal backdrop scrim"
    );

    let _ = run_ui_pass_frame(&ctx, &mut app, click_events(Pos2::new(10.0, 10.0)));
    assert!(
        matches!(app.pending, Some(Intent::Close)),
        "clicking outside modal backdrop must do nothing"
    );
}

#[test]
fn finish_equation_modal_opens_on_canvas_escape_and_closes_on_second_escape() {
    let ctx = egui::Context::default();
    let mut app = super::helpers::app();
    app.editor.insert_text("x");
    app.mark_canvas_changed();

    // Prime canvas focus first.
    let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
    let canvas_id = App::editor_focus_id(StartupFocus::Canvas);
    assert!(ctx.memory(|mem| mem.has_focus(canvas_id)));

    // Pressing Escape on canvas opens the modal without immediately self-closing on the same frame.
    let _ = run_ui_pass_frame(
        &ctx,
        &mut app,
        vec![modal_key_event(egui::Key::Escape, false)],
    );
    assert!(matches!(app.pending, Some(Intent::Close)));

    // Settle the modal sizing pass, then press Escape again to continue editing.
    let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
    let _ = run_ui_pass_frame(
        &ctx,
        &mut app,
        vec![modal_key_event(egui::Key::Escape, false)],
    );
    assert!(app.pending.is_none());
    assert_eq!(app.editor.math().to_latex(), "x");

    let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
    assert!(
        ctx.memory(|mem| mem.has_focus(canvas_id)),
        "Escape should restore focus back to the active editor"
    );
}

#[test]
fn finish_equation_modal_keyboard_navigation_and_enter_activation() {
    // 1. Default initial focus lands on Save; Enter triggers finish_session(true).
    {
        let ctx = egui::Context::default();
        let mut app = super::helpers::app();
        app.session = Some("office".into());
        app.editor.insert_text("x");
        app.mark_canvas_changed();
        app.pending = Some(Intent::Close);

        let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
        let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
        let _ = run_ui_pass_frame(
            &ctx,
            &mut app,
            vec![modal_key_event(egui::Key::Enter, false)],
        );
        assert!(app.busy, "Enter on initial focus should activate Save");
    }

    // 2. Tab, Shift+Tab, ArrowLeft, and ArrowRight cycle cleanly among modal buttons without escaping.
    {
        let ctx = egui::Context::default();
        let mut app = super::helpers::app();
        app.bridge_ready = false; // Show status bar Retry button behind modal to verify Tab traps inside modal.
        app.editor.insert_text("x");
        app.mark_canvas_changed();
        app.pending = Some(Intent::Close);

        let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
        let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
        let save_id = ctx.memory(|mem| mem.focused()).expect("Save initial focus");

        // Tab: Save -> Discard -> Continue -> Save (wrapped)
        let _ = run_ui_pass_frame(&ctx, &mut app, vec![modal_key_event(egui::Key::Tab, false)]);
        let discard_id = ctx.memory(|mem| mem.focused()).expect("Discard focus");
        assert_ne!(discard_id, save_id);

        let _ = run_ui_pass_frame(&ctx, &mut app, vec![modal_key_event(egui::Key::Tab, false)]);
        let continue_id = ctx.memory(|mem| mem.focused()).expect("Continue focus");
        assert_ne!(continue_id, save_id);
        assert_ne!(continue_id, discard_id);

        let _ = run_ui_pass_frame(&ctx, &mut app, vec![modal_key_event(egui::Key::Tab, false)]);
        assert_eq!(ctx.memory(|mem| mem.focused()), Some(save_id));

        // Shift+Tab: Save -> Continue
        let _ = run_ui_pass_frame(&ctx, &mut app, vec![modal_key_event(egui::Key::Tab, true)]);
        assert_eq!(ctx.memory(|mem| mem.focused()), Some(continue_id));

        // ArrowLeft / ArrowRight: Continue -> Discard -> Save -> Discard -> Continue
        let _ = run_ui_pass_frame(
            &ctx,
            &mut app,
            vec![modal_key_event(egui::Key::ArrowLeft, false)],
        );
        assert_eq!(ctx.memory(|mem| mem.focused()), Some(discard_id));
        let _ = run_ui_pass_frame(
            &ctx,
            &mut app,
            vec![modal_key_event(egui::Key::ArrowLeft, false)],
        );
        assert_eq!(ctx.memory(|mem| mem.focused()), Some(save_id));
        let _ = run_ui_pass_frame(
            &ctx,
            &mut app,
            vec![modal_key_event(egui::Key::ArrowRight, false)],
        );
        assert_eq!(ctx.memory(|mem| mem.focused()), Some(discard_id));
        let _ = run_ui_pass_frame(
            &ctx,
            &mut app,
            vec![modal_key_event(egui::Key::ArrowRight, false)],
        );
        assert_eq!(ctx.memory(|mem| mem.focused()), Some(continue_id));

        // Enter on Continue editing dismisses the modal cleanly.
        let _ = run_ui_pass_frame(
            &ctx,
            &mut app,
            vec![modal_key_event(egui::Key::Enter, false)],
        );
        assert!(app.pending.is_none());
    }

    // 3. When batch_running is true, Stop batch receives initial focus and activates on Enter.
    {
        let ctx = egui::Context::default();
        let mut app = super::helpers::app();
        app.batch_running = true;
        app.pending = Some(Intent::Close);

        let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
        let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
        let stop_id = ctx
            .memory(|mem| mem.focused())
            .expect("Stop batch initial focus");
        let _ = run_ui_pass_frame(&ctx, &mut app, vec![modal_key_event(egui::Key::Tab, false)]);
        assert_eq!(ctx.memory(|mem| mem.focused()), Some(stop_id));
        let _ = run_ui_pass_frame(
            &ctx,
            &mut app,
            vec![modal_key_event(egui::Key::Enter, false)],
        );
        assert!(app.stop.load(std::sync::atomic::Ordering::Relaxed));
    }
}
