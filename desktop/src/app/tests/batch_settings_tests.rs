use super::super::*;
use super::helpers::*;
use crate::settings::ThemeMode;
use std::time::Duration;

#[test]
fn with_settings_path_restores_all_persisted_preferences() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let saved = Settings {
        zoom: 3.0,
        theme: ThemeMode::Dark,
        canvas_opacity: 0.65,
        startup_focus: StartupFocus::Latex,
        font_pt: 18.5,
        latex_font_pt: 21.5,
        target: "mathtype".into(),
        category: palette::RECENT_GROUP.into(),
        recent_palette: vec![r"\alpha".into(), r"\frac{#?}{#?}".into()],
        batch_target: 2,
        batch_overwrite: true,
        ..Settings::default()
    };
    saved.save_to_path(&path).unwrap();

    let isolated = super::helpers::app();
    assert!(isolated.settings_path.is_none());
    assert_eq!(isolated.zoom, settings::DEFAULT_ZOOM);
    assert_eq!(isolated.font_pt, settings::DEFAULT_FONT_PT);
    assert_eq!(isolated.latex_font_pt, settings::DEFAULT_LATEX_FONT_PT);

    let ctx = egui::Context::default();
    let restored = super::helpers::app().with_settings_path(path, Some(&ctx));
    assert_eq!(restored.zoom, 3.0);
    assert!((restored.canvas_opacity - 0.65).abs() < 1e-6);
    assert_eq!(ctx.theme(), egui::Theme::Dark);
    assert_eq!(restored.startup_focus, StartupFocus::Latex);
    assert_eq!(restored.font_pt, 18.5);
    assert_eq!(restored.saved_font, 18.5);
    assert_eq!(restored.latex_font_pt, 21.5);
    assert_eq!(restored.target, "mathtype");
    assert_eq!(restored.saved_mode, "mathtype");
    assert_eq!(restored.category, palette::RECENT_GROUP);
    assert_eq!(restored.recent_palette, vec![r"\alpha", r"\frac{#?}{#?}"]);
    assert_eq!(restored.batch_target, 2);
    assert!(restored.batch_overwrite);
    assert!(!restored.is_dirty());
    assert!(!restored.settings_dirty);
}

#[test]
fn office_session_overrides_do_not_overwrite_saved_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    Settings {
        font_pt: 16.0,
        latex_font_pt: 17.5,
        target: "mathtype".into(),
        ..Settings::default()
    }
    .save_to_path(&path)
    .unwrap();

    let ctx = egui::Context::default();
    let mut app = super::helpers::app().with_settings_path(path.clone(), Some(&ctx));

    // Standalone load_math with font=None keeps the persisted font_pt.
    assert!(app.load_math("<math><mi>y</mi></math>", None, None));
    assert_eq!(app.font_pt, 16.0);

    // Office session overrides live font_pt and target without dirtying persisted settings.
    app.loading_session = Some("office-1".into());
    app.tx
        .send(Event::Session(
            "office-1".into(),
            Ok(serde_json::json!({
                "state": "waiting",
                "mathml": "<math><mi>x</mi></math>",
                "font_pt": 24.0,
                "target_mode": "omml"
            })),
        ))
        .unwrap();
    app.process_events(&ctx);

    assert_eq!(app.font_pt, 24.0);
    assert_eq!(app.target, "omml");
    assert_eq!(app.session.as_deref(), Some("office-1"));
    assert!(!app.settings_dirty);

    // Completing/closing the session leaves the saved defaults intact on disk.
    run_ui_pass_frame(&ctx, &mut app, vec![]);
    app.tx.send(Event::Finished(Ok(()))).unwrap();
    app.process_events(&ctx);

    let on_disk = Settings::load_from_path(&path);
    assert_eq!(on_disk.font_pt, 16.0);
    assert_eq!(on_disk.latex_font_pt, 17.5);
    assert_eq!(on_disk.target, "mathtype");
}

#[test]
fn user_ui_edits_persist_atomically_and_coalesce_pointer_drag() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let ctx = egui::Context::default();
    ctx.set_theme(egui::Theme::Dark);
    let mut app = super::helpers::app().with_settings_path(path.clone(), Some(&ctx));
    // Switch back to Dark explicitly for initial baseline via theme toggle test later.
    let out = run_ui_pass_frame(&ctx, &mut app, vec![]);

    // 1. Click sidebar category "Scripts".
    let scripts_tex = app
        .ui_icon_texture_id(palette::ui_group("Scripts").unwrap().icons[0])
        .expect("Scripts sidebar texture");
    let scripts_pos = out
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect_shape)
                if rect_shape
                    .brush
                    .as_ref()
                    .is_some_and(|brush| brush.fill_texture_id == scripts_tex) =>
            {
                Some(rect_shape.rect.center())
            }
            _ => None,
        })
        .expect("Scripts sidebar icon");
    let out = run_ui_pass_frame(&ctx, &mut app, click_events(scripts_pos));
    assert_eq!(app.category, "Scripts");
    assert_eq!(Settings::load_from_path(&path).category, "Scripts");

    // 2. Click a palette item in "Scripts".
    let scripts_tex = app
        .palette_atlas
        .as_ref()
        .expect("shared palette atlas")
        .texture
        .id();
    let item_pos = out
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect_shape)
                if rect_shape
                    .brush
                    .as_ref()
                    .is_some_and(|brush| brush.fill_texture_id == scripts_tex) =>
            {
                Some(rect_shape.rect.center())
            }
            _ => None,
        })
        .expect("Scripts palette button");
    let out = run_ui_pass_frame(&ctx, &mut app, click_events(item_pos));
    assert_eq!(app.recent_palette.len(), 1);
    let clicked_source = app.recent_palette[0];
    assert_eq!(
        Settings::load_from_path(&path).recent_palette,
        vec![clicked_source.to_string()]
    );

    // 3. Click footer "MathType" radio button.
    let mathtype_pos = find_text_center(&out, |text| text == "MathType");
    let out = run_ui_pass_frame(&ctx, &mut app, click_events(mathtype_pos));
    assert_eq!(app.target, "mathtype");
    assert_eq!(Settings::load_from_path(&path).target, "mathtype");

    // 4. Drag footer Font size DragValue while pointer is down, then release.
    let font_pos = find_text_center(&out, |text| text.ends_with(" pt"));
    let drag_to = Pos2::new(font_pos.x + 80.0, font_pos.y);
    run_ui_pass_frame(
        &ctx,
        &mut app,
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
    run_ui_pass_frame(&ctx, &mut app, vec![egui::Event::PointerMoved(drag_to)]);
    assert!(app.font_pt > settings::DEFAULT_FONT_PT);
    assert!(app.settings_dirty);
    // Disk write is deferred until pointer drag releases.
    assert_eq!(
        Settings::load_from_path(&path).font_pt,
        settings::DEFAULT_FONT_PT
    );
    run_ui_pass_frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerButton {
            pos: drag_to,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert!(!app.settings_dirty);
    let dragged_font = app.font_pt;
    assert_eq!(Settings::load_from_path(&path).font_pt, dragged_font);

    // 5. Drag footer Zoom DragValue while pointer is down, then release.
    let zoom_pos = find_text_center(&out, |text| text == "200%");
    let zoom_drag_end = Pos2::new(zoom_pos.x + 60.0, zoom_pos.y);
    run_ui_pass_frame(
        &ctx,
        &mut app,
        vec![
            egui::Event::PointerMoved(zoom_pos),
            egui::Event::PointerButton {
                pos: zoom_pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    run_ui_pass_frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerMoved(zoom_drag_end)],
    );
    assert!(app.settings_dirty);
    assert_eq!(Settings::load_from_path(&path).zoom, settings::DEFAULT_ZOOM);
    let out = run_ui_pass_frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerButton {
            pos: zoom_drag_end,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert!(!app.settings_dirty);
    let dragged_zoom = app.zoom;
    assert!((dragged_zoom - settings::DEFAULT_ZOOM).abs() > 0.05);
    assert_eq!(Settings::load_from_path(&path).zoom, dragged_zoom);

    // Open Settings tab and toggle theme (Dark -> Light).
    let settings_pos = find_text_center(&out, |text| text == "Settings");
    run_ui_pass_frame(&ctx, &mut app, click_events(settings_pos));
    assert_eq!(app.tab, 2);
    let out = run_ui_pass_frame(&ctx, &mut app, vec![]);
    let light_pos = find_text_center(&out, |text| text == "Light");
    let out = run_ui_pass_frame(&ctx, &mut app, click_events(light_pos));
    assert_eq!(ctx.theme(), egui::Theme::Light);
    assert_eq!(Settings::load_from_path(&path).theme, ThemeMode::Light);

    // 6. Open Batch DOCX tab, toggle overwrite and change batch target.
    let back_pos = find_text_center(&out, |text| text == "Back");
    run_ui_pass_frame(&ctx, &mut app, click_events(back_pos));
    let out = run_ui_pass_frame(&ctx, &mut app, vec![]);
    let batch_pos = find_text_center(&out, |text| text == "Batch DOCX");
    run_ui_pass_frame(&ctx, &mut app, click_events(batch_pos));
    assert_eq!(app.tab, 1);
    let out = run_ui_pass_frame(&ctx, &mut app, vec![]);

    let overwrite_pos = find_text_center(&out, |text| text.starts_with("Overwrite original"));
    let out = run_ui_pass_frame(&ctx, &mut app, click_events(overwrite_pos));
    assert!(app.batch_overwrite);
    assert!(Settings::load_from_path(&path).batch_overwrite);

    let combo_pos = find_text_center(&out, |text| text == batch::TARGETS[0].1);
    run_ui_pass_frame(&ctx, &mut app, click_events(combo_pos));
    let out = run_ui_pass_frame(&ctx, &mut app, vec![]);
    let option_pos = find_text_center(&out, |text| text == batch::TARGETS[1].1);
    run_ui_pass_frame(&ctx, &mut app, click_events(option_pos));
    assert_eq!(app.batch_target, 1);
    assert_eq!(Settings::load_from_path(&path).batch_target, 1);

    // 7. Reload in a brand new App instance and confirm full restoration.
    let fresh_ctx = egui::Context::default();
    let mut reloaded = super::helpers::app().with_settings_path(path.clone(), Some(&fresh_ctx));
    assert_eq!(reloaded.category, "Scripts");
    assert_eq!(reloaded.recent_palette, vec![clicked_source]);
    assert_eq!(reloaded.target, "mathtype");
    assert_eq!(reloaded.font_pt, dragged_font);
    assert_eq!(reloaded.zoom, dragged_zoom);
    assert_eq!(fresh_ctx.theme(), egui::Theme::Light);
    assert!(reloaded.batch_overwrite);
    assert_eq!(reloaded.batch_target, 1);

    // 8. Verify hover wheel over footer Zoom adjusts by 10% and persists immediately.
    reloaded.tab = 0;
    let out = run_ui_pass_frame(&fresh_ctx, &mut reloaded, vec![]);
    let reloaded_zoom_pos = find_text_center(&out, |text| {
        text == format!("{:.0}%", (dragged_zoom * 100.0).round())
    });
    let _ = run_ui_pass_frame(
        &fresh_ctx,
        &mut reloaded,
        vec![
            egui::Event::PointerMoved(reloaded_zoom_pos),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: Vec2::new(0.0, 1.0),
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Unknown,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    let expected_wheel_zoom = ((dragged_zoom + 0.1) * 10.0).round() / 10.0;
    assert_eq!(reloaded.zoom, expected_wheel_zoom);
    assert_eq!(Settings::load_from_path(&path).zoom, expected_wheel_zoom);
}

#[test]
fn settings_ui_startup_focus_selector_persists_preference() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let ctx = egui::Context::default();
    ThemeTokens::apply_style(&ctx);
    let mut app = super::helpers::app().with_settings_path(path.clone(), Some(&ctx));
    assert_eq!(app.startup_focus, StartupFocus::Canvas);

    app.tab = 2;
    let tall = Vec2::new(MIN_WINDOW_SIZE[0], 900.0);
    let out = run_ui_pass_sized(&ctx, &mut app, tall);
    let latex_btn = find_text_center(&out, |text| text == "LaTeX source");
    run_ui_pass_sized_events(&ctx, &mut app, tall, click_events(latex_btn));

    assert_eq!(app.startup_focus, StartupFocus::Latex);
    let on_disk = Settings::load_from_path(&path);
    assert_eq!(on_disk.startup_focus, StartupFocus::Latex);

    let fresh_ctx = egui::Context::default();
    let reloaded = super::helpers::app().with_settings_path(path, Some(&fresh_ctx));
    assert_eq!(reloaded.startup_focus, StartupFocus::Latex);
}

#[test]
fn startup_focus_applies_on_launch_and_return_to_editor() {
    let canvas_id = egui::Id::unique("equation-canvas");
    let latex_id = egui::Id::unique("latex-source");

    // Default startup focus activates Equation Canvas immediately.
    let ctx = egui::Context::default();
    ThemeTokens::apply_style(&ctx);
    let mut canvas_app = super::helpers::app();
    run_ui_pass_frame(&ctx, &mut canvas_app, vec![]);
    assert!(ctx.memory(|mem| mem.has_focus(canvas_id)));
    assert!(!ctx.memory(|mem| mem.has_focus(latex_id)));

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
    let mut latex_app = super::helpers::app().with_settings_path(path, Some(&latex_ctx));
    run_ui_pass_frame(&latex_ctx, &mut latex_app, vec![]);
    assert!(latex_ctx.memory(|mem| mem.has_focus(latex_id)));
    assert!(!latex_ctx.memory(|mem| mem.has_focus(canvas_id)));

    // Open Settings and click "Back" -> focuses LaTeX source again.
    let out = run_ui_pass_frame(&latex_ctx, &mut latex_app, vec![]);
    let settings_btn = find_text_center(&out, |text| text == "Settings");
    run_ui_pass_frame(&latex_ctx, &mut latex_app, click_events(settings_btn));
    assert_eq!(latex_app.tab, 2);
    let out = run_ui_pass_frame(&latex_ctx, &mut latex_app, vec![]);
    let back_btn = find_text_center(&out, |text| text == "Back");
    run_ui_pass_frame(&latex_ctx, &mut latex_app, click_events(back_btn));
    assert_eq!(latex_app.tab, 0);
    run_ui_pass_frame(&latex_ctx, &mut latex_app, vec![]);
    assert!(latex_ctx.memory(|mem| mem.has_focus(latex_id)));
}

#[test]
fn settings_cards_span_full_central_panel_width() {
    let ctx = egui::Context::default();
    ThemeTokens::apply_style(&ctx);
    ctx.set_theme(egui::Theme::Dark);
    let tokens = ThemeTokens::for_mode(true);
    let mut app = super::helpers::app();
    app.tab = 2;

    for width in [MIN_WINDOW_SIZE[0], 1400.0] {
        let size = Vec2::new(width, 900.0);
        let _ = run_ui_pass_sized(&ctx, &mut app, size);
        let out = run_ui_pass_sized(&ctx, &mut app, size);
        let expected_width = width - 2.0 * theme::SPACE_LG;
        assert!(!has_rendered_text(&out, "Canvas zoom"));
        assert!(!has_rendered_text(&out, "Reset (200%)"));
        let cards = card_rects_for_section_titles(
            &out,
            tokens,
            &["Appearance", "App background", "Startup focus"],
        );
        assert!(cards[1].top() > cards[0].bottom());
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
fn settings_opacity_level_controls_reflect_accent_and_interactions() {
    let dir = tempfile::tempdir().unwrap();
    let bg_png = dir.path().join("bg.png");
    let img = image::RgbaImage::from_pixel(32, 32, image::Rgba([120, 120, 120, 255]));
    image::DynamicImage::ImageRgba8(img)
        .save_with_format(&bg_png, image::ImageFormat::Png)
        .unwrap();

    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        let dark_mode = theme == egui::Theme::Dark;
        let tokens = ThemeTokens::for_mode(dark_mode);
        let ctx = egui::Context::default();
        ThemeTokens::apply_style(&ctx);
        ctx.set_theme(theme);
        let mut app = super::helpers::app();
        app.set_background_image_from_path(&bg_png, &ctx);
        app.background_accent_enabled = false;
        app.tab = 2;

        let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
        let out = run_ui_pass_frame(&ctx, &mut app, vec![]);

        let find_level_box = |frame: &egui::FullOutput,
                              row_y: f32,
                              expected_track: egui::Color32,
                              expected_fill: egui::Color32,
                              label: &str| {
            let track = frame
                .shapes
                .iter()
                .find_map(|clipped| match &clipped.shape {
                    egui::Shape::Rect(rect_shape)
                        if rect_shape.fill == expected_track
                            && (rect_shape.rect.width() - 220.0).abs() <= 0.5
                            && (rect_shape.rect.height() - theme::BUTTON_HEIGHT).abs() <= 0.5
                            && rect_shape.corner_radius == theme::CONTROL_CORNER_RADIUS
                            && (rect_shape.rect.center().y - row_y).abs() <= 2.0 =>
                    {
                        Some((rect_shape.rect, rect_shape.stroke))
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("missing {label} level-fill track in {theme:?}"));
            let fill = frame
                .shapes
                .iter()
                .find_map(|clipped| match &clipped.shape {
                    egui::Shape::Rect(rect_shape)
                        if rect_shape.fill == expected_fill
                            && (rect_shape.rect.left() - track.0.left()).abs() <= 0.5
                            && rect_shape.rect.top() >= track.0.top() - 0.5
                            && rect_shape.rect.bottom() <= track.0.bottom() + 0.5 =>
                    {
                        Some((rect_shape.rect, rect_shape.stroke))
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("missing {label} level-fill fill in {theme:?}"));
            (track, fill)
        };

        let overlay_label_pos = find_text_center(&out, |text| text == "Overlay opacity");
        let canvas_label_pos = find_text_center(&out, |text| text == "Canvas opacity");
        let ((overlay_track, overlay_track_stroke), (overlay_fill, overlay_fill_stroke)) =
            find_level_box(
                &out,
                overlay_label_pos.y,
                tokens.control_bg,
                tokens.soft_accent_fill(false, true),
                "Overlay opacity",
            );
        assert_eq!(overlay_track_stroke, Stroke::NONE);
        assert_eq!(overlay_fill_stroke, Stroke::NONE);
        let expected_fraction = (app.background_opacity - 0.38) / 0.62;
        assert!((overlay_fill.width() - 220.0 * expected_fraction).abs() <= 1.0);
        let overlay_pct_pos = find_text_center(&out, |text| text == "81%");
        assert!(overlay_track.contains(overlay_pct_pos));
        assert!((overlay_pct_pos.x - overlay_track.center().x).abs() <= 2.0);
        assert!(!out.shapes.iter().any(|clipped| match &clipped.shape {
            egui::Shape::Text(text_shape) => {
                text_shape.galley.text() == "81%"
                    && clipped.shape.visual_bounding_rect().left() > overlay_track.right()
            }
            _ => false,
        }));
        assert!(!out.shapes.iter().any(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect_shape) => {
                (rect_shape.rect.center().y - overlay_label_pos.y).abs() <= 2.0
                    && (rect_shape.rect.width() - rect_shape.rect.height()).abs() <= 0.5
                    && rect_shape.rect.height() >= 10.0
                    && rect_shape.rect.height() <= 18.0
            }
            _ => false,
        }));

        let ((canvas_track, canvas_track_stroke), (canvas_fill, _)) = find_level_box(
            &out,
            canvas_label_pos.y,
            tokens.control_bg,
            tokens.soft_accent_fill(false, true),
            "Canvas opacity",
        );
        assert_eq!(canvas_track_stroke, Stroke::NONE);
        assert!((canvas_fill.width() - 220.0).abs() <= 0.5);
        let canvas_pct_pos = find_text_center(&out, |text| text == "100%");
        assert!(canvas_track.contains(canvas_pct_pos));
        assert!((canvas_pct_pos.x - canvas_track.center().x).abs() <= 2.0);

        // Hovering updates track and fill tones without painting a border stroke.
        let hovered_out = run_ui_pass_frame(
            &ctx,
            &mut app,
            vec![egui::Event::PointerMoved(overlay_track.center())],
        );
        let ((_, hovered_track_stroke), (_, hovered_fill_stroke)) = find_level_box(
            &hovered_out,
            overlay_label_pos.y,
            tokens.control_hover_bg,
            tokens.soft_accent_fill(true, true),
            "hovered Overlay opacity",
        );
        assert_eq!(hovered_track_stroke, Stroke::NONE);
        assert_eq!(hovered_fill_stroke, Stroke::NONE);

        // Mouse-wheel scrolling over Settings opacity bars never alters opacity values.
        let _ = run_ui_pass_frame(
            &ctx,
            &mut app,
            vec![
                egui::Event::PointerMoved(overlay_track.center()),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Line,
                    delta: Vec2::new(0.0, 1.0),
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Unknown,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        assert!((app.background_opacity - 0.81).abs() < 1e-6);
        let _ = run_ui_pass_frame(
            &ctx,
            &mut app,
            vec![
                egui::Event::PointerMoved(canvas_track.center()),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Line,
                    delta: Vec2::new(0.0, -1.0),
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Unknown,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        assert!((app.canvas_opacity - 1.0).abs() < 1e-6);

        // Horizontal dragging updates Overlay opacity across 38%..=100% cleanly.
        let drag_start = overlay_track.center();
        let drag_end = Pos2::new(drag_start.x - 40.0, drag_start.y);
        let _ = run_ui_pass_frame(
            &ctx,
            &mut app,
            vec![
                egui::Event::PointerMoved(drag_start),
                egui::Event::PointerButton {
                    pos: drag_start,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        let _ = run_ui_pass_frame(&ctx, &mut app, vec![egui::Event::PointerMoved(drag_end)]);
        let _ = run_ui_pass_frame(
            &ctx,
            &mut app,
            vec![egui::Event::PointerButton {
                pos: drag_end,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(app.background_opacity < 0.81 && app.background_opacity >= 0.38);

        // Clicking enters text-edit focus with a 1px accent border and accepts numeric entry.
        let _ = run_ui_pass_frame(&ctx, &mut app, click_events(overlay_track.center()));
        let focused_out = run_ui_pass_frame(&ctx, &mut app, vec![]);
        assert!(focused_out
            .shapes
            .iter()
            .any(|clipped| match &clipped.shape {
                egui::Shape::Rect(rect_shape) => {
                    rect_shape.rect.contains(overlay_track.center())
                        && (rect_shape.rect.width() - 220.0).abs() <= 1.0
                        && rect_shape.stroke == Stroke::new(1.0, tokens.accent)
                }
                _ => false,
            }));
        let _ = run_ui_pass_frame(
            &ctx,
            &mut app,
            vec![
                egui::Event::Text("50".into()),
                egui::Event::Key {
                    key: egui::Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        assert!((app.background_opacity - 0.50).abs() < 1e-6);
    }
}

#[test]
fn batch_cards_and_empty_state_span_full_central_panel_width() {
    let ctx = egui::Context::default();
    ThemeTokens::apply_style(&ctx);
    ctx.set_theme(egui::Theme::Dark);
    let tokens = ThemeTokens::for_mode(true);
    let mut app = super::helpers::app();
    app.tab = 1;

    for width in [MIN_WINDOW_SIZE[0], 1400.0] {
        let size = Vec2::new(width, MIN_WINDOW_SIZE[1]);
        let expected_width = width - 2.0 * theme::SPACE_LG;

        // Empty state: only controls card renders (no empty drop-zone frame).
        app.files.clear();
        app.batch_logs.clear();
        let _ = run_ui_pass_sized(&ctx, &mut app, size);
        let out = run_ui_pass_sized(&ctx, &mut app, size);
        assert!(!out.shapes.iter().any(|clipped| match &clipped.shape {
            egui::Shape::Text(text_shape) => text_shape.galley.text() == "No .docx files queued",
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
        app.files = vec![batch::BatchItem::file(PathBuf::from(
            "/tmp/sample_document.docx",
        ))];
        app.batch_logs = vec!["Ready".into()];
        let _ = run_ui_pass_sized(&ctx, &mut app, size);
        let out = run_ui_pass_sized(&ctx, &mut app, size);
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
fn batch_folder_collapse_releases_the_scrollbar_gutter() {
    let ctx = egui::Context::default();
    ThemeTokens::apply_style(&ctx);
    let mut app = super::helpers::app();
    let directory = tempfile::tempdir().unwrap();
    let folder = directory.path().join("documents");
    std::fs::create_dir(&folder).unwrap();
    for index in 0..24 {
        std::fs::write(folder.join(format!("document-{index}.docx")), []).unwrap();
    }
    app.tab = 1;
    app.add_files(vec![folder.clone()]);
    let track_color = ThemeTokens::for_mode(ctx.theme() == egui::Theme::Dark).extreme_bg;
    for _ in 0..10 {
        run_ui_pass_frame(&ctx, &mut app, vec![]);
    }
    let expanded = run_ui_pass_frame(&ctx, &mut app, vec![]);
    assert_eq!(scroll_tracks(&expanded, track_color, 8.0).len(), 1);

    app.batch_collapsed_dirs
        .insert(format!("root:{}", folder.display()));
    for _ in 0..10 {
        run_ui_pass_frame(&ctx, &mut app, vec![]);
    }
    let collapsed = run_ui_pass_frame(&ctx, &mut app, vec![]);
    assert!(scroll_tracks(&collapsed, track_color, 8.0).is_empty());

    app.batch_selection_mode = batch::BatchSelectionMode::FlatList;
    for _ in 0..10 {
        run_ui_pass_frame(&ctx, &mut app, vec![]);
    }
    let flat = run_ui_pass_frame(&ctx, &mut app, vec![]);
    assert_eq!(scroll_tracks(&flat, track_color, 8.0).len(), 1);
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
    let mut app = super::helpers::app();
    app.tab = 1;
    app.batch_target = 1; // "mathtype"
    app.add_files(vec![folder.clone(), nested_doc.clone(), extra_doc.clone()]);

    assert_eq!(app.files.len(), 3);
    assert_eq!(app.files[0].input, nested_doc);
    assert_eq!(app.files[0].source_root.as_deref(), Some(folder.as_path()));
    assert_eq!(app.files[1].input, root_doc);
    assert_eq!(app.files[1].source_root.as_deref(), Some(folder.as_path()));
    assert_eq!(app.files[2].input, extra_doc);
    assert_eq!(app.files[2].source_root, None);

    let out = run_ui_pass_frame(&ctx, &mut app, vec![]);
    let add_folder_pos = find_text_center(&out, |text| text == "Add folder…");
    assert!(add_folder_pos.x > 0.0);

    let convert_pos = find_text_center(&out, |text| text == "Convert");
    run_ui_pass_frame(&ctx, &mut app, click_events(convert_pos));

    let deadline = Instant::now() + Duration::from_secs(10);
    while app.batch_running && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
        run_ui_pass_frame(&ctx, &mut app, vec![]);
    }
    assert!(!app.batch_running);

    let nested_out = app.batch_rows[0]
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
        app.batch_rows[2]
            .last_output
            .as_ref()
            .unwrap()
            .root_output_dir,
        None
    );
    assert_eq!(
        app.batch_rows[2].last_output.as_ref().unwrap().file_dir,
        dir.path()
    );

    // Default mode is Folder tree: root folder, subfolder, and standalone file show "Open folder";
    // folder-derived file rows (both under root and under subfolders) and Individual files header do not.
    let tree_out = run_ui_pass_sized(&ctx, &mut app, Vec2::new(1056.0, 900.0));
    let has_label_near_y = |frame: &egui::FullOutput, label: &str, y: f32| {
        frame.shapes.iter().any(|clipped| match &clipped.shape {
            egui::Shape::Text(text_shape) if text_shape.galley.text() == label => {
                (clipped.shape.visual_bounding_rect().center().y - y).abs() <= 14.0
            }
            _ => false,
        })
    };
    assert!(!tree_out.shapes.iter().any(|clipped| match &clipped.shape {
        egui::Shape::Text(text_shape) => {
            text_shape.galley.text() == "Open output folder"
                || text_shape.galley.text() == "Open root output folder"
        }
        _ => false,
    }));
    let root_group_y = find_text_center(&tree_out, |text| text == "course_docs").y;
    let sub_group_y = find_text_center(&tree_out, |text| text == "chapter1").y;
    let tree_nested_y = find_text_center(&tree_out, |text| text == "lesson1.docx").y;
    let tree_intro_y = find_text_center(&tree_out, |text| text == "intro.docx").y;
    let individual_group_y = find_text_center(&tree_out, |text| text == "Individual files").y;
    let tree_solo_y = find_text_center(&tree_out, |text| text.ends_with("standalone.docx")).y;
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
    app.batch_selection_mode = batch::BatchSelectionMode::FlatList;
    let post_out = run_ui_pass_sized(&ctx, &mut app, Vec2::new(1056.0, 900.0));
    let nested_y = find_text_center(&post_out, |text| text.ends_with("lesson1.docx")).y;
    let root_file_y = find_text_center(&post_out, |text| text.ends_with("intro.docx")).y;
    let solo_y = find_text_center(&post_out, |text| text.ends_with("standalone.docx")).y;
    assert!(has_label_near_y(&post_out, "Open folder", nested_y));
    assert!(has_label_near_y(&post_out, "Open folder", root_file_y));
    assert!(has_label_near_y(&post_out, "Open folder", solo_y));
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
    let mut app = super::helpers::app();
    app.tab = 1;
    app.batch_target = 1; // "mathtype"
                          // Add doc_a1 individually first, then add folder_a (should upgrade source_root), then folder_b.
    app.add_files(vec![doc_a1.clone()]);
    assert_eq!(app.files[0].source_root, None);
    app.add_files(vec![folder_a.clone()]);
    assert_eq!(app.files.len(), 2);
    assert_eq!(
        app.files[0].source_root.as_deref(),
        Some(folder_a.as_path())
    );
    app.add_files(vec![folder_b.clone()]);
    assert_eq!(app.files.len(), 3);

    // Uncheck doc_a1 (index 0) in default FolderTree mode; run only doc_a2 (1) and doc_b1 (2).
    app.batch_rows[0].set_selected(batch::BatchSelectionMode::FolderTree, false);
    assert_eq!(app.selected_batch_count(), 2);

    let out = run_ui_pass_frame(&ctx, &mut app, vec![]);
    let convert_pos = find_text_center(&out, |text| text == "Convert");
    run_ui_pass_frame(&ctx, &mut app, click_events(convert_pos));

    let deadline = Instant::now() + Duration::from_secs(10);
    while app.batch_running && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
        run_ui_pass_frame(&ctx, &mut app, vec![]);
    }
    assert!(!app.batch_running);
    assert_eq!(app.batch_run_total, 2);
    assert_eq!(app.batch_completed, 2);
    assert_eq!(app.batch_rows[0].last_output, None);
    assert_eq!(app.batch_logs[0], "Ready");

    let out_a2 = app.batch_rows[1].last_output.as_ref().expect("a2 output");
    assert_eq!(
        out_a2.file_dir,
        out_a2.root_output_dir.as_ref().unwrap().join("ch1")
    );
    assert_eq!(
        out_a2.root_output_dir.as_ref().unwrap().parent(),
        Some(dir_a.path())
    );

    let out_b1 = app.batch_rows[2].last_output.as_ref().expect("b1 output");
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

    let mut app = super::helpers::app();
    app.tab = 1;
    let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
    let empty_out = run_ui_pass_frame(&ctx, &mut app, vec![]);

    // Toolbar sections exist; global "Open output folder" button is removed.
    assert!(find_text_center(&empty_out, |text| text == "Input").x > 0.0);
    assert!(find_text_center(&empty_out, |text| text == "Options & Actions").x > 0.0);
    assert!(
        !empty_out.shapes.iter().any(|clipped| match &clipped.shape {
            egui::Shape::Text(text_shape) => text_shape.galley.text() == "Open output folder",
            _ => false,
        })
    );

    // Disabled Convert, Stop, and Clear list share identical text color.
    let text_color_for = |out: &egui::FullOutput, label: &str| {
        out.shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text_shape) if text_shape.galley.text() == label => Some(
                    text_shape
                        .override_text_color
                        .unwrap_or(text_shape.fallback_color),
                ),
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
    app.files = vec![batch::BatchItem::file(PathBuf::from("/tmp/doc1.docx"))];
    app.sync_batch_row_state();
    app.batch_rows[0].set_selected(batch::BatchSelectionMode::FolderTree, false);
    let unchecked_out = run_ui_pass_frame(&ctx, &mut app, vec![]);
    assert_eq!(
        text_color_for(&unchecked_out, "Clear list"),
        tokens.text_primary
    );
    assert_eq!(
        text_color_for(&unchecked_out, "Convert"),
        expected_disabled_text
    );

    // Clicking Clear list clears everything; re-selecting enables Convert.
    app.batch_rows[0].set_selected(batch::BatchSelectionMode::FolderTree, true);
    let ready_out = run_ui_pass_frame(&ctx, &mut app, vec![]);
    assert_eq!(text_color_for(&ready_out, "Convert"), tokens.primary_fg);
    let clear_pos = find_text_center(&ready_out, |text| text == "Clear list");
    let _ = run_ui_pass_frame(&ctx, &mut app, click_events(clear_pos));
    assert!(app.files.is_empty());
    assert!(app.batch_rows.is_empty());
}

#[test]
fn missing_or_corrupt_background_falls_back_gracefully_and_preserves_state() {
    let dir = tempfile::tempdir().unwrap();
    let settings_path = dir.path().join("settings.json");
    Settings {
        zoom: 2.75,
        background_image: Some("does-not-exist.png".into()),
        ..Settings::default()
    }
    .save_to_path(&settings_path)
    .unwrap();

    let ctx = egui::Context::default();
    ThemeTokens::apply_style(&ctx);
    let mut app = super::helpers::app().with_settings_path(settings_path.clone(), Some(&ctx));
    ctx.set_theme(egui::Theme::Light);
    let out = run_ui_pass_frame(&ctx, &mut app, vec![]);
    assert!(app.background_texture.is_none());
    assert_eq!(app.zoom, 2.75);
    let opaque_tokens = ThemeTokens::for_mode(false);
    assert!(out.shapes.iter().any(|clipped| match &clipped.shape {
        egui::Shape::Rect(rect_shape) => rect_shape.fill == opaque_tokens.panel_bg,
        _ => false,
    }));

    // Importing a valid image, then attempting to import a corrupt image preserves the working background.
    let valid_png = dir.path().join("ok.png");
    let img = image::RgbaImage::from_pixel(32, 32, image::Rgba([10, 20, 30, 255]));
    image::DynamicImage::ImageRgba8(img)
        .save_with_format(&valid_png, image::ImageFormat::Png)
        .unwrap();
    app.set_background_image_from_path(&valid_png, &ctx);
    let active_name = app.background_image.clone().expect("active background");
    assert!(app.background_texture.is_some());

    let corrupt = dir.path().join("bad.webp");
    std::fs::write(&corrupt, b"corrupt image bytes").unwrap();
    app.tx.send(Event::BackgroundImage(corrupt)).unwrap();
    let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
    assert_eq!(app.background_image.as_deref(), Some(active_name.as_str()));
    assert!(app.background_texture.is_some());
    assert!(app.background_error.is_some());
}

#[test]
fn settings_app_background_controls_persist_opacity_and_clear_managed_file() {
    let dir = tempfile::tempdir().unwrap();
    let settings_path = dir.path().join("settings.json");
    let img_path = dir.path().join("bg.jpg");
    let img = image::RgbaImage::from_pixel(80, 60, image::Rgba([90, 130, 170, 255]));
    image::DynamicImage::ImageRgba8(img)
        .to_rgb8()
        .save_with_format(&img_path, image::ImageFormat::Jpeg)
        .unwrap();

    let ctx = egui::Context::default();
    ThemeTokens::apply_style(&ctx);
    let mut app = super::helpers::app().with_settings_path(settings_path.clone(), Some(&ctx));
    app.tab = 2;
    app.set_background_image_from_path(&img_path, &ctx);
    let managed_name = app.background_image.clone().unwrap();
    let managed_path =
        settings::resolve_managed_background_path(&settings_path, &managed_name).unwrap();
    assert!(managed_path.exists());

    // Adjust overlay and canvas opacity via Settings UI / persisted state.
    assert_eq!(app.canvas_opacity, settings::DEFAULT_CANVAS_OPACITY);
    app.background_opacity = 0.45;
    app.persisted.background_opacity = 0.45;
    app.canvas_opacity = 0.62;
    app.persisted.canvas_opacity = 0.62;
    app.settings_dirty = true;
    let out = run_ui_pass_sized(&ctx, &mut app, Vec2::new(MIN_WINDOW_SIZE[0], 900.0));
    let on_disk = Settings::load_from_path(&settings_path);
    assert!((on_disk.background_opacity - 0.45).abs() < 1e-6);
    assert!((on_disk.canvas_opacity - 0.62).abs() < 1e-6);
    assert!(find_text_center(&out, |text| text == "App background").x > 0.0);
    let overlay_pos = find_text_center(&out, |text| text == "Overlay opacity");
    let canvas_pos = find_text_center(&out, |text| text == "Canvas opacity");
    assert!(overlay_pos.x > 0.0);
    assert!(canvas_pos.x > 0.0);
    assert!(
        canvas_pos.y > overlay_pos.y,
        "Canvas opacity ({}) should render below Overlay opacity ({})",
        canvas_pos.y,
        overlay_pos.y
    );
    assert!(find_text_center(&out, |text| text == "45%").x > 0.0);
    assert!(find_text_center(&out, |text| text == "62%").x > 0.0);

    // No filename or status text is rendered beside the thumbnail.
    assert!(!out.shapes.iter().any(|clipped| match &clipped.shape {
        egui::Shape::Text(text_shape) => {
            let txt = text_shape.galley.text();
            txt.starts_with("Active:")
                || txt.starts_with("Saved (Solid mode):")
                || txt == "Solid background"
                || txt.contains(&managed_name)
        }
        _ => false,
    }));

    // Thumbnail, Choose image…, and Clear are vertically centered on the same row.
    let thumb_rect = out
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect_shape)
                if (rect_shape.rect.width() - 72.0).abs() <= 1.0
                    && (rect_shape.rect.height() - 48.0).abs() <= 1.0 =>
            {
                Some(rect_shape.rect)
            }
            _ => None,
        })
        .expect("background thumbnail rect");
    let button_rect_for = |label: &str| {
        let text_pos = find_text_center(&out, |text| text == label);
        out.shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(rect_shape)
                    if rect_shape.rect.contains(text_pos)
                        && rect_shape.corner_radius == theme::CONTROL_CORNER_RADIUS =>
                {
                    Some(rect_shape.rect)
                }
                _ => None,
            })
            .min_by(|first, second| first.width().total_cmp(&second.width()))
            .unwrap_or_else(|| panic!("missing button rect for {label}"))
    };
    let choose_btn_rect = button_rect_for("Choose image…");
    let clear_btn_rect = button_rect_for("Clear");
    assert!(
        (choose_btn_rect.center().y - thumb_rect.center().y).abs() <= 1.0,
        "Choose image button center y {} != thumbnail center y {}",
        choose_btn_rect.center().y,
        thumb_rect.center().y
    );
    assert!(
        (clear_btn_rect.center().y - thumb_rect.center().y).abs() <= 1.0,
        "Clear button center y {} != thumbnail center y {}",
        clear_btn_rect.center().y,
        thumb_rect.center().y
    );

    // Click "Clear" button in Settings.
    let clear_pos = find_text_center(&out, |text| text == "Clear");
    let _ = run_ui_pass_sized_events(
        &ctx,
        &mut app,
        Vec2::new(MIN_WINDOW_SIZE[0], 900.0),
        click_events(clear_pos),
    );
    assert_eq!(app.background_image, None);
    assert!(app.background_texture.is_none());
    assert!(!managed_path.exists());
    let after_clear = Settings::load_from_path(&settings_path);
    assert_eq!(after_clear.background_image, None);
}

#[test]
fn async_background_import_events_apply_and_discard_stale_results() {
    let dir = tempfile::tempdir().unwrap();
    let settings_path = dir.path().join("settings.json");
    let img_path = dir.path().join("async-bg.png");
    let img = image::RgbaImage::from_pixel(64, 48, image::Rgba([45, 95, 160, 255]));
    image::DynamicImage::ImageRgba8(img)
        .save_with_format(&img_path, image::ImageFormat::Png)
        .unwrap();

    let ctx = egui::Context::default();
    ThemeTokens::apply_style(&ctx);
    let mut app = super::helpers::app().with_settings_path(settings_path.clone(), Some(&ctx));
    app.tab = 2;

    // Current sequence displays importing indicator in the bottom status bar and applies result upon completion.
    app.background_import_seq = 1;
    app.tx.send(Event::BackgroundImportStarted(1)).unwrap();
    let out = run_ui_pass_frame(&ctx, &mut app, vec![]);
    assert!(app.background_importing);
    let importing_pos = find_text_center(&out, |text| text == "Importing background image…");
    assert!(
        importing_pos.y >= MIN_WINDOW_SIZE[1] - 32.0,
        "Importing indicator y={} should render in the bottom status bar",
        importing_pos.y
    );

    let prepared = settings::prepare_background_import(&img_path, Some(&settings_path)).unwrap();
    let stale_path =
        settings::resolve_managed_background_path(&settings_path, &prepared.filename).unwrap();
    assert!(stale_path.exists());

    // Clearing while an import is in flight supersedes seq=1 so late result is discarded and cleaned up.
    let clear_pos = find_text_center(&out, |text| text == "Clear");
    let _ = run_ui_pass_frame(&ctx, &mut app, click_events(clear_pos));
    assert!(!app.background_importing);

    app.tx
        .send(Event::BackgroundImported(1, Ok(prepared)))
        .unwrap();
    let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
    assert_eq!(app.background_image, None);
    assert!(app.background_texture.is_none());
    assert!(!stale_path.exists());

    // Matching sequence applies immediately.
    let seq = app.background_import_seq + 1;
    app.background_import_seq = seq;
    let fresh = settings::prepare_background_import(&img_path, Some(&settings_path)).unwrap();
    let fresh_name = fresh.filename.clone();
    app.tx.send(Event::BackgroundImportStarted(seq)).unwrap();
    app.tx
        .send(Event::BackgroundImported(seq, Ok(fresh)))
        .unwrap();
    let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
    assert!(!app.background_importing);
    assert_eq!(app.background_image.as_deref(), Some(fresh_name.as_str()));
    assert!(app.background_texture.is_some());
}

#[test]
fn settings_background_mode_toggle_is_non_destructive_and_gates_accent_checkbox() {
    let dir = tempfile::tempdir().unwrap();
    let settings_path = dir.path().join("settings.json");
    let img_path = dir.path().join("azure-bg.png");
    let img = image::RgbaImage::from_pixel(72, 48, image::Rgba([20, 150, 230, 255]));
    image::DynamicImage::ImageRgba8(img)
        .save_with_format(&img_path, image::ImageFormat::Png)
        .unwrap();

    let ctx = egui::Context::default();
    ThemeTokens::apply_style(&ctx);
    let mut app = super::helpers::app().with_settings_path(settings_path.clone(), Some(&ctx));
    app.native_software_background = true;
    app.tab = 2;

    // Without an imported background, the accent checkbox is visible and disabled in Solid mode.
    let min_out = run_ui_pass_sized(&ctx, &mut app, Vec2::from(MIN_WINDOW_SIZE));
    assert!(find_text_center(&min_out, |text| text == "Solid").x > 0.0);
    assert!(find_text_center(&min_out, |text| text == "Image").x > 0.0);
    let checkbox_label = "Match accent color to background";
    let disabled_cb_pos = find_text_center(&min_out, |text| text == checkbox_label);
    let _ = run_ui_pass_sized_events(
        &ctx,
        &mut app,
        Vec2::from(MIN_WINDOW_SIZE),
        click_events(disabled_cb_pos),
    );
    assert!(app.background_accent_enabled);

    // Import an image: switches to Image mode, extracts chromatic accent, and persists.
    app.set_background_image_from_path(&img_path, &ctx);
    let managed_name = app
        .background_image
        .clone()
        .expect("managed background name");
    let managed_path =
        settings::resolve_managed_background_path(&settings_path, &managed_name).unwrap();
    assert!(managed_path.exists());
    assert_eq!(app.background_mode, BackgroundMode::Image);
    let extracted_seed = app.background_accent.expect("extracted background accent");
    assert_eq!(app.active_accent_seed(), Some(extracted_seed));
    assert!(app.software_background(&egui::Visuals::dark()).is_some());

    // Switch to Solid via Settings button: non-destructive (managed file + accent seed preserved).
    let tall = Vec2::new(MIN_WINDOW_SIZE[0], 900.0);
    let image_out = run_ui_pass_sized(&ctx, &mut app, tall);
    let solid_pos = find_text_center(&image_out, |text| text == "Solid");
    let solid_out = run_ui_pass_sized_events(&ctx, &mut app, tall, click_events(solid_pos));
    assert_eq!(app.background_mode, BackgroundMode::Solid);
    assert_eq!(app.background_image.as_deref(), Some(managed_name.as_str()));
    assert_eq!(app.background_accent, Some(extracted_seed));
    assert_eq!(app.active_accent_seed(), None);
    assert!(managed_path.exists());
    assert!(app.software_background(&egui::Visuals::dark()).is_none());
    assert!(
        !solid_out.shapes.iter().any(|clipped| match &clipped.shape {
            egui::Shape::Text(text_shape) => {
                let txt = text_shape.galley.text();
                txt.starts_with("Saved (Solid mode):") || txt.starts_with("Active:")
            }
            _ => false,
        })
    );
    // Disabled Overlay opacity level-fill box stays crisp with muted track/fill contrast.
    let opacity_y = find_text_center(&solid_out, |text| text == "Overlay opacity").y;
    let solid_tokens = ThemeTokens::for_mode(ctx.theme() == egui::Theme::Dark);
    let find_level_track =
        |frame: &egui::FullOutput, row_y: f32, expected_fill: egui::Color32, name: &str| {
            frame
                .shapes
                .iter()
                .find_map(|clipped| match &clipped.shape {
                    egui::Shape::Rect(rect_shape)
                        if rect_shape.fill == expected_fill
                            && (rect_shape.rect.center().y - row_y).abs() <= 2.0
                            && (rect_shape.rect.width() - 220.0).abs() <= 0.5
                            && (rect_shape.rect.height() - theme::BUTTON_HEIGHT).abs() <= 0.5
                            && rect_shape.corner_radius == theme::CONTROL_CORNER_RADIUS =>
                    {
                        Some((rect_shape.rect, rect_shape.stroke))
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("missing {name}"))
        };
    let (disabled_overlay_track, disabled_overlay_stroke) = find_level_track(
        &solid_out,
        opacity_y,
        solid_tokens.control_disabled_bg,
        "disabled Overlay opacity track",
    );
    assert_eq!(disabled_overlay_stroke, Stroke::NONE);
    assert!(solid_out.shapes.iter().any(|clipped| matches!(
        &clipped.shape,
        egui::Shape::Rect(rect_shape)
            if rect_shape.fill == solid_tokens.soft_accent_fill(false, false)
                && (rect_shape.rect.left() - disabled_overlay_track.left()).abs() <= 0.5
                && (rect_shape.rect.center().y - opacity_y).abs() <= 2.0
    )));
    assert!(solid_out.shapes.iter().any(|clipped| matches!(
        &clipped.shape,
        egui::Shape::Rect(rect_shape)
            if (rect_shape.rect.width() - 72.0).abs() <= 0.5
                && (rect_shape.rect.height() - 48.0).abs() <= 0.5
                && rect_shape.stroke == Stroke::NONE
    )));

    // Disabled Canvas opacity level-fill box also stays muted below Overlay opacity and ignores input.
    let canvas_opacity_pos = find_text_center(&solid_out, |text| text == "Canvas opacity");
    assert!(canvas_opacity_pos.y > opacity_y);
    let (disabled_canvas_track, disabled_canvas_stroke) = find_level_track(
        &solid_out,
        canvas_opacity_pos.y,
        solid_tokens.control_disabled_bg,
        "disabled Canvas opacity track",
    );
    assert_eq!(disabled_canvas_stroke, Stroke::NONE);
    assert!(solid_out.shapes.iter().any(|clipped| matches!(
        &clipped.shape,
        egui::Shape::Rect(rect_shape)
            if rect_shape.fill == solid_tokens.soft_accent_fill(false, false)
                && (rect_shape.rect.left() - disabled_canvas_track.left()).abs() <= 0.5
                && (rect_shape.rect.center().y - canvas_opacity_pos.y).abs() <= 2.0
    )));
    let _ = run_ui_pass_sized_events(
        &ctx,
        &mut app,
        tall,
        click_events(Pos2::new(
            disabled_canvas_track.center().x - 40.0,
            canvas_opacity_pos.y,
        )),
    );
    let _ = run_ui_pass_sized_events(
        &ctx,
        &mut app,
        tall,
        vec![
            egui::Event::PointerMoved(disabled_canvas_track.center()),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: Vec2::new(0.0, -1.0),
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Unknown,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    assert_eq!(app.canvas_opacity, settings::DEFAULT_CANVAS_OPACITY);

    let saved_solid = Settings::load_from_path(&settings_path);
    assert_eq!(saved_solid.background_mode, BackgroundMode::Solid);
    assert_eq!(
        saved_solid.background_image.as_deref(),
        Some(managed_name.as_str())
    );
    assert_eq!(saved_solid.background_accent, Some(extracted_seed));

    // Clicking the disabled accent checkbox in Solid mode leaves it unchanged.
    let cb_pos_solid = find_text_center(&solid_out, |text| text == checkbox_label);
    let _ = run_ui_pass_sized_events(&ctx, &mut app, tall, click_events(cb_pos_solid));
    assert!(app.background_accent_enabled);

    // Switch back to Image via Settings button: immediately restores image + extracted accent.
    let img_btn_pos = find_text_center(&solid_out, |text| text == "Image");
    let back_to_img_out = run_ui_pass_sized_events(&ctx, &mut app, tall, click_events(img_btn_pos));
    // Dragging the enabled Canvas opacity level-fill box updates state and writes to disk.
    let image_tokens = ThemeTokens::for_mode_with_background_and_accent(
        ctx.theme() == egui::Theme::Dark,
        true,
        app.background_opacity,
        Some(extracted_seed),
    );
    let canvas_row_y = find_text_center(&back_to_img_out, |text| text == "Canvas opacity").y;
    let (enabled_canvas_track, _) = find_level_track(
        &back_to_img_out,
        canvas_row_y,
        image_tokens.control_bg,
        "enabled Canvas opacity track",
    );
    let enabled_canvas_center = enabled_canvas_track.center();
    let drag_end = Pos2::new(enabled_canvas_center.x - 80.0, canvas_row_y);
    let _ = run_ui_pass_sized_events(
        &ctx,
        &mut app,
        tall,
        vec![
            egui::Event::PointerMoved(enabled_canvas_center),
            egui::Event::PointerButton {
                pos: enabled_canvas_center,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    let _ = run_ui_pass_sized_events(
        &ctx,
        &mut app,
        tall,
        vec![egui::Event::PointerMoved(drag_end)],
    );
    let _ = run_ui_pass_sized_events(
        &ctx,
        &mut app,
        tall,
        vec![egui::Event::PointerButton {
            pos: drag_end,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert!(app.canvas_opacity < 0.95);
    assert!(
        (Settings::load_from_path(&settings_path).canvas_opacity - app.canvas_opacity).abs() < 1e-6
    );
    assert_eq!(app.background_mode, BackgroundMode::Image);
    assert_eq!(app.active_accent_seed(), Some(extracted_seed));
    assert!(app.software_background(&egui::Visuals::dark()).is_some());

    // Unchecking and re-checking the accent checkbox toggles active_accent_seed and persists.
    let cb_pos_img = find_text_center(&back_to_img_out, |text| text == checkbox_label);
    let unchecked_out = run_ui_pass_sized_events(&ctx, &mut app, tall, click_events(cb_pos_img));
    assert!(!app.background_accent_enabled);
    assert_eq!(app.active_accent_seed(), None);
    assert!(!Settings::load_from_path(&settings_path).background_accent_enabled);

    let cb_pos_recheck = find_text_center(&unchecked_out, |text| text == checkbox_label);
    let _ = run_ui_pass_sized_events(&ctx, &mut app, tall, click_events(cb_pos_recheck));
    assert!(app.background_accent_enabled);
    assert_eq!(app.active_accent_seed(), Some(extracted_seed));
    assert!(Settings::load_from_path(&settings_path).background_accent_enabled);

    // Clicking Clear removes the managed PNG, clears accent seed, and resets mode to Solid.
    let clear_pos = find_text_center(&unchecked_out, |text| text == "Clear");
    let _ = run_ui_pass_sized_events(&ctx, &mut app, tall, click_events(clear_pos));
    assert_eq!(app.background_mode, BackgroundMode::Solid);
    assert_eq!(app.background_image, None);
    assert_eq!(app.background_accent, None);
    assert!(!managed_path.exists());
    let cleared = Settings::load_from_path(&settings_path);
    assert_eq!(cleared.background_mode, BackgroundMode::Solid);
    assert_eq!(cleared.background_image, None);
    assert_eq!(cleared.background_accent, None);
}

#[test]
fn batch_progress_bar_combobox_and_checkboxes_use_active_accent() {
    use eframe::egui::Color32;

    fn extract_text_color(text_shape: &egui::epaint::TextShape) -> Color32 {
        text_shape
            .override_text_color
            .filter(|col| *col != Color32::PLACEHOLDER)
            .or_else(|| {
                text_shape
                    .galley
                    .job
                    .sections
                    .first()
                    .map(|section| section.format.color)
                    .filter(|col| *col != Color32::PLACEHOLDER)
            })
            .unwrap_or(text_shape.fallback_color)
    }

    fn text_colors_for(out: &egui::FullOutput, needle: &str) -> Vec<(Color32, Pos2)> {
        out.shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text_shape) if text_shape.galley.text() == needle => Some((
                    extract_text_color(text_shape),
                    clipped.shape.visual_bounding_rect().center(),
                )),
                _ => None,
            })
            .collect()
    }

    fn checkbox_box_near_y(
        out: &egui::FullOutput,
        row_y: f32,
        max_x: f32,
    ) -> (Rect, Color32, Stroke) {
        out.shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(rect_shape)
                    if rect_shape.brush.is_none()
                        && rect_shape.stroke.width > 0.0
                        && (rect_shape.rect.center().y - row_y).abs() <= 4.0
                        && rect_shape.rect.right() <= max_x + 4.0
                        && (rect_shape.rect.width() - rect_shape.rect.height()).abs() <= 0.5
                        && rect_shape.rect.height() >= 10.0
                        && rect_shape.rect.height() <= 22.0 =>
                {
                    Some((rect_shape.rect, rect_shape.fill, rect_shape.stroke))
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing checkbox square near y={row_y}"))
    }

    let dir = tempfile::tempdir().unwrap();
    let settings_path = dir.path().join("settings.json");
    let img_path = dir.path().join("cyan-bg.png");
    let img = image::RgbaImage::from_pixel(64, 48, image::Rgba([20, 150, 230, 255]));
    image::DynamicImage::ImageRgba8(img)
        .save_with_format(&img_path, image::ImageFormat::Png)
        .unwrap();
    let folder = dir.path().join("course");
    std::fs::create_dir_all(&folder).unwrap();
    let doc_one = folder.join("one.docx");
    let doc_two = folder.join("two.docx");

    let ctx = egui::Context::default();
    ThemeTokens::apply_style(&ctx);
    let mut app = super::helpers::app().with_settings_path(settings_path, Some(&ctx));
    app.set_background_image_from_path(&img_path, &ctx);
    let seed = app.background_accent.expect("cyan accent seed");
    app.files = vec![
        batch::BatchItem::from_folder(doc_one, folder.clone()),
        batch::BatchItem::from_folder(doc_two, folder),
    ];
    app.sync_batch_row_state();
    app.batch_rows[1].set_selected(batch::BatchSelectionMode::FolderTree, false);
    app.batch_overwrite = true;

    let size = Vec2::new(1056.0, 900.0);
    let mut frame_time = 0.0_f64;
    let mut run_frame = |app: &mut App, events: Vec<egui::Event>| {
        frame_time += 1.0;
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                time: Some(frame_time),
                events,
                ..Default::default()
            },
            |ui| app.draw_ui(ui),
        );
        out.textures_delta.clear();
        out
    };

    for (mode, use_adaptive) in [
        (BackgroundMode::Solid, false),
        (BackgroundMode::Image, true),
    ] {
        app.background_mode = mode;
        app.background_accent_enabled = use_adaptive;
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            let dark_mode = theme == egui::Theme::Dark;
            ctx.set_theme(theme);
            let expected = ThemeTokens::for_mode_with_background_and_accent(
                dark_mode,
                mode == BackgroundMode::Image,
                app.background_opacity,
                if use_adaptive { Some(seed) } else { None },
            );

            // 1. Batch tab: 0% progress bar shows no partial stub; checked and indeterminate boxes use accent.
            app.tab = 1;
            app.batch_run_total = 0;
            app.batch_completed = 0;
            let _ = run_frame(&mut app, vec![]);
            let zero_out = run_frame(&mut app, vec![]);

            let zero_labels = text_colors_for(&zero_out, "0/1 files");
            assert_eq!(zero_labels.len(), 1);
            assert_eq!(zero_labels[0].0, expected.text_primary);
            let zero_center = zero_labels[0].1;
            assert!(zero_out.shapes.iter().any(|clipped| match &clipped.shape {
                egui::Shape::Rect(rect_shape) => {
                    rect_shape.rect.contains(zero_center)
                        && rect_shape.fill == expected.extreme_bg
                        && rect_shape.stroke == Stroke::NONE
                }
                _ => false,
            }));
            assert!(!zero_out.shapes.iter().any(|clipped| match &clipped.shape {
                egui::Shape::Rect(rect_shape) => {
                    rect_shape.rect.contains(zero_center) && rect_shape.fill == expected.primary_bg
                }
                _ => false,
            }));

            // Checked overwrite checkbox keeps neutral label and accent box.
            let overwrite_label = "Overwrite original documents (replace each selected input)";
            let overwrite_colors = text_colors_for(&zero_out, overwrite_label);
            assert_eq!(overwrite_colors[0].0, expected.text_primary);
            let (overwrite_box, overwrite_fill, overwrite_stroke) =
                checkbox_box_near_y(&zero_out, overwrite_colors[0].1.y, overwrite_colors[0].1.x);
            assert_eq!(overwrite_fill, expected.selected_bg);
            assert_eq!(overwrite_stroke.color, expected.selected_border);
            assert!(zero_out.shapes.iter().any(|clipped| match &clipped.shape {
                egui::Shape::Path(path) => {
                    path.stroke.color == eframe::epaint::ColorMode::Solid(expected.selected_fg)
                        && path.points.iter().any(|pt| overwrite_box.contains(*pt))
                }
                _ => false,
            }));

            // Indeterminate folder group checkbox uses accent fill, border, and horizontal dash.
            let group_y = find_text_center(&zero_out, |text| text == "course").y;
            let group_x = find_text_center(&zero_out, |text| text == "course").x;
            let (indet_box, indet_fill, indet_stroke) =
                checkbox_box_near_y(&zero_out, group_y, group_x);
            assert_eq!(indet_fill, expected.selected_bg);
            assert_eq!(indet_stroke.color, expected.selected_border);
            assert!(zero_out.shapes.iter().any(|clipped| match &clipped.shape {
                egui::Shape::LineSegment { points, stroke } => {
                    stroke.color == expected.selected_fg
                        && indet_box.contains(points[0])
                        && indet_box.contains(points[1])
                }
                _ => false,
            }));

            // 2. Active 1/2 progress bar uses primary_bg fill and split primary_fg / text_primary label.
            app.batch_run_total = 2;
            app.batch_completed = 1;
            let half_out = run_frame(&mut app, vec![]);
            let half_labels = text_colors_for(&half_out, "1/2 files");
            assert!(half_labels
                .iter()
                .any(|(col, _)| *col == expected.primary_fg));
            assert!(half_labels
                .iter()
                .any(|(col, _)| *col == expected.text_primary));
            let track_y = half_labels[0].1.y;
            assert!(half_out.shapes.iter().any(|clipped| match &clipped.shape {
                egui::Shape::Rect(rect_shape) => {
                    rect_shape.fill == expected.primary_bg
                        && (rect_shape.rect.center().y - track_y).abs() <= 4.0
                        && rect_shape.rect.width() > 40.0
                }
                _ => false,
            }));

            // 3. Batch target ComboBox popup highlights the selected row using selected_* tokens.
            let combo_pos = find_text_center(&half_out, |text| text == batch::TARGETS[0].1);
            let _ = run_frame(&mut app, click_events(combo_pos));
            let popup_out = run_frame(&mut app, vec![]);
            let popup_item_colors = text_colors_for(&popup_out, batch::TARGETS[0].1);
            let selected_item = popup_item_colors
                .iter()
                .find(|(col, pos)| {
                    *col == expected.selected_fg && (pos.y - combo_pos.y).abs() > 8.0
                })
                .unwrap_or_else(|| {
                    panic!(
                        "missing selected ComboBox row in {theme:?}/{mode:?}: {popup_item_colors:?}"
                    )
                });
            assert!(popup_out.shapes.iter().any(|clipped| match &clipped.shape {
                egui::Shape::Rect(rect_shape) => {
                    rect_shape.rect.contains(selected_item.1)
                        && rect_shape.fill == expected.selected_bg
                        && rect_shape.stroke == Stroke::NONE
                }
                _ => false,
            }));
            fn has_popup_container(shape: &egui::Shape, point: Pos2, fill: Color32) -> bool {
                match shape {
                    egui::Shape::Rect(rect_shape) => {
                        rect_shape.rect.contains(point)
                            && rect_shape.rect.height() > 30.0
                            && rect_shape.fill == fill
                            && rect_shape.stroke == Stroke::NONE
                    }
                    egui::Shape::Vec(shapes) => shapes
                        .iter()
                        .any(|child| has_popup_container(child, point, fill)),
                    _ => false,
                }
            }
            assert!(popup_out.shapes.iter().any(|clipped| has_popup_container(
                &clipped.shape,
                selected_item.1,
                expected.control_bg
            )));
            // Close ComboBox popup before next pass.
            let _ = run_frame(
                &mut app,
                vec![egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );

            // 4. Settings checkbox "Match accent color to background" distinguishes enabled vs disabled.
            app.tab = 2;
            let _ = run_frame(&mut app, vec![]);
            let settings_out = run_frame(&mut app, vec![]);
            let cb_label = "Match accent color to background";
            let cb_colors = text_colors_for(&settings_out, cb_label);
            let (_, cb_fill, cb_stroke) =
                checkbox_box_near_y(&settings_out, cb_colors[0].1.y, cb_colors[0].1.x);
            if mode == BackgroundMode::Image {
                assert_eq!(cb_colors[0].0, expected.text_primary);
                assert_eq!(cb_fill, expected.selected_bg);
                assert_eq!(cb_stroke.color, expected.selected_border);
            } else {
                assert_eq!(cb_colors[0].0, expected.text_disabled);
                assert_eq!(cb_fill, expected.control_disabled_bg);
                assert_eq!(cb_stroke.color, expected.border);

                // Disabled + checked uses disabled_accent() while keeping control_disabled_bg fill.
                app.background_accent_enabled = true;
                let checked_disabled_out = run_frame(&mut app, vec![]);
                let checked_colors = text_colors_for(&checked_disabled_out, cb_label);
                let (_, checked_fill, checked_stroke) = checkbox_box_near_y(
                    &checked_disabled_out,
                    checked_colors[0].1.y,
                    checked_colors[0].1.x,
                );
                assert_eq!(checked_colors[0].0, expected.text_disabled);
                assert_eq!(checked_fill, expected.control_disabled_bg);
                assert_eq!(checked_stroke.color, expected.disabled_accent());
                app.background_accent_enabled = use_adaptive;
            }
        }
    }
}

#[test]
fn latex_font_pt_ctrl_wheel_persists_and_restores_across_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let ctx = egui::Context::default();
    ThemeTokens::apply_style(&ctx);
    let mut app = super::helpers::app().with_settings_path(path.clone(), Some(&ctx));
    app.latex = "restored-latex".into();

    let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
    let out = run_ui_pass_frame(&ctx, &mut app, vec![]);
    let pal = app.active_canvas_palette(ctx.theme() == egui::Theme::Dark);
    let latex_rect = out
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect_shape)
                if rect_shape.fill == pal.bg
                    && rect_shape.corner_radius == theme::CONTROL_CORNER_RADIUS
                    && rect_shape.stroke == Stroke::NONE
                    && rect_shape.rect.width() > 300.0 =>
            {
                Some(rect_shape.rect)
            }
            _ => None,
        })
        .expect("latex frame");

    for _ in 0..3 {
        let _ = run_ui_pass_frame(
            &ctx,
            &mut app,
            vec![
                egui::Event::PointerMoved(latex_rect.center()),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Line,
                    delta: Vec2::new(0.0, 1.0),
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Unknown,
                    modifiers: egui::Modifiers::CTRL,
                },
            ],
        );
    }
    assert_eq!(app.latex_font_pt, 14.5);
    assert_eq!(Settings::load_from_path(&path).latex_font_pt, 14.5);

    let fresh_ctx = egui::Context::default();
    ThemeTokens::apply_style(&fresh_ctx);
    let mut reloaded = super::helpers::app().with_settings_path(path, Some(&fresh_ctx));
    reloaded.latex = "restored-latex".into();
    assert_eq!(reloaded.latex_font_pt, 14.5);
    let reloaded_out = run_ui_pass_frame(&fresh_ctx, &mut reloaded, vec![]);
    let rendered_font = reloaded_out
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text_shape)
                if text_shape.galley.text().contains("restored-latex") =>
            {
                text_shape
                    .galley
                    .job
                    .sections
                    .first()
                    .map(|section| section.format.font_id.clone())
            }
            _ => None,
        })
        .expect("reloaded latex galley");
    assert_eq!(rendered_font, egui::FontId::monospace(14.5));
}
