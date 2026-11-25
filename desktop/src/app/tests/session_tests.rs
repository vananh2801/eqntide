use super::super::*;
use super::helpers::*;
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
            app.font_pt += 2.0;
        }
        assert!(draw_dialogs(&mut app));
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
