//! Labeled arrow templates (MathType `tmARROW`) and horizontal arrow assembly.
//!
//! Stretchy horizontal arrows are assembled from unscaled `STIXTwoMath` glyphs
//! and tiled shaft extenders (`\u{23AF}` for single-line / stacked arrows, `=`
//! for double-shaft arrows / equilibrium harpoons), using [`clip_poly_x`] only
//! to trim straight shafts on composite caps and short spans without distorting
//! arrowheads.
//!
//! [`template_of`] mirrors the cases that `mtef::encode` turns into
//! `extensible_arrow`, so the preview picture matches the MathType object.

use super::*;

/// Units per em in `STIXTwoMath-Regular.ttf`.
const UPEM: f32 = 1000.0;
/// Single horizontal shaft extender (`\u{23AF}`): ink `[0, 322] × [225, 293]`.
const EXT_BAR: ExtenderSpec = ExtenderSpec {
    ch: '\u{23AF}',
    ink_min_x: 0.0,
    ink_max_x: 322.0,
    overlap: 60.0,
};
/// Double horizontal shaft extender (`=`): ink `[62, 658] × {123..190, 327..395}`.
const EXT_EQUAL: ExtenderSpec = ExtenderSpec {
    ch: '=',
    ink_min_x: 62.0,
    ink_max_x: 658.0,
    overlap: 60.0,
};

/// Direction and glyph family of a stretchy horizontal arrow.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum ArrowDir {
    Right,
    Left,
    Both,
    DoubleRight,
    DoubleLeft,
    DoubleBoth,
    HarpoonRightUp,
    HarpoonLeftUp,
    HarpoonRightDown,
    HarpoonLeftDown,
    EquilibriumRightLeft,
    EquilibriumLeftRight,
    MapsToRight,
    MapsToLeft,
    OpposingRightLeft,
    OpposingLeftRight,
}

/// Recognized arrow template together with its optional top and bottom labels.
pub(super) struct ArrowTemplate<'a> {
    pub(super) dir: ArrowDir,
    pub(super) top: Option<&'a MathNode>,
    pub(super) bottom: Option<&'a MathNode>,
}

/// Direction and glyph family of a horizontal arrow character (spacing or combining).
pub(super) fn arrow_dir(ch: char) -> Option<ArrowDir> {
    match ch {
        '\u{2192}' | '\u{27F6}' | '\u{20D7}' => Some(ArrowDir::Right),
        '\u{2190}' | '\u{27F5}' | '\u{20D6}' => Some(ArrowDir::Left),
        '\u{2194}' | '\u{27F7}' | '\u{20E1}' => Some(ArrowDir::Both),
        '⇒' | '⟹' => Some(ArrowDir::DoubleRight),
        '⇐' | '⟸' => Some(ArrowDir::DoubleLeft),
        '⇔' | '⟺' => Some(ArrowDir::DoubleBoth),
        '⇀' | '\u{20D1}' => Some(ArrowDir::HarpoonRightUp),
        '↼' | '\u{20D0}' => Some(ArrowDir::HarpoonLeftUp),
        '⇁' => Some(ArrowDir::HarpoonRightDown),
        '↽' => Some(ArrowDir::HarpoonLeftDown),
        '⇌' => Some(ArrowDir::EquilibriumRightLeft),
        '⇋' => Some(ArrowDir::EquilibriumLeftRight),
        '↦' | '⟼' => Some(ArrowDir::MapsToRight),
        '↤' | '⟻' => Some(ArrowDir::MapsToLeft),
        '⇄' => Some(ArrowDir::OpposingRightLeft),
        '⇆' => Some(ArrowDir::OpposingLeftRight),
        _ => None,
    }
}

/// Vertical ink bounds `(y_min_font, y_max_font)` in font units (`UPEM = 1000`)
/// for the glyph representing `dir`.
pub(super) fn glyph_font_y_bounds(dir: ArrowDir) -> (f32, f32) {
    match recipe_of(dir) {
        AssemblyRecipe::Single(spec) => (spec.y_min_font, spec.y_max_font),
        AssemblyRecipe::DualHeads(spec) => (spec.y_min_font, spec.y_max_font),
        AssemblyRecipe::CompositeCaps(spec) => (spec.y_min_font, spec.y_max_font),
    }
}

/// Recognize the `munder` / `mover` / `munderover` shapes that MathType draws
/// as a labeled arrow:
///
/// 1. an arrow with a label over and/or under it (`A ->(x) B`);
/// 2. `↔` / `←` / etc. over some text: the arrow is drawn above, the text below it;
/// 3. any arrow under some text (not a limit): the text is drawn above the arrow.
///
/// `→` over text stays a vector accent (`draw_accent`), as in MathType.
pub(super) fn template_of(node: &MathNode) -> Option<ArrowTemplate<'_>> {
    if node.attr("accent") == Some("true") || node.attr("accentunder") == Some("true") {
        return None;
    }
    let base = node.children.first()?;
    let (under, over) = match node.tag.as_str() {
        "munder" => (node.children.get(1), None),
        "mover" => (None, node.children.get(1)),
        "munderover" => (node.children.get(1), node.children.get(2)),
        _ => return None,
    };
    let is_label =
        |label_node: Option<&MathNode>| label_node.is_none_or(|inner| mo_arrow(inner).is_none());

    // 1. Base is the arrow; children are its labels.
    if let Some(dir) = mo_arrow(base) {
        if (under.is_some() || over.is_some()) && is_label(under) && is_label(over) {
            return Some(ArrowTemplate {
                dir,
                top: over,
                bottom: under,
            });
        }
    }
    // 2. Non-right arrow drawn over the base text.
    if let Some(dir) = over
        .and_then(mo_arrow)
        .filter(|dir| *dir != ArrowDir::Right)
    {
        return Some(ArrowTemplate {
            dir,
            top: None,
            bottom: Some(base),
        });
    }
    // 3. Arrow drawn under the base text (limits such as `lim` keep their subscript).
    if let Some(dir) = under.and_then(mo_arrow) {
        if !base.text.contains("lim") {
            return Some(ArrowTemplate {
                dir,
                top: Some(base),
                bottom: None,
            });
        }
    }
    None
}

impl Layout {
    /// Lay out an arrow whose length follows its labels, assembled from font glyphs
    /// with `top` centered above and `bottom` centered below the shaft.
    pub(super) fn labeled_arrow(
        &mut self,
        dir: ArrowDir,
        top: Option<&MathNode>,
        bottom: Option<&MathNode>,
        em: f32,
        ctx: MathContext,
    ) -> Result<LBox> {
        let constants = self.font_set.constants();
        let lim_ctx = ctx.for_limits();
        let lim_em = em * lim_ctx.scale(constants);
        let over = match top {
            Some(over_node) => Some(self.node(over_node, lim_em, lim_ctx)?),
            None => None,
        };
        let under = match bottom {
            Some(under_node) => Some(self.node(under_node, lim_em, lim_ctx)?),
            None => None,
        };

        // Shaft: wide enough so labels clear arrowheads on both sides.
        let label_w = [
            over.as_ref().map(|over_box| over_box.w),
            under.as_ref().map(|under_box| under_box.w),
        ]
        .into_iter()
        .flatten()
        .fold(0.0f32, f32::max);
        let len = (label_w + 0.72 * em).max(1.40 * em);

        let (head_asc, head_desc) = self.font_set.ink(Face::Symbol, '\u{2192}', em);
        let mut layout_box = LBox {
            w: len,
            asc: head_asc,
            desc: head_desc,
            ..Default::default()
        };

        let y_glyph = 0.0; // on the baseline, the math glyphs' axis sits on the math axis
        let (min_y, max_y) =
            self.assemble_horizontal_arrow(dir, 0.0, len, y_glyph, em, &mut layout_box.items);
        layout_box.asc = layout_box.asc.max(-min_y);
        layout_box.desc = layout_box.desc.max(max_y);

        let y_axis = -constants.axis_height * em;
        let rule_thick = constants.fraction_rule_thickness * em;
        let (arrow_top_y, arrow_bot_y) = if is_single_shaft_with_center_clear(dir) {
            (y_axis - rule_thick / 2.0, y_axis + rule_thick / 2.0)
        } else {
            (min_y, max_y)
        };

        let gap = 0.10 * em;
        if let Some(over_box) = &over {
            layout_box.place(
                over_box,
                (len - over_box.w) / 2.0,
                arrow_top_y - gap - over_box.desc,
            );
        }
        if let Some(under_box) = &under {
            layout_box.place(
                under_box,
                (len - under_box.w) / 2.0,
                arrow_bot_y + gap + under_box.asc,
            );
        }
        layout_box.w = len;
        Ok(layout_box)
    }

    /// Assemble an unscaled horizontal arrow `dir` across `[x_start, x_start + span]`
    /// with baseline at `y_glyph` and nominal em size `em`, returning its vertical ink
    /// range `(min_y, max_y)` in layout coordinates (y positive downwards).
    pub(super) fn assemble_horizontal_arrow(
        &self,
        dir: ArrowDir,
        x_start: f32,
        span: f32,
        y_glyph: f32,
        em: f32,
        items: &mut Vec<Item>,
    ) -> (f32, f32) {
        let recipe = recipe_of(dir);
        match recipe {
            AssemblyRecipe::Single(spec) => {
                emit_single_sided(self.font_set, items, &spec, x_start, span, y_glyph, em);
                glyph_layout_y_bounds(spec.y_min_font, spec.y_max_font, y_glyph, em)
            }
            AssemblyRecipe::DualHeads(spec) => {
                emit_dual_heads(self.font_set, items, &spec, x_start, span, y_glyph, em);
                glyph_layout_y_bounds(spec.y_min_font, spec.y_max_font, y_glyph, em)
            }
            AssemblyRecipe::CompositeCaps(spec) => {
                emit_composite_caps(self.font_set, items, &spec, x_start, span, y_glyph, em);
                glyph_layout_y_bounds(spec.y_min_font, spec.y_max_font, y_glyph, em)
            }
        }
    }
}

#[derive(Clone, Copy)]
struct ExtenderSpec {
    ch: char,
    ink_min_x: f32,
    ink_max_x: f32,
    overlap: f32,
}

impl ExtenderSpec {
    fn ink_w(self) -> f32 {
        self.ink_max_x - self.ink_min_x
    }
}

#[derive(Clone, Copy)]
struct SingleHeadSpec {
    ch: char,
    points_right: bool,
    ink_min_x: f32,
    ink_max_x: f32,
    min_clipped_w: f32,
    y_min_font: f32,
    y_max_font: f32,
    ext: ExtenderSpec,
}

#[derive(Clone, Copy)]
struct DualHeadSpec {
    left_ch: char,
    left_min_x: f32,
    left_max_x: f32,
    left_cap_max_x: f32,
    right_ch: char,
    right_min_x: f32,
    right_max_x: f32,
    right_cap_min_x: f32,
    fallback_ch: char,
    fallback_min_x: f32,
    fallback_max_x: f32,
    unclipped_min_span: f32,
    min_clipped_span: f32,
    y_min_font: f32,
    y_max_font: f32,
    ext: ExtenderSpec,
}

#[derive(Clone, Copy)]
struct CompositeCapSpec {
    ch: char,
    ink_min_x: f32,
    ink_max_x: f32,
    left_min_w: f32,
    right_min_w: f32,
    left_wide_cap_max_x: f32,
    right_wide_cap_min_x: f32,
    wide_ext_left_inset: f32,
    wide_ext_right_inset: f32,
    y_min_font: f32,
    y_max_font: f32,
    /// Offsets relative to `y_glyph` (in layout pixels per font unit, positive downwards)
    /// for each horizontal extender row (1 row for `↦`/`↤`/`⇌`/`⇋`, 2 rows for `⇄`/`⇆`).
    ext_rows: &'static [(ExtenderSpec, f32)],
}

#[derive(Clone, Copy)]
enum AssemblyRecipe {
    Single(SingleHeadSpec),
    DualHeads(DualHeadSpec),
    CompositeCaps(CompositeCapSpec),
}

const SINGLE_BAR_ROW: &[(ExtenderSpec, f32)] = &[(EXT_BAR, 0.0)];
const DOUBLE_EQUAL_ROW: &[(ExtenderSpec, f32)] = &[(EXT_EQUAL, 0.0)];
const DUAL_BAR_ROWS: &[(ExtenderSpec, f32)] = &[(EXT_BAR, -163.5), (EXT_BAR, 164.0)];

fn mo_arrow(node: &MathNode) -> Option<ArrowDir> {
    if node.tag != "mo" {
        return None;
    }
    node.text.trim().chars().next().and_then(arrow_dir)
}

fn is_single_shaft_with_center_clear(dir: ArrowDir) -> bool {
    matches!(
        dir,
        ArrowDir::Right
            | ArrowDir::Left
            | ArrowDir::Both
            | ArrowDir::MapsToRight
            | ArrowDir::MapsToLeft
    )
}

fn recipe_of(dir: ArrowDir) -> AssemblyRecipe {
    match dir {
        ArrowDir::Right => AssemblyRecipe::Single(SingleHeadSpec {
            ch: '\u{2192}',
            points_right: true,
            ink_min_x: 71.0,
            ink_max_x: 876.0,
            min_clipped_w: 300.0,
            y_min_font: 58.0,
            y_max_font: 459.0,
            ext: EXT_BAR,
        }),
        ArrowDir::Left => AssemblyRecipe::Single(SingleHeadSpec {
            ch: '\u{2190}',
            points_right: false,
            ink_min_x: 72.0,
            ink_max_x: 877.0,
            min_clipped_w: 300.0,
            y_min_font: 59.0,
            y_max_font: 460.0,
            ext: EXT_BAR,
        }),
        ArrowDir::Both => AssemblyRecipe::DualHeads(DualHeadSpec {
            left_ch: '\u{2190}',
            left_min_x: 72.0,
            left_max_x: 877.0,
            left_cap_max_x: 360.0,
            right_ch: '\u{2192}',
            right_min_x: 71.0,
            right_max_x: 876.0,
            right_cap_min_x: 588.0,
            fallback_ch: '\u{2194}',
            fallback_min_x: 39.0,
            fallback_max_x: 909.0,
            unclipped_min_span: 1100.0,
            min_clipped_span: 576.0,
            y_min_font: 58.0,
            y_max_font: 460.0,
            ext: EXT_BAR,
        }),
        ArrowDir::DoubleRight => AssemblyRecipe::Single(SingleHeadSpec {
            ch: '⇒',
            points_right: true,
            ink_min_x: 0.0,
            ink_max_x: 826.0,
            min_clipped_w: 430.0,
            y_min_font: -46.0,
            y_max_font: 564.0,
            ext: EXT_EQUAL,
        }),
        ArrowDir::DoubleLeft => AssemblyRecipe::Single(SingleHeadSpec {
            ch: '⇐',
            points_right: false,
            ink_min_x: 61.0,
            ink_max_x: 887.0,
            min_clipped_w: 430.0,
            y_min_font: -46.0,
            y_max_font: 564.0,
            ext: EXT_EQUAL,
        }),
        ArrowDir::DoubleBoth => AssemblyRecipe::DualHeads(DualHeadSpec {
            left_ch: '⇐',
            left_min_x: 61.0,
            left_max_x: 887.0,
            left_cap_max_x: 475.0,
            right_ch: '⇒',
            right_min_x: 0.0,
            right_max_x: 826.0,
            right_cap_min_x: 410.0,
            fallback_ch: '⇔',
            fallback_min_x: 20.0,
            fallback_max_x: 927.0,
            unclipped_min_span: 1250.0,
            min_clipped_span: 820.0,
            y_min_font: -46.0,
            y_max_font: 564.0,
            ext: EXT_EQUAL,
        }),
        ArrowDir::HarpoonRightUp => AssemblyRecipe::Single(SingleHeadSpec {
            ch: '⇀',
            points_right: true,
            ink_min_x: 55.0,
            ink_max_x: 922.0,
            min_clipped_w: 360.0,
            y_min_font: 225.0,
            y_max_font: 506.0,
            ext: EXT_BAR,
        }),
        ArrowDir::HarpoonLeftUp => AssemblyRecipe::Single(SingleHeadSpec {
            ch: '↼',
            points_right: false,
            ink_min_x: 55.0,
            ink_max_x: 922.0,
            min_clipped_w: 360.0,
            y_min_font: 225.0,
            y_max_font: 506.0,
            ext: EXT_BAR,
        }),
        ArrowDir::HarpoonRightDown => AssemblyRecipe::Single(SingleHeadSpec {
            ch: '⇁',
            points_right: true,
            ink_min_x: 55.0,
            ink_max_x: 922.0,
            min_clipped_w: 360.0,
            y_min_font: 12.0,
            y_max_font: 293.0,
            ext: EXT_BAR,
        }),
        ArrowDir::HarpoonLeftDown => AssemblyRecipe::Single(SingleHeadSpec {
            ch: '↽',
            points_right: false,
            ink_min_x: 55.0,
            ink_max_x: 922.0,
            min_clipped_w: 360.0,
            y_min_font: 12.0,
            y_max_font: 293.0,
            ext: EXT_BAR,
        }),
        ArrowDir::EquilibriumRightLeft => AssemblyRecipe::CompositeCaps(CompositeCapSpec {
            ch: '⇌',
            ink_min_x: 71.0,
            ink_max_x: 876.0,
            left_min_w: 285.0,
            right_min_w: 285.0,
            left_wide_cap_max_x: 400.0,
            right_wide_cap_min_x: 547.0,
            wide_ext_left_inset: 280.0,
            wide_ext_right_inset: 280.0,
            y_min_font: -34.0,
            y_max_font: 552.0,
            ext_rows: DOUBLE_EQUAL_ROW,
        }),
        ArrowDir::EquilibriumLeftRight => AssemblyRecipe::CompositeCaps(CompositeCapSpec {
            ch: '⇋',
            ink_min_x: 71.0,
            ink_max_x: 876.0,
            left_min_w: 285.0,
            right_min_w: 285.0,
            left_wide_cap_max_x: 400.0,
            right_wide_cap_min_x: 547.0,
            wide_ext_left_inset: 280.0,
            wide_ext_right_inset: 280.0,
            y_min_font: -34.0,
            y_max_font: 552.0,
            ext_rows: DOUBLE_EQUAL_ROW,
        }),
        ArrowDir::MapsToRight => AssemblyRecipe::CompositeCaps(CompositeCapSpec {
            ch: '↦',
            ink_min_x: 71.0,
            ink_max_x: 877.0,
            left_min_w: 62.0,
            right_min_w: 278.0,
            left_wide_cap_max_x: 220.0,
            right_wide_cap_min_x: 560.0,
            wide_ext_left_inset: 100.0,
            wide_ext_right_inset: 270.0,
            y_min_font: 58.0,
            y_max_font: 461.0,
            ext_rows: SINGLE_BAR_ROW,
        }),
        ArrowDir::MapsToLeft => AssemblyRecipe::CompositeCaps(CompositeCapSpec {
            ch: '↤',
            ink_min_x: 71.0,
            ink_max_x: 877.0,
            left_min_w: 278.0,
            right_min_w: 62.0,
            left_wide_cap_max_x: 388.0,
            right_wide_cap_min_x: 728.0,
            wide_ext_left_inset: 270.0,
            wide_ext_right_inset: 100.0,
            y_min_font: 58.0,
            y_max_font: 461.0,
            ext_rows: SINGLE_BAR_ROW,
        }),
        ArrowDir::OpposingRightLeft => AssemblyRecipe::CompositeCaps(CompositeCapSpec {
            ch: '⇄',
            ink_min_x: 72.0,
            ink_max_x: 876.0,
            left_min_w: 285.0,
            right_min_w: 285.0,
            left_wide_cap_max_x: 400.0,
            right_wide_cap_min_x: 548.0,
            wide_ext_left_inset: 280.0,
            wide_ext_right_inset: 280.0,
            y_min_font: -94.0,
            y_max_font: 612.0,
            ext_rows: DUAL_BAR_ROWS,
        }),
        ArrowDir::OpposingLeftRight => AssemblyRecipe::CompositeCaps(CompositeCapSpec {
            ch: '⇆',
            ink_min_x: 72.0,
            ink_max_x: 876.0,
            left_min_w: 285.0,
            right_min_w: 285.0,
            left_wide_cap_max_x: 400.0,
            right_wide_cap_min_x: 548.0,
            wide_ext_left_inset: 280.0,
            wide_ext_right_inset: 280.0,
            y_min_font: -94.0,
            y_max_font: 612.0,
            ext_rows: DUAL_BAR_ROWS,
        }),
    }
}

fn glyph_layout_y_bounds(y_min_font: f32, y_max_font: f32, y_glyph: f32, em: f32) -> (f32, f32) {
    let unit = em / UPEM;
    (y_glyph - y_max_font * unit, y_glyph - y_min_font * unit)
}

fn emit_single_sided(
    font_set: &Fonts,
    items: &mut Vec<Item>,
    spec: &SingleHeadSpec,
    x_start: f32,
    span: f32,
    y_glyph: f32,
    em: f32,
) {
    let unit = em / UPEM;
    let x_end = x_start + span;
    let ink_w = (spec.ink_max_x - spec.ink_min_x) * unit;
    let min_w = spec.min_clipped_w * unit;

    if span >= ink_w {
        if spec.points_right {
            let glyph_x = x_end - spec.ink_max_x * unit;
            items.push(Item::Glyph {
                ch: spec.ch,
                face: Face::Symbol,
                em,
                x: glyph_x,
                y: y_glyph,
                ssty: 0,
            });
            let head_ink_left = glyph_x + spec.ink_min_x * unit;
            if head_ink_left > x_start + 0.5 * unit {
                let to_x = (head_ink_left + spec.ext.overlap * unit).min(x_end);
                tile_extender(font_set, items, spec.ext, x_start, to_x, y_glyph, em);
            }
        } else {
            let glyph_x = x_start - spec.ink_min_x * unit;
            items.push(Item::Glyph {
                ch: spec.ch,
                face: Face::Symbol,
                em,
                x: glyph_x,
                y: y_glyph,
                ssty: 0,
            });
            let head_ink_right = glyph_x + spec.ink_max_x * unit;
            if head_ink_right < x_end - 0.5 * unit {
                let from_x = (head_ink_right - spec.ext.overlap * unit).max(x_start);
                tile_extender(font_set, items, spec.ext, from_x, x_end, y_glyph, em);
            }
        }
    } else if span >= min_w {
        let glyph_x = if spec.points_right {
            x_end - spec.ink_max_x * unit
        } else {
            x_start - spec.ink_min_x * unit
        };
        emit_clipped_glyph(
            font_set, items, spec.ch, glyph_x, y_glyph, em, x_start, x_end,
        );
    } else {
        emit_natural_centered(
            items,
            spec.ch,
            spec.ink_min_x,
            spec.ink_max_x,
            x_start,
            span,
            y_glyph,
            em,
        );
    }
}

fn emit_dual_heads(
    font_set: &Fonts,
    items: &mut Vec<Item>,
    spec: &DualHeadSpec,
    x_start: f32,
    span: f32,
    y_glyph: f32,
    em: f32,
) {
    let unit = em / UPEM;
    let x_end = x_start + span;
    if span >= spec.unclipped_min_span * unit {
        let left_x = x_start - spec.left_min_x * unit;
        let right_x = x_end - spec.right_max_x * unit;
        items.push(Item::Glyph {
            ch: spec.left_ch,
            face: Face::Symbol,
            em,
            x: left_x,
            y: y_glyph,
            ssty: 0,
        });
        items.push(Item::Glyph {
            ch: spec.right_ch,
            face: Face::Symbol,
            em,
            x: right_x,
            y: y_glyph,
            ssty: 0,
        });
        let left_ink_right = left_x + spec.left_max_x * unit;
        let right_ink_left = right_x + spec.right_min_x * unit;
        if right_ink_left > left_ink_right + 1.0 * unit {
            let from_x = left_ink_right - spec.ext.overlap * unit;
            let to_x = right_ink_left + spec.ext.overlap * unit;
            tile_extender(font_set, items, spec.ext, from_x, to_x, y_glyph, em);
        }
    } else if span >= spec.min_clipped_span * unit {
        let left_x = x_start - spec.left_min_x * unit;
        let right_x = x_end - spec.right_max_x * unit;
        let left_cap_end = (x_start + (spec.left_cap_max_x - spec.left_min_x) * unit).min(x_end);
        let right_cap_start =
            (x_end - (spec.right_max_x - spec.right_cap_min_x) * unit).max(x_start);
        let x_mid = x_start + span / 2.0;
        let clip_left_max = left_cap_end.max(x_mid + 2.0 * unit);
        let clip_right_min = right_cap_start.min(x_mid - 2.0 * unit);
        emit_clipped_glyph(
            font_set,
            items,
            spec.left_ch,
            left_x,
            y_glyph,
            em,
            x_start,
            clip_left_max,
        );
        emit_clipped_glyph(
            font_set,
            items,
            spec.right_ch,
            right_x,
            y_glyph,
            em,
            clip_right_min,
            x_end,
        );
    } else {
        emit_natural_centered(
            items,
            spec.fallback_ch,
            spec.fallback_min_x,
            spec.fallback_max_x,
            x_start,
            span,
            y_glyph,
            em,
        );
    }
}

fn emit_composite_caps(
    font_set: &Fonts,
    items: &mut Vec<Item>,
    spec: &CompositeCapSpec,
    x_start: f32,
    span: f32,
    y_glyph: f32,
    em: f32,
) {
    let unit = em / UPEM;
    let x_end = x_start + span;
    let ink_w = (spec.ink_max_x - spec.ink_min_x) * unit;
    let min_w = (spec.left_min_w + spec.right_min_w) * unit;

    if span > ink_w + 1.0 * unit {
        let left_x = x_start - spec.ink_min_x * unit;
        let right_x = x_end - spec.ink_max_x * unit;
        let left_clip_max = x_start + (spec.left_wide_cap_max_x - spec.ink_min_x) * unit;
        let right_clip_min = x_end - (spec.ink_max_x - spec.right_wide_cap_min_x) * unit;
        emit_clipped_glyph(
            font_set,
            items,
            spec.ch,
            left_x,
            y_glyph,
            em,
            x_start,
            left_clip_max,
        );
        emit_clipped_glyph(
            font_set,
            items,
            spec.ch,
            right_x,
            y_glyph,
            em,
            right_clip_min,
            x_end,
        );
        let ext_from = x_start + spec.wide_ext_left_inset * unit;
        let ext_to = x_end - spec.wide_ext_right_inset * unit;
        for &(ext, y_offset_units) in spec.ext_rows {
            tile_extender(
                font_set,
                items,
                ext,
                ext_from,
                ext_to,
                y_glyph + y_offset_units * unit,
                em,
            );
        }
    } else if (span - ink_w).abs() <= 1.0 * unit {
        emit_natural_centered(
            items,
            spec.ch,
            spec.ink_min_x,
            spec.ink_max_x,
            x_start,
            span,
            y_glyph,
            em,
        );
    } else if span >= min_w {
        let left_x = x_start - spec.ink_min_x * unit;
        let right_x = x_end - spec.ink_max_x * unit;
        let left_min_px = spec.left_min_w * unit;
        let right_min_px = spec.right_min_w * unit;
        let split_x = x_start + left_min_px + (span - left_min_px - right_min_px) / 2.0;
        emit_clipped_glyph(
            font_set,
            items,
            spec.ch,
            left_x,
            y_glyph,
            em,
            x_start,
            (split_x + 2.0 * unit).min(x_end),
        );
        emit_clipped_glyph(
            font_set,
            items,
            spec.ch,
            right_x,
            y_glyph,
            em,
            (split_x - 2.0 * unit).max(x_start),
            x_end,
        );
    } else {
        emit_natural_centered(
            items,
            spec.ch,
            spec.ink_min_x,
            spec.ink_max_x,
            x_start,
            span,
            y_glyph,
            em,
        );
    }
}

fn tile_extender(
    font_set: &Fonts,
    items: &mut Vec<Item>,
    ext: ExtenderSpec,
    from_x: f32,
    to_x: f32,
    y_glyph: f32,
    em: f32,
) {
    let gap = to_x - from_x;
    if gap <= 0.0 {
        return;
    }
    let unit = em / UPEM;
    let ext_w = ext.ink_w() * unit;
    if gap < ext_w {
        let glyph_x = from_x - ext.ink_min_x * unit;
        emit_clipped_glyph(font_set, items, ext.ch, glyph_x, y_glyph, em, from_x, to_x);
        return;
    }
    let seam = (25.0 * unit).min(0.25 * ext_w);
    let stride = (ext_w - seam).max(1.0 * unit);
    let mut cursor = from_x;
    while cursor + ext_w <= to_x + 0.5 * unit {
        let glyph_x = cursor - ext.ink_min_x * unit;
        items.push(Item::Glyph {
            ch: ext.ch,
            face: Face::Symbol,
            em,
            x: glyph_x,
            y: y_glyph,
            ssty: 0,
        });
        cursor += stride;
    }
    if cursor < to_x - 0.5 * unit {
        let clip_min = (cursor - seam).max(from_x).max(to_x - ext_w);
        let glyph_x = clip_min - ext.ink_min_x * unit;
        emit_clipped_glyph(
            font_set, items, ext.ch, glyph_x, y_glyph, em, clip_min, to_x,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_clipped_glyph(
    font_set: &Fonts,
    items: &mut Vec<Item>,
    ch: char,
    origin_x: f32,
    origin_y: f32,
    em: f32,
    clip_min_x: f32,
    clip_max_x: f32,
) {
    let contours: Vec<Vec<(f32, f32)>> = font_set
        .glyph_polys(Face::Symbol, ch, em, em, origin_x, origin_y)
        .into_iter()
        .map(|poly| clip_poly_x(&poly, clip_min_x, clip_max_x))
        .filter(|clipped| clipped.len() >= 3)
        .collect();
    if contours.len() == 1 {
        if let Some(single) = contours.into_iter().next() {
            items.push(Item::Poly(single));
        }
    } else if contours.len() > 1 {
        items.push(Item::Contours(contours));
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_natural_centered(
    items: &mut Vec<Item>,
    ch: char,
    ink_min_x: f32,
    ink_max_x: f32,
    x_start: f32,
    span: f32,
    y_glyph: f32,
    em: f32,
) {
    let unit = em / UPEM;
    let mid_font_x = (ink_min_x + ink_max_x) / 2.0;
    let glyph_x = x_start + span / 2.0 - mid_font_x * unit;
    items.push(Item::Glyph {
        ch,
        face: Face::Symbol,
        em,
        x: glyph_x,
        y: y_glyph,
        ssty: 0,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn box_x_extent(layout_box: &LBox, item_filter: impl Fn(&Item) -> bool) -> (f32, f32) {
        let font_set = fonts();
        let (mut min_x, mut max_x) = (f32::MAX, f32::MIN);
        for item in &layout_box.items {
            if !item_filter(item) {
                continue;
            }
            match item {
                Item::Glyph {
                    ch,
                    face,
                    em,
                    x,
                    ssty,
                    ..
                } => {
                    let (glyph_x0, glyph_x1) = font_set.ink_x_styled(*face, *ch, *em, *ssty);
                    min_x = min_x.min(x + glyph_x0);
                    max_x = max_x.max(x + glyph_x1);
                }
                Item::Poly(points) => {
                    for &(x, _) in points {
                        min_x = min_x.min(x);
                        max_x = max_x.max(x);
                    }
                }
                Item::Contours(contours) => {
                    for points in contours {
                        for &(x, _) in points {
                            min_x = min_x.min(x);
                            max_x = max_x.max(x);
                        }
                    }
                }
            }
        }
        (min_x, max_x)
    }

    #[test]
    fn test_labeled_arrow_glyph_assembly() {
        let mut layout = Layout {
            font_set: fonts(),
            depth: 0,
        };
        let em = 12.0 * 96.0 / 72.0;
        let ctx = MathContext::DISPLAY;

        let node_top =
            MathNode::parse("<math><mover><mo>→</mo><mi>yields</mi></mover></math>").unwrap();
        let top_box = layout.node(&node_top.children[0], em, ctx).unwrap();
        assert!(top_box.w > 0.0);
        let glyphs: Vec<char> = top_box
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Glyph { ch, .. } => Some(*ch),
                _ => None,
            })
            .collect();
        assert!(
            glyphs.contains(&'\u{2192}'),
            "must contain right arrow glyph"
        );
        assert!(glyphs.contains(&'\u{23AF}'), "must contain extender glyph");

        // Bidirectional arrow with label below.
        let node_under =
            MathNode::parse("<math><munder><mo>↔</mo><mi>x</mi></munder></math>").unwrap();
        let under_box = layout.node(&node_under.children[0], em, ctx).unwrap();
        let glyphs_under: Vec<char> = under_box
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Glyph { ch, .. } => Some(*ch),
                _ => None,
            })
            .collect();
        assert!(
            glyphs_under.contains(&'\u{2190}'),
            "must contain left arrow glyph"
        );
        assert!(
            glyphs_under.contains(&'\u{2192}'),
            "must contain right arrow glyph"
        );

        let math_full = MathNode::parse(
            "<math><mtable><mtr><mtd><mi>A</mi><mover><mo>→</mo><mi>yields</mi></mover><mi>B</mi></mtd></mtr>\
            <mtr><mtd><mi>A</mi><munder><mo>↔</mo><mi>x</mi></munder><mi>B</mi></mtd></mtr></mtable></math>",
        )
        .unwrap();
        let preview = crate::render::render_preview(&math_full, 12.0).unwrap();
        assert!(!preview.png.is_empty(), "preview PNG must not be empty");
    }

    #[test]
    fn test_equilibrium_arrow_labels_clear_harpoons() {
        let mut layout = Layout {
            font_set: fonts(),
            depth: 0,
        };
        let em = 24.0;
        let node =
            MathNode::parse("<math><munderover><mo>⇌</mo><mi>g</mi><mi>f</mi></munderover></math>")
                .unwrap();
        let layout_box = layout
            .node(&node.children[0], em, MathContext::DISPLAY)
            .unwrap();
        let (mut arrow_top, mut arrow_bot) = (f32::MAX, f32::MIN);
        let mut top_label_y = None;
        let mut bot_label_y = None;
        for item in &layout_box.items {
            match item {
                Item::Poly(points) => {
                    for &(_, y) in points {
                        arrow_top = arrow_top.min(y);
                        arrow_bot = arrow_bot.max(y);
                    }
                }
                Item::Contours(contours) => {
                    for points in contours {
                        for &(_, y) in points {
                            arrow_top = arrow_top.min(y);
                            arrow_bot = arrow_bot.max(y);
                        }
                    }
                }
                Item::Glyph { ch: 'f', y, .. } => top_label_y = Some(*y),
                Item::Glyph { ch: 'g', y, .. } => bot_label_y = Some(*y),
                _ => {}
            }
        }
        assert!(
            top_label_y.unwrap() < arrow_top,
            "top label baseline {} must clear harpoon top {}",
            top_label_y.unwrap(),
            arrow_top
        );
        assert!(
            bot_label_y.unwrap() > arrow_bot,
            "bottom label baseline {} must clear harpoon bottom {}",
            bot_label_y.unwrap(),
            arrow_bot
        );
    }

    #[test]
    fn wide_labeled_arrows_assemble_from_unscaled_caps_and_extenders() {
        let mut layout = Layout {
            font_set: fonts(),
            depth: 0,
        };
        let em = 24.0;
        let unit = em / UPEM;

        // Stacked opposing, equilibrium, double, harpoon, and maps-to arrows with wide label.
        let cases = [
            ('⇄', '\u{23AF}'),
            ('⇆', '\u{23AF}'),
            ('⇌', '='),
            ('⇋', '='),
            ('⇒', '='),
            ('⇐', '='),
            ('⇔', '='),
            ('⇀', '\u{23AF}'),
            ('↼', '\u{23AF}'),
            ('↦', '\u{23AF}'),
        ];

        for (arrow_ch, ext_ch) in cases {
            let xml = format!("<math><munder><mo>{arrow_ch}</mo><mi>dfdsfsd</mi></munder></math>");
            let math = MathNode::parse(&xml).unwrap();
            let layout_box = layout
                .node(&math.children[0], em, MathContext::DISPLAY)
                .unwrap();

            let ext_count = layout_box
                .items
                .iter()
                .filter(|item| matches!(item, Item::Glyph { ch, em: item_em, .. } if *ch == ext_ch && (*item_em - em).abs() < 1e-3))
                .count();
            assert!(
                ext_count >= 2,
                "{arrow_ch}: expected multiple unscaled '{ext_ch}' extenders, got {ext_count}"
            );

            // Clipped caps and final extender tiles (Poly/Contours) must stay within
            // natural single-glyph width (< 0.65 * em; `=` is 596 units), never stretched
            // across the whole multi-character width.
            for item in &layout_box.items {
                match item {
                    Item::Poly(points) => {
                        let (mut min_x, mut max_x) = (f32::MAX, f32::MIN);
                        for &(x, _) in points {
                            min_x = min_x.min(x);
                            max_x = max_x.max(x);
                        }
                        assert!(
                            max_x - min_x <= 650.0 * unit + 1e-3,
                            "{arrow_ch}: clipped Poly width {} exceeds natural glyph width",
                            max_x - min_x
                        );
                    }
                    Item::Contours(contours) => {
                        let (mut min_x, mut max_x) = (f32::MAX, f32::MIN);
                        for points in contours {
                            for &(x, _) in points {
                                min_x = min_x.min(x);
                                max_x = max_x.max(x);
                            }
                        }
                        assert!(
                            max_x - min_x <= 650.0 * unit + 1e-3,
                            "{arrow_ch}: clipped Contours width {} exceeds natural glyph width",
                            max_x - min_x
                        );
                    }
                    _ => {}
                }
            }

            let (arrow_x0, arrow_x1) = box_x_extent(&layout_box, |item| {
                !matches!(
                    item,
                    Item::Glyph {
                        face: Face::Variable,
                        ..
                    }
                )
            });
            assert!(
                (arrow_x0 - 0.0).abs() < 1.0 && (arrow_x1 - layout_box.w).abs() < 1.0,
                "{arrow_ch}: assembled span [{arrow_x0}, {arrow_x1}] must cover [0, {}]",
                layout_box.w
            );

            // Bottom label must clear the arrow ink directly above the label column range.
            let (label_x0, label_x1) = box_x_extent(&layout_box, |item| {
                matches!(
                    item,
                    Item::Glyph {
                        face: Face::Variable,
                        ..
                    }
                )
            });
            let label_top = layout_box
                .items
                .iter()
                .filter_map(|item| match item {
                    Item::Glyph {
                        ch,
                        face: Face::Variable,
                        em: item_em,
                        y,
                        ssty,
                        ..
                    } => Some(*y - fonts().ink_styled(Face::Variable, *ch, *item_em, *ssty).0),
                    _ => None,
                })
                .fold(f32::MAX, f32::min);
            let arrow_bot_over_label = layout_box
                .items
                .iter()
                .filter(|item| {
                    !matches!(
                        item,
                        Item::Glyph {
                            face: Face::Variable,
                            ..
                        }
                    )
                })
                .flat_map(|item| match item {
                    Item::Glyph {
                        ch,
                        face,
                        em: item_em,
                        x,
                        y,
                        ..
                    } => fonts()
                        .glyph_polys(*face, *ch, *item_em, *item_em, *x, *y)
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>(),
                    Item::Poly(points) => points.clone(),
                    Item::Contours(contours) => contours.iter().flatten().copied().collect(),
                })
                .filter(|&(px, _)| px >= label_x0 && px <= label_x1)
                .fold(f32::MIN, |acc, (_, py)| acc.max(py));
            assert!(
                label_top > arrow_bot_over_label,
                "{arrow_ch}: label top ({label_top}) must clear arrow ink above it ({arrow_bot_over_label})"
            );
        }
    }
}
