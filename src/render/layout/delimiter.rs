//! Tall delimiters (parentheses, brackets, braces, floors, ceilings,
//! absolute value, norm, and evaluation bars) that stretch to match their contents.
//!
//! Delimiters are drawn in one of three tiers:
//! 1. Straight bars (`|`, `‖`): scaled vertically via exact vector outline scaling
//!    from `STIXTwoMath-Regular.ttf`, preserving stroke weight and bar spacing.
//! 2. Curved delimiters (Tier 1, `span < min_piece_h`): single complete glyph scaled
//!    vertically (`em_y`) while keeping horizontal scale at normal `em`.
//! 3. Tier 2 & Tier 3 (moderate and tall spans): glyph piece assembly
//!    (head, middle cusp, tail, and clipped/tiled extenders).

use super::*;
use crate::render::font::Seg;

/// Check whether `node` represents a vertical evaluation bar base (`|`).
pub(crate) fn is_eval_bar_base(node: &MathNode) -> bool {
    let text = node.text.trim();
    if (node.tag == "mo" || node.tag == "mi") && (text == "|" || text == "\u{2223}") {
        return true;
    }
    if node.tag == "mrow" && node.children.len() == 1 {
        let child = &node.children[0];
        let child_text = child.text.trim();
        return (child.tag == "mo" || child.tag == "mi")
            && (child_text == "|" || child_text == "\u{2223}");
    }
    false
}

/// Glyph pieces of one stretchy delimiter (`None` when a hook is absent).
struct Pieces {
    top: Option<char>,
    ext: char,
    mid: Option<char>,
    bot: Option<char>,
}

impl Pieces {
    const fn new(top: Option<char>, ext: char, mid: Option<char>, bot: Option<char>) -> Self {
        Self { top, ext, mid, bot }
    }
}

/// Vertical metrics of one assembly piece (pixels).
#[derive(Clone, Copy)]
struct PieceGeom {
    asc: f32,
    h: f32,
    stem_top: f32,
    stem_bot: f32,
}

/// Pinned assembly piece together with its vertical clip band `[clip_top, clip_bot]`.
struct Placed {
    ch: char,
    y: f32,
    top: f32,
    bot: f32,
    stem_top: f32,
    stem_bot: f32,
    clip_top: f32,
    clip_bot: f32,
}

impl Layout {
    /// Build delimiter `ch` spanning `asc` above and `desc` below the baseline.
    pub(super) fn stretchy_delimiter(
        &self,
        ch: char,
        asc: f32,
        desc: f32,
        em: f32,
    ) -> Option<LBox> {
        let norm_ch = match ch {
            '|' | '\u{2223}' => '|',
            '‖' | '\u{2225}' => '‖',
            '\u{301A}' => '⟦',
            '\u{301B}' => '⟧',
            _ => ch,
        };

        // Straight vertical bars (`|.|` and `‖.‖`): scale font outlines vertically
        // while keeping horizontal stroke thickness and double-bar gap unchanged.
        if matches!(norm_ch, '|' | '‖') {
            let face = Face::Symbol;
            let span = asc + desc;
            let top_y = -asc;
            let bot_y = desc;
            let pad = 0.04 * em;

            let natural: Vec<Vec<(f32, f32)>> =
                self.font_set.glyph_polys(face, norm_ch, em, em, 0.0, 0.0);
            if natural.is_empty() {
                return None;
            }
            let (y_lo, y_hi) = natural
                .iter()
                .flatten()
                .fold((f32::MAX, f32::MIN), |(lo, hi), point| {
                    (lo.min(point.1), hi.max(point.1))
                });
            let natural_h = y_hi - y_lo;
            if natural_h <= 0.0 {
                return None;
            }

            let em_y = em * span / natural_h;
            let em_x = em;

            let center = (top_y + bot_y) / 2.0;
            let y = center - (y_lo + y_hi) / 2.0 * (em_y / em);
            let polys = self.font_set.glyph_polys(face, norm_ch, em_x, em_y, pad, y);
            let width = self.font_set.advance(face, norm_ch, em_x) + 2.0 * pad;

            return Some(LBox {
                hits: Vec::new(),
                w: width,
                asc,
                desc,
                items: vec![Item::Contours(polys)],
            });
        }

        // Double brackets (`⟦`, `⟧`): four disjoint rectangles so both the rasterizer
        // and EMF winding-fill keep the hollow stem gap and uniform cap thickness.
        if matches!(norm_ch, '⟦' | '⟧') {
            let span = asc + desc;
            let top_y = -asc;
            let bot_y = desc;
            let pad = 0.05 * em;
            let v_thick = (0.065 * em).max(1.0);
            let h_thick = (0.052 * em).max(1.0);
            let gap = 0.115 * em;
            let overhang = 0.14 * em;
            let arm_w = 2.0 * v_thick + gap + overhang;
            let stem_h = (span - 2.0 * h_thick).max(0.0);
            let stem_y = top_y + h_thick;
            let (stem1_x, stem2_x) = if norm_ch == '⟦' {
                (pad, pad + v_thick + gap)
            } else {
                (pad + overhang, pad + overhang + v_thick + gap)
            };
            let mut items = Vec::with_capacity(4);
            items.push(Item::Poly(rect(pad, top_y, arm_w, h_thick)));
            items.push(Item::Poly(rect(pad, bot_y - h_thick, arm_w, h_thick)));
            if stem_h > 0.0 {
                items.push(Item::Poly(rect(stem1_x, stem_y, v_thick, stem_h)));
                items.push(Item::Poly(rect(stem2_x, stem_y, v_thick, stem_h)));
            }
            return Some(LBox {
                hits: Vec::new(),
                w: arm_w + 2.0 * pad,
                asc,
                desc,
                items,
            });
        }

        let piece_recipe = recipe(norm_ch)?;
        let face = Face::Symbol;
        let span = asc + desc;
        let top_y = -asc;
        let bot_y = desc;
        let center = (top_y + bot_y) / 2.0;
        let pad = 0.05 * em;

        // Match piece stem weight (~112 font units) to regular delimiters (~82 font units).
        const PIECE_SCALE: f32 = 82.0 / 112.0;
        let eff_em = em * PIECE_SCALE;

        let min_piece_h = self.min_assembly_span(face, &piece_recipe, eff_em);

        // Tier 1: span < min_piece_h uses vertical-only scaling of the complete glyph.
        if span < min_piece_h {
            let natural: Vec<Vec<(f32, f32)>> =
                self.font_set.glyph_polys(face, norm_ch, em, em, 0.0, 0.0);
            if natural.is_empty() {
                return None;
            }
            let (y_lo, y_hi) = natural
                .iter()
                .flatten()
                .fold((f32::MAX, f32::MIN), |(lo, hi), point| {
                    (lo.min(point.1), hi.max(point.1))
                });
            let natural_h = y_hi - y_lo;
            if natural_h <= 0.0 {
                return None;
            }

            let em_y = em * span / natural_h;
            let em_x = em;

            let y = center - (y_lo + y_hi) / 2.0 * (em_y / em);
            let polys = self.font_set.glyph_polys(face, norm_ch, em_x, em_y, pad, y);
            let width = self.font_set.advance(face, norm_ch, em_x) + 2.0 * pad;

            return Some(LBox {
                hits: Vec::new(),
                w: width,
                asc,
                desc,
                items: vec![Item::Contours(polys)],
            });
        }

        // Tier 2 & Tier 3: clipped glyph assembly.
        let (_, _, ext_xmin, ext_xmax) = self.exact_ink(face, piece_recipe.ext, eff_em);
        let overlap = 0.02 * eff_em;

        let mut pieces: Vec<Placed> = Vec::new();
        let mut place = |piece_ch: char, top: f32| {
            let geom = self.piece_geom(face, piece_ch, eff_em, ext_xmin, ext_xmax);
            pieces.push(Placed {
                ch: piece_ch,
                y: top + geom.asc,
                top,
                bot: top + geom.h,
                stem_top: geom.stem_top,
                stem_bot: geom.stem_bot,
                clip_top: f32::NEG_INFINITY,
                clip_bot: f32::INFINITY,
            });
        };
        if let Some(top_ch) = piece_recipe.top {
            place(top_ch, top_y);
        }
        if let Some(mid_ch) = piece_recipe.mid {
            let (piece_asc, piece_desc, _, _) = self.exact_ink(face, mid_ch, eff_em);
            place(mid_ch, center - (piece_asc + piece_desc) / 2.0);
        }
        if let Some(bot_ch) = piece_recipe.bot {
            let (piece_asc, piece_desc, _, _) = self.exact_ink(face, bot_ch, eff_em);
            place(bot_ch, bot_y - (piece_asc + piece_desc));
        }

        // Resolve every joint, including the outer boundaries for pieces without a
        // top or bottom hook (floors, ceilings).
        let piece_count = pieces.len();
        let mut fills: Vec<(f32, f32)> = Vec::new();
        for i in 0..=piece_count {
            let upper_idx = i.checked_sub(1);
            let lower_idx = (i < piece_count).then_some(i);
            let upper_end = upper_idx.map_or(top_y, |idx| pieces[idx].bot);
            let lower_start = lower_idx.map_or(bot_y, |idx| pieces[idx].top);
            if lower_start > upper_end {
                let from = upper_end - if upper_idx.is_some() { overlap } else { 0.0 };
                let to = lower_start + if lower_idx.is_some() { overlap } else { 0.0 };
                fills.push((from, to));
                continue;
            }
            let (joint_lo, joint_hi, seam) = match (upper_idx, lower_idx) {
                (Some(upper_i), Some(lower_i)) => {
                    let (upper_piece, lower_piece) = (&pieces[upper_i], &pieces[lower_i]);
                    (
                        (upper_piece.bot - upper_piece.stem_bot).max(lower_piece.top),
                        upper_piece.bot.min(lower_piece.top + lower_piece.stem_top),
                        overlap / 2.0,
                    )
                }
                (None, Some(_)) => (top_y, top_y, 0.0),
                (Some(_), None) => (bot_y, bot_y, 0.0),
                (None, None) => continue,
            };
            let want = (upper_end + lower_start) / 2.0;
            let joint = if joint_lo <= joint_hi {
                want.clamp(joint_lo, joint_hi)
            } else {
                (joint_lo + joint_hi) / 2.0
            };
            if let Some(upper_i) = upper_idx {
                pieces[upper_i].clip_bot = pieces[upper_i].clip_bot.min(joint + seam);
            }
            if let Some(lower_i) = lower_idx {
                pieces[lower_i].clip_top = pieces[lower_i].clip_top.max(joint - seam);
            }
        }

        // Unclipped pieces stay exact glyphs; clipped ones become cut polygons.
        let mut items: Vec<Item> = Vec::new();
        let emit =
            |piece_ch: char, y: f32, ink: (f32, f32), clip: (f32, f32), items: &mut Vec<Item>| {
                if clip.0 <= ink.0 + 1e-3 && clip.1 >= ink.1 - 1e-3 {
                    items.push(Item::Glyph {
                        ch: piece_ch,
                        face,
                        em: eff_em,
                        x: pad,
                        y,
                        ssty: 0,
                    });
                    return;
                }
                for poly in self
                    .font_set
                    .glyph_polys(face, piece_ch, eff_em, eff_em, pad, y)
                {
                    let cut = clip_poly_y(&poly, clip.0, clip.1);
                    if cut.len() >= 3 {
                        items.push(Item::Poly(cut));
                    }
                }
            };
        for piece in &pieces {
            emit(
                piece.ch,
                piece.y,
                (piece.top, piece.bot),
                (piece.clip_top, piece.clip_bot),
                &mut items,
            );
        }
        let (ext_asc, ext_desc, _, _) = self.exact_ink(face, piece_recipe.ext, eff_em);
        let ext_h = (ext_asc + ext_desc).max(1.0);
        let step = (ext_h - overlap).max(1.0);
        for (from, to) in fills {
            let mut cursor_y = from;
            while cursor_y < to - 1e-3 {
                emit(
                    piece_recipe.ext,
                    cursor_y + ext_asc,
                    (cursor_y, cursor_y + ext_h),
                    (from, to),
                    &mut items,
                );
                cursor_y += step;
            }
        }

        let width = [
            Some(piece_recipe.ext),
            piece_recipe.top,
            piece_recipe.mid,
            piece_recipe.bot,
        ]
        .into_iter()
        .flatten()
        .map(|piece_ch| self.font_set.advance(face, piece_ch, eff_em))
        .fold(0.0f32, f32::max);

        Some(LBox {
            hits: Vec::new(),
            w: width + 2.0 * pad,
            asc,
            desc,
            items,
        })
    }

    /// Build an evaluation bar (`|_{sub}^{sup}`) stretching across `(asc, desc)`.
    pub(super) fn evaluation_bar(
        &mut self,
        sub_node: Option<&MathNode>,
        sup_node: Option<&MathNode>,
        asc: f32,
        desc: f32,
        em: f32,
        ctx: MathContext,
    ) -> Result<LBox> {
        let span = (asc + desc).max(1.35 * em);
        let center = (desc - asc) / 2.0;
        let actual_asc = (span / 2.0 - center).max(asc);
        let actual_desc = (span / 2.0 + center).max(desc);

        let bar_box = self
            .stretchy_delimiter('|', actual_asc, actual_desc, em)
            .unwrap_or_else(|| self.glyph('|', Face::Symbol, em));

        let constants = self.font_set.constants();
        let sub_ctx = ctx.for_subscript();
        let sup_ctx = ctx.for_superscript();
        let sub_em = em * ctx.rel_scale(sub_ctx, constants);
        let sup_em = em * ctx.rel_scale(sup_ctx, constants);

        let sub = match sub_node {
            Some(node)
                if node.tag != "none"
                    && (!node.children.is_empty() || !node.text.trim().is_empty()) =>
            {
                Some(self.node(node, sub_em, sub_ctx)?)
            }
            _ => None,
        };
        let sup = match sup_node {
            Some(node)
                if node.tag != "none"
                    && (!node.children.is_empty() || !node.text.trim().is_empty()) =>
            {
                Some(self.node(node, sup_em, sup_ctx)?)
            }
            _ => None,
        };

        if sub.is_none() && sup.is_none() {
            return Ok(bar_box);
        }

        let script_x = bar_box.w + 0.04 * em;
        let shift_up = constants.superscript_shift_up * em;
        let shift_down = constants.subscript_shift_down * em;
        let gap_min = constants.sub_superscript_gap_min * em;

        let sup_y = sup
            .as_ref()
            .map(|sup_box| (-actual_asc + sup_box.asc + 0.04 * em).min(-shift_up));

        let mut sub_y = sub
            .as_ref()
            .map(|sub_box| (actual_desc - sub_box.desc - 0.04 * em).max(shift_down));

        if let (Some(sub_box), Some(sup_box), Some(sub_pos), Some(sup_pos)) =
            (&sub, &sup, sub_y, sup_y)
        {
            let gap = (sub_pos - sub_box.asc) - (sup_pos + sup_box.desc);
            if gap < gap_min {
                sub_y = Some(sub_pos + (gap_min - gap));
            }
        }

        let mut out = LBox {
            hits: Vec::new(),
            w: script_x
                + sub
                    .as_ref()
                    .map_or(0.0, |sub_box| sub_box.w)
                    .max(sup.as_ref().map_or(0.0, |sup_box| sup_box.w)),
            asc: actual_asc.max(
                sup.as_ref()
                    .map_or(0.0, |sup_box| -sup_y.unwrap_or(0.0) + sup_box.asc),
            ),
            desc: actual_desc.max(
                sub.as_ref()
                    .map_or(0.0, |sub_box| sub_y.unwrap_or(0.0) + sub_box.desc),
            ),
            items: Vec::new(),
        };

        out.place(&bar_box, 0.0, 0.0);
        if let (Some(sub_box), Some(y)) = (&sub, sub_y) {
            out.place(sub_box, script_x, y);
        }
        if let (Some(sup_box), Some(y)) = (&sup, sup_y) {
            out.place(sup_box, script_x, y);
        }

        Ok(out)
    }

    /// Exact (unrounded) ink box of `ch` at `em`: `(asc, desc, xmin, xmax)` in pixels.
    fn exact_ink(&self, face: Face, ch: char, em: f32) -> (f32, f32, f32, f32) {
        let Some(segs) = self.font_set.glyph_segments(face, ch, em, 0.0, 0.0) else {
            return (0.0, 0.0, 0.0, 0.0);
        };
        let (mut x0, mut x1, mut y0, mut y1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
        let mut add = |x: f32, y: f32| {
            x0 = x0.min(x);
            x1 = x1.max(x);
            y0 = y0.min(y);
            y1 = y1.max(y);
        };
        for seg in segs {
            match seg {
                Seg::Move(x, y)
                | Seg::Line(x, y)
                | Seg::Quad(_, _, x, y)
                | Seg::Cubic(_, _, _, _, x, y) => add(x, y),
                Seg::Close => {}
            }
        }
        if x0 > x1 {
            return (0.0, 0.0, 0.0, 0.0);
        }
        ((-y0).max(0.0), y1.max(0.0), x0, x1)
    }

    /// Calculate `(straight_stem_len_top, straight_stem_len_bot)` in pixels at `eff_em`.
    fn piece_stem_lengths(
        &self,
        face: Face,
        ch: char,
        eff_em: f32,
        ext_xmin: f32,
        ext_xmax: f32,
    ) -> (f32, f32) {
        let (asc, desc, _, _) = self.exact_ink(face, ch, eff_em);
        let total_h = asc + desc;
        if total_h <= 0.0 {
            return (0.0, 0.0);
        }
        let y_top = -asc;
        let y_bot = desc;

        let Some(segs) = self.font_set.glyph_segments(face, ch, eff_em, 0.0, 0.0) else {
            return (0.0, 0.0);
        };

        // ~5 font units: only points lying on the column edges count as straight stem.
        let tol = 0.005 * eff_em;
        let on_edge = |x: f32| (x - ext_xmin).abs() <= tol || (x - ext_xmax).abs() <= tol;

        let mut min_curved_y = f32::INFINITY;
        let mut max_curved_y = f32::NEG_INFINITY;
        let mut mark = |points: &[(f32, f32)]| {
            if points.iter().any(|&(x, _)| !on_edge(x)) {
                for &(_, y) in points {
                    min_curved_y = min_curved_y.min(y);
                    max_curved_y = max_curved_y.max(y);
                }
            }
        };

        let mut cur = (0.0f32, 0.0f32);
        let mut start = cur;
        for seg in segs {
            match seg {
                Seg::Move(x, y) => {
                    cur = (x, y);
                    start = cur;
                }
                Seg::Line(x, y) => {
                    mark(&[cur, (x, y)]);
                    cur = (x, y);
                }
                Seg::Quad(cx, cy, x, y) => {
                    mark(&[cur, (cx, cy), (x, y)]);
                    cur = (x, y);
                }
                Seg::Cubic(c1x, c1y, c2x, c2y, x, y) => {
                    mark(&[cur, (c1x, c1y), (c2x, c2y), (x, y)]);
                    cur = (x, y);
                }
                Seg::Close => {
                    mark(&[cur, start]);
                    cur = start;
                }
            }
        }

        if min_curved_y.is_infinite() {
            (total_h, total_h)
        } else {
            let top_stem = (min_curved_y - y_top).max(0.0);
            let bot_stem = (y_bot - max_curved_y).max(0.0);
            (top_stem, bot_stem)
        }
    }

    fn piece_geom(
        &self,
        face: Face,
        ch: char,
        eff_em: f32,
        ext_xmin: f32,
        ext_xmax: f32,
    ) -> PieceGeom {
        let (asc, desc, _, _) = self.exact_ink(face, ch, eff_em);
        let (stem_top, stem_bot) = self.piece_stem_lengths(face, ch, eff_em, ext_xmin, ext_xmax);
        PieceGeom {
            asc,
            h: asc + desc,
            stem_top,
            stem_bot,
        }
    }

    fn min_assembly_span(&self, face: Face, pieces: &Pieces, eff_em: f32) -> f32 {
        let (_, _, xmin, xmax) = self.exact_ink(face, pieces.ext, eff_em);
        let geom = |ch: Option<char>| {
            ch.map(|piece_ch| self.piece_geom(face, piece_ch, eff_em, xmin, xmax))
        };
        let (top, mid, bot) = (geom(pieces.top), geom(pieces.mid), geom(pieces.bot));

        let span = match mid {
            Some(mid_geom) => {
                let upper = top.map_or(mid_geom.h / 2.0 - mid_geom.stem_top, |top_geom| {
                    top_geom.h + mid_geom.h / 2.0 - (top_geom.stem_bot + mid_geom.stem_top)
                });
                let lower = bot.map_or(mid_geom.h / 2.0 - mid_geom.stem_bot, |bot_geom| {
                    bot_geom.h + mid_geom.h / 2.0 - (bot_geom.stem_top + mid_geom.stem_bot)
                });
                2.0 * upper.max(lower)
            }
            None => match (top, bot) {
                (Some(top_geom), Some(bot_geom)) => {
                    top_geom.h + bot_geom.h - (top_geom.stem_bot + bot_geom.stem_top)
                }
                (Some(top_geom), None) => top_geom.h - top_geom.stem_bot,
                (None, Some(bot_geom)) => bot_geom.h - bot_geom.stem_top,
                (None, None) => 0.0,
            },
        };
        span.max(0.0)
    }
}

/// Assembly recipe for `ch`, or `None` when `ch` is not a glyph-assembled delimiter.
fn recipe(ch: char) -> Option<Pieces> {
    Some(match ch {
        '(' => Pieces::new(Some('\u{239B}'), '\u{239C}', None, Some('\u{239D}')),
        ')' => Pieces::new(Some('\u{239E}'), '\u{239F}', None, Some('\u{23A0}')),
        '[' => Pieces::new(Some('\u{23A1}'), '\u{23A2}', None, Some('\u{23A3}')),
        ']' => Pieces::new(Some('\u{23A4}'), '\u{23A5}', None, Some('\u{23A6}')),
        '⌈' => Pieces::new(Some('\u{23A1}'), '\u{23A2}', None, None),
        '⌉' => Pieces::new(Some('\u{23A4}'), '\u{23A5}', None, None),
        '⌊' => Pieces::new(None, '\u{23A2}', None, Some('\u{23A3}')),
        '⌋' => Pieces::new(None, '\u{23A5}', None, Some('\u{23A6}')),
        '{' => Pieces::new(
            Some('\u{23A7}'),
            '\u{23AA}',
            Some('\u{23A8}'),
            Some('\u{23A9}'),
        ),
        '}' => Pieces::new(
            Some('\u{23AB}'),
            '\u{23AA}',
            Some('\u{23AC}'),
            Some('\u{23AD}'),
        ),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> Layout {
        Layout {
            font_set: fonts(),
            depth: 0,
        }
    }

    #[test]
    fn inspect_auto_calculated_min_spans() {
        let layout = layout();
        let em = 100.0;
        let face = Face::Symbol;
        const PIECE_SCALE: f32 = 82.0 / 112.0;
        let eff_em = em * PIECE_SCALE;
        for ch in ['(', ')', '[', ']', '{', '}'] {
            let piece_recipe = recipe(ch).unwrap();
            let min_span = layout.min_assembly_span(face, &piece_recipe, eff_em);
            println!(
                "DELIMITER '{ch}': auto-calculated min_piece_h = {:.2} px ({:.3} em)",
                min_span,
                min_span / em
            );
        }
    }

    #[test]
    fn tall_delimiters_piece_assembly_and_vertical_stretch() {
        let layout = layout();
        let em = 24.0;
        for ch in ['(', ')', '[', ']', '{', '}', '|', '‖'] {
            let short_span = 0.6 * em;
            let asc_short = short_span / 2.0 + AXIS * em;
            let desc_short = short_span / 2.0 - AXIS * em;
            let box_short = layout
                .stretchy_delimiter(ch, asc_short, desc_short, em)
                .expect("delimiter supported");
            assert_eq!(box_short.asc, asc_short);
            assert_eq!(box_short.desc, desc_short);
            if ch != '[' && ch != ']' {
                assert_eq!(box_short.items.len(), 1);
            }
            assert!(
                box_short
                    .items
                    .iter()
                    .all(|item| matches!(item, Item::Poly(_) | Item::Contours(_))),
                "{ch}: short should be Poly or Contours"
            );
            if ch == '‖' {
                assert!(matches!(
                    box_short.items.first(),
                    Some(Item::Contours(contours)) if contours.len() == 2
                ));
            }

            let tall_span = 6.0 * em;
            let asc_tall = tall_span / 2.0 + AXIS * em;
            let desc_tall = tall_span / 2.0 - AXIS * em;
            let box_tall = layout
                .stretchy_delimiter(ch, asc_tall, desc_tall, em)
                .expect("delimiter supported");
            assert_eq!(box_tall.asc, asc_tall);
            assert_eq!(box_tall.desc, desc_tall);
            if ch == '|' || ch == '‖' {
                assert!(
                    box_tall
                        .items
                        .iter()
                        .all(|item| matches!(item, Item::Poly(_) | Item::Contours(_))),
                    "{ch}: tall bar should be Poly or Contours"
                );
            } else {
                assert!(
                    box_tall
                        .items
                        .iter()
                        .any(|item| matches!(item, Item::Glyph { .. })),
                    "{ch}: tall should have Glyph pieces"
                );
                if ch == '{' {
                    assert!(
                        box_tall.items.len() > 3,
                        "curly brace at 6.0em should have extenders"
                    );
                }
            }
        }

        for scale in [1.6f32, 2.0, 2.5, 3.0] {
            let span = scale * em;
            let asc = span / 2.0 + AXIS * em;
            let desc = span / 2.0 - AXIS * em;
            let box_brace = layout
                .stretchy_delimiter('{', asc, desc, em)
                .expect("delimiter supported");
            assert_eq!(
                box_brace.items.len(),
                3,
                "curly brace at {scale}em should be 3 pieces without extenders"
            );
            for item in &box_brace.items {
                if let Item::Poly(points) = item {
                    for &(_, y) in points {
                        assert!(
                            y >= -asc - 1e-2 && y <= desc + 1e-2,
                            "point y={y} outside [{}, {desc}]",
                            -asc
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn double_brackets_render_as_four_disjoint_hollow_rects() {
        let layout = layout();
        let em = 24.0;
        let asc = 1.4 * em;
        let desc = 0.9 * em;
        for ch in ['⟦', '⟧'] {
            let box_br = layout.stretchy_delimiter(ch, asc, desc, em).unwrap();
            assert_eq!(
                box_br.items.len(),
                4,
                "{ch}: must have top, bottom, and two stems"
            );
            let rects: Vec<(f32, f32, f32, f32)> = box_br
                .items
                .iter()
                .map(|item| match item {
                    Item::Poly(points) => {
                        let (mut x0, mut x1, mut y0, mut y1) =
                            (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
                        for &(x, y) in points {
                            x0 = x0.min(x);
                            x1 = x1.max(x);
                            y0 = y0.min(y);
                            y1 = y1.max(y);
                        }
                        (x0, y0, x1, y1)
                    }
                    _ => panic!("expected Poly rect"),
                })
                .collect();
            let top_bot = rects[0].3;
            let bot_top = rects[1].1;
            let (stem1, stem2) = (rects[2], rects[3]);
            assert!((stem1.1 - top_bot).abs() < 1e-3);
            assert!((stem1.3 - bot_top).abs() < 1e-3);
            assert!(
                stem2.0 - stem1.2 > 0.05 * em,
                "hollow gap between vertical bars"
            );
        }
    }

    #[test]
    fn evaluation_bar_layout_and_limits() {
        let mut layout = layout();
        let em = 24.0;
        let ctx = MathContext::DISPLAY;
        let sub = MathNode::leaf("mn", "0");
        let sup = MathNode::leaf("mn", "1");

        let bar_box = layout
            .evaluation_bar(Some(&sub), Some(&sup), 1.5 * em, 1.2 * em, em, ctx)
            .unwrap();
        assert!(bar_box.w > 0.4 * em);
        assert!(bar_box.asc >= 1.5 * em);
        assert!(bar_box.desc >= 1.2 * em);
        assert!(bar_box
            .items
            .iter()
            .any(|item| matches!(item, Item::Poly(_) | Item::Contours(_))));
        assert!(bar_box
            .items
            .iter()
            .any(|item| matches!(item, Item::Glyph { ch: '0', .. })));
        assert!(bar_box
            .items
            .iter()
            .any(|item| matches!(item, Item::Glyph { ch: '1', .. })));
    }

    #[test]
    fn unsupported_characters_return_none() {
        let layout = layout();
        let em = 24.0;
        assert!(layout.stretchy_delimiter('x', 1.0, 1.0, em).is_none());
        assert!(layout.stretchy_delimiter('?', 1.0, 1.0, em).is_none());
    }

    #[test]
    fn brace_piece_stems_stop_where_curve_starts() {
        let layout = layout();
        let face = Face::Symbol;
        let em = 1000.0;
        let (_, _, xmin, xmax) = layout.exact_ink(face, '\u{23AA}', em);
        let near = |left: f32, right: f32| (left - right).abs() <= 2.0;
        let (_, bot) = layout.piece_stem_lengths(face, '\u{23A7}', em, xmin, xmax);
        assert!(near(bot, 645.0), "⎧ bottom stem = {bot}");
        let (top, bot) = layout.piece_stem_lengths(face, '\u{23A8}', em, xmin, xmax);
        assert!(
            near(top, 560.0) && near(bot, 558.0),
            "⎨ stems = {top}, {bot}"
        );
        let (top, _) = layout.piece_stem_lengths(face, '\u{23A9}', em, xmin, xmax);
        assert!(near(top, 646.0), "⎩ top stem = {top}");
    }
}
