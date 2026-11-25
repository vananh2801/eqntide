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

    let isolated = super::helpers::app();
    assert!(isolated.settings_path.is_none());
    assert_eq!(isolated.zoom, settings::DEFAULT_ZOOM);
    assert_eq!(isolated.font_pt, settings::DEFAULT_FONT_PT);

    let ctx = egui::Context::default();
    let restored = super::helpers::app().with_settings_path(path, Some(&ctx));
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
    assert!(!restored.is_dirty());
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
    let scripts_tex = app.palette_atlases["Scripts"].texture.id();
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

    // 5. Open Settings tab, adjust zoom and toggle theme.
    let settings_pos = find_text_center(&out, |text| text == "Settings");
    run_ui_pass_frame(&ctx, &mut app, click_events(settings_pos));
    assert_eq!(app.tab, 2);
    let out = run_ui_pass_frame(&ctx, &mut app, vec![]);

    // Drag zoom slider right while pointer is down, then release.
    let pct_pos = find_text_center(&out, |text| text == "200%");
    let slider_center = out
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect_shape)
                if (rect_shape.rect.center().y - pct_pos.y).abs() < 12.0
                    && rect_shape.rect.width() >= 150.0
                    && rect_shape.rect.width() <= 260.0
                    && rect_shape.rect.height() <= 24.0 =>
            {
                Some(rect_shape.rect.center())
            }
            _ => None,
        })
        .unwrap_or(pct_pos);
    let slider_drag_end = Pos2::new(slider_center.x + 60.0, slider_center.y);
    run_ui_pass_frame(
        &ctx,
        &mut app,
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
        &mut app,
        vec![egui::Event::PointerMoved(slider_drag_end)],
    );
    assert!(app.settings_dirty);
    assert_eq!(Settings::load_from_path(&path).zoom, settings::DEFAULT_ZOOM);
    let out = run_ui_pass_frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerButton {
            pos: slider_drag_end,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert!(!app.settings_dirty);
    let dragged_zoom = app.zoom;
    assert!((dragged_zoom - settings::DEFAULT_ZOOM).abs() > 0.05);
    assert_eq!(Settings::load_from_path(&path).zoom, dragged_zoom);

    // Select Light theme (Dark -> Light).
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

    // 8. Verify "Reset (200%)" persists default zoom back to disk.
    reloaded.tab = 2;
    let out = run_ui_pass_frame(&fresh_ctx, &mut reloaded, vec![]);
    let reset_pos = find_text_center(&out, |text| text == "Reset (200%)");
    run_ui_pass_frame(&fresh_ctx, &mut reloaded, click_events(reset_pos));
    assert_eq!(reloaded.zoom, settings::DEFAULT_ZOOM);
    assert_eq!(Settings::load_from_path(&path).zoom, settings::DEFAULT_ZOOM);
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
        let mut app = super::helpers::app();
        app.tab = 2;

        let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
        let out = run_ui_pass_frame(&ctx, &mut app, vec![]);

        let pct_pos = find_text_center(&out, |text| text == "200%");
        let reset_pos = find_text_center(&out, |text| text == "Reset (200%)");
        let control_rect = |pos: Pos2, label: &str| {
            out.shapes
                .iter()
                .find_map(|clipped| match &clipped.shape {
                    egui::Shape::Rect(rect_shape)
                        if rect_shape.brush.is_none()
                            && rect_shape.rect.contains(pos)
                            && rect_shape.rect.height() <= 40.0 =>
                    {
                        Some(rect_shape.rect)
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
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(rect_shape)
                    if rect_shape.fill == tokens.border
                        && rect_shape.rect.width() >= 150.0
                        && rect_shape.rect.height() <= 12.0
                        && (rect_shape.rect.center().y - pct_rect.center().y).abs() < 12.0 =>
                {
                    Some(rect_shape.rect)
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
                .find_map(|clipped| match &clipped.shape {
                    egui::Shape::Rect(rect_shape)
                        if (rect_shape.rect.center().y - rail_rect.center().y).abs() <= 1.0
                            && (rect_shape.rect.width() - rect_shape.rect.height()).abs()
                                <= 0.5
                            && rect_shape.rect.height() >= 10.0
                            && rect_shape.rect.height() <= 18.0
                            && rect_shape.rect.left() >= rail_rect.left() - 2.0
                            && rect_shape.rect.right() <= rail_rect.right() + 2.0 =>
                    {
                        Some((rect_shape.rect, rect_shape.stroke))
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
            &mut app,
            vec![egui::Event::PointerMoved(idle_handle.center())],
        );
        let (_, hovered_stroke) = find_handle(&hovered_out, "hovered");
        assert_eq!(hovered_stroke.color, tokens.accent);

        // Active/dragged handle preserves accent border color.
        let active_out = run_ui_pass_frame(
            &ctx,
            &mut app,
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
fn batch_dual_selection_modes_preserve_independent_state_and_tree_controls() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("course");
    let sub = folder.join("ch1");
    std::fs::create_dir_all(&sub).unwrap();
    let root_doc = folder.join("intro.docx");
    let sub_doc1 = sub.join("a.docx");
    let sub_doc2 = sub.join("b.docx");
    let solo_doc = dir.path().join("solo.docx");
    for path in [&root_doc, &sub_doc1, &sub_doc2, &solo_doc] {
        std::fs::write(path, b"placeholder").unwrap();
    }

    let ctx = egui::Context::default();
    ThemeTokens::apply_style(&ctx);
    let mut app = super::helpers::app();
    app.tab = 1;
    app.add_files(vec![folder.clone(), solo_doc.clone()]);
    assert_eq!(app.files.len(), 4);
    assert_eq!(
        app.batch_selection_mode,
        batch::BatchSelectionMode::FolderTree
    );
    assert_eq!(app.selected_batch_count(), 4);

    // Default Folder tree layout & header assertions.
    let tree_out = run_ui_pass_sized(&ctx, &mut app, Vec2::new(1056.0, 900.0));
    assert!(!tree_out.shapes.iter().any(|clipped| match &clipped.shape {
        egui::Shape::Text(text_shape) => {
            let txt = text_shape.galley.text();
            txt == "Ready" || txt == "Waiting" || txt == "View:" || txt.starts_with("Selected:")
        }
        _ => false,
    }));
    let progress_pos = find_text_center(&tree_out, |text| text == "0/4 files");
    let deselect_pos = find_text_center(&tree_out, |text| text == "Deselect all");
    let found_pos = find_text_center(&tree_out, |text| text.starts_with("Found:"));
    let flat_pos = find_text_center(&tree_out, |text| text == "File tree");
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
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text_shape) if text_shape.galley.text() == label => {
                    Some(clipped.shape.visual_bounding_rect().left())
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
    assert!(find_text_center(&tree_out, |text| text == "Individual files").x > 0.0);

    // Collapse ch1 by clicking its "-" button.
    let ch1_y = find_text_center(&tree_out, |text| text == "ch1").y;
    let ch1_toggle_pos = tree_out
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text_shape)
                if text_shape.galley.text() == "-"
                    && (clipped.shape.visual_bounding_rect().center().y - ch1_y).abs() <= 14.0 =>
            {
                Some(clipped.shape.visual_bounding_rect().center())
            }
            _ => None,
        })
        .expect("ch1 collapse toggle");
    let _ = run_ui_pass_frame(&ctx, &mut app, click_events(ch1_toggle_pos));
    let collapsed_out = run_ui_pass_sized(&ctx, &mut app, Vec2::new(1056.0, 900.0));
    assert!(!collapsed_out
        .shapes
        .iter()
        .any(|clipped| match &clipped.shape {
            egui::Shape::Text(text_shape) => {
                text_shape.galley.text() == "a.docx" || text_shape.galley.text() == "b.docx"
            }
            _ => false,
        }));

    // Click ch1's parent checkbox to uncheck both files inside ch1 -> course shows 1/3 selected.
    let ch1_row_y = find_text_center(&collapsed_out, |text| text == "ch1").y;
    let toggle_x = ch1_toggle_pos.x;
    // Checkbox sits immediately to the right of the +/- compact button on the ch1 row.
    let ch1_checkbox_pos = Pos2::new(toggle_x + 22.0, ch1_row_y);
    let _ = run_ui_pass_frame(&ctx, &mut app, click_events(ch1_checkbox_pos));
    let after_ch1_uncheck = run_ui_pass_sized(&ctx, &mut app, Vec2::new(1056.0, 900.0));
    assert_eq!(app.selected_batch_count(), 2); // intro.docx + solo.docx
    assert!(find_text_center(&after_ch1_uncheck, |text| text == "1/3 selected").x > 0.0);

    // Click course's indeterminate parent checkbox -> selects all 3 files in course (4 total).
    let course_row_y = find_text_center(&after_ch1_uncheck, |text| text == "course").y;
    let course_toggle_x = after_ch1_uncheck
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text_shape)
                if text_shape.galley.text() == "-"
                    && (clipped.shape.visual_bounding_rect().center().y - course_row_y).abs()
                        <= 14.0 =>
            {
                Some(clipped.shape.visual_bounding_rect().center().x)
            }
            _ => None,
        })
        .expect("course collapse toggle");
    let _ = run_ui_pass_frame(
        &ctx,
        &mut app,
        click_events(Pos2::new(course_toggle_x + 22.0, course_row_y)),
    );
    assert_eq!(app.selected_batch_count(), 4);

    // Uncheck one in Folder tree, switch to File tree -> all 4 still checked in File tree.
    app.batch_rows[1].tree_selected = false;
    assert_eq!(app.selected_batch_count(), 3);
    let flat_btn = find_text_center(&after_ch1_uncheck, |text| text == "File tree");
    let _ = run_ui_pass_frame(&ctx, &mut app, click_events(flat_btn));
    assert_eq!(
        app.batch_selection_mode,
        batch::BatchSelectionMode::FlatList
    );
    assert_eq!(app.selected_batch_count(), 4);

    // Uncheck in File tree via Deselect all, re-enable 2 items, then switch back to Folder tree ->
    // Folder tree still retains its 3 selections!
    let flat_out = run_ui_pass_sized(&ctx, &mut app, Vec2::new(1056.0, 900.0));
    let flat_deselect = find_text_center(&flat_out, |text| text == "Deselect all");
    let _ = run_ui_pass_frame(&ctx, &mut app, click_events(flat_deselect));
    assert_eq!(app.selected_batch_count(), 0);
    assert_eq!(app.files.len(), 4);
    app.batch_rows[0].flat_selected = true;
    app.batch_rows[3].flat_selected = true;
    assert_eq!(app.selected_batch_count(), 2);
    let two_selected_out = run_ui_pass_sized(&ctx, &mut app, Vec2::new(1056.0, 900.0));
    assert!(find_text_center(&two_selected_out, |text| text == "0/2 files").x > 0.0);

    let tree_btn = find_text_center(&two_selected_out, |text| text == "Folder tree");
    let _ = run_ui_pass_frame(&ctx, &mut app, click_events(tree_btn));
    assert_eq!(
        app.batch_selection_mode,
        batch::BatchSelectionMode::FolderTree
    );
    assert_eq!(app.selected_batch_count(), 3);
    assert!(app.batch_rows[0].flat_selected);
    assert!(!app.batch_rows[1].flat_selected);
    assert!(!app.batch_rows[2].flat_selected);
    assert!(app.batch_rows[3].flat_selected);
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

    // Adjust opacity via Settings UI / persisted state.
    app.background_opacity = 0.45;
    app.persisted.background_opacity = 0.45;
    app.settings_dirty = true;
    let out = run_ui_pass_frame(&ctx, &mut app, vec![]);
    let on_disk = Settings::load_from_path(&settings_path);
    assert!((on_disk.background_opacity - 0.45).abs() < 1e-6);
    assert!(find_text_center(&out, |text| text == "App background").x > 0.0);
    assert!(find_text_center(&out, |text| text == "Overlay opacity").x > 0.0);
    assert!(find_text_center(&out, |text| text == "45%").x > 0.0);
    let active_rect = out
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text_shape) if text_shape.galley.text().starts_with("Active:") => {
                Some(clipped.shape.visual_bounding_rect())
            }
            _ => None,
        })
        .expect("Active status text");
    let choose_text_pos = find_text_center(&out, |text| text == "Choose image…");
    let choose_btn_rect = out
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect_shape)
                if rect_shape.rect.contains(choose_text_pos)
                    && rect_shape.corner_radius == theme::CONTROL_CORNER_RADIUS =>
            {
                Some(rect_shape.rect)
            }
            _ => None,
        })
        .min_by(|first, second| first.width().total_cmp(&second.width()))
        .expect("Choose image button rect");
    assert!(
        (active_rect.min.x - choose_btn_rect.min.x).abs() <= 0.5,
        "Active text left {} should align with Choose image button left {}",
        active_rect.min.x,
        choose_btn_rect.min.x
    );

    // Click "Clear" button in Settings.
    let clear_pos = find_text_center(&out, |text| text == "Clear");
    let _ = run_ui_pass_frame(&ctx, &mut app, click_events(clear_pos));
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

    // Current sequence displays importing indicator and applies result upon completion.
    app.background_import_seq = 1;
    app.tx.send(Event::BackgroundImportStarted(1)).unwrap();
    let out = run_ui_pass_frame(&ctx, &mut app, vec![]);
    assert!(app.background_importing);
    assert!(find_text_center(&out, |text| text == "Importing background image…").x > 0.0);

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
