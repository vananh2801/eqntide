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

fn button_rect_at(output: &egui::FullOutput, pos: egui::Pos2) -> Rect {
    output
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape)
                if shape.brush.is_none()
                    && shape.rect.contains(pos)
                    && shape.rect.height() <= 45.0 =>
            {
                Some(shape.rect)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing button rect at {pos:?}"))
}

fn text_button_rect(output: &egui::FullOutput, label: &str) -> Rect {
    let pos = output
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text_shape) if text_shape.galley.text() == label => {
                Some(text_shape.pos + text_shape.galley.size() * 0.5)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing button label {label}"));
    button_rect_at(output, pos)
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
fn directional_integral_tiles_insert_the_operator_and_limit_their_icon_depicts() {
    let width = MIN_WINDOW_SIZE[0];
    for (command, glyph) in [(r"\varointclockwise", "∲"), (r"\ointctrclockwise", "∳")] {
        for (suffix, limit_tag) in [
            ("", None),
            (r"\nolimits_{#?} #?", Some("msub")),
            (r"\limits_{#?} #?", Some("munder")),
        ] {
            let source = format!("{command}{suffix}");
            let ctx = egui::Context::default();
            let mut app = super::helpers::app();
            app.category = "Integrals".into();
            let output = frame(&ctx, &mut app, width, vec![]);
            let (icon, clip) = tile(&output, &app, &source);
            assert!(clip.contains(icon.center()));
            let _ = frame(&ctx, &mut app, width, click_events(icon.center()));

            let math = app.editor.math();
            let entry = &math.children[0];
            let operator = if let Some(tag) = limit_tag {
                let script = if entry.tag == "mrow" {
                    &entry.children[0]
                } else {
                    entry
                };
                assert_eq!(script.tag, tag, "{source}");
                &script.children[0]
            } else {
                entry
            };
            assert_eq!(
                (operator.tag.as_str(), operator.text.as_str()),
                ("mo", glyph)
            );
            assert_eq!(app.recent_palette, [source.as_str()]);
        }
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
    let out = frame(&ctx, &mut app, 1400.0, vec![]);
    let origin = button_rect_at(&out, tile(&out, &app, items[0].2).0.center());
    let last_tile = button_rect_at(
        &out,
        tile(&out, &app, items.last().expect("matrix items").2)
            .0
            .center(),
    );
    let custom_matrix = button_rect_at(&out, ui_icon_rects(&out, &app, "dots")[0].center());
    assert_eq!(custom_matrix.top(), origin.top());
    assert_eq!(custom_matrix.left() - last_tile.right(), theme::SPACE_LG);
    assert!(custom_matrix.height() > PALETTE_BUTTON_SIZE.y);

    let plus_row = text_button_rect(&out, "+ row");
    let minus_row = text_button_rect(&out, "− row");
    let plus_col = text_button_rect(&out, "+ col");
    let minus_col = text_button_rect(&out, "− col");
    assert_eq!(plus_row.left(), custom_matrix.left());
    assert_eq!(plus_row.top() - custom_matrix.bottom(), PALETTE_GAP);
    assert_eq!(minus_row.top(), plus_row.top());
    assert_eq!(minus_row.right(), custom_matrix.right());
    assert_eq!(plus_col.left(), custom_matrix.left());
    assert_eq!(plus_col.top() - plus_row.bottom(), PALETTE_GAP);
    assert_eq!(minus_col.top(), plus_col.top());
    assert_eq!(minus_col.right(), custom_matrix.right());
    assert!(custom_matrix.height() > plus_row.height());

    app.category = palette::RECENT_GROUP.into();
    let out = frame(&ctx, &mut app, MIN_WINDOW_SIZE[0], vec![]);
    check_first_four(&out, &app, recent);
}

#[test]
fn matrix_tool_block_edits_active_matrix_from_trailing_palette_column() {
    let ctx = egui::Context::default();
    let mut app = super::helpers::app();
    app.category = "MatricesAndBoxes".into();
    let width = 1400.0;
    let out = frame(&ctx, &mut app, width, vec![]);
    let two_by_two = tile(
        &out,
        &app,
        r"\begin{matrix} #? & #? \\ #? & #? \end{matrix}",
    )
    .0
    .center();
    let _ = frame(&ctx, &mut app, width, click_events(two_by_two));
    assert_eq!(app.editor.math().children[0].children.len(), 2);

    let plus_row = text_button_rect(&out, "+ row").center();
    let _ = frame(&ctx, &mut app, width, click_events(plus_row));
    assert_eq!(app.editor.math().children[0].children.len(), 3);

    let minus_col = text_button_rect(&out, "− col").center();
    let _ = frame(&ctx, &mut app, width, click_events(minus_col));
    assert_eq!(app.editor.math().children[0].children[0].children.len(), 1);
}

#[test]
fn subgroup_column_breaks_align_five_item_families_and_two_item_pairs() {
    let ctx = egui::Context::default();
    let mut app = super::helpers::app();
    let row_stride = PALETTE_BUTTON_SIZE.y + PALETTE_GAP;
    let column_stride = PALETTE_BUTTON_SIZE.x + PALETTE_GAP;
    let subgroup_stride = PALETTE_BUTTON_SIZE.x + theme::SPACE_LG;

    app.category = "Fractions".into();
    let frac_out = frame(&ctx, &mut app, MIN_WINDOW_SIZE[0], vec![]);
    let frac = tile(&frac_out, &app, r"\frac{#?}{#?}").0.center();
    let linear = tile(&frac_out, &app, r"{#?}/{#?}").0.center();
    let sqrt = tile(&frac_out, &app, r"\sqrt{#?}").0.center();
    let root = tile(&frac_out, &app, r"\sqrt[#?]{#?}").0.center();
    let long_div = tile(&frac_out, &app, r"\enclose{longdiv}{#?}").0.center();
    let binom = tile(&frac_out, &app, r"\binom{#?}{#?}").0.center();
    assert_eq!(linear - frac, Vec2::new(column_stride, 0.0));
    assert_eq!(sqrt.y, frac.y);
    assert_eq!(sqrt.x - linear.x, subgroup_stride);
    assert_eq!(root - sqrt, Vec2::new(0.0, row_stride));
    assert_eq!(long_div.y, frac.y);
    assert_eq!(long_div.x - sqrt.x, subgroup_stride);
    assert_eq!(binom - long_div, Vec2::new(0.0, row_stride));

    app.category = "Scripts".into();
    let scripts_out = frame(&ctx, &mut app, MIN_WINDOW_SIZE[0], vec![]);
    let sup = tile(&scripts_out, &app, "^{#?}").0.center();
    let pre_sup = tile(&scripts_out, &app, r"\prescript{#?}{}{#?}").0.center();
    let pre_sup_post_sup = tile(&scripts_out, &app, r"\prescript{#?}{}{#?}^{#?}")
        .0
        .center();
    let pre_sub_post_sub = tile(&scripts_out, &app, r"\prescript{}{#?}{#?}_{#?}")
        .0
        .center();
    let pre_sup_post_sub = tile(&scripts_out, &app, r"\prescript{#?}{}{#?}_{#?}")
        .0
        .center();
    let pre_both_post_both = tile(&scripts_out, &app, r"\prescript{#?}{#?}{#?}_{#?}^{#?}")
        .0
        .center();
    let pre_sup_post_both = tile(&scripts_out, &app, r"\prescript{#?}{}{#?}_{#?}^{#?}")
        .0
        .center();
    let overset = tile(&scripts_out, &app, r"\overset{#?}{#?}").0.center();
    assert_eq!(pre_sup - sup, Vec2::new(column_stride, 0.0));
    assert_eq!(pre_sup_post_sup.y, sup.y);
    assert_eq!(pre_sup_post_sup.x - pre_sup.x, subgroup_stride);
    assert_eq!(
        pre_sub_post_sub - pre_sup_post_sup,
        Vec2::new(column_stride, 0.0)
    );
    assert_eq!(
        pre_sup_post_sub - pre_sub_post_sub,
        Vec2::new(0.0, row_stride)
    );
    assert_eq!(
        pre_both_post_both - pre_sub_post_sub,
        Vec2::new(column_stride, 0.0)
    );
    assert_eq!(
        pre_sup_post_both - pre_both_post_both,
        Vec2::new(0.0, 2.0 * row_stride)
    );
    assert_eq!(overset.y, sup.y);
    assert_eq!(overset.x - pre_both_post_both.x, subgroup_stride);

    app.category = "SumsAndBigOps".into();
    let width = 1600.0;
    let sums_out = frame(&ctx, &mut app, width, vec![]);
    let sum = tile(&sums_out, &app, r"\sum").0.center();
    let sum_side_lower = tile(&sums_out, &app, r"\sum\nolimits_{#?} #?").0.center();
    let sum_side_both = tile(&sums_out, &app, r"\sum\nolimits_{#?}^{#?} #?")
        .0
        .center();
    let sum_stack_lower = tile(&sums_out, &app, r"\sum\limits_{#?} #?").0.center();
    let sum_stack_both = tile(&sums_out, &app, r"\sum\limits_{#?}^{#?} #?")
        .0
        .center();
    let prod = tile(&sums_out, &app, r"\prod").0.center();
    let coprod = tile(&sums_out, &app, r"\coprod").0.center();

    assert_eq!(sum_side_lower - sum, Vec2::new(0.0, row_stride));
    assert_eq!(sum_side_both - sum, Vec2::new(0.0, 2.0 * row_stride));
    assert_eq!(sum_stack_lower - sum, Vec2::new(column_stride, 0.0));
    assert_eq!(sum_stack_both - sum, Vec2::new(column_stride, row_stride));
    assert_eq!(prod.y, sum.y);
    assert_eq!(prod.x - sum_stack_lower.x, subgroup_stride);
    assert_eq!(coprod.y, sum.y);
    assert_eq!(coprod.x - prod.x, column_stride + subgroup_stride);

    let scrolled_end = scroll_to_palette_end(&ctx, &mut app, 240.0);
    let (last_bigodot, clip) = tile(&scrolled_end, &app, r"\bigodot\limits_{#?}^{#?} #?");
    assert!(clip.contains_rect(last_bigodot));
    assert!(clip.right() - last_bigodot.right() <= 6.0);

    app.category = "MatricesAndBoxes".into();
    let boxes_out = frame(&ctx, &mut app, 1400.0, vec![]);
    let full_box = tile(&boxes_out, &app, r"\enclose{box}{#?}").0.center();
    let rounded_box = tile(&boxes_out, &app, r"\enclose{roundedbox}{#?}")
        .0
        .center();
    let top_left = tile(&boxes_out, &app, r"\enclose{top left}{#?}").0.center();
    let top_right = tile(&boxes_out, &app, r"\enclose{top right}{#?}")
        .0
        .center();
    let bottom_left = tile(&boxes_out, &app, r"\enclose{bottom left}{#?}")
        .0
        .center();
    let bottom_right = tile(&boxes_out, &app, r"\enclose{bottom right}{#?}")
        .0
        .center();
    let open_right = tile(&boxes_out, &app, r"\enclose{top bottom left}{#?}")
        .0
        .center();
    let open_left = tile(&boxes_out, &app, r"\enclose{top bottom right}{#?}")
        .0
        .center();
    let open_bottom = tile(&boxes_out, &app, r"\enclose{top left right}{#?}")
        .0
        .center();
    let open_top = tile(&boxes_out, &app, r"\enclose{bottom left right}{#?}")
        .0
        .center();
    assert_eq!(rounded_box - full_box, Vec2::new(0.0, row_stride));
    assert_eq!(top_left - full_box, Vec2::new(subgroup_stride, 0.0));
    assert_eq!(top_right - top_left, Vec2::new(0.0, row_stride));
    assert_eq!(bottom_left - top_left, Vec2::new(column_stride, 0.0));
    assert_eq!(bottom_right - bottom_left, Vec2::new(0.0, row_stride));
    assert_eq!(open_right - bottom_left, Vec2::new(subgroup_stride, 0.0));
    assert_eq!(open_left - open_right, Vec2::new(0.0, row_stride));
    assert_eq!(open_bottom - open_right, Vec2::new(column_stride, 0.0));
    assert_eq!(open_top - open_bottom, Vec2::new(0.0, row_stride));

    app.category = "AccentsAndBars".into();
    let bars_out = frame(&ctx, &mut app, 2400.0, vec![]);
    let check_tile = tile(&bars_out, &app, r"\check{#?}").0.center();
    let over_left_harpoon = tile(&bars_out, &app, r"\overleftharpoon{#?}").0.center();
    let under_left_harpoon = tile(&bars_out, &app, r"\underleftharpoon{#?}").0.center();
    let over_right_harpoon = tile(&bars_out, &app, r"\overrightharpoon{#?}").0.center();
    let under_right_harpoon = tile(&bars_out, &app, r"\underrightharpoon{#?}").0.center();
    let prime = tile(&bars_out, &app, "{#?}'").0.center();
    assert_eq!(
        over_left_harpoon - check_tile,
        Vec2::new(subgroup_stride, 0.0)
    );
    assert_eq!(
        under_left_harpoon - over_left_harpoon,
        Vec2::new(0.0, row_stride)
    );
    assert_eq!(
        over_right_harpoon - over_left_harpoon,
        Vec2::new(column_stride, 0.0)
    );
    assert_eq!(
        under_right_harpoon - over_right_harpoon,
        Vec2::new(0.0, row_stride)
    );
    assert_eq!(prime - over_right_harpoon, Vec2::new(subgroup_stride, 0.0));

    let overline = tile(&bars_out, &app, r"\overline{#?}").0.center();
    let overline_label = tile(&bars_out, &app, r"\overset{#?}{\overline{#?}}")
        .0
        .center();
    let underline = tile(&bars_out, &app, r"\underline{#?}").0.center();
    assert_eq!(overline_label - overline, Vec2::new(0.0, row_stride));
    assert_eq!(underline.y, overline.y);
    assert_eq!(underline.x - overline.x, subgroup_stride);
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
