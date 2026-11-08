//! Tall delimiters (parentheses, brackets, braces, floors, ceilings,
//! absolute value, norm, and evaluation bars) that stretch to match the exact height of their contents.
//!
//! Delimiters are drawn in one of three tiers:
//! 1. Straight bars (Absolute value `|`, Norm `‖`): Stretched vertically via exact vector
//!    outline scaling from `STIXTwoMath-Regular.ttf`, preserving font line weight and double-bar spacing.
//! 2. Curved delimiters (Tier 1, Small span < min_piece_h): Single complete glyph scaled vertically (`em_y`)
//!    while keeping horizontal scale at normal `em`.
//! 3. Tier 2 & Tier 3 (Moderate and tall spans): Glyph piece assembly (head, mid, tail, extenders).

use super::*;
use crate::render::font::Seg;

/// The glyph pieces of one stretchy delimiter. `None` means "not present"
/// (e.g. a floor has no top hook, a ceiling has no bottom hook).
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

/// Assembly recipe for `c`, or `None` when `c` is not a glyph-assembled delimiter.
fn recipe(c: char) -> Option<Pieces> {
    Some(match c {
        '(' => Pieces::new(Some('\u{239B}'), '\u{239C}', None, Some('\u{239D}')),
        ')' => Pieces::new(Some('\u{239E}'), '\u{239F}', None, Some('\u{23A0}')),
        '[' => Pieces::new(Some('\u{23A1}'), '\u{23A2}', None, Some('\u{23A3}')),
        ']' => Pieces::new(Some('\u{23A4}'), '\u{23A5}', None, Some('\u{23A6}')),
        '⌈' => Pieces::new(Some('\u{23A1}'), '\u{23A2}', None, None),
        '⌉' => Pieces::new(Some('\u{23A4}'), '\u{23A5}', None, None),
        '⌊' => Pieces::new(None, '\u{23A2}', None, Some('\u{23A3}')),
        '⌋' => Pieces::new(None, '\u{23A5}', None, Some('\u{23A6}')),
        '{' => Pieces::new(Some('\u{23A7}'), '\u{23AA}', Some('\u{23A8}'), Some('\u{23A9}')),
        '}' => Pieces::new(Some('\u{23AB}'), '\u{23AA}', Some('\u{23AC}'), Some('\u{23AD}')),
        _ => return None,
    })
}

/// Helper to check if a node represents a vertical evaluation bar base `|`.
pub(crate) fn is_eval_bar_base(node: &MathNode) -> bool {
    let t = node.text.trim();
    if (node.tag == "mo" || node.tag == "mi") && (t == "|" || t == "\u{2223}") {
        return true;
    }
    if node.tag == "mrow" && node.children.len() == 1 {
        let child = &node.children[0];
        let ct = child.text.trim();
        return (child.tag == "mo" || child.tag == "mi") && (ct == "|" || ct == "\u{2223}");
    }
    false
}

impl Layout {
    /// Exact (unrounded) ink box of `ch` at `em`: `(asc, desc, xmin, xmax)` in pixels.
    ///
    /// `Fonts::ink` / `Fonts::ink_x` snap to the pixel grid, which is too coarse for
    /// aligning assembly pieces and for detecting where a stem stops being straight.
    fn exact_ink(&self, face: Face, ch: char, em: f32) -> (f32, f32, f32, f32) {
        let Some(segs) = self.f.glyph_segments(face, ch, em, 0.0, 0.0) else {
            return (0.0, 0.0, 0.0, 0.0);
        };
        let (mut x0, mut x1, mut y0, mut y1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
        let mut add = |x: f32, y: f32| {
            x0 = x0.min(x);
            x1 = x1.max(x);
            y0 = y0.min(y);
            y1 = y1.max(y);
        };
        for s in segs {
            match s {
                Seg::Move(x, y) | Seg::Line(x, y) | Seg::Quad(_, _, x, y) | Seg::Cubic(_, _, _, _, x, y) => add(x, y),
                Seg::Close => {}
            }
        }
        if x0 > x1 {
            return (0.0, 0.0, 0.0, 0.0);
        }
        ((-y0).max(0.0), y1.max(0.0), x0, x1)
    }

    /// Calculate the straight stem lengths of a delimiter piece glyph.
    ///
    /// Returns `(straight_stem_len_top, straight_stem_len_bot)` in pixels at `eff_em`.
    ///
    /// A piece is "straight" only where its outline runs exactly along the extender
    /// column edges `ext_xmin` / `ext_xmax`. Any segment that leaves those edges —
    /// whether it bends *outside* the column (hook tip, cusp point) or *inside* it
    /// (the stroke starting to drift towards the hook) — is a curved feature, and its
    /// whole vertical range (including its start point) is excluded from the stem.
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

        let Some(segs) = self.f.glyph_segments(face, ch, eff_em, 0.0, 0.0) else {
            return (0.0, 0.0);
        };

        // ~5 font units: only points lying on the column edges count as stem.
        let tol = 0.005 * eff_em;
        let on_edge = |x: f32| (x - ext_xmin).abs() <= tol || (x - ext_xmax).abs() <= tol;

        let mut min_curved_y = f32::INFINITY;
        let mut max_curved_y = f32::NEG_INFINITY;
        let mut mark = |pts: &[(f32, f32)]| {
            if pts.iter().any(|&(x, _)| !on_edge(x)) {
                for &(_, y) in pts {
                    min_curved_y = min_curved_y.min(y);
                    max_curved_y = max_curved_y.max(y);
                }
            }
        };

        let mut cur = (0.0f32, 0.0f32);
        let mut start = cur;
        for s in segs {
            match s {
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

    /// Calculate the minimum delimiter height (`span`) at which font glyph pieces can
    /// begin to be assembled safely without visual collision artifacts.
    ///
    /// - For delimiters without middle cusp (parentheses, brackets): end pieces meet end-to-end at `h_top + h_bot`.
    /// - For delimiters with middle cusp (curly braces): 3 pieces (head, cusp, tail) can safely overlap
    ///   along their straight stems up to the curved cusp boundary, giving:
    ///   `min_span = (h_top + h_mid + h_bot) - (safe_overlap_top + safe_overlap_bot)`.
    fn min_assembly_span(&self, face: Face, r: &Pieces, eff_em: f32) -> f32 {
        let piece_h = |ch: Option<char>| -> f32 {
            ch.map_or(0.0, |c| {
                let (a, d, _, _) = self.exact_ink(face, c, eff_em);
                a + d
            })
        };

        let h_top = piece_h(r.top);
        let h_bot = piece_h(r.bot);

        let Some(mid_ch) = r.mid else {
            return h_top + h_bot;
        };

        let h_mid = piece_h(Some(mid_ch));
        let (_, _, ext_xmin, ext_xmax) = self.exact_ink(face, r.ext, eff_em);

        let (_top_stem_top, bot_stem_top) = r.top.map_or((0.0, 0.0), |t| {
            self.piece_stem_lengths(face, t, eff_em, ext_xmin, ext_xmax)
        });
        let (top_stem_mid, bot_stem_mid) = self.piece_stem_lengths(face, mid_ch, eff_em, ext_xmin, ext_xmax);
        let (top_stem_bot, _bot_stem_bot) = r.bot.map_or((0.0, 0.0), |b| {
            self.piece_stem_lengths(face, b, eff_em, ext_xmin, ext_xmax)
        });

        let safe_overlap_top = bot_stem_top.min(top_stem_mid);
        let safe_overlap_bot = top_stem_bot.min(bot_stem_mid);
        // The cusp is centred between head and tail, so both joints overlap by the
        // same amount: the tighter joint limits the total overlap.
        let max_safe_overlap = 2.0 * safe_overlap_top.min(safe_overlap_bot);

        (h_top + h_mid + h_bot - max_safe_overlap).max(0.0)
    }

    /// Build the delimiter `c` so that it spans exactly `asc` above and `desc`
    /// below the baseline. Supports parentheses, brackets, braces, floors, ceilings,
    /// absolute value (`|`), and norm (`‖`).
    pub(super) fn stretchy_delimiter(&self, c: char, asc: f32, desc: f32, em: f32) -> Option<LBox> {
        let norm_c = match c {
            '|' | '\u{2223}' => '|',
            '‖' | '\u{2225}' => '‖',
            _ => c,
        };

        // Straight vertical bars (absolute value `|.|` and norm `||.||`):
        // Scaled vertically using their exact font glyph contours from STIX Two Math,
        // preserving horizontal stroke thickness and double-bar gap identically to the font design.
        if norm_c == '|' || norm_c == '‖' {
            let face = Face::Symbol;
            let span = asc + desc;
            let top_y = -asc;
            let bot_y = desc;
            let pad = 0.04 * em;

            let natural: Vec<Vec<(f32, f32)>> = self.f.glyph_polys(face, norm_c, em, em, 0.0, 0.0);
            if natural.is_empty() {
                return None;
            }
            let (y_lo, y_hi) = natural.iter().flatten().fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p.1), hi.max(p.1)));
            let h0 = y_hi - y_lo;
            if h0 <= 0.0 {
                return None;
            }

            let em_y = em * span / h0;
            let em_x = em; // Preserve line thickness and double-bar gap

            let centre = (top_y + bot_y) / 2.0;
            let y = centre - (y_lo + y_hi) / 2.0 * (em_y / em);
            let polys = self.f.glyph_polys(face, norm_c, em_x, em_y, pad, y);
            let w = self.f.advance(face, norm_c, em_x) + 2.0 * pad;

            return Some(LBox {
                w,
                asc,
                desc,
                items: polys.into_iter().map(Item::Poly).collect(),
            });
        }

        let r = recipe(norm_c)?;
        let face = Face::Symbol;
        let span = asc + desc;
        let top_y = -asc;
        let bot_y = desc;
        let pad = 0.05 * em;

        // Scale factor for font glyph pieces so their stroke thickness matches complete delimiters:
        // In STIX Two Math, piece extenders have stem width 112 font units, whereas normal complete
        // delimiters have stroke width ~82 font units (82 / 112 ≈ 0.732).
        const PIECE_SCALE: f32 = 82.0 / 112.0;
        let eff_em = em * PIECE_SCALE;

        let min_piece_h = self.min_assembly_span(face, &r, eff_em);

        // Tier 1: When span < min_piece_h:
        // Use vertical-only outline scaling of the complete glyph to avoid overlapping piece stems.
        if span < min_piece_h {
            let natural: Vec<Vec<(f32, f32)>> = self.f.glyph_polys(face, norm_c, em, em, 0.0, 0.0);
            if natural.is_empty() {
                return None;
            }
            let (y_lo, y_hi) = natural.iter().flatten().fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p.1), hi.max(p.1)));
            let h0 = y_hi - y_lo;
            if h0 <= 0.0 {
                return None;
            }

            let em_y = em * span / h0;
            let em_x = em; // Scale vertically only; keep horizontal scale at normal em.

            let centre = (top_y + bot_y) / 2.0;
            let y = centre - (y_lo + y_hi) / 2.0 * (em_y / em);
            let polys = self.f.glyph_polys(face, norm_c, em_x, em_y, pad, y);
            let w = self.f.advance(face, norm_c, em_x) + 2.0 * pad;

            return Some(LBox {
                w,
                asc,
                desc,
                items: polys.into_iter().map(Item::Poly).collect(),
            });
        }

        // Tier 2 & Tier 3:
        // When span >= min_piece_h, the pieces are assembled from font glyphs:
        // - Tier 2: 3 pieces without extenders, safely overlapping along their straight stems.
        // - Tier 3: 3 pieces + extenders tiled seamlessly into positive stem gaps.
        let centre = (top_y + bot_y) / 2.0;
        let top_y = centre - span / 2.0;
        let bot_y = centre + span / 2.0;
        let asc = -top_y;
        let desc = bot_y;

        let mut items: Vec<Item> = Vec::new();
        let push = |ch: char, y: f32, items: &mut Vec<Item>| {
            items.push(Item::Glyph { ch, face, em: eff_em, x: 0.0, y, ssty: 0 });
        };
        let ink = |ch: char| {
            let (a, d, _, _) = self.exact_ink(face, ch, eff_em);
            (a, d)
        };

        if r.mid.is_some() {
            // 1. Top piece (head):
            let (a_top, d_top) = ink(r.top.unwrap());
            let y_top = top_y + a_top;
            push(r.top.unwrap(), y_top, &mut items);
            let top_end_y = y_top + d_top;

            // 2. Bottom piece (tail):
            let (a_bot, d_bot) = ink(r.bot.unwrap());
            let y_bot = bot_y - d_bot;
            push(r.bot.unwrap(), y_bot, &mut items);
            let bot_start_y = y_bot - a_bot;

            // 3. Middle cusp:
            let mid_ch = r.mid.unwrap();
            let (a_mid, d_mid) = ink(mid_ch);
            // Centre the cusp's ink box on the delimiter centre.
            let y_mid = centre + (a_mid - d_mid) / 2.0;
            push(mid_ch, y_mid, &mut items);
            let mid_start_y = y_mid - a_mid;
            let mid_end_y = y_mid + d_mid;

            // 4. Extenders tiled into positive gaps (Tier 3 only):
            let (ea, ed) = ink(r.ext);
            let e_h = (ea + ed).max(1.0);
            let fill = |from: f32, to: f32, items: &mut Vec<Item>| {
                let ov = 0.02 * eff_em;
                let (from, to) = (from - ov, to + ov);
                let gap = to - from;
                if gap <= 2.0 * ov + 0.5 {
                    return;
                }
                let count = (gap / e_h).ceil().max(1.0) as usize;
                let step = if count > 1 { (gap - e_h) / (count - 1) as f32 } else { 0.0 };
                let first = if count > 1 { from } else { from + (gap - e_h) / 2.0 };
                for i in 0..count {
                    push(r.ext, first + i as f32 * step + ea, items);
                }
            };

            fill(top_end_y, mid_start_y, &mut items);
            fill(mid_end_y, bot_start_y, &mut items);

            let width = [Some(r.ext), r.top, r.mid, r.bot]
                .into_iter()
                .flatten()
                .map(|ch| self.f.advance(face, ch, eff_em))
                .fold(0.0f32, f32::max);

            for it in &mut items {
                if let Item::Glyph { x, .. } = it {
                    *x += pad;
                }
            }

            return Some(LBox {
                w: width + 2.0 * pad,
                asc,
                desc,
                items,
            });
        }

        // Without mid piece (parentheses, brackets):
        let width = [Some(r.ext), r.top, r.bot]
            .into_iter()
            .flatten()
            .map(|ch| self.f.advance(face, ch, eff_em))
            .fold(0.0f32, f32::max);

        let mut gap_top = top_y;
        let mut gap_bot = bot_y;
        if let Some(t) = r.top {
            let (a, d) = ink(t);
            push(t, top_y + a, &mut items);
            gap_top = top_y + a + d;
        }
        if let Some(b) = r.bot {
            let (a, d) = ink(b);
            push(b, bot_y - d, &mut items);
            gap_bot = bot_y - d - a;
        }

        let (ea, ed) = ink(r.ext);
        let e_h = (ea + ed).max(1.0);
        let fill = |from: f32, to: f32, items: &mut Vec<Item>| {
            let ov = 0.02 * eff_em;
            let (from, to) = (from - ov, to + ov);
            let gap = to - from;
            if gap <= 2.0 * ov + 0.5 {
                return;
            }
            let count = (gap / e_h).ceil().max(1.0) as usize;
            let step = if count > 1 { (gap - e_h) / (count - 1) as f32 } else { 0.0 };
            let first = if count > 1 { from } else { from + (gap - e_h) / 2.0 };
            for i in 0..count {
                push(r.ext, first + i as f32 * step + ea, items);
            }
        };

        fill(gap_top, gap_bot, &mut items);

        for it in &mut items {
            if let Item::Glyph { x, .. } = it {
                *x += pad;
            }
        }

        Some(LBox {
            w: width + 2.0 * pad,
            asc,
            desc,
            items,
        })
    }

    /// Build an evaluation bar (substitution bar for definite integrals) `|_{sub}^{sup}`
    /// that stretches vertically to match `asc` and `desc`, with the lower limit
    /// placed at the bottom and the upper limit placed at the top of the bar.
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
        let centre = (desc - asc) / 2.0;
        let actual_asc = (span / 2.0 - centre).max(asc);
        let actual_desc = (span / 2.0 + centre).max(desc);

        let bar_box = self
            .stretchy_delimiter('|', actual_asc, actual_desc, em)
            .unwrap_or_else(|| self.glyph('|', Face::Symbol, em));

        let c = self.f.constants();
        let sub_ctx = ctx.for_subscript();
        let sup_ctx = ctx.for_superscript();
        let sub_em = em * ctx.rel_scale(sub_ctx, c);
        let sup_em = em * ctx.rel_scale(sup_ctx, c);

        let sub = match sub_node {
            Some(n) if n.tag != "none" && (!n.children.is_empty() || !n.text.trim().is_empty()) => {
                Some(self.node(n, sub_em, sub_ctx)?)
            }
            _ => None,
        };
        let sup = match sup_node {
            Some(n) if n.tag != "none" && (!n.children.is_empty() || !n.text.trim().is_empty()) => {
                Some(self.node(n, sup_em, sup_ctx)?)
            }
            _ => None,
        };

        if sub.is_none() && sup.is_none() {
            return Ok(bar_box);
        }

        let script_x = bar_box.w + 0.04 * em;
        let shift_up = c.superscript_shift_up * em;
        let shift_down = c.subscript_shift_down * em;
        let gap_min = c.sub_superscript_gap_min * em;

        // Position superscript near top of bar:
        let sup_y = sup.as_ref().map(|s| {
            (-actual_asc + s.asc + 0.04 * em).min(-shift_up)
        });

        // Position subscript near bottom of bar:
        let mut sub_y = sub.as_ref().map(|s| {
            (actual_desc - s.desc - 0.04 * em).max(shift_down)
        });

        // Ensure minimum gap between sub and sup:
        if let (Some(s1), Some(s2), Some(sy), Some(py)) = (&sub, &sup, sub_y, sup_y) {
            let gap = (sy - s1.asc) - (py + s2.desc);
            if gap < gap_min {
                sub_y = Some(sy + (gap_min - gap));
            }
        }

        let mut out = LBox {
            w: script_x + sub.as_ref().map_or(0.0, |s| s.w).max(sup.as_ref().map_or(0.0, |s| s.w)),
            asc: actual_asc.max(sup.as_ref().map_or(0.0, |s| -sup_y.unwrap_or(0.0) + s.asc)),
            desc: actual_desc.max(sub.as_ref().map_or(0.0, |s| sub_y.unwrap_or(0.0) + s.desc)),
            items: Vec::new(),
        };

        out.place(&bar_box, 0.0, 0.0);
        if let (Some(s), Some(y)) = (&sub, sub_y) {
            out.place(s, script_x, y);
        }
        if let (Some(s), Some(y)) = (&sup, sup_y) {
            out.place(s, script_x, y);
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> Layout {
        Layout { f: fonts(), depth: 0 }
    }

    #[test]
    fn inspect_auto_calculated_min_spans() {
        let l = layout();
        let em = 100.0;
        let face = Face::Symbol;
        const PIECE_SCALE: f32 = 82.0 / 112.0;
        let eff_em = em * PIECE_SCALE;
        for c in ['(', ')', '[', ']', '{', '}'] {
            let r = recipe(c).unwrap();
            let min_span = l.min_assembly_span(face, &r, eff_em);
            println!("DELIMITER '{c}': auto-calculated min_piece_h = {:.2} px ({:.3} em)", min_span, min_span / em);
        }
    }

    #[test]
    fn tall_delimiters_piece_assembly_and_vertical_stretch() {
        let l = layout();
        let em = 24.0;
        for c in ['(', ')', '[', ']', '{', '}', '|', '‖'] {
            // Short span:
            let short_span = 1.0 * em;
            let asc_s = short_span / 2.0 + AXIS * em;
            let desc_s = short_span / 2.0 - AXIS * em;
            let b_s = l.stretchy_delimiter(c, asc_s, desc_s, em).expect("delimiter supported");
            assert_eq!(b_s.asc, asc_s);
            assert_eq!(b_s.desc, desc_s);
            assert!(b_s.items.iter().all(|it| matches!(it, Item::Poly(_))), "{c}: short should be Poly");

            // Tall span:
            let tall_span = 6.0 * em;
            let asc_t = tall_span / 2.0 + AXIS * em;
            let desc_t = tall_span / 2.0 - AXIS * em;
            let b_t = l.stretchy_delimiter(c, asc_t, desc_t, em).expect("delimiter supported");
            assert_eq!(b_t.asc, asc_t);
            assert_eq!(b_t.desc, desc_t);
            if c == '|' || c == '‖' {
                assert!(b_t.items.iter().all(|it| matches!(it, Item::Poly(_))), "{c}: tall bar should be Poly");
            } else {
                assert!(b_t.items.iter().all(|it| matches!(it, Item::Glyph { .. })), "{c}: tall should be Glyph");
                if c == '{' {
                    assert!(b_t.items.len() > 3, "curly brace at 6.0em should have extenders");
                }
            }
        }

        // For '{': moderate span (Tier 2, e.g. 2.5 * em) assembles with EXACTLY 3 pieces without extenders:
        let mod_span = 2.5 * em;
        let asc_m = mod_span / 2.0 + AXIS * em;
        let desc_m = mod_span / 2.0 - AXIS * em;
        let b_m = l.stretchy_delimiter('{', asc_m, desc_m, em).expect("delimiter supported");
        assert_eq!(b_m.items.len(), 3, "curly brace at 2.5em should have exactly 3 pieces without extenders");
        assert!(b_m.items.iter().all(|it| matches!(it, Item::Glyph { .. })), "curly brace at 2.5em should be Glyphs");
    }

    #[test]
    fn evaluation_bar_layout_and_limits() {
        let mut l = layout();
        let em = 24.0;
        let ctx = MathContext::DISPLAY;
        let sub = MathNode::leaf("mn", "0");
        let sup = MathNode::leaf("mn", "1");

        let b = l.evaluation_bar(Some(&sub), Some(&sup), 1.5 * em, 1.2 * em, em, ctx).unwrap();
        assert!(b.w > 0.4 * em);
        assert!(b.asc >= 1.5 * em);
        assert!(b.desc >= 1.2 * em);
        // Verify bar poly and limits are present
        assert!(b.items.iter().any(|it| matches!(it, Item::Poly(_))));
        assert!(b.items.iter().any(|it| matches!(it, Item::Glyph { ch: '0', .. })));
        assert!(b.items.iter().any(|it| matches!(it, Item::Glyph { ch: '1', .. })));
    }

    #[test]
    fn unsupported_characters_return_none() {
        let l = layout();
        let em = 24.0;
        assert!(l.stretchy_delimiter('x', 1.0, 1.0, em).is_none());
        assert!(l.stretchy_delimiter('?', 1.0, 1.0, em).is_none());
    }

    /// Regression: the straight stem must stop where the outline leaves the extender
    /// column edges, even when it drifts *inside* the column (the hook of `⎧` starts
    /// bending at y = 645 units, the cusp `⎨` at 1946 - 1386 = 560 / 558 units).
    /// Over-estimating these lets the cusp's flat end poke out as a stray `|` stub.
    #[test]
    fn brace_piece_stems_stop_where_curve_starts() {
        let l = layout();
        let face = Face::Symbol;
        let em = 1000.0; // 1 px == 1 font unit
        let (_, _, xmin, xmax) = l.exact_ink(face, '\u{23AA}', em);
        let near = |a: f32, b: f32| (a - b).abs() <= 2.0;
        let (_, bot) = l.piece_stem_lengths(face, '\u{23A7}', em, xmin, xmax);
        assert!(near(bot, 645.0), "⎧ bottom stem = {bot}");
        let (top, bot) = l.piece_stem_lengths(face, '\u{23A8}', em, xmin, xmax);
        assert!(near(top, 560.0) && near(bot, 558.0), "⎨ stems = {top}, {bot}");
        let (top, _) = l.piece_stem_lengths(face, '\u{23A9}', em, xmin, xmax);
        assert!(near(top, 646.0), "⎩ top stem = {top}");
    }
}
