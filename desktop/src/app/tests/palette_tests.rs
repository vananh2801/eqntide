use super::{super::*, helpers::click_events};

fn frame(
    ctx: &egui::Context,
    app: &mut App,
    width: f32,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                egui::Pos2::ZERO,
                Vec2::new(width, 560.0),
            )),
            events,
            ..Default::default()
        },
        |ui| app.draw_palette(ui),
    );
    output.textures_delta.clear();
    output
}

fn tile(output: &egui::FullOutput, app: &App, source: &str) -> (Rect, Rect) {
    let atlas = app.palette_atlas.as_ref().expect("palette atlas");
    let uv = atlas.entries[source].uv;
    output
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape)
                if shape.brush.as_ref().is_some_and(|brush| {
                    brush.fill_texture_id == atlas.texture.id() && brush.uv == uv
                }) =>
            {
                Some((shape.rect, clipped.clip_rect))
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing palette tile {source}"))
}

fn ui_icon_rects(output: &egui::FullOutput, app: &App, name: &str) -> Vec<Rect> {
    let texture = app.ui_icon_texture_id(name).expect("palette UI icon");
    output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape)
                if shape
                    .brush
                    .as_ref()
                    .is_some_and(|brush| brush.fill_texture_id == texture) =>
            {
                Some(shape.rect)
            }
            _ => None,
        })
        .collect()
}

fn press(pos: egui::Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    }
}

#[test]
fn palette_fills_columns_in_catalog_and_recent_order() {
    let ctx = egui::Context::default();
    let mut app = super::helpers::app();
    app.category = "MatricesAndBoxes".into();
    let items = palette::items_for_ui_group(&app.category, &[]);
    let recent = [r"\alpha", r"\beta", r"\gamma", r"\delta"];
    app.recent_palette = recent.to_vec();

    let check_first_four = |out: &egui::FullOutput, app: &App, sources: [&str; 4]| {
        let [first, second, third, fourth] =
            sources.map(|source| tile(out, app, source).0.center());
        let row_stride = PALETTE_BUTTON_SIZE.y + PALETTE_GAP;
        let column_stride = PALETTE_BUTTON_SIZE.x + PALETTE_GAP;
        assert_eq!(second - first, Vec2::new(0.0, row_stride));
        assert_eq!(third - first, Vec2::new(0.0, 2.0 * row_stride));
        assert_eq!(fourth - first, Vec2::new(column_stride, 0.0));
    };

    for width in [240.0, MIN_WINDOW_SIZE[0]] {
        let out = frame(&ctx, &mut app, width, vec![]);
        check_first_four(&out, &app, std::array::from_fn(|index| items[index].2));
    }
    let out = frame(&ctx, &mut app, MIN_WINDOW_SIZE[0], vec![]);
    let origin = tile(&out, &app, items[0].2).0.center();
    let index = items.len();
    let custom_matrix = ui_icon_rects(&out, &app, "dots")[0].center();
    assert_eq!(
        custom_matrix - origin,
        Vec2::new(
            (index / 3) as f32 * (PALETTE_BUTTON_SIZE.x + PALETTE_GAP),
            (index % 3) as f32 * (PALETTE_BUTTON_SIZE.y + PALETTE_GAP)
        )
    );
    app.category = palette::RECENT_GROUP.into();
    let out = frame(&ctx, &mut app, MIN_WINDOW_SIZE[0], vec![]);
    check_first_four(&out, &app, recent);
}

#[test]
fn overflowing_palette_has_the_same_height_as_a_fitting_category() {
    for dpi in [1.0, 1.25, 2.0] {
        let ctx = egui::Context::default();
        ctx.set_pixels_per_point(dpi);
        let mut app = super::helpers::app();
        app.category = "Fractions".into();
        let _ = super::helpers::run_ui_pass_frame(&ctx, &mut app, vec![]);
        let canvas_id = egui::Id::unique("equation-canvas");
        let top = ctx.read_response(canvas_id).expect("canvas").rect.top();

        app.category = "LogicSetsSymbols".into();
        let overflow = super::helpers::run_ui_pass_frame(&ctx, &mut app, vec![]);
        let items = palette::items_for_ui_group(&app.category, &[]);
        let (_, first_clip) = tile(&overflow, &app, items[0].2);
        let (third, _) = tile(&overflow, &app, items[2].2);
        assert!(first_clip.contains_rect(third));
        assert!(app.ui_icon_texture_id("chevron-left").is_none());
        assert_eq!(
            ctx.read_response(canvas_id).expect("canvas").rect.top(),
            top
        );
    }
}

#[test]
fn palette_uses_the_full_width_and_resets_scroll_when_resized_to_fit() {
    let ctx = egui::Context::default();
    let mut app = super::helpers::app();
    app.category = "MatricesAndBoxes".into();
    let items = palette::items_for_ui_group(&app.category, &[]);
    let width = 240.0;
    let initial = frame(&ctx, &mut app, width, vec![]);
    let (first, clip) = tile(&initial, &app, items[0].2);
    assert_eq!(clip.left(), 0.0);
    assert_eq!(clip.right(), width);
    assert!((first.left() - clip.left()).abs() <= 3.0);

    let at_end = scroll_to_palette_end(&ctx, &mut app, width);
    let (_, clip) = tile(&at_end, &app, items.last().expect("matrix items").2);
    assert!(clip.contains_rect(ui_icon_rects(&at_end, &app, "dots")[0]));

    let _ = frame(&ctx, &mut app, MIN_WINDOW_SIZE[0], vec![]);
    let narrowed = frame(&ctx, &mut app, width, vec![]);
    assert_eq!(tile(&narrowed, &app, items[0].2).0, first);
}

#[test]
fn palette_drag_from_a_tile_cancels_its_click_and_shares_the_scroll_offset() {
    let ctx = egui::Context::default();
    let mut app = super::helpers::app();
    app.category = "MatricesAndBoxes".into();
    let items = palette::items_for_ui_group(&app.category, &[]);
    let width = 240.0;
    let initial = frame(&ctx, &mut app, width, vec![]);
    let source = items[6].2;
    let start = tile(&initial, &app, source).0.center();
    let moved = start - Vec2::new(28.0, 0.0);
    let _ = frame(
        &ctx,
        &mut app,
        width,
        vec![egui::Event::PointerMoved(start), press(start, true)],
    );
    let dragged = frame(
        &ctx,
        &mut app,
        width,
        vec![egui::Event::PointerMoved(moved)],
    );
    assert_eq!(tile(&dragged, &app, source).0.center().x, moved.x);
    let _ = frame(&ctx, &mut app, width, vec![press(moved, false)]);
    assert!(app.recent_palette.is_empty());

    let wheel = egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: Vec2::new(0.0, -20.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
        modifiers: egui::Modifiers::NONE,
    };
    let _ = frame(
        &ctx,
        &mut app,
        width,
        vec![egui::Event::PointerMoved(moved), wheel],
    );
    let wheeled = frame(&ctx, &mut app, width, vec![]);
    let after_wheel = tile(&wheeled, &app, source).0.center();
    assert!(after_wheel.x < moved.x);

    let click = after_wheel + Vec2::new(1.0, 0.0);
    let _ = frame(
        &ctx,
        &mut app,
        width,
        vec![
            egui::Event::PointerMoved(after_wheel),
            press(after_wheel, true),
        ],
    );
    let _ = frame(
        &ctx,
        &mut app,
        width,
        vec![egui::Event::PointerMoved(click), press(click, false)],
    );
    assert_eq!(app.recent_palette, [source]);
}

#[test]
fn palette_custom_matrix_drag_does_not_open_the_dialog() {
    let ctx = egui::Context::default();
    let mut app = super::helpers::app();
    app.category = "MatricesAndBoxes".into();
    let width = 240.0;
    let _ = frame(&ctx, &mut app, width, vec![]);
    let out = scroll_to_palette_end(&ctx, &mut app, width);
    let start = ui_icon_rects(&out, &app, "dots")[0].center();
    let _ = frame(
        &ctx,
        &mut app,
        width,
        vec![egui::Event::PointerMoved(start), press(start, true)],
    );
    let _ = frame(
        &ctx,
        &mut app,
        width,
        vec![egui::Event::PointerMoved(start + Vec2::new(24.0, 0.0))],
    );
    let _ = frame(
        &ctx,
        &mut app,
        width,
        vec![egui::Event::PointerMoved(start)],
    );
    let _ = frame(&ctx, &mut app, width, vec![press(start, false)]);
    assert!(app.matrix_dialog.is_none());

    let _ = frame(&ctx, &mut app, width, click_events(start));
    assert!(app.matrix_dialog.is_some());
}

#[test]
fn palette_drag_from_a_gap_finishes_on_release_and_disabled_palette_stays_still() {
    let ctx = egui::Context::default();
    let mut app = super::helpers::app();
    app.category = "MatricesAndBoxes".into();
    let items = palette::items_for_ui_group(&app.category, &[]);
    let width = 240.0;
    let initial = frame(&ctx, &mut app, width, vec![]);
    let source = items[6].2;
    let start = tile(&initial, &app, source).0.center();
    let gap = start + Vec2::new(0.0, (PALETTE_BUTTON_SIZE.y + PALETTE_GAP) / 2.0);
    let end = gap - Vec2::new(22.0, 0.0);

    let _ = frame(
        &ctx,
        &mut app,
        width,
        vec![egui::Event::PointerMoved(gap), press(gap, true)],
    );
    let released = frame(
        &ctx,
        &mut app,
        width,
        vec![egui::Event::PointerMoved(end), press(end, false)],
    );
    let scrolled_tile = tile(&released, &app, source).0.center();
    assert_eq!(scrolled_tile.x, start.x - 22.0);
    assert!(app.recent_palette.is_empty());

    app.draft_dirty = true;
    let _ = frame(&ctx, &mut app, width, vec![]);
    let _ = frame(
        &ctx,
        &mut app,
        width,
        vec![
            egui::Event::PointerMoved(scrolled_tile),
            press(scrolled_tile, true),
        ],
    );
    let moved = scrolled_tile - Vec2::new(28.0, 0.0);
    let _ = frame(
        &ctx,
        &mut app,
        width,
        vec![egui::Event::PointerMoved(moved), press(moved, false)],
    );

    let scroll_down = egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: Vec2::new(0.0, -120.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
        modifiers: egui::Modifiers::NONE,
    };
    let _ = frame(
        &ctx,
        &mut app,
        width,
        vec![egui::Event::PointerMoved(scrolled_tile), scroll_down],
    );
    let after = frame(&ctx, &mut app, width, vec![]);
    assert_eq!(tile(&after, &app, source).0.center(), scrolled_tile);
}

fn scroll_to_palette_end(ctx: &egui::Context, app: &mut App, width: f32) -> egui::FullOutput {
    let _ = frame(
        ctx,
        app,
        width,
        vec![
            egui::Event::PointerMoved(egui::Pos2::new(width / 2.0, 48.0)),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: Vec2::new(0.0, -10_000.0),
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Unknown,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    // Let the remaining smoothed wheel input reach the far edge.
    for _ in 0..4 {
        let _ = frame(ctx, app, width, vec![]);
    }
    frame(ctx, app, width, vec![])
}
