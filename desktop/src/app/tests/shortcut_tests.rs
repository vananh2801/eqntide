use super::super::*;
use super::helpers::*;
use crate::shortcuts::{ShortcutChord, ShortcutConflict, ShortcutKey, DEFAULT_PALETTE_SHORTCUTS};
use crate::theme;

fn chord_key_event(chord: ShortcutChord) -> egui::Event {
    let key = match chord.key {
        ShortcutKey::A => egui::Key::A,
        ShortcutKey::B => egui::Key::B,
        ShortcutKey::C => egui::Key::C,
        ShortcutKey::D => egui::Key::D,
        ShortcutKey::E => egui::Key::E,
        ShortcutKey::F => egui::Key::F,
        ShortcutKey::G => egui::Key::G,
        ShortcutKey::H => egui::Key::H,
        ShortcutKey::I => egui::Key::I,
        ShortcutKey::J => egui::Key::J,
        ShortcutKey::K => egui::Key::K,
        ShortcutKey::L => egui::Key::L,
        ShortcutKey::M => egui::Key::M,
        ShortcutKey::N => egui::Key::N,
        ShortcutKey::O => egui::Key::O,
        ShortcutKey::P => egui::Key::P,
        ShortcutKey::Q => egui::Key::Q,
        ShortcutKey::R => egui::Key::R,
        ShortcutKey::S => egui::Key::S,
        ShortcutKey::T => egui::Key::T,
        ShortcutKey::U => egui::Key::U,
        ShortcutKey::V => egui::Key::V,
        ShortcutKey::W => egui::Key::W,
        ShortcutKey::X => egui::Key::X,
        ShortcutKey::Y => egui::Key::Y,
        ShortcutKey::Z => egui::Key::Z,
        ShortcutKey::Num0 => egui::Key::Num0,
        ShortcutKey::Num1 => egui::Key::Num1,
        ShortcutKey::Num2 => egui::Key::Num2,
        ShortcutKey::Num3 => egui::Key::Num3,
        ShortcutKey::Num4 => egui::Key::Num4,
        ShortcutKey::Num5 => egui::Key::Num5,
        ShortcutKey::Num6 => egui::Key::Num6,
        ShortcutKey::Num7 => egui::Key::Num7,
        ShortcutKey::Num8 => egui::Key::Num8,
        ShortcutKey::Num9 => egui::Key::Num9,
        ShortcutKey::F1 => egui::Key::F1,
        ShortcutKey::F2 => egui::Key::F2,
        ShortcutKey::F3 => egui::Key::F3,
        ShortcutKey::F4 => egui::Key::F4,
        ShortcutKey::F5 => egui::Key::F5,
        ShortcutKey::F6 => egui::Key::F6,
        ShortcutKey::F7 => egui::Key::F7,
        ShortcutKey::F8 => egui::Key::F8,
        ShortcutKey::F9 => egui::Key::F9,
        ShortcutKey::F10 => egui::Key::F10,
        ShortcutKey::F11 => egui::Key::F11,
        ShortcutKey::F12 => egui::Key::F12,
        ShortcutKey::OpenBracket => egui::Key::OpenBracket,
        ShortcutKey::CloseBracket => egui::Key::CloseBracket,
        ShortcutKey::Comma => egui::Key::Comma,
        ShortcutKey::Period => egui::Key::Period,
        ShortcutKey::Semicolon => egui::Key::Semicolon,
        ShortcutKey::Minus => egui::Key::Minus,
        ShortcutKey::Equals => egui::Key::Equals,
        ShortcutKey::Slash => egui::Key::Slash,
        ShortcutKey::Backslash => egui::Key::Backslash,
        ShortcutKey::Backtick => egui::Key::Backtick,
        ShortcutKey::Quote => egui::Key::Quote,
        ShortcutKey::Plus => egui::Key::Plus,
    };
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers {
            ctrl: chord.command,
            command: chord.command,
            alt: chord.alt,
            shift: chord.shift,
            ..Default::default()
        },
    }
}

fn plain_key_event(key: egui::Key, shift: bool) -> egui::Event {
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

fn has_rendered_text(output: &egui::FullOutput, needle: &str) -> bool {
    output.shapes.iter().any(|clipped| match &clipped.shape {
        egui::Shape::Text(text_shape) => text_shape.galley.text().contains(needle),
        _ => false,
    })
}

fn run_settings_frame(
    ctx: &egui::Context,
    app: &mut App,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    run_ui_pass_sized_events(ctx, app, Vec2::new(MIN_WINDOW_SIZE[0], 1400.0), events)
}

fn run_wheel_and_settle(
    ctx: &egui::Context,
    app: &mut App,
    screen_rect: Rect,
    time: &mut f64,
    pointer_pos: Pos2,
    delta_y: f32,
) -> egui::FullOutput {
    *time += 0.1;
    ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(screen_rect),
            time: Some(*time),
            predicted_dt: 1.0 / 60.0,
            events: vec![
                egui::Event::PointerMoved(pointer_pos),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::Vec2::new(0.0, delta_y),
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Unknown,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            ..Default::default()
        },
        |ui| app.draw_ui(ui),
    )
    .textures_delta
    .clear();
    let mut output = egui::FullOutput::default();
    for _ in 0..8 {
        *time += 0.05;
        output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(screen_rect),
                time: Some(*time),
                predicted_dt: 1.0 / 60.0,
                events: vec![egui::Event::PointerMoved(pointer_pos)],
                ..Default::default()
            },
            |ui| app.draw_ui(ui),
        );
        output.textures_delta.clear();
    }
    output
}

#[test]
fn default_and_customized_shortcuts_dispatch_on_canvas_and_respect_latex_focus() {
    // 1. All 15 default palette shortcuts insert their canonical template and update Recent.
    for &(source, chord) in DEFAULT_PALETTE_SHORTCUTS {
        let mut app = super::helpers::app();
        let ctx = egui::Context::default();
        let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
        let _ = run_ui_pass_frame(&ctx, &mut app, vec![chord_key_event(chord)]);
        assert!(
            !app.editor.math().children.is_empty(),
            "default chord {chord} for {source} should insert template"
        );
        assert_eq!(app.recent_palette.first().copied(), Some(source));
    }

    // 2. Rebinding Fraction to Ctrl+Shift+F and clearing Ctrl+R persist cleanly and control dispatch.
    let dir = tempfile::tempdir().expect("tempdir");
    let settings_path = dir.path().join("settings.json");
    {
        let mut app = super::helpers::app().with_settings_path(settings_path.clone(), None);
        app.shortcuts
            .replace_binding(r"\frac{#?}{#?}", ShortcutChord::cmd_shift(ShortcutKey::F));
        app.shortcuts.clear_binding(r"\sqrt{#?}");
        app.sync_shortcut_overrides();
    }

    let ctx = egui::Context::default();
    let mut reloaded = super::helpers::app().with_settings_path(settings_path, Some(&ctx));
    let _ = run_ui_pass_frame(&ctx, &mut reloaded, vec![]);

    // Old Ctrl+F and cleared Ctrl+R do nothing.
    let _ = run_ui_pass_frame(
        &ctx,
        &mut reloaded,
        vec![chord_key_event(ShortcutChord::cmd(ShortcutKey::F))],
    );
    assert!(reloaded.editor.math().children.is_empty());
    let _ = run_ui_pass_frame(
        &ctx,
        &mut reloaded,
        vec![chord_key_event(ShortcutChord::cmd(ShortcutKey::R))],
    );
    assert!(reloaded.editor.math().children.is_empty());

    // Rebound Ctrl+Shift+F inserts fraction and records Recent.
    let _ = run_ui_pass_frame(
        &ctx,
        &mut reloaded,
        vec![chord_key_event(ShortcutChord::cmd_shift(ShortcutKey::F))],
    );
    assert_eq!(reloaded.editor.math().children[0].tag, "mfrac");
    assert_eq!(
        reloaded.recent_palette.first().copied(),
        Some(r"\frac{#?}{#?}")
    );

    // 3. Pressing a custom palette shortcut while LaTeX source has focus does not mutate canvas state.
    let before_math = reloaded.editor.math();
    let before_recent = reloaded.recent_palette.clone();
    let latex_id = App::editor_focus_id(StartupFocus::Latex);
    ctx.memory_mut(|mem| mem.request_focus(latex_id));
    let _ = run_ui_pass_frame(&ctx, &mut reloaded, vec![]);
    assert!(ctx.memory(|mem| mem.has_focus(latex_id)));
    let _ = run_ui_pass_frame(
        &ctx,
        &mut reloaded,
        vec![chord_key_event(ShortcutChord::cmd(ShortcutKey::H))],
    );
    assert_eq!(reloaded.editor.math(), before_math);
    assert_eq!(reloaded.recent_palette, before_recent);
}

#[test]
fn save_and_palette_buttons_show_shortcuts_only_in_tooltips() {
    for office_mode in [false, true] {
        let ctx = egui::Context::default();
        ThemeTokens::apply_style(&ctx);
        ctx.all_styles_mut(|style| {
            style.interaction.tooltip_delay = 0.0;
            style.interaction.show_tooltips_only_when_still = false;
        });
        let mut app = super::helpers::app();
        if office_mode {
            app.session = Some("office-1".into());
        }

        let out = run_ui_pass_frame(&ctx, &mut app, vec![]);
        let expected_label = if office_mode {
            "Save & Return to Document"
        } else {
            "Save MathML"
        };
        assert!(!has_rendered_text(&out, "(Ctrl+S)"));
        let save_pos = find_text_center(&out, |text| text == expected_label);

        let _ = run_ui_pass_frame(&ctx, &mut app, vec![egui::Event::PointerMoved(save_pos)]);
        let hover_out =
            run_ui_pass_frame(&ctx, &mut app, vec![egui::Event::PointerMoved(save_pos)]);
        assert!(has_rendered_text(&hover_out, "Ctrl+S"));
    }

    // Palette button face omits shortcut text; hover tooltip tracks active binding dynamically.
    let ctx = egui::Context::default();
    ThemeTokens::apply_style(&ctx);
    ctx.all_styles_mut(|style| {
        style.interaction.tooltip_delay = 0.0;
        style.interaction.show_tooltips_only_when_still = false;
    });
    let mut app = super::helpers::app();
    app.category = "Fractions".into();
    let out = run_ui_pass_frame(&ctx, &mut app, vec![]);
    assert!(!has_rendered_text(&out, "Ctrl+F"));

    let atlas = app.palette_atlas.as_ref().expect("palette atlas");
    let atlas_tex = atlas.texture.id();
    let frac_uv = atlas.entries[r"\frac{#?}{#?}"].uv;
    let frac_pos = out
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect_shape)
                if rect_shape.brush.as_ref().is_some_and(|brush| {
                    brush.fill_texture_id == atlas_tex
                        && (brush.uv.min.x - frac_uv.min.x).abs() < 1e-4
                        && (brush.uv.min.y - frac_uv.min.y).abs() < 1e-4
                }) =>
            {
                Some(rect_shape.rect.center())
            }
            _ => None,
        })
        .expect("fraction palette tile");

    let _ = run_ui_pass_frame(&ctx, &mut app, vec![egui::Event::PointerMoved(frac_pos)]);
    let hover_default =
        run_ui_pass_frame(&ctx, &mut app, vec![egui::Event::PointerMoved(frac_pos)]);
    assert!(has_rendered_text(&hover_default, "Fraction (Ctrl+F)"));

    app.shortcuts
        .replace_binding(r"\frac{#?}{#?}", ShortcutChord::cmd_shift(ShortcutKey::F));
    let hover_rebound =
        run_ui_pass_frame(&ctx, &mut app, vec![egui::Event::PointerMoved(frac_pos)]);
    assert!(has_rendered_text(&hover_rebound, "Fraction (Ctrl+Shift+F)"));
    assert!(!has_rendered_text(&hover_rebound, "Fraction (Ctrl+F)"));

    app.shortcuts.clear_binding(r"\frac{#?}{#?}");
    let hover_cleared =
        run_ui_pass_frame(&ctx, &mut app, vec![egui::Event::PointerMoved(frac_pos)]);
    assert!(has_rendered_text(&hover_cleared, "Fraction"));
    assert!(!has_rendered_text(&hover_cleared, "Ctrl+"));
}

#[test]
fn settings_keyboard_shortcuts_card_filters_captures_clears_and_resets() {
    let dir = tempfile::tempdir().expect("tempdir");
    let settings_path = dir.path().join("settings.json");

    let ctx = egui::Context::default();
    ThemeTokens::apply_style(&ctx);
    let mut app = super::helpers::app().with_settings_path(settings_path.clone(), Some(&ctx));
    app.tab = 2;

    for _ in 0..10 {
        run_settings_frame(&ctx, &mut app, vec![]);
    }
    let out = run_settings_frame(&ctx, &mut app, vec![]);

    // Even at compact MIN_WINDOW_SIZE height, the scrollable palette shortcuts table keeps its full
    // height (> 2 rows), stretches across the card with outer margin, and isolates wheel boundaries.
    let compact_ctx = egui::Context::default();
    ThemeTokens::apply_style(&compact_ctx);
    let mut compact_app = super::helpers::app();
    compact_app.tab = 2;
    let outer_hover_pos = Pos2::new(120.0, 280.0);
    let mut time = 0.0;
    let compact_screen = Rect::from_min_size(Pos2::ZERO, Vec2::from(MIN_WINDOW_SIZE));
    for _ in 0..2 {
        time += 0.1;
        compact_ctx
            .run_ui(
                egui::RawInput {
                    screen_rect: Some(compact_screen),
                    time: Some(time),
                    predicted_dt: 1.0 / 60.0,
                    events: vec![egui::Event::PointerMoved(outer_hover_pos)],
                    ..Default::default()
                },
                |ui| compact_app.draw_ui(ui),
            )
            .textures_delta
            .clear();
    }
    let compact_out = run_wheel_and_settle(
        &compact_ctx,
        &mut compact_app,
        compact_screen,
        &mut time,
        outer_hover_pos,
        -480.0,
    );
    let visible_set_buttons = compact_out
        .shapes
        .iter()
        .filter(|clipped| {
            matches!(&clipped.shape, egui::Shape::Text(text_shape)
                if text_shape.galley.text() == "Set shortcut")
        })
        .count();
    assert!(
        visible_set_buttons >= 5,
        "expected >= 5 visible shortcut rows inside Settings list, found {visible_set_buttons}"
    );
    let first_set_pos = find_text_center(&compact_out, |text| text == "Set shortcut");
    let save_eq_pos = find_text_center(&compact_out, |text| text == "Save equation");
    assert!(
        first_set_pos.x > 750.0,
        "shortcut list should stretch horizontally across the card, found button center x={}",
        first_set_pos.x
    );

    // Wheeling up at the top of the right list must not chain to the outer Settings ScrollArea.
    let right_list_pos = Pos2::new(first_set_pos.x, first_set_pos.y);
    let top_boundary_out = run_wheel_and_settle(
        &compact_ctx,
        &mut compact_app,
        compact_screen,
        &mut time,
        right_list_pos,
        240.0,
    );
    let after_top_set_pos = find_text_center(&top_boundary_out, |text| text == "Set shortcut");
    let after_top_save_pos = find_text_center(&top_boundary_out, |text| text == "Save equation");
    assert!(
        (after_top_set_pos.y - first_set_pos.y).abs() < 1.0
            && (after_top_save_pos.y - save_eq_pos.y).abs() < 1.0,
        "expected wheeling up at top of right list not to scroll outer Settings (set: {} -> {}, save_eq: {} -> {})",
        first_set_pos.y,
        after_top_set_pos.y,
        save_eq_pos.y,
        after_top_save_pos.y
    );

    // Scroll the right list all the way to its bottom, then wheel down further: outer Settings must stay fixed.
    let _ = run_wheel_and_settle(
        &compact_ctx,
        &mut compact_app,
        compact_screen,
        &mut time,
        right_list_pos,
        -30000.0,
    );
    let bottom_boundary_out = run_wheel_and_settle(
        &compact_ctx,
        &mut compact_app,
        compact_screen,
        &mut time,
        right_list_pos,
        -240.0,
    );
    let after_bottom_save_pos =
        find_text_center(&bottom_boundary_out, |text| text == "Save equation");
    assert!(
        (after_bottom_save_pos.y - save_eq_pos.y).abs() < 1.0,
        "expected wheeling down at bottom of right list not to scroll outer Settings ({} -> {})",
        save_eq_pos.y,
        after_bottom_save_pos.y
    );

    // Wheeling over the left 30% column still scrolls the outer Settings ScrollArea.
    let left_column_pos = Pos2::new(120.0, save_eq_pos.y);
    let scrolled_up_out = run_wheel_and_settle(
        &compact_ctx,
        &mut compact_app,
        compact_screen,
        &mut time,
        left_column_pos,
        240.0,
    );
    let after_left_save_pos = find_text_center(&scrolled_up_out, |text| text == "Save equation");
    assert!(
        after_left_save_pos.y > save_eq_pos.y + 20.0,
        "expected outer Settings ScrollArea to scroll when wheeling over left 30% column ({} -> {})",
        save_eq_pos.y,
        after_left_save_pos.y
    );

    assert!(!has_rendered_text(&out, "System & Navigation (Fixed)"));
    assert!(!has_rendered_text(&out, "Palette Shortcuts (Customizable)"));
    let title_pos = find_text_center(&out, |text| text == "Keyboard Shortcuts");
    let reset_pos = find_text_center(&out, |text| text == "Reset to defaults");
    let search_pos = find_text_center(&out, |text| text.contains("Search label, LaTeX"));
    let combo_pos = find_text_center(&out, |text| text == "All categories");
    assert!(
        (reset_pos.y - title_pos.y).abs() < 4.0
            && (search_pos.y - title_pos.y).abs() < 4.0
            && (combo_pos.y - title_pos.y).abs() < 4.0
            && title_pos.x < reset_pos.x
            && reset_pos.x < search_pos.x
            && search_pos.x < combo_pos.x
            && combo_pos.x > 800.0,
        "expected Reset left of Search and ComboBox on top row: title={title_pos:?}, reset={reset_pos:?}, search={search_pos:?}, combo={combo_pos:?}"
    );
    let tokens = ThemeTokens::for_mode(ctx.theme() == egui::Theme::Dark);
    let left_frame_rect = out
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect_shape)
                if rect_shape.fill == tokens.panel_bg
                    && rect_shape.corner_radius == theme::CONTROL_CORNER_RADIUS
                    && rect_shape.rect.height() >= 300.0
                    && rect_shape.rect.width() < 350.0 =>
            {
                Some(rect_shape.rect)
            }
            _ => None,
        })
        .expect("system shortcuts framed box");
    let list_frame_rect = out
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect_shape)
                if rect_shape.fill == tokens.panel_bg
                    && rect_shape.corner_radius == theme::CONTROL_CORNER_RADIUS
                    && rect_shape.rect.height() >= 300.0
                    && rect_shape.rect.width() >= 600.0 =>
            {
                Some(rect_shape.rect)
            }
            _ => None,
        })
        .expect("palette shortcuts framed box");
    assert!(
        left_frame_rect.left() < 100.0
            && list_frame_rect.left() > 300.0
            && (left_frame_rect.top() - list_frame_rect.top()).abs() <= 2.0
            && (left_frame_rect.bottom() - list_frame_rect.bottom()).abs() <= 2.0,
        "expected matching 30/70 framed boxes: left={left_frame_rect:?}, right={list_frame_rect:?}"
    );
    let has_bordered_box_or_keycap = out.shapes.iter().any(|clipped| {
        matches!(
            &clipped.shape,
            egui::Shape::Rect(rect_shape)
                if (rect_shape.fill == tokens.panel_bg || rect_shape.fill == tokens.control_bg)
                    && rect_shape.corner_radius == theme::CONTROL_CORNER_RADIUS
                    && rect_shape.stroke.width > 0.0
                    && (rect_shape.rect == left_frame_rect
                        || rect_shape.rect == list_frame_rect
                        || left_frame_rect.contains_rect(rect_shape.rect))
        )
    });
    assert!(
        !has_bordered_box_or_keycap,
        "expected borderless shortcut panels and keycap badges"
    );
    for (left_label, shortcut_text) in [
        ("Save equation", "Ctrl+S"),
        ("Undo", "Ctrl+Z"),
        ("Redo", "Ctrl+Y / Ctrl+Shift+Z"),
        ("Select all", "Ctrl+A"),
        ("Copy", "Ctrl+C"),
        ("Cut", "Ctrl+X"),
        ("Paste", "Ctrl+V"),
        ("Next placeholder", "Tab"),
        ("Previous placeholder", "Shift+Tab"),
        ("Close popup / Exit", "Esc"),
    ] {
        let label_pos = find_text_center(&out, |text| text == left_label);
        let key_pos = find_text_center(&out, |text| text == shortcut_text);
        assert!(
            label_pos.x < key_pos.x
                && key_pos.x < left_frame_rect.right()
                && left_frame_rect.right() < list_frame_rect.left()
                && (label_pos.y - key_pos.y).abs() < 4.0,
            "expected `{left_label}` left-aligned and `{shortcut_text}` right-aligned in left box: label={label_pos:?}, key={key_pos:?}, left_box={left_frame_rect:?}"
        );
    }

    for expected in [
        "Keyboard Shortcuts",
        "Save equation",
        "Ctrl+S",
        "Undo",
        "Ctrl+Z",
        "Ctrl+Y / Ctrl+Shift+Z",
        "Copy",
        "Cut",
        "Paste",
        "Ctrl+C",
        "Ctrl+X",
        "Ctrl+V",
        "Next placeholder",
        "Previous placeholder",
        "Tab",
        "Shift+Tab",
        "Reset to defaults",
    ] {
        assert!(has_rendered_text(&out, expected), "missing `{expected}`");
    }

    // 1. Search & Assigned-only filter narrow rows cleanly.
    app.shortcut_filter = ShortcutCategoryFilter::AssignedOnly;
    app.shortcut_search = "Binomial".into();
    let empty_filtered = run_settings_frame(&ctx, &mut app, vec![]);
    assert!(has_rendered_text(
        &empty_filtered,
        "No palette items match the current filter."
    ));

    assert!(!has_rendered_text(&out, "Display fraction"));
    assert!(has_rendered_text(&out, r"\frac{#?}{#?}"));
    let clear_pos_unfiltered = find_text_center(&out, |text| text == "×");
    let track = scroll_tracks(&out, tokens.extreme_bg, 8.0)
        .into_iter()
        .find(|track| list_frame_rect.contains_rect(*track))
        .expect("scrollbar inside the overflowing list");
    assert!(
        clear_pos_unfiltered.x + 14.0 <= track.left(),
        "clear [×] button at {clear_pos_unfiltered:?} overlaps track {track:?}"
    );

    app.shortcut_filter = ShortcutCategoryFilter::All;
    for _ in 0..10 {
        run_settings_frame(&ctx, &mut app, vec![]);
    }
    let filtered = run_settings_frame(&ctx, &mut app, vec![]);
    assert!(scroll_tracks(&filtered, tokens.extreme_bg, 8.0)
        .iter()
        .all(|track| !list_frame_rect.contains_rect(*track)));
    assert!(has_rendered_text(&filtered, r"\binom{#?}{#?}"));
    assert!(!has_rendered_text(&filtered, r"\sqrt{#?}"));

    // 2. Click "Set shortcut" on Binomial, reject bare keys, cancel on Esc, then bind Ctrl+Alt+V.
    let set_pos = find_text_center(&filtered, |text| text == "Set shortcut");
    let _ = run_settings_frame(&ctx, &mut app, click_events(set_pos));
    let recording_out = run_settings_frame(&ctx, &mut app, vec![]);
    assert_eq!(app.shortcut_capture, Some(r"\binom{#?}{#?}"));
    assert!(has_rendered_text(&recording_out, "Press shortcut…"));

    for bad_event in [
        plain_key_event(egui::Key::F, false),
        plain_key_event(egui::Key::F, true),
        plain_key_event(egui::Key::Slash, false),
        plain_key_event(egui::Key::ArrowLeft, false),
    ] {
        let invalid_out = run_settings_frame(&ctx, &mut app, vec![bad_event]);
        assert_eq!(app.shortcut_capture, Some(r"\binom{#?}{#?}"));
        assert!(app.shortcut_capture_error.is_some());
        assert!(has_rendered_text(
            &invalid_out,
            "Press Ctrl, Alt, or F1–F12"
        ));
        assert_eq!(app.shortcuts.binding_for(r"\binom{#?}{#?}"), None);
    }

    let _ = run_settings_frame(
        &ctx,
        &mut app,
        vec![plain_key_event(egui::Key::Escape, false)],
    );
    assert_eq!(app.shortcut_capture, None);
    assert_eq!(app.shortcuts.binding_for(r"\binom{#?}{#?}"), None);

    // Re-enter capture and bind Ctrl+Alt+V cleanly.
    let _ = run_settings_frame(&ctx, &mut app, click_events(set_pos));
    let alt_v = ShortcutChord::new(true, true, false, ShortcutKey::V);
    let _ = run_settings_frame(&ctx, &mut app, vec![chord_key_event(alt_v)]);
    assert_eq!(app.shortcut_capture, None);
    assert_eq!(app.shortcut_conflict.as_ref().map(|_| ()), None);
    assert_eq!(app.shortcuts.binding_for(r"\binom{#?}{#?}"), Some(alt_v));

    // Verify immediate disk persistence.
    let persisted = Settings::load_from_path(&settings_path);
    assert_eq!(
        persisted
            .palette_shortcuts
            .get(r"\binom{#?}{#?}")
            .and_then(|val| val.as_deref()),
        Some("Ctrl+Alt+V")
    );

    // 3. Backspace during capture clears the binding immediately.
    let bound_out = run_settings_frame(&ctx, &mut app, vec![]);
    let bound_pos = find_text_center(&bound_out, |text| text == "Ctrl+Alt+V");
    let _ = run_settings_frame(&ctx, &mut app, click_events(bound_pos));
    let _ = run_settings_frame(
        &ctx,
        &mut app,
        vec![plain_key_event(egui::Key::Backspace, false)],
    );
    assert_eq!(app.shortcut_capture, None);
    assert_eq!(app.shortcuts.binding_for(r"\binom{#?}{#?}"), None);

    // 4. Clicking clear [×] on Fraction clears its default Ctrl+F, and Reset to defaults restores it.
    app.shortcut_search = "Fraction".into();
    let frac_out = run_settings_frame(&ctx, &mut app, vec![]);
    let clear_pos = find_text_center(&frac_out, |text| text == "×");
    let _ = run_settings_frame(&ctx, &mut app, click_events(clear_pos));
    assert_eq!(app.shortcuts.binding_for(r"\frac{#?}{#?}"), None);

    let reset_out = run_settings_frame(&ctx, &mut app, vec![]);
    let reset_pos = find_text_center(&reset_out, |text| text == "Reset to defaults");
    let _ = run_settings_frame(&ctx, &mut app, click_events(reset_pos));
    assert!(app.shortcuts.is_default());
    assert!(Settings::load_from_path(&settings_path)
        .palette_shortcuts
        .is_empty());

    // 5. Pressing Ctrl+S while recording suppresses global Save and opens Shortcut reserved modal.
    app.session = Some("office-1".into());
    app.editor.insert_text("x");
    app.mark_canvas_changed();
    let frac_default_out = run_settings_frame(&ctx, &mut app, vec![]);
    let frac_btn_pos = find_text_center(&frac_default_out, |text| text == "Ctrl+F");
    let _ = run_settings_frame(&ctx, &mut app, click_events(frac_btn_pos));
    assert_eq!(app.shortcut_capture, Some(r"\frac{#?}{#?}"));

    let _ = run_settings_frame(&ctx, &mut app, vec![save_key(false)]);
    let conflict_out = run_settings_frame(&ctx, &mut app, vec![]);
    assert!(!app.busy, "Ctrl+S during capture must NOT trigger Save");
    assert_eq!(app.shortcut_capture, None);
    assert!(matches!(
        app.shortcut_conflict.as_ref().map(|dialog| dialog.conflict),
        Some(ShortcutConflict::System("Save equation"))
    ));
    assert!(has_rendered_text(&conflict_out, "Shortcut reserved"));
}

#[test]
fn shortcut_conflict_modal_supports_system_cancel_replace_swap_and_keyboard_navigation() {
    let dir = tempfile::tempdir().expect("tempdir");
    let settings_path = dir.path().join("settings.json");

    // 1. System conflict with Ctrl+C (Copy event) opens Cancel-only modal, ignores backdrop clicks, and closes on Esc.
    {
        let ctx = egui::Context::default();
        ThemeTokens::apply_style(&ctx);
        let mut app = super::helpers::app().with_settings_path(settings_path.clone(), Some(&ctx));
        app.tab = 2;
        app.shortcut_search = "Binomial".into();

        let out = run_settings_frame(&ctx, &mut app, vec![]);
        let set_pos = find_text_center(&out, |text| text == "Set shortcut");
        let _ = run_settings_frame(&ctx, &mut app, click_events(set_pos));
        let _ = run_settings_frame(&ctx, &mut app, vec![egui::Event::Copy]);
        let modal_out = run_settings_frame(&ctx, &mut app, vec![]);
        assert!(matches!(
            app.shortcut_conflict.as_ref().map(|dialog| dialog.conflict),
            Some(ShortcutConflict::System("Copy"))
        ));
        assert!(has_rendered_text(&modal_out, "Shortcut reserved"));
        assert!(!has_rendered_text(&modal_out, "Replace"));
        assert!(!has_rendered_text(&modal_out, "Swap"));

        // Backdrop click outside modal keeps it open.
        let _ = run_settings_frame(&ctx, &mut app, click_events(Pos2::new(8.0, 8.0)));
        assert!(app.shortcut_conflict.is_some());

        // Escape dismisses modal cleanly.
        let _ = run_settings_frame(
            &ctx,
            &mut app,
            vec![plain_key_event(egui::Key::Escape, false)],
        );
        assert!(app.shortcut_conflict.is_none());
        assert_eq!(app.shortcuts.binding_for(r"\binom{#?}{#?}"), None);
    }

    // 2. Palette conflict on unassigned entry offers Replace + Cancel (no Swap); Cancel keeps old binding, Replace reassigns.
    {
        let ctx = egui::Context::default();
        ThemeTokens::apply_style(&ctx);
        let mut app = super::helpers::app().with_settings_path(settings_path.clone(), Some(&ctx));
        app.tab = 2;
        app.shortcut_search = "Binomial".into();

        let out = run_settings_frame(&ctx, &mut app, vec![]);
        let set_pos = find_text_center(&out, |text| text == "Set shortcut");
        let _ = run_settings_frame(&ctx, &mut app, click_events(set_pos));
        let _ = run_settings_frame(
            &ctx,
            &mut app,
            vec![chord_key_event(ShortcutChord::cmd(ShortcutKey::F))],
        );
        let modal_out = run_settings_frame(&ctx, &mut app, vec![]);
        assert!(has_rendered_text(&modal_out, "Shortcut already in use"));
        assert!(has_rendered_text(&modal_out, "Replace"));
        assert!(has_rendered_text(&modal_out, "Cancel"));
        assert!(!has_rendered_text(&modal_out, "Swap"));

        // Cancel leaves both unchanged.
        let cancel_pos = find_text_center(&modal_out, |text| text == "Cancel");
        let _ = run_settings_frame(&ctx, &mut app, click_events(cancel_pos));
        let _ = run_settings_frame(&ctx, &mut app, vec![]);
        assert!(app.shortcut_conflict.is_none());
        assert_eq!(
            app.shortcuts.binding_for(r"\frac{#?}{#?}"),
            Some(ShortcutChord::cmd(ShortcutKey::F))
        );
        assert_eq!(app.shortcuts.binding_for(r"\binom{#?}{#?}"), None);

        // Re-trigger conflict and confirm with Enter (initial focus is Replace).
        let _ = run_settings_frame(&ctx, &mut app, click_events(set_pos));
        let _ = run_settings_frame(
            &ctx,
            &mut app,
            vec![chord_key_event(ShortcutChord::cmd(ShortcutKey::F))],
        );
        let _ = run_settings_frame(&ctx, &mut app, vec![]);
        let _ = run_settings_frame(
            &ctx,
            &mut app,
            vec![plain_key_event(egui::Key::Enter, false)],
        );
        assert!(app.shortcut_conflict.is_none());
        assert_eq!(
            app.shortcuts.binding_for(r"\binom{#?}{#?}"),
            Some(ShortcutChord::cmd(ShortcutKey::F))
        );
        assert_eq!(app.shortcuts.binding_for(r"\frac{#?}{#?}"), None);
    }

    // 3. Palette conflict between two assigned entries supports Swap and traps Tab/Arrow keyboard focus.
    {
        let ctx = egui::Context::default();
        ThemeTokens::apply_style(&ctx);
        let mut app = super::helpers::app();
        app.shortcuts = crate::shortcuts::ShortcutMap::default_map();
        app.tab = 2;
        app.shortcut_search = "Fraction".into();

        let out = run_settings_frame(&ctx, &mut app, vec![]);
        let frac_pos = find_text_center(&out, |text| text == "Ctrl+F");
        let _ = run_settings_frame(&ctx, &mut app, click_events(frac_pos));
        let _ = run_settings_frame(
            &ctx,
            &mut app,
            vec![chord_key_event(ShortcutChord::cmd(ShortcutKey::R))],
        );
        let modal_out = run_settings_frame(&ctx, &mut app, vec![]);
        assert!(has_rendered_text(&modal_out, "Swap"));
        assert!(has_rendered_text(&modal_out, "Replace"));
        assert!(has_rendered_text(&modal_out, "Cancel"));

        // Verify button height aligns with theme::BUTTON_HEIGHT (30px).
        let swap_pos = find_text_center(&modal_out, |text| text == "Swap");
        let swap_rect = modal_out
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(rect_shape)
                    if rect_shape.brush.is_none()
                        && rect_shape.rect.contains(swap_pos)
                        && rect_shape.rect.height() <= 40.0 =>
                {
                    Some(rect_shape.rect)
                }
                _ => None,
            })
            .expect("Swap button rect");
        assert!((swap_rect.height() - theme::BUTTON_HEIGHT).abs() <= 0.5);

        // Settle focus: initial focus lands on Replace -> Tab -> Cancel -> Tab -> Swap.
        let _ = run_settings_frame(&ctx, &mut app, vec![]);
        let replace_id = ctx
            .memory(|mem| mem.focused())
            .expect("Replace initial focus");
        let _ = run_settings_frame(&ctx, &mut app, vec![plain_key_event(egui::Key::Tab, false)]);
        let cancel_id = ctx.memory(|mem| mem.focused()).expect("Cancel focus");
        assert_ne!(cancel_id, replace_id);

        let _ = run_settings_frame(&ctx, &mut app, vec![plain_key_event(egui::Key::Tab, false)]);
        let swap_id = ctx.memory(|mem| mem.focused()).expect("Swap focus");
        assert_ne!(swap_id, replace_id);
        assert_ne!(swap_id, cancel_id);

        // ArrowRight moves Swap -> Replace, ArrowLeft moves Replace -> Swap, Shift+Tab wraps.
        let _ = run_settings_frame(
            &ctx,
            &mut app,
            vec![plain_key_event(egui::Key::ArrowRight, false)],
        );
        assert_eq!(ctx.memory(|mem| mem.focused()), Some(replace_id));
        let _ = run_settings_frame(
            &ctx,
            &mut app,
            vec![plain_key_event(egui::Key::ArrowLeft, false)],
        );
        assert_eq!(ctx.memory(|mem| mem.focused()), Some(swap_id));
        let _ = run_settings_frame(&ctx, &mut app, vec![plain_key_event(egui::Key::Tab, true)]);
        assert_eq!(ctx.memory(|mem| mem.focused()), Some(cancel_id));
        let _ = run_settings_frame(
            &ctx,
            &mut app,
            vec![plain_key_event(egui::Key::ArrowRight, false)],
        );
        assert_eq!(ctx.memory(|mem| mem.focused()), Some(swap_id));

        // Activate Swap with Enter.
        let _ = run_settings_frame(
            &ctx,
            &mut app,
            vec![plain_key_event(egui::Key::Enter, false)],
        );
        assert!(app.shortcut_conflict.is_none());
        assert_eq!(
            app.shortcuts.binding_for(r"\frac{#?}{#?}"),
            Some(ShortcutChord::cmd(ShortcutKey::R))
        );
        assert_eq!(
            app.shortcuts.binding_for(r"\sqrt{#?}"),
            Some(ShortcutChord::cmd(ShortcutKey::F))
        );
    }
}

#[test]
fn global_egui_zoom_and_quit_shortcuts_are_disabled_and_rebindable() {
    let direct_ctx = egui::Context::default();
    App::configure_egui_options(&direct_ctx);
    direct_ctx.options(|opts| {
        assert!(!opts.zoom_with_keyboard);
        assert!(opts.quit_shortcuts.is_empty());
    });

    let ctx = egui::Context::default();
    let mut app = super::helpers::app();
    app.editor.insert_text("x");
    app.mark_canvas_changed();
    let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
    ctx.options(|opts| {
        assert!(!opts.zoom_with_keyboard);
        assert!(opts.quit_shortcuts.is_empty());
    });

    let ctrl_key = |key: egui::Key, shift: bool, alt: bool| egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers {
            ctrl: true,
            command: true,
            shift,
            alt,
            ..Default::default()
        },
    };

    for (key, shift, alt) in [
        (egui::Key::Minus, false, false),
        (egui::Key::Minus, true, false),
        (egui::Key::Minus, false, true),
        (egui::Key::Plus, false, false),
        (egui::Key::Plus, true, false),
        (egui::Key::Equals, false, false),
        (egui::Key::Equals, true, false),
        (egui::Key::Num0, false, false),
        (egui::Key::Num0, false, true),
    ] {
        let _ = run_ui_pass_frame(&ctx, &mut app, vec![ctrl_key(key, shift, alt)]);
        assert!(
            (ctx.zoom_factor() - 1.0).abs() < f32::EPSILON,
            "UI zoom_factor changed after {key:?} (shift={shift}, alt={alt})"
        );
    }

    for (shift, alt) in [(false, false), (true, false), (false, true)] {
        let out = run_ui_pass_frame(&ctx, &mut app, vec![ctrl_key(egui::Key::Q, shift, alt)]);
        assert!(
            out.viewport_output
                .values()
                .all(|viewport| !viewport.commands.contains(&egui::ViewportCommand::Close)),
            "Ctrl+Q (shift={shift}, alt={alt}) must not emit ViewportCommand::Close"
        );
        assert!(app.pending.is_none());
        assert!(!app.allow_close);
    }

    // Custom bindings on Ctrl+Minus, Ctrl+Equals, and Ctrl+Q execute on focused Equation Canvas.
    app.shortcuts
        .replace_binding(r"\alpha", ShortcutChord::cmd(ShortcutKey::Minus));
    app.shortcuts
        .replace_binding(r"\beta", ShortcutChord::cmd(ShortcutKey::Equals));
    app.shortcuts
        .replace_binding(r"\gamma", ShortcutChord::cmd(ShortcutKey::Q));

    for (key, expected_source) in [
        (egui::Key::Minus, r"\alpha"),
        (egui::Key::Equals, r"\beta"),
        (egui::Key::Q, r"\gamma"),
    ] {
        let out = run_ui_pass_frame(&ctx, &mut app, vec![ctrl_key(key, false, false)]);
        assert!((ctx.zoom_factor() - 1.0).abs() < f32::EPSILON);
        assert!(out
            .viewport_output
            .values()
            .all(|viewport| !viewport.commands.contains(&egui::ViewportCommand::Close)));
        assert_eq!(app.recent_palette.first().copied(), Some(expected_source));
    }

    // Ctrl+Minus, Ctrl+Equals, and Ctrl+Q can still be recorded in Settings.
    let capture_ctx = egui::Context::default();
    let mut capture_app = super::helpers::app();
    capture_app.tab = 2;
    capture_app.shortcut_search = "Binomial".into();
    let _ = run_settings_frame(&capture_ctx, &mut capture_app, vec![]);
    for (key, expected_key) in [
        (egui::Key::Minus, ShortcutKey::Minus),
        (egui::Key::Equals, ShortcutKey::Equals),
        (egui::Key::Q, ShortcutKey::Q),
    ] {
        let out = run_settings_frame(&capture_ctx, &mut capture_app, vec![]);
        let btn_label = capture_app
            .shortcuts
            .binding_for(r"\binom{#?}{#?}")
            .map_or_else(|| "Set shortcut".to_owned(), |chord| chord.to_string());
        let btn_pos = find_text_center(&out, |text| text == btn_label);
        let _ = run_settings_frame(&capture_ctx, &mut capture_app, click_events(btn_pos));
        assert_eq!(capture_app.shortcut_capture, Some(r"\binom{#?}{#?}"));
        let rec_out = run_settings_frame(
            &capture_ctx,
            &mut capture_app,
            vec![ctrl_key(key, false, false)],
        );
        assert!(rec_out
            .viewport_output
            .values()
            .all(|viewport| !viewport.commands.contains(&egui::ViewportCommand::Close)));
        assert!((capture_ctx.zoom_factor() - 1.0).abs() < f32::EPSILON);
        assert_eq!(
            capture_app.shortcuts.binding_for(r"\binom{#?}{#?}"),
            Some(ShortcutChord::cmd(expected_key))
        );
    }
}

#[test]
fn text_edit_readline_chords_suppressed_outside_canvas_but_allowed_on_canvas_and_capture() {
    let ctrl_letter = |key: egui::Key| egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        },
    };
    let ctrl_key = |key: egui::Key| egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        },
    };

    // 1. LaTeX source: Ctrl+H/K/U/W leave text & cursor unchanged; Ctrl+ArrowLeft & Ctrl+Backspace still work.
    {
        let ctx = egui::Context::default();
        let mut app = super::helpers::app();
        let latex_id = App::editor_focus_id(StartupFocus::Latex);
        app.latex = "alpha beta".into();
        let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
        ctx.memory_mut(|mem| mem.request_focus(latex_id));
        let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
        assert!(ctx.memory(|mem| mem.has_focus(latex_id)));

        let mut state = egui::TextEdit::load_state(&ctx, latex_id).expect("latex text state");
        let mid_cursor = egui::text::CCursor::new(5);
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::one(mid_cursor)));
        state.store(&ctx, latex_id);

        for key in [egui::Key::H, egui::Key::K, egui::Key::U, egui::Key::W] {
            let _ = run_ui_pass_frame(&ctx, &mut app, vec![ctrl_letter(key)]);
            assert_eq!(app.latex, "alpha beta", "Ctrl+{key:?} mutated LaTeX source");
            let cursor_after = egui::TextEdit::load_state(&ctx, latex_id)
                .and_then(|loaded| loaded.cursor.char_range())
                .map(|range| usize::from(range.primary.index));
            assert_eq!(
                cursor_after,
                Some(5),
                "Ctrl+{key:?} altered LaTeX source cursor"
            );
        }

        // Standard editing: Ctrl+ArrowLeft moves back over "alpha" (index 0), and Ctrl+Backspace deletes word.
        let _ = run_ui_pass_frame(&ctx, &mut app, vec![ctrl_key(egui::Key::ArrowLeft)]);
        let moved_cursor = egui::TextEdit::load_state(&ctx, latex_id)
            .and_then(|loaded| loaded.cursor.char_range())
            .map(|range| usize::from(range.primary.index));
        assert_eq!(moved_cursor, Some(0));

        let mut end_state = egui::TextEdit::load_state(&ctx, latex_id).expect("latex text state");
        end_state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::one(
                egui::text::CCursor::new("alpha beta".len()),
            )));
        end_state.store(&ctx, latex_id);
        let _ = run_ui_pass_frame(&ctx, &mut app, vec![ctrl_key(egui::Key::Backspace)]);
        assert_eq!(app.latex, "alpha ");
    }

    // 2. Settings shortcut_search: Ctrl+H/K/U/W leave query unchanged.
    {
        let ctx = egui::Context::default();
        let mut app = super::helpers::app();
        app.tab = 2;
        app.shortcut_search = "alpha beta".into();
        let out = run_settings_frame(&ctx, &mut app, vec![]);
        let search_pos = find_text_center(&out, |text| text == "alpha beta");
        let _ = run_settings_frame(&ctx, &mut app, click_events(search_pos));
        for key in [egui::Key::H, egui::Key::K, egui::Key::U, egui::Key::W] {
            let _ = run_settings_frame(&ctx, &mut app, vec![ctrl_letter(key)]);
            assert_eq!(
                app.shortcut_search, "alpha beta",
                "Ctrl+{key:?} mutated shortcut_search"
            );
        }
    }

    // 3. Inline \command popup input: Ctrl+H/K/U/W leave command buffer unchanged.
    {
        let ctx = egui::Context::default();
        let mut app = super::helpers::app();
        let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
        let _ = run_ui_pass_frame(&ctx, &mut app, vec![egui::Event::Text(r"\alpha".into())]);
        let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
        let cmd_id = egui::Id::unique("canvas-command-input");
        assert!(ctx.memory(|mem| mem.has_focus(cmd_id)));
        assert_eq!(
            app.command_input.as_ref().map(|cmd| cmd.name.as_str()),
            Some("alpha")
        );

        let mut cmd_state = egui::TextEdit::load_state(&ctx, cmd_id).expect("cmd text state");
        cmd_state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::one(
                egui::text::CCursor::new(2),
            )));
        cmd_state.store(&ctx, cmd_id);

        for key in [egui::Key::H, egui::Key::K, egui::Key::U, egui::Key::W] {
            let _ = run_ui_pass_frame(&ctx, &mut app, vec![ctrl_letter(key)]);
            assert_eq!(
                app.command_input.as_ref().map(|cmd| cmd.name.as_str()),
                Some("alpha"),
                "Ctrl+{key:?} mutated inline command input"
            );
        }
    }

    // 4. Focused Equation Canvas still dispatches default Ctrl+H and custom Ctrl+K/U/W bindings.
    {
        let ctx = egui::Context::default();
        let mut app = super::helpers::app();
        app.shortcuts
            .replace_binding(r"\alpha", ShortcutChord::cmd(ShortcutKey::K));
        app.shortcuts
            .replace_binding(r"\beta", ShortcutChord::cmd(ShortcutKey::U));
        app.shortcuts
            .replace_binding(r"\gamma", ShortcutChord::cmd(ShortcutKey::W));

        let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
        let _ = run_ui_pass_frame(&ctx, &mut app, vec![ctrl_letter(egui::Key::H)]);
        assert_eq!(app.editor.math().children[0].tag, "msup");
        assert_eq!(app.recent_palette.first().copied(), Some(r"^{#?}"));

        for (key, expected_source) in [
            (egui::Key::K, r"\alpha"),
            (egui::Key::U, r"\beta"),
            (egui::Key::W, r"\gamma"),
        ] {
            let _ = run_ui_pass_frame(&ctx, &mut app, vec![ctrl_letter(key)]);
            assert_eq!(app.recent_palette.first().copied(), Some(expected_source));
        }
    }

    // 5. Settings shortcut recorder still captures Ctrl+H, Ctrl+K, Ctrl+U, and Ctrl+W.
    {
        let ctx = egui::Context::default();
        let mut app = super::helpers::app();
        app.tab = 2;
        app.shortcut_search = "Binomial".into();
        let _ = run_settings_frame(&ctx, &mut app, vec![]);

        for (key, expected_key) in [
            (egui::Key::K, ShortcutKey::K),
            (egui::Key::U, ShortcutKey::U),
            (egui::Key::W, ShortcutKey::W),
        ] {
            let out = run_settings_frame(&ctx, &mut app, vec![]);
            let btn_label = app
                .shortcuts
                .binding_for(r"\binom{#?}{#?}")
                .map_or_else(|| "Set shortcut".to_owned(), |chord| chord.to_string());
            let btn_pos = find_text_center(&out, |text| text == btn_label);
            let _ = run_settings_frame(&ctx, &mut app, click_events(btn_pos));
            assert_eq!(app.shortcut_capture, Some(r"\binom{#?}{#?}"));
            let _ = run_settings_frame(&ctx, &mut app, vec![ctrl_letter(key)]);
            assert_eq!(
                app.shortcuts.binding_for(r"\binom{#?}{#?}"),
                Some(ShortcutChord::cmd(expected_key))
            );
        }

        // Ctrl+H triggers conflict modal against default Superscript binding instead of being suppressed.
        let out = run_settings_frame(&ctx, &mut app, vec![]);
        let btn_pos = find_text_center(&out, |text| text == "Ctrl+W");
        let _ = run_settings_frame(&ctx, &mut app, click_events(btn_pos));
        assert_eq!(app.shortcut_capture, Some(r"\binom{#?}{#?}"));
        let _ = run_settings_frame(&ctx, &mut app, vec![ctrl_letter(egui::Key::H)]);
        assert_eq!(app.shortcut_capture, None);
        assert!(matches!(
            app.shortcut_conflict.as_ref().map(|dialog| dialog.conflict),
            Some(ShortcutConflict::Palette(r"^{#?}"))
        ));
    }
}

#[test]
fn settings_recorder_rejects_preserved_editing_clipboard_and_navigation_shortcuts() {
    let ctx = egui::Context::default();
    let mut app = super::helpers::app();
    app.tab = 2;
    app.shortcut_search = "Binomial".into();

    let out = run_settings_frame(&ctx, &mut app, vec![]);
    let set_pos = find_text_center(&out, |text| text == "Set shortcut");
    let _ = run_settings_frame(&ctx, &mut app, click_events(set_pos));
    assert_eq!(app.shortcut_capture, Some(r"\binom{#?}{#?}"));

    let key_event = |key: egui::Key, ctrl: bool, shift: bool| egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers {
            ctrl,
            command: ctrl,
            shift,
            ..Default::default()
        },
    };

    for (key, ctrl, shift) in [
        (egui::Key::ArrowLeft, true, false),
        (egui::Key::ArrowRight, true, false),
        (egui::Key::Backspace, true, false),
        (egui::Key::Delete, true, false),
        (egui::Key::Home, true, false),
        (egui::Key::End, true, false),
        (egui::Key::Insert, true, false),
        (egui::Key::Insert, false, true),
        (egui::Key::Delete, false, true),
        (egui::Key::Enter, false, false),
        (egui::Key::Space, false, false),
    ] {
        let invalid_out = run_settings_frame(&ctx, &mut app, vec![key_event(key, ctrl, shift)]);
        assert_eq!(app.shortcut_capture, Some(r"\binom{#?}{#?}"));
        assert!(has_rendered_text(
            &invalid_out,
            "Reserved for text editing, clipboard, or navigation"
        ));
        assert_eq!(app.shortcuts.binding_for(r"\binom{#?}{#?}"), None);
    }

    for shift in [false, true] {
        if app.shortcut_capture.is_none() {
            let refresh_out = run_settings_frame(&ctx, &mut app, vec![]);
            let fresh_pos = find_text_center(&refresh_out, |text| text == "Set shortcut");
            let _ = run_settings_frame(&ctx, &mut app, click_events(fresh_pos));
        }
        let _ = run_settings_frame(
            &ctx,
            &mut app,
            vec![key_event(egui::Key::Tab, false, shift)],
        );
        assert_eq!(app.shortcuts.binding_for(r"\binom{#?}{#?}"), None);
    }

    for (clipboard_event, expected_label) in [
        (egui::Event::Copy, "Copy"),
        (egui::Event::Cut, "Cut"),
        (egui::Event::Paste("x".into()), "Paste"),
    ] {
        if app.shortcut_capture.is_none() {
            let refresh_out = run_settings_frame(&ctx, &mut app, vec![]);
            let fresh_pos = find_text_center(&refresh_out, |text| text == "Set shortcut");
            let _ = run_settings_frame(&ctx, &mut app, click_events(fresh_pos));
        }
        let _ = run_settings_frame(&ctx, &mut app, vec![clipboard_event]);
        assert!(
            matches!(
                app.shortcut_conflict.as_ref().map(|dialog| dialog.conflict),
                Some(ShortcutConflict::System(label)) if label == expected_label
            ),
            "expected System({expected_label}), got {:?}, capture={:?}",
            app.shortcut_conflict.as_ref().map(|dialog| dialog.conflict),
            app.shortcut_capture
        );
        assert_eq!(app.shortcuts.binding_for(r"\binom{#?}{#?}"), None);
        app.shortcut_conflict = None;
    }
}
