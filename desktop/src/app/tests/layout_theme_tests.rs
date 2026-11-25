use super::super::*;
use super::helpers::*;
use crate::settings::ThemeMode;
use eframe::egui::Color32;
use eqntide::MathNode;

#[test]
fn fixed_layout_keeps_save_and_source_visible_across_sizes_and_themes() {
    for office in [false, true] {
        for size in [
            Vec2::from(MIN_WINDOW_SIZE),
            Vec2::new(1100.0, 650.0),
            Vec2::new(1440.0, 900.0),
        ] {
            for theme in [egui::Theme::Light, egui::Theme::Dark] {
                let mut app = super::helpers::app();
                app.bridge_ready = true;
                app.latex = "source-marker".into();
                if office {
                    app.session = Some("office".into());
                }
                let ctx = egui::Context::default();
                ThemeTokens::apply_style(&ctx);
                ctx.set_theme(theme);
                let screen = Rect::from_min_size(Pos2::ZERO, size);
                // Run twice to cover initial panel sizing and the settled layout.
                for _ in 0..2 {
                    let mut output = ctx.run_ui(
                        egui::RawInput {
                            screen_rect: Some(screen),
                            ..Default::default()
                        },
                        |ui| app.draw_shell(ui),
                    );
                    output.textures_delta.clear();
                    let label = if office {
                        "Save & Return to Document (Ctrl+S)"
                    } else {
                        "Save MathML (Ctrl+S)"
                    };
                    for text in [label, "source-marker", "MathType", "OMML", "Font size"] {
                        let shape = output
                            .shapes
                            .iter()
                            .find(|clipped| {
                                matches!(&clipped.shape, egui::Shape::Text(text_shape)
                                    if text_shape.galley.text() == text)
                            })
                            .unwrap_or_else(|| panic!("missing {text} at {size:?}"));
                        let rect = shape.shape.visual_bounding_rect();
                        if text != "source-marker" {
                            let save = output
                                .shapes
                                .iter()
                                .find(|clipped| {
                                    matches!(&clipped.shape, egui::Shape::Text(text_shape)
                                        if text_shape.galley.text() == label)
                                })
                                .unwrap();
                            assert!(
                                (rect.center().y - save.shape.visual_bounding_rect().center().y)
                                    .abs()
                                    <= 3.0,
                                "footer control {text} must share the save row"
                            );
                        }
                        if text == "source-marker" {
                            let pal = app.active_canvas_palette(theme == egui::Theme::Dark);
                            for painted in &output.shapes {
                                if let egui::Shape::Rect(frame) = &painted.shape {
                                    if frame.fill == pal.bg
                                        && frame.stroke.color == pal.border
                                        && frame.corner_radius == theme::SURFACE_CORNER_RADIUS
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
                    let save_center_y = find_text_center(&output, |text| text == label).y;
                    for icon_name in ["arrow-back-up", "arrow-forward-up"] {
                        let tex_id = app
                            .ui_icon_texture_id(icon_name)
                            .unwrap_or_else(|| panic!("missing footer icon {icon_name}"));
                        let icon_rect = output
                            .shapes
                            .iter()
                            .find_map(|clipped| match &clipped.shape {
                                egui::Shape::Rect(rect_shape)
                                    if rect_shape
                                        .brush
                                        .as_ref()
                                        .is_some_and(|brush| brush.fill_texture_id == tex_id) =>
                                {
                                    Some(rect_shape.rect)
                                }
                                _ => None,
                            })
                            .unwrap_or_else(|| panic!("missing painted footer icon {icon_name}"));
                        assert!(
                            (icon_rect.center().y - save_center_y).abs() <= 3.0,
                            "footer icon {icon_name} must share the save row"
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
        let mut app = super::helpers::app();
        app.category = "Symbols".into();
        for width in [960.0, MIN_WINDOW_SIZE[0], 1100.0] {
            for _ in 0..2 {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 650.0))),
                        ..Default::default()
                    },
                    |ui| app.draw_shell(ui),
                );
                output.textures_delta.clear();
                let textures = app.palette_texture_ids();
                let images: Vec<_> = output
                    .shapes
                    .iter()
                    .filter_map(|shape| {
                        if let egui::Shape::Rect(rect_shape) = &shape.shape {
                            if rect_shape
                                .brush
                                .as_ref()
                                .is_some_and(|brush| textures.contains(&brush.fill_texture_id))
                            {
                                return Some(rect_shape.rect);
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
                        .filter(|(group, _, _)| *group == "Symbols")
                        .count();
                    assert_eq!(
                        images.len(),
                        count,
                        "every symbol must fit at the minimum width"
                    );
                    assert!(images
                        .iter()
                        .all(|rect| output.shapes.iter().any(|clipped| {
                            matches!(&clipped.shape, egui::Shape::Rect(painted)
                            if painted.rect == *rect && clipped.clip_rect.contains_rect(*rect))
                        })));
                }
            }
        }
    }
}

#[test]
fn every_category_keeps_three_palette_rows_at_compact_sizes() {
    for dpi in [1.0, 1.25, 1.5] {
        let ctx = egui::Context::default();
        ctx.set_pixels_per_point(dpi);
        let mut app = super::helpers::app();
        for group in palette::UI_GROUPS {
            if group.id == palette::RECENT_GROUP {
                continue;
            }
            app.category = group.id.into();
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
                        |ui| app.draw_shell(ui),
                    );
                    output.textures_delta.clear();
                    let textures = app.palette_texture_ids();
                    let mut rows = Vec::<f32>::new();
                    for shape in &output.shapes {
                        if let egui::Shape::Rect(rect_shape) = &shape.shape {
                            if rect_shape
                                .brush
                                .as_ref()
                                .is_some_and(|brush| textures.contains(&brush.fill_texture_id))
                                && !rows
                                    .iter()
                                    .any(|y| (y - rect_shape.rect.center().y).abs() < 2.0)
                            {
                                rows.push(rect_shape.rect.center().y);
                            }
                        }
                    }
                    assert!(
                        !rows.is_empty(),
                        "{} must contain rendered buttons",
                        group.id
                    );
                    assert!(
                        rows.len() <= 3,
                        "{}: {} rows at width {width}, DPI {dpi}",
                        group.id,
                        rows.len()
                    );
                    let batch = output
                        .shapes
                        .iter()
                        .find(|clipped| {
                            matches!(&clipped.shape, egui::Shape::Text(text_shape)
                                if text_shape.galley.text() == "Batch DOCX")
                        })
                        .unwrap();
                    assert!(
                        batch
                            .clip_rect
                            .contains_rect(batch.shape.visual_bounding_rect()),
                        "{}",
                        group.id
                    );
                }
            }
        }
    }
}

#[test]
fn all_sidebar_categories_and_batch_stay_visible_without_scrolling() {
    for dpi in [1.0, 1.25, 1.5] {
        let ctx = egui::Context::default();
        ctx.set_pixels_per_point(dpi);
        let mut app = super::helpers::app();
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::from(MIN_WINDOW_SIZE))),
                ..Default::default()
            },
            |ui| app.draw_shell(ui),
        );
        output.textures_delta.clear();
        let sidebar_tex_ids: std::collections::HashSet<_> = palette::UI_GROUPS
            .iter()
            .flat_map(|group| group.icons.iter().copied())
            .map(|icon_name| {
                app.ui_icon_texture_id(icon_name)
                    .unwrap_or_else(|| panic!("missing sidebar icon {icon_name}"))
            })
            .collect();
        let expected_icon_count: usize = palette::UI_GROUPS
            .iter()
            .map(|group| group.icons.len())
            .sum();
        let icon_rects: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| {
                if let egui::Shape::Rect(rect_shape) = &shape.shape {
                    if rect_shape
                        .brush
                        .as_ref()
                        .is_some_and(|brush| sidebar_tex_ids.contains(&brush.fill_texture_id))
                    {
                        return Some((rect_shape.rect, shape.clip_rect));
                    }
                }
                None
            })
            .collect();
        assert_eq!(
            icon_rects.len(),
            expected_icon_count,
            "all {expected_icon_count} category icons must render at DPI {dpi}"
        );
        for (rect, clip) in icon_rects {
            assert!(
                clip.contains_rect(rect),
                "category icon {rect:?} clipped by {clip:?} at DPI {dpi}"
            );
        }
        for nav in ["Batch DOCX", "Settings", "About"] {
            let (rect, clip) = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text_shape) if text_shape.galley.text() == nav => {
                        Some((shape.shape.visual_bounding_rect(), shape.clip_rect))
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("missing sidebar button {nav}"));
            assert!(
                clip.contains_rect(rect),
                "{nav} clipped at minimum window size"
            );
        }
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
            output.textures_delta.clear();
            let requested = output.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .iter()
                .find_map(|cmd| {
                    if let egui::ViewportCommand::InnerSize(inner_size) = cmd {
                        Some(*inner_size)
                    } else {
                        None
                    }
                });
            assert_eq!(requested, expected, "Wayland fallback: {wayland}");
        }
    }
}

#[test]
fn canvas_scrolls_only_for_formula_overflow_across_dpi_changes() {
    for dpi in [1.0, 1.25, 1.5, 2.0] {
        let ctx = egui::Context::default();
        ctx.set_pixels_per_point(dpi);
        let mut app = super::helpers::app();
        let short = MathNode::with_children("math", vec![MathNode::leaf("mi", "x")]);
        let long = MathNode::with_children("math", vec![MathNode::leaf("mi", &"x".repeat(100))]);
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
            app.rendered = Some(Rendered {
                placeholders: vec![],
                image: egui::ColorImage::from_rgba_unmultiplied([1, 1], &[0, 0, 0, 255]),
                preview: eqntide::render::render_interactive(math, app.font_pt).unwrap(),
            });
            for _ in 0..4 {
                let mut scroll = None;
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(320.3, 200.7))),
                        time: Some(frame as f64 * 0.25),
                        ..Default::default()
                    },
                    |ui| {
                        scroll = Some(app.draw_canvas(ui));
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
                let pal = app.active_canvas_palette(ctx.theme() == egui::Theme::Dark);
                let canvas = output
                    .shapes
                    .iter()
                    .find_map(|clipped| match &clipped.shape {
                        egui::Shape::Rect(rect_shape)
                            if rect_shape.fill == pal.bg
                                && rect_shape.stroke.color == pal.border =>
                        {
                            Some(rect_shape)
                        }
                        _ => None,
                    })
                    .unwrap();
                assert_eq!(canvas.corner_radius, theme::SURFACE_CORNER_RADIUS);
            }
        }
    }
}

#[test]
fn sidebar_batch_and_back_preserve_unsaved_equation() {
    let mut app = super::helpers::app();
    app.editor.insert_text("x");
    app.mark_canvas_changed();
    let before = app.editor.math();
    let source = app.latex.clone();
    let ctx = egui::Context::default();
    for (label, expected_tab) in [("Batch DOCX", 1), ("Back", 0)] {
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| app.draw_shell(ui));
        output.textures_delta.clear();
        let pos = output
            .shapes
            .iter()
            .find_map(|clipped| {
                if let egui::Shape::Text(text_shape) = &clipped.shape {
                    if text_shape.galley.text() == label {
                        return Some(clipped.shape.visual_bounding_rect().center());
                    }
                }
                None
            })
            .unwrap();
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
            |ui| app.draw_shell(ui),
        )
        .textures_delta
        .clear();
        assert_eq!(app.tab, expected_tab);
        assert_eq!(app.editor.math(), before);
        assert_eq!(app.latex, source);
        assert!(app.is_dirty());
    }
}

#[test]
fn palette_icons_use_theme_contrast_and_physical_dpi() {
    let mut app = super::helpers::app();
    for (theme, expected_tint) in [
        (egui::Theme::Dark, ThemeTokens::for_mode(true).text_primary),
        (
            egui::Theme::Light,
            ThemeTokens::for_mode(false).text_primary,
        ),
    ] {
        let ctx = egui::Context::default();
        ThemeTokens::apply_style(&ctx);
        ctx.set_theme(theme);
        ctx.set_pixels_per_point(1.5);
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| app.draw_shell(ui));
        output.textures_delta.clear();
        let texture_ids = app.palette_texture_ids();
        assert!(!texture_ids.is_empty());
        assert!((app.palette_ppp - 1.5).abs() < f32::EPSILON);
        let tinted_match = output.shapes.iter().any(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect_shape) => rect_shape.brush.as_ref().is_some_and(|brush| {
                texture_ids.contains(&brush.fill_texture_id) && rect_shape.fill == expected_tint
            }),
            _ => false,
        });
        assert!(
            tinted_match,
            "expected palette icons to tint {expected_tint:?} in {theme:?}"
        );
    }
}

#[test]
fn sidebar_settings_and_about_tabs_work_and_toggle_theme() {
    let mut app = super::helpers::app();
    let ctx = egui::Context::default();
    ctx.set_theme(egui::Theme::Dark);

    // Open Settings from sidebar
    for (label, expected_tab) in [("Settings", 2), ("Back", 0), ("About", 3), ("Back", 0)] {
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| app.draw_shell(ui));
        output.textures_delta.clear();
        let pos = output
            .shapes
            .iter()
            .find_map(|clipped| {
                if let egui::Shape::Text(text_shape) = &clipped.shape {
                    if text_shape.galley.text() == label {
                        return Some(clipped.shape.visual_bounding_rect().center());
                    }
                }
                None
            })
            .unwrap_or_else(|| panic!("missing button {label}"));
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
            |ui| app.draw_shell(ui),
        )
        .textures_delta
        .clear();
        assert_eq!(app.tab, expected_tab);
    }

    // Verify About displays logo, version, author, and GitHub link
    app.tab = 3;
    let mut about_out = ctx.run_ui(egui::RawInput::default(), |ui| app.draw_shell(ui));
    about_out.textures_delta.clear();
    let texts: Vec<_> = about_out
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text_shape) => Some(text_shape.galley.text().to_owned()),
            _ => None,
        })
        .collect();
    assert!(texts
        .iter()
        .any(|text| text == concat!("EqnTide ", env!("CARGO_PKG_VERSION"))));
    assert!(!texts.iter().any(|text| text == "About EqnTide"));
    assert!(!texts
        .iter()
        .any(|text| text == "Version:" || text.starts_with("Version ")));
    assert!(texts
        .iter()
        .any(|text| text.starts_with("Bidirectional math equation")));
    assert!(texts.iter().any(|text| text == env!("CARGO_PKG_AUTHORS")));
    assert!(texts
        .iter()
        .any(|text| text == env!("CARGO_PKG_REPOSITORY")));
    assert!(app.logo_texture.is_some());

    // Verify Settings switches theme from Dark -> Light via selectable Light/Dark buttons
    app.tab = 2;
    let mut settings_out = ctx.run_ui(egui::RawInput::default(), |ui| app.draw_shell(ui));
    settings_out.textures_delta.clear();
    let dark_tokens = ThemeTokens::for_mode(true);
    let dark_btn_pos = find_text_center(&settings_out, |text| text == "Dark");
    let light_btn_pos = find_text_center(&settings_out, |text| text == "Light");
    assert!(settings_out.shapes.iter().any(|clipped| matches!(&clipped.shape,
        egui::Shape::Rect(rect_shape) if rect_shape.rect.contains(dark_btn_pos) && rect_shape.fill == dark_tokens.selected_bg)));
    assert!(!settings_out.shapes.iter().any(|clipped| matches!(&clipped.shape,
        egui::Shape::Rect(rect_shape) if rect_shape.rect.contains(light_btn_pos) && rect_shape.fill == dark_tokens.selected_bg)));

    ctx.run_ui(
        egui::RawInput {
            events: click_events(light_btn_pos),
            ..Default::default()
        },
        |ui| app.draw_shell(ui),
    )
    .textures_delta
    .clear();
    assert_eq!(ctx.theme(), egui::Theme::Light);

    let mut after_light = ctx.run_ui(egui::RawInput::default(), |ui| app.draw_shell(ui));
    after_light.textures_delta.clear();

    let light_tokens = ThemeTokens::for_mode(false);
    let light_btn_pos = find_text_center(&after_light, |text| text == "Light");
    let dark_btn_pos = find_text_center(&after_light, |text| text == "Dark");
    assert!(after_light.shapes.iter().any(|clipped| matches!(&clipped.shape,
        egui::Shape::Rect(rect_shape)
            if rect_shape.rect.contains(light_btn_pos)
                && (rect_shape.fill == light_tokens.selected_bg || rect_shape.fill == light_tokens.selected_hover_bg)
                && rect_shape.stroke.color == light_tokens.selected_border)));
    assert!(!after_light.shapes.iter().any(|clipped| matches!(&clipped.shape,
        egui::Shape::Rect(rect_shape)
            if rect_shape.rect.contains(dark_btn_pos)
                && (rect_shape.fill == light_tokens.selected_bg || rect_shape.fill == light_tokens.selected_hover_bg))));
    assert!(!after_light
        .shapes
        .iter()
        .any(|clipped| matches!(&clipped.shape,
        egui::Shape::Text(text_shape) if text_shape.galley.text() == "Theme")));
    // Zoom slider rail in Light mode contrasts against card_bg (not white-on-white).
    let pct_pos = find_text_center(&after_light, |text| text == "200%");
    assert!(after_light
        .shapes
        .iter()
        .any(|clipped| matches!(&clipped.shape,
        egui::Shape::Rect(rect_shape)
            if (rect_shape.rect.center().y - pct_pos.y).abs() < 12.0
                && rect_shape.rect.width() >= 150.0
                && rect_shape.rect.height() <= 24.0
                && rect_shape.fill == light_tokens.border)));
    assert!(after_light
        .shapes
        .iter()
        .any(|clipped| matches!(&clipped.shape,
        egui::Shape::Rect(rect_shape)
            if (rect_shape.rect.center().y - pct_pos.y).abs() < 12.0
                && rect_shape.rect.height() <= 24.0
                && rect_shape.fill == light_tokens.accent)));
}

#[test]
fn sidebar_icons_load_48px_textures_and_tint_across_themes() {
    let mut app = super::helpers::app();
    let ctx = egui::Context::default();
    for dpi in [1.0, 2.0] {
        ctx.set_pixels_per_point(dpi);
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| app.draw_shell(ui));
        out.textures_delta.clear();
        for group in palette::UI_GROUPS {
            for &icon_name in group.icons {
                let handle = app
                    .ui_icons
                    .get(icon_name)
                    .unwrap_or_else(|| panic!("missing texture for {icon_name}"));
                assert_eq!(handle.size(), [48, 48]);
                let tex_id = handle.id();
                let icon_rect = out
                    .shapes
                    .iter()
                    .find_map(|clipped| match &clipped.shape {
                        egui::Shape::Rect(rect_shape)
                            if rect_shape
                                .brush
                                .as_ref()
                                .is_some_and(|brush| brush.fill_texture_id == tex_id) =>
                        {
                            Some(rect_shape.rect)
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| panic!("missing painted sidebar icon {icon_name}"));
                assert!((icon_rect.width() - SIDEBAR_CATEGORY_ICON_SIZE).abs() <= 0.5);
                assert!((icon_rect.height() - SIDEBAR_CATEGORY_ICON_SIZE).abs() <= 0.5);
            }
        }
    }

    for (theme, expected_tint) in [
        (egui::Theme::Dark, ThemeTokens::for_mode(true).text_primary),
        (
            egui::Theme::Light,
            ThemeTokens::for_mode(false).text_primary,
        ),
    ] {
        ThemeTokens::apply_style(&ctx);
        ctx.set_theme(theme);
        let mut tint = None;
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            tint = Some(App::icon_tint(ui));
        });
        out.textures_delta.clear();
        assert_eq!(tint, Some(expected_tint));
    }
}

#[test]
fn recent_palette_tracks_mru_without_duplicates_and_renders() {
    let mut app = super::helpers::app();
    let ctx = egui::Context::default();
    assert_eq!(app.category, palette::DEFAULT_GROUP);
    assert!(app.recent_palette.is_empty());

    app.category = palette::RECENT_GROUP.into();
    let mut empty_out = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::from(MIN_WINDOW_SIZE))),
            ..Default::default()
        },
        |ui| app.draw_shell(ui),
    );
    empty_out.textures_delta.clear();
    assert!(empty_out
        .shapes
        .iter()
        .any(|clipped| matches!(&clipped.shape,
        egui::Shape::Text(text_shape) if text_shape.galley.text().contains("Click any symbol"))));

    app.category = "Fractions".into();
    let mut frac_out = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::from(MIN_WINDOW_SIZE))),
            ..Default::default()
        },
        |ui| app.draw_shell(ui),
    );
    frac_out.textures_delta.clear();
    let frac_tex = app.palette_atlases["Fractions"].texture.id();
    let first_pos = frac_out
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect_shape)
                if rect_shape
                    .brush
                    .as_ref()
                    .is_some_and(|brush| brush.fill_texture_id == frac_tex) =>
            {
                Some(rect_shape.rect.center())
            }
            _ => None,
        })
        .expect("fraction palette button");

    ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::from(MIN_WINDOW_SIZE))),
            events: vec![
                egui::Event::PointerMoved(first_pos),
                egui::Event::PointerButton {
                    pos: first_pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::PointerButton {
                    pos: first_pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            ..Default::default()
        },
        |ui| app.draw_shell(ui),
    )
    .textures_delta
    .clear();
    assert_eq!(app.recent_palette.len(), 1);
    let first_source = app.recent_palette[0];

    app.record_recent_palette(r"\alpha");
    app.record_recent_palette(first_source);
    assert_eq!(app.recent_palette, vec![first_source, r"\alpha"]);

    app.category = palette::RECENT_GROUP.into();
    let mut recent_out = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::from(MIN_WINDOW_SIZE))),
            ..Default::default()
        },
        |ui| app.draw_shell(ui),
    );
    recent_out.textures_delta.clear();
    let recent_tex = app.palette_atlases[palette::RECENT_GROUP].texture.id();
    let rendered_recent = recent_out
        .shapes
        .iter()
        .filter(|clipped| {
            matches!(&clipped.shape, egui::Shape::Rect(rect_shape)
                if rect_shape.brush.as_ref().is_some_and(|brush| brush.fill_texture_id == recent_tex))
        })
        .count();
    assert_eq!(rendered_recent, 2);
}

#[test]
fn palette_scrolls_horizontally_on_vertical_mouse_wheel() {
    fn run_wheel_case(category: &str, wheel_delta: Vec2, expected_overflow: bool) {
        let ctx = egui::Context::default();
        let mut app = super::helpers::app();
        app.category = category.into();
        let hover_pos = Pos2::new(360.0, 48.0);
        let mut time = 0.0f64;

        // Prime layout and palette rect.
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::from(MIN_WINDOW_SIZE))),
                time: Some(time),
                predicted_dt: 1.0 / 60.0,
                events: vec![egui::Event::PointerMoved(hover_pos)],
                ..Default::default()
            },
            |ui| app.draw_shell(ui),
        );
        out.textures_delta.clear();

        // Send mouse-wheel scroll while hovering over palette.
        time += 0.1;
        let mut wheel_out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::from(MIN_WINDOW_SIZE))),
                time: Some(time),
                predicted_dt: 1.0 / 60.0,
                events: vec![
                    egui::Event::PointerMoved(hover_pos),
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: wheel_delta,
                        phase: egui::TouchPhase::Move,
                        source: egui::MouseWheelSource::Unknown,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                ..Default::default()
            },
            |ui| app.draw_shell(ui),
        );
        wheel_out.textures_delta.clear();

        // Step settle frames so smooth/kinetic scroll finishes.
        let mut settle_out = wheel_out;
        for _ in 0..4 {
            time += 0.05;
            settle_out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::from(MIN_WINDOW_SIZE))),
                    time: Some(time),
                    predicted_dt: 1.0 / 60.0,
                    events: vec![egui::Event::PointerMoved(hover_pos)],
                    ..Default::default()
                },
                |ui| app.draw_shell(ui),
            );
            settle_out.textures_delta.clear();
        }

        let palette_tex = app.palette_atlases[category].texture.id();
        let first_x = |shapes: &[egui::epaint::ClippedShape]| -> f32 {
            shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::Shape::Rect(rect_shape)
                        if rect_shape
                            .brush
                            .as_ref()
                            .is_some_and(|brush| brush.fill_texture_id == palette_tex) =>
                    {
                        Some(rect_shape.rect.left())
                    }
                    _ => None,
                })
                .fold(f32::INFINITY, f32::min)
        };
        let initial_x = first_x(&out.shapes);
        let final_x = first_x(&settle_out.shapes);
        if expected_overflow {
            assert!(
                final_x < initial_x - 20.0,
                "expected overflowing category {category} to pan left (scroll right) with {wheel_delta:?}: {initial_x} -> {final_x}"
            );
        } else {
            assert!(
                (final_x - initial_x).abs() < 1.0,
                "expected non-overflowing category {category} to stay stationary with {wheel_delta:?}: {initial_x} -> {final_x}"
            );
        }
    }

    run_wheel_case("LogicSetsSymbols", Vec2::new(0.0, -180.0), true);
    run_wheel_case("LogicSetsSymbols", Vec2::new(-180.0, 0.0), true);
    run_wheel_case("Fractions", Vec2::new(0.0, -180.0), false);
}

#[test]
fn modern_theme_applies_zero_hover_expansion_radii_and_tinted_sidebar_selection() {
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        let dark_mode = theme == egui::Theme::Dark;
        let tokens = ThemeTokens::for_mode(dark_mode);
        let ctx = egui::Context::default();
        ThemeTokens::apply_style(&ctx);
        ctx.set_theme(theme);

        let mut app = super::helpers::app();
        let out = run_ui_pass_frame(&ctx, &mut app, vec![]);

        // Selected sidebar category ("Fractions") displays tinted-accent surface and matching icon tint.
        let fractions_tex = app
            .ui_icon_texture_id(palette::ui_group("Fractions").unwrap().icons[0])
            .expect("Fractions sidebar texture");
        assert!(out.shapes.iter().any(|clipped| matches!(&clipped.shape,
            egui::Shape::Rect(rect_shape) if rect_shape.fill == tokens.selected_bg && rect_shape.stroke.color == tokens.selected_border)));
        assert!(out.shapes.iter().any(|clipped| matches!(&clipped.shape,
            egui::Shape::Rect(rect_shape)
                if rect_shape.brush.as_ref().is_some_and(|brush| brush.fill_texture_id == fractions_tex)
                    && rect_shape.fill == tokens.selected_fg)));

        // Primary Save button uses primary_bg at rest, primary_hover_bg on hover without growing, and disabled fill when disabled.
        let save_pos = find_text_center(&out, |text| text == "Save MathML (Ctrl+S)");
        let save_rest = out
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(rect_shape)
                    if rect_shape.fill == tokens.primary_bg
                        && rect_shape.rect.contains(save_pos) =>
                {
                    Some((rect_shape.rect, rect_shape.corner_radius))
                }
                _ => None,
            })
            .expect("Save button rest rect");
        assert_eq!(save_rest.1, theme::CONTROL_CORNER_RADIUS);

        let hover_out =
            run_ui_pass_frame(&ctx, &mut app, vec![egui::Event::PointerMoved(save_pos)]);
        let save_hover = hover_out
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(rect_shape)
                    if rect_shape.fill == tokens.primary_hover_bg
                        && rect_shape.rect.contains(save_pos) =>
                {
                    Some(rect_shape.rect)
                }
                _ => None,
            })
            .expect("Save button hover rect");
        assert_eq!(save_hover, save_rest.0);

        // Disabled Save button suppresses primary/hover blue emphasis.
        app.draft_dirty = true;
        let disabled_out =
            run_ui_pass_frame(&ctx, &mut app, vec![egui::Event::PointerMoved(save_pos)]);
        assert!(disabled_out
            .shapes
            .iter()
            .any(|clipped| matches!(&clipped.shape,
            egui::Shape::Rect(rect_shape)
                if rect_shape.rect.contains(save_pos)
                    && rect_shape.fill != tokens.primary_bg
                    && rect_shape.fill != tokens.primary_hover_bg)));
        app.draft_dirty = false;

        // Settings zoom slider shares CONTROL_CORNER_RADIUS (no 4px override).
        app.tab = 2;
        let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
        let style = ctx.style_of(theme);
        assert_eq!(
            style.visuals.widgets.inactive.corner_radius,
            theme::CONTROL_CORNER_RADIUS
        );
        assert_eq!(style.visuals.widgets.hovered.expansion, 0.0);
    }
}

#[test]
fn canvas_presets_switch_adapt_ink_and_persist_per_mode() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let ctx = egui::Context::default();
    ThemeTokens::apply_style(&ctx);
    let mut app = super::helpers::app().with_settings_path(path.clone(), Some(&ctx));
    app.load_math("<math><mi>x</mi></math>", None, None);

    // Render preview texture for ink verification.
    let (mask, preview) =
        eqntide::render::render_interactive_alpha_scaled(&app.editor.math(), app.font_pt, 1.0)
            .unwrap();
    let img = egui::ColorImage {
        size: [mask.width as usize, mask.height as usize],
        source_size: Vec2::new(mask.width as f32, mask.height as f32),
        pixels: mask
            .alpha
            .into_iter()
            .map(Color32::from_white_alpha)
            .collect(),
    };
    app.texture = Some(ctx.load_texture("eq-test", img, egui::TextureOptions::LINEAR));
    app.rendered = Some(Rendered {
        placeholders: Vec::new(),
        preview,
        image: egui::ColorImage::filled([1, 1], Color32::WHITE),
    });
    app.requested_key = app.render_key;

    let has_formula_ink = |output: &egui::FullOutput, tex_id: egui::TextureId, ink: Color32| {
        output.shapes.iter().any(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect_shape) => {
                rect_shape.fill == ink
                    && rect_shape
                        .brush
                        .as_ref()
                        .is_some_and(|brush| brush.fill_texture_id == tex_id)
            }
            egui::Shape::Mesh(mesh) => {
                mesh.texture_id == tex_id && mesh.vertices.iter().all(|vert| vert.color == ink)
            }
            _ => false,
        })
    };

    // Default in Dark mode still uses SoftPaper (light paper background + dark ink) on both canvas and LaTeX source.
    ctx.set_theme(egui::Theme::Dark);
    let out = run_ui_pass_frame(&ctx, &mut app, vec![]);
    let tex_id = app.texture.as_ref().unwrap().id();
    let latex_rect = ctx
        .read_response(egui::Id::unique("latex-source"))
        .unwrap()
        .rect;
    let soft_dark = CanvasPreset::SoftPaper.palette(true);
    assert!(out.shapes.iter().any(|clipped| matches!(&clipped.shape,
        egui::Shape::Rect(rect_shape) if rect_shape.fill == soft_dark.bg && rect_shape.stroke.color == soft_dark.border)));
    assert!(out.shapes.iter().any(|clipped| matches!(&clipped.shape,
        egui::Shape::Rect(rect_shape) if rect_shape.fill == soft_dark.bg && rect_shape.rect == latex_rect)));
    assert!(has_formula_ink(&out, tex_id, soft_dark.ink));

    // Open Settings and pick CoolSlate for Light mode and DimSlate for Dark mode.
    app.tab = 2;
    let tall = Vec2::new(MIN_WINDOW_SIZE[0], 900.0);
    let out = run_ui_pass_sized(&ctx, &mut app, tall);
    let cool_pos = find_text_center(&out, |text| text == CanvasPreset::CoolSlate.label());
    let out = run_ui_pass_sized_events(&ctx, &mut app, tall, click_events(cool_pos));
    let dim_pos = find_text_center(&out, |text| text == CanvasPreset::DimSlate.label());
    run_ui_pass_sized_events(&ctx, &mut app, tall, click_events(dim_pos));

    let on_disk = Settings::load_from_path(&path);
    assert_eq!(on_disk.canvas_light, CanvasPreset::CoolSlate);
    assert_eq!(on_disk.canvas_dark, CanvasPreset::DimSlate);

    // Verify Dark mode now uses DimSlate surface + light ink, and Light mode uses CoolSlate surface + dark ink.
    app.tab = 0;
    let dark_out = run_ui_pass_frame(&ctx, &mut app, vec![]);
    let dim_pal = CanvasPreset::DimSlate.palette(true);
    assert!(dark_out.shapes.iter().any(|clipped| matches!(&clipped.shape,
        egui::Shape::Rect(rect_shape) if rect_shape.fill == dim_pal.bg && rect_shape.stroke.color == dim_pal.border)));
    assert!(dark_out.shapes.iter().any(|clipped| matches!(&clipped.shape,
        egui::Shape::Rect(rect_shape) if rect_shape.fill == dim_pal.bg && rect_shape.rect == latex_rect)));
    assert!(has_formula_ink(&dark_out, tex_id, dim_pal.ink));

    ctx.set_theme(egui::Theme::Light);
    let light_out = run_ui_pass_frame(&ctx, &mut app, vec![]);
    let cool_pal = CanvasPreset::CoolSlate.palette(false);
    assert!(light_out.shapes.iter().any(|clipped| matches!(&clipped.shape,
        egui::Shape::Rect(rect_shape) if rect_shape.fill == cool_pal.bg && rect_shape.stroke.color == cool_pal.border)));
    assert!(light_out.shapes.iter().any(|clipped| matches!(&clipped.shape,
        egui::Shape::Rect(rect_shape) if rect_shape.fill == cool_pal.bg && rect_shape.rect == latex_rect)));
    assert!(has_formula_ink(&light_out, tex_id, cool_pal.ink));

    // Reload in a fresh App instance and confirm presets restored.
    let fresh_ctx = egui::Context::default();
    let reloaded = super::helpers::app().with_settings_path(path, Some(&fresh_ctx));
    assert_eq!(reloaded.canvas_light, CanvasPreset::CoolSlate);
    assert_eq!(reloaded.canvas_dark, CanvasPreset::DimSlate);
}

#[test]
fn sidebar_and_palette_icons_render_at_uniform_base_em_scale() {
    let ctx = egui::Context::default();
    ThemeTokens::apply_style(&ctx);
    let mut app = super::helpers::app();
    app.category = "Scripts".into();
    let out = run_ui_pass_frame(&ctx, &mut app, vec![]);

    for group in palette::UI_GROUPS {
        for &icon_name in group.icons {
            let tex = app
                .ui_icon_texture_id(icon_name)
                .unwrap_or_else(|| panic!("missing sidebar icon {icon_name}"));
            let rect = out
                .shapes
                .iter()
                .find_map(|clipped| match &clipped.shape {
                    egui::Shape::Rect(rect_shape)
                        if rect_shape
                            .brush
                            .as_ref()
                            .is_some_and(|brush| brush.fill_texture_id == tex) =>
                    {
                        Some(rect_shape.rect)
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("missing painted sidebar icon {icon_name}"));
            assert!((rect.width() - SIDEBAR_CATEGORY_ICON_SIZE).abs() <= 0.5);
            assert!((rect.height() - SIDEBAR_CATEGORY_ICON_SIZE).abs() <= 0.5);
        }
    }

    let scripts_items = palette::items_for_ui_group("Scripts", &[]);
    let scripts_atlas = &app.palette_atlases["Scripts"];
    let first_h = scripts_atlas.entries[scripts_items[0].2].display_size.y;
    let second_h = scripts_atlas.entries[scripts_items[1].2].display_size.y;
    assert!(first_h <= 26.0 && second_h <= 26.0);
    assert!((first_h - second_h).abs() <= 2.0);

    // Verify palette button frames share uniform 46x32 dimensions.
    let scripts_tex = scripts_atlas.texture.id();
    let icon_rects: Vec<_> = out
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect_shape)
                if rect_shape
                    .brush
                    .as_ref()
                    .is_some_and(|brush| brush.fill_texture_id == scripts_tex) =>
            {
                Some(rect_shape.rect)
            }
            _ => None,
        })
        .collect();
    assert!(!icon_rects.is_empty());
    let first_icon = icon_rects[0];
    let button_rect = out
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect_shape)
                if rect_shape.brush.is_none()
                    && rect_shape.rect.contains(first_icon.center())
                    && (rect_shape.rect.width() - PALETTE_BUTTON_SIZE.x).abs() < 1.0 =>
            {
                Some(rect_shape.rect)
            }
            _ => None,
        })
        .expect("palette tile rect");
    assert!((button_rect.width() - PALETTE_BUTTON_SIZE.x).abs() < 1.0);
    assert!((button_rect.height() - PALETTE_BUTTON_SIZE.y).abs() < 1.0);
}

#[test]
fn button_height_tiers_align_footer_tabs_and_compact_controls() {
    let ctx = egui::Context::default();
    ThemeTokens::apply_style(&ctx);
    let mut app = super::helpers::app();
    app.category = "MatricesAndBoxes".into();
    let out = run_ui_pass_frame(&ctx, &mut app, vec![]);

    let button_rect_around = |label: &str| -> Rect {
        let pos = find_text_center(&out, |text| text == label);
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
            .unwrap_or_else(|| panic!("missing button rect for {label}"))
    };

    let icon_button_rect = |icon_name: &str| -> Rect {
        let tex = app
            .ui_icon_texture_id(icon_name)
            .unwrap_or_else(|| panic!("missing icon texture {icon_name}"));
        let pos = out
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(rect_shape)
                    if rect_shape
                        .brush
                        .as_ref()
                        .is_some_and(|brush| brush.fill_texture_id == tex) =>
                {
                    Some(rect_shape.rect.center())
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing painted icon {icon_name}"));
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
            .unwrap_or_else(|| panic!("missing button rect for icon {icon_name}"))
    };

    // Footer standard buttons and DragValue share BUTTON_HEIGHT (30px).
    let save_rect = button_rect_around("Save MathML (Ctrl+S)");
    let undo_rect = icon_button_rect("arrow-back-up");
    let redo_rect = icon_button_rect("arrow-forward-up");
    assert!((save_rect.height() - theme::BUTTON_HEIGHT).abs() <= 0.5);
    assert!((undo_rect.height() - theme::BUTTON_HEIGHT).abs() <= 0.5);
    assert!((redo_rect.height() - theme::BUTTON_HEIGHT).abs() <= 0.5);

    // Sidebar navigation tabs share SIDEBAR_TAB_HEIGHT (34px).
    for tab_label in ["Batch DOCX", "Settings", "About"] {
        let tab_rect = button_rect_around(tab_label);
        assert!((tab_rect.height() - theme::SIDEBAR_TAB_HEIGHT).abs() <= 0.5);
    }

    // Compact matrix controls share COMPACT_BUTTON_HEIGHT (24px).
    for matrix_label in ["+ row", "− row", "+ col", "− col"] {
        let rect = button_rect_around(matrix_label);
        assert!((rect.height() - theme::COMPACT_BUTTON_HEIGHT).abs() <= 0.5);
    }
}

#[test]
fn multiline_latex_scrolls_within_bounds_without_canvas_overlap() {
    let ctx = egui::Context::default();
    ThemeTokens::apply_style(&ctx);
    let mut app = super::helpers::app();
    let pal = app.active_canvas_palette(ctx.theme() == egui::Theme::Dark);
    let latex_id = egui::Id::unique("latex-source");

    let find_canvas_and_latex_frames = |output: &egui::FullOutput| {
        let canvas = output
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(rect_shape)
                    if rect_shape.fill == pal.bg
                        && rect_shape.corner_radius == theme::SURFACE_CORNER_RADIUS
                        && rect_shape.rect.width() > 100.0 =>
                {
                    Some(rect_shape.rect)
                }
                _ => None,
            })
            .expect("canvas frame");
        let latex = output
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(rect_shape)
                    if rect_shape.corner_radius == theme::CONTROL_CORNER_RADIUS
                        && rect_shape.stroke == Stroke::new(1.0, pal.border)
                        && rect_shape.rect.width() > 300.0 =>
                {
                    Some(rect_shape.rect)
                }
                _ => None,
            })
            .expect("latex frame");
        (canvas, latex)
    };

    let out = run_ui_pass_frame(&ctx, &mut app, vec![]);
    let (canvas_single, latex_single) = find_canvas_and_latex_frames(&out);
    assert!(canvas_single.bottom() + CONTENT_PADDING as f32 <= latex_single.top() + 1.0);
    assert!((canvas_single.left() - latex_single.left()).abs() <= 1.0);
    assert!((canvas_single.right() - latex_single.right()).abs() <= 1.0);
    // No scrollbar handle/track appears inside the empty LaTeX box.
    assert!(!out.shapes.iter().any(|clipped| matches!(&clipped.shape,
        egui::Shape::Rect(rect_shape) if latex_single.contains_rect(rect_shape.rect) && rect_shape.fill == pal.scroll_track)));

    // Clicking inside the LaTeX box focuses it without adding a bright focus border.
    let out = run_ui_pass_frame(&ctx, &mut app, click_events(latex_single.center()));
    assert!(ctx.memory(|mem| mem.has_focus(latex_id)));
    assert!(!out.shapes.iter().any(|clipped| matches!(&clipped.shape,
        egui::Shape::Rect(rect_shape) if rect_shape.rect == latex_single && rect_shape.stroke.color == pal.caret)));
    assert!(out.shapes.iter().any(|clipped| matches!(&clipped.shape,
        egui::Shape::Rect(rect_shape) if rect_shape.rect == latex_single && rect_shape.stroke.color == pal.border)));

    // Pressing Enter repeatedly to produce many lines keeps the LaTeX box bounded and scrollable.
    let mut events = Vec::new();
    for i in 1..=8 {
        events.push(egui::Event::Text(format!("line{i}")));
        events.push(egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
    }
    run_ui_pass_frame(&ctx, &mut app, events);
    let out = run_ui_pass_frame(&ctx, &mut app, vec![]);
    let (canvas_multi, latex_multi) = find_canvas_and_latex_frames(&out);
    assert_eq!(canvas_multi, canvas_single);
    assert_eq!(latex_multi, latex_single);
    assert!(canvas_multi.bottom() + CONTENT_PADDING as f32 <= latex_multi.top() + 1.0);

    let save_center = find_text_center(&out, |text| text == "Save MathML (Ctrl+S)");
    assert!(latex_multi.bottom() < save_center.y);

    // Rendered LaTeX lines stay clipped inside the bounded ScrollArea, never leaking out.
    let line_shapes: Vec<_> = out
        .shapes
        .iter()
        .filter(|clipped| {
            matches!(&clipped.shape, egui::Shape::Text(text_shape)
                if text_shape.galley.text().contains("line"))
        })
        .collect();
    assert!(!line_shapes.is_empty());
    for clipped in line_shapes {
        assert!(
            clipped.clip_rect.top() >= latex_multi.top() - 0.5
                && clipped.clip_rect.bottom() <= latex_multi.bottom() + 0.5,
            "latex text clip {:?} escaped frame {:?}",
            clipped.clip_rect,
            latex_multi
        );
    }
}

#[test]
fn about_card_spans_full_central_panel_width() {
    let ctx = egui::Context::default();
    ThemeTokens::apply_style(&ctx);
    ctx.set_theme(egui::Theme::Dark);
    let tokens = ThemeTokens::for_mode(true);
    let mut app = super::helpers::app();
    app.tab = 3;

    for width in [MIN_WINDOW_SIZE[0], 1400.0] {
        let size = Vec2::new(width, MIN_WINDOW_SIZE[1]);
        let _ = run_ui_pass_sized(&ctx, &mut app, size);
        let out = run_ui_pass_sized(&ctx, &mut app, size);
        let expected_width = width - 2.0 * theme::SPACE_LG;
        let cards = card_rects_for_section_titles(
            &out,
            tokens,
            &[concat!("EqnTide ", env!("CARGO_PKG_VERSION"))],
        );
        let rect = cards[0];
        assert!(
            rect.width() >= expected_width - 24.0 && rect.width() <= expected_width + 0.5,
            "About card width {} did not match panel width {} (rect={rect:?})",
            rect.width(),
            expected_width
        );
        assert!(
            (rect.left() - theme::SPACE_LG).abs() <= 1.0,
            "About card left {} misaligned at width {width}",
            rect.left()
        );
        let github_pos = find_text_center(&out, |text| text == env!("CARGO_PKG_REPOSITORY"));
        assert!(
            rect.contains(github_pos),
            "About metadata grid should sit inside full-width card {rect:?}"
        );
        let logo_tex = app.logo_texture.as_ref().expect("about logo texture").id();
        let logo_rect = out
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(rect_shape)
                    if rect_shape
                        .brush
                        .as_ref()
                        .is_some_and(|brush| brush.fill_texture_id == logo_tex) =>
                {
                    Some(rect_shape.rect)
                }
                _ => None,
            })
            .expect("about logo rect");
        let title_rect = out
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text_shape)
                    if text_shape.galley.text()
                        == concat!("EqnTide ", env!("CARGO_PKG_VERSION")) =>
                {
                    Some(clipped.shape.visual_bounding_rect())
                }
                _ => None,
            })
            .expect("about title rect");
        let desc_rect = out
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text_shape)
                    if text_shape
                        .galley
                        .text()
                        .starts_with("Bidirectional math equation") =>
                {
                    Some(clipped.shape.visual_bounding_rect())
                }
                _ => None,
            })
            .expect("about desc rect");
        let text_mid_y = (title_rect.top() + desc_rect.bottom()) * 0.5;
        assert!(
            (text_mid_y - logo_rect.center().y).abs() <= 2.5,
            "about header text ({text_mid_y}) should align vertically with logo ({})",
            logo_rect.center().y
        );
    }
}

#[test]
fn desktop_ui_views_render_assigned_tabler_icons() {
    let assert_painted = |app: &App, out: &egui::FullOutput, icon: &str, tint: Color32| {
        let tex = app
            .ui_icon_texture_id(icon)
            .unwrap_or_else(|| panic!("missing icon texture {icon}"));
        assert!(
            out.shapes.iter().any(|clipped| match &clipped.shape {
                egui::Shape::Rect(rect_shape) => {
                    rect_shape
                        .brush
                        .as_ref()
                        .is_some_and(|brush| brush.fill_texture_id == tex)
                        && rect_shape.fill == tint
                }
                _ => false,
            }),
            "expected painted icon {icon} with tint {tint:?}"
        );
    };

    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        let tokens = ThemeTokens::for_mode(theme == egui::Theme::Dark);
        let ctx = egui::Context::default();
        ThemeTokens::apply_style(&ctx);
        ctx.set_theme(theme);
        let mut app = super::helpers::app();

        // Editor view: sidebar tabs + Undo/Redo.
        let editor_out = run_ui_pass_frame(&ctx, &mut app, vec![]);
        for icon in [
            "file-type-docx",
            "settings",
            "info-circle",
            "arrow-back-up",
            "arrow-forward-up",
        ] {
            assert_painted(&app, &editor_out, icon, tokens.text_primary);
        }

        // Batch DOCX view (empty drop-zone + buttons).
        app.tab = 1;
        let batch_out = run_ui_pass_frame(&ctx, &mut app, vec![]);
        for icon in ["chevron-left", "file", "folder"] {
            assert_painted(&app, &batch_out, icon, tokens.text_primary);
        }
        assert_painted(
            &app,
            &batch_out,
            "arrows-left-right",
            tokens
                .text_primary
                .gamma_multiply(ctx.style_of(theme).visuals.disabled_alpha()),
        );

        // Settings view.
        app.tab = 2;
        let settings_out = run_ui_pass_frame(&ctx, &mut app, vec![]);
        assert_painted(&app, &settings_out, "chevron-left", tokens.text_primary);

        // About view.
        app.tab = 3;
        let about_out = run_ui_pass_frame(&ctx, &mut app, vec![]);
        assert_painted(&app, &about_out, "chevron-left", tokens.text_primary);
        for icon in ["user", "license", "brand-github"] {
            assert_painted(&app, &about_out, icon, tokens.text_muted);
        }
    }
}

#[test]
fn panel_dividers_draw_over_opaque_fills_and_clear_color_matches_theme() {
    let base_app = super::helpers::app();
    let light_tokens = ThemeTokens::for_mode(false);
    let dark_tokens = ThemeTokens::for_mode(true);
    assert_eq!(
        eframe::App::clear_color(&base_app, &egui::Visuals::light()),
        light_tokens.app_bg.to_normalized_gamma_f32()
    );
    assert_eq!(
        eframe::App::clear_color(&base_app, &egui::Visuals::dark()),
        dark_tokens.app_bg.to_normalized_gamma_f32()
    );

    let ctx = egui::Context::default();
    ThemeTokens::apply_style(&ctx);
    let mut app = super::helpers::app();
    app.persisted.theme = ThemeMode::Light;
    ctx.set_theme(egui::Theme::Light);
    let out = run_ui_pass_sized(&ctx, &mut app, Vec2::new(1100.0, 650.0));

    // Both CentralPanel (app_bg) and app-footer (panel_bg) meet flush with no 1px gap.
    let mut central_max_y = None;
    let mut footer_min_y = None;
    let mut central_idx = None;
    let mut footer_divider_idx = None;
    let mut sidebar_divider_idx = None;
    for (idx, clipped) in out.shapes.iter().enumerate() {
        match &clipped.shape {
            egui::Shape::Rect(rect_shape)
                if rect_shape.fill == light_tokens.app_bg && rect_shape.rect.width() > 800.0 =>
            {
                central_max_y = Some(rect_shape.rect.max.y);
                central_idx = Some(idx);
            }
            egui::Shape::Rect(rect_shape)
                if rect_shape.fill == light_tokens.panel_bg
                    && (rect_shape.rect.height() - 56.0).abs() < 0.01 =>
            {
                footer_min_y = Some(rect_shape.rect.min.y);
            }
            egui::Shape::LineSegment { points, stroke }
                if stroke.color == light_tokens.border && stroke.width == 1.0 =>
            {
                if (points[0].y - points[1].y).abs() < 0.01 && (points[1].x - points[0].x) > 800.0 {
                    footer_divider_idx = Some(idx);
                } else if (points[0].x - points[1].x).abs() < 0.01
                    && (points[1].y - points[0].y) > 500.0
                {
                    sidebar_divider_idx = Some(idx);
                }
            }
            _ => {}
        }
    }
    let central_max_y = central_max_y.expect("central panel fill");
    let footer_min_y = footer_min_y.expect("footer panel fill");
    assert!((central_max_y - footer_min_y).abs() < 0.01);
    let central_idx = central_idx.expect("central panel index");
    assert!(footer_divider_idx.expect("footer divider") > central_idx);
    assert!(sidebar_divider_idx.expect("sidebar divider") > central_idx);
}

#[test]
fn status_bar_is_quiet_when_idle_or_busy_and_clears_stale_messages_on_edit() {
    use egui::Key;

    // Idle launch has empty status and only renders the bridge indicator text.
    let mut app = super::helpers::app();
    app.bridge_ready = true;
    assert!(app.status.is_empty());

    let ctx = egui::Context::default();
    let collect_texts = |out: &egui::FullOutput| -> Vec<String> {
        out.shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text_shape) => Some(text_shape.galley.text().to_owned()),
                _ => None,
            })
            .collect()
    };

    let mut idle_out = ctx.run_ui(egui::RawInput::default(), |ui| app.draw_status_bar(ui));
    idle_out.textures_delta.clear();
    assert_eq!(collect_texts(&idle_out), vec!["●".to_owned()]);

    // Busy state does not animate or add "Working…" / spinner shapes in draw_status_bar.
    for profile in [GpuProfile::Native, GpuProfile::Software] {
        app.gpu_profile = profile;
        app.busy = true;
        let mut busy_out = ctx.run_ui(egui::RawInput::default(), |ui| app.draw_status_bar(ui));
        busy_out.textures_delta.clear();
        assert_eq!(collect_texts(&busy_out), vec!["●".to_owned()]);
        assert_eq!(busy_out.shapes.len(), idle_out.shapes.len());
    }
    app.busy = false;
    app.gpu_profile = GpuProfile::Native;

    // Routine LaTeX command commit, Escape cancel, and custom matrix insert leave status empty.
    type_canvas(&mut app, r"\alpha");
    canvas_key(&mut app, Key::Enter);
    assert!(app.status.is_empty());

    type_canvas(&mut app, r"\be");
    canvas_key(&mut app, Key::Escape);
    assert!(app.command_input.is_none());
    assert!(app.status.is_empty());

    app.insert_custom_matrix(&MatrixDialog {
        rows: 2,
        columns: 2,
        alignment: "center",
        fence: "none",
    });
    assert!(app.status.is_empty());

    // Subsequent edits clear stale notices like "Saved." or matrix guidance.
    app.status = "Saved.".into();
    type_canvas(&mut app, "x");
    assert!(app.status.is_empty());

    app.status = "Place the caret inside a matrix first.".into();
    app.mark_source_changed(Instant::now());
    assert!(app.status.is_empty());

    app.status = "Saved.".into();
    type_canvas(&mut app, r"\");
    assert!(app.command_input.is_some());
    assert!(app.status.is_empty());
    canvas_key(&mut app, Key::Escape);

    // Office session sets the concise notice after load and clears on the next edit.
    let mut session_app = super::helpers::app();
    session_app.loading_session = Some("office-1".into());
    session_app
        .tx
        .send(Event::Session(
            "office-1".into(),
            Ok(serde_json::json!({
                "state": "waiting",
                "mathml": "<math><mi>x</mi></math>",
            })),
        ))
        .unwrap();
    session_app.process_events(&egui::Context::default());
    assert_eq!(session_app.status, "Office session — Save to return");
    type_canvas(&mut session_app, "+1");
    assert!(session_app.status.is_empty());

    // Failed load_math retains its error message.
    assert!(!session_app.load_math("<math><unclosed>", None, None));
    assert!(session_app.status.contains("Could not load equation"));
}
