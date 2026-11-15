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

/// Vertical metrics of one assembly piece (pixels).
#[derive(Clone, Copy)]
struct PieceGeom {
    asc: f32,
    h: f32,
    stem_top: f32,
    stem_bot: f32,
}

/// A piece pinned in the assembly, with the band `[clip_top, clip_bot]` it is cut to.
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

    /// Height, ascent and straight-stem lengths of one assembly piece at `eff_em`.
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

    /// The smallest span (in pixels) that can be built by *clipped* glyph assembly.
    ///
    /// Two neighbouring pieces are cut along one horizontal line that lies inside the
    /// straight stem of both, so a joint can hide up to `stem_below_upper +
    /// stem_above_lower` of overlap without showing any flat piece end. With a middle
    /// cusp (braces) the cusp is centred, so each half of the span must hold one end
    /// piece plus half the cusp, minus what its joint can hide.
    fn min_assembly_span(&self, face: Face, r: &Pieces, eff_em: f32) -> f32 {
        let (_, _, xmin, xmax) = self.exact_ink(face, r.ext, eff_em);
        let geom = |ch: Option<char>| ch.map(|c| self.piece_geom(face, c, eff_em, xmin, xmax));
        let (top, mid, bot) = (geom(r.top), geom(r.mid), geom(r.bot));

        let span = match mid {
            Some(m) => {
                let upper = top.map_or(m.h / 2.0 - m.stem_top, |t| {
                    t.h + m.h / 2.0 - (t.stem_bot + m.stem_top)
                });
                let lower = bot.map_or(m.h / 2.0 - m.stem_bot, |b| {
                    b.h + m.h / 2.0 - (b.stem_top + m.stem_bot)
                });
                2.0 * upper.max(lower)
            }
            None => match (top, bot) {
                (Some(t), Some(b)) => t.h + b.h - (t.stem_bot + b.stem_top),
                (Some(t), None) => t.h - t.stem_bot,
                (None, Some(b)) => b.h - b.stem_top,
                (None, None) => 0.0,
            },
        };
        span.max(0.0)
    }

    /// Build the delimiter `c` so that it spans exactly `asc` above and `desc`
    /// below the baseline. Supports parentheses, brackets, double brackets, braces,
    /// floors, ceilings, absolute value (`|`), and norm (`‖`).
    pub(super) fn stretchy_delimiter(&self, c: char, asc: f32, desc: f32, em: f32) -> Option<LBox> {
        let norm_c = match c {
            '|' | '\u{2223}' => '|',
            '‖' | '\u{2225}' => '‖',
            '\u{301A}' => '⟦',
            '\u{301B}' => '⟧',
            _ => c,
        };

        // Straight vertical bars (absolute value `|.|` and norm `||.||`):
        // Scaled vertically using their exact font glyph contours from STIX Two Math,
        // preserving horizontal stroke thickness and double-bar spacingspacing identically to the font design.
        if matches!(norm_c, '|' | '‖') {
            let face = Face::Symbol;
            let span = asc + desc;
            let top_y = -asc;
            let bot_y = desc;
            let pad = 0.04 * em;

            let natural: Vec<Vec<(f32, f32)>> = self.f.glyph_polys(face, norm_c, em, em, 0.0, 0.0);
            if natural.is_empty() {
                return None;
            }
            let (y_lo, y_hi) = natural
                .iter()
                .flatten()
                .fold((f32::MAX, f32::MIN), |(lo, hi), p| {
                    (lo.min(p.1), hi.max(p.1))
                });
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
                hits: Vec::new(),
                w,
                asc,
                desc,
                items: vec![Item::Contours(polys)],
            });
        }

        // Double brackets (`⟦`, `⟧`): constructed from four non-overlapping crisp rectangles
        // so both the rasterizer (which enforces positive polygon area) and EMF winding-fill
        // preserve the hollow gap between the two vertical stems and uniform cap thickness.
        if matches!(norm_c, '⟦' | '⟧') {
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
            let (s1_x, s2_x) = if norm_c == '⟦' {
                (pad, pad + v_thick + gap)
            } else {
                (pad + overhang, pad + overhang + v_thick + gap)
            };
            let mut items = Vec::with_capacity(4);
            items.push(Item::Poly(rect(pad, top_y, arm_w, h_thick)));
            items.push(Item::Poly(rect(pad, bot_y - h_thick, arm_w, h_thick)));
            if stem_h > 0.0 {
                items.push(Item::Poly(rect(s1_x, stem_y, v_thick, stem_h)));
                items.push(Item::Poly(rect(s2_x, stem_y, v_thick, stem_h)));
            }
            return Some(LBox {
                hits: Vec::new(),
                w: arm_w + 2.0 * pad,
                asc,
                desc,
                items,
            });
        }

        let r = recipe(norm_c)?;
        let face = Face::Symbol;
        let span = asc + desc;
        let top_y = -asc;
        let bot_y = desc;
        let centre = (top_y + bot_y) / 2.0;
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
            let (y_lo, y_hi) = natural
                .iter()
                .flatten()
                .fold((f32::MAX, f32::MIN), |(lo, hi), p| {
                    (lo.min(p.1), hi.max(p.1))
                });
            let h0 = y_hi - y_lo;
            if h0 <= 0.0 {
                return None;
            }

            let em_y = em * span / h0;
            let em_x = em; // Scale vertically only; keep horizontal scale at normal em.

            let y = centre - (y_lo + y_hi) / 2.0 * (em_y / em);
            let polys = self.f.glyph_polys(face, norm_c, em_x, em_y, pad, y);
            let w = self.f.advance(face, norm_c, em_x) + 2.0 * pad;

            return Some(LBox {
                hits: Vec::new(),
                w,
                asc,
                desc,
                items: vec![Item::Contours(polys)],
            });
        }

        // Tier 2 & Tier 3: clipped glyph assembly.
        // End pieces are pinned to the top / bottom, the cusp (if any) is centred.
        // - Where neighbours overlap (Tier 2), both are *cut* along one horizontal line
        //   inside their straight stems, so no flat piece end can poke out past a curve.
        // - Where they leave a gap (Tier 3), extenders are tiled in and the last one is
        //   cut to fit exactly, instead of being centred and overflowing.
        let (_, _, ext_xmin, ext_xmax) = self.exact_ink(face, r.ext, eff_em);
        let ov = 0.02 * eff_em; // seam overlap inside straight stems (hides AA hairlines)

        let mut pieces: Vec<Placed> = Vec::new();
        let mut place = |ch: char, top: f32| {
            let g = self.piece_geom(face, ch, eff_em, ext_xmin, ext_xmax);
            pieces.push(Placed {
                ch,
                y: top + g.asc,
                top,
                bot: top + g.h,
                stem_top: g.stem_top,
                stem_bot: g.stem_bot,
                clip_top: f32::NEG_INFINITY,
                clip_bot: f32::INFINITY,
            });
        };
        if let Some(t) = r.top {
            place(t, top_y);
        }
        if let Some(m) = r.mid {
            let (a, d, _, _) = self.exact_ink(face, m, eff_em);
            place(m, centre - (a + d) / 2.0);
        }
        if let Some(b) = r.bot {
            let (a, d, _, _) = self.exact_ink(face, b, eff_em);
            place(b, bot_y - (a + d));
        }

        // Resolve every joint, including the outer boundaries for pieces without a
        // top / bottom hook (floors, ceilings).
        let n = pieces.len();
        let mut fills: Vec<(f32, f32)> = Vec::new();
        for i in 0..=n {
            let up = i.checked_sub(1);
            let lo = (i < n).then_some(i);
            let up_end = up.map_or(top_y, |k| pieces[k].bot);
            let lo_start = lo.map_or(bot_y, |k| pieces[k].top);
            if lo_start > up_end {
                let from = up_end - if up.is_some() { ov } else { 0.0 };
                let to = lo_start + if lo.is_some() { ov } else { 0.0 };
                fills.push((from, to));
                continue;
            }
            let (j_lo, j_hi, seam) = match (up, lo) {
                (Some(u), Some(l)) => {
                    let (u, l) = (&pieces[u], &pieces[l]);
                    (
                        (u.bot - u.stem_bot).max(l.top),
                        u.bot.min(l.top + l.stem_top),
                        ov / 2.0,
                    )
                }
                (None, Some(_)) => (top_y, top_y, 0.0),
                (Some(_), None) => (bot_y, bot_y, 0.0),
                (None, None) => continue,
            };
            let want = (up_end + lo_start) / 2.0;
            let j = if j_lo <= j_hi {
                want.clamp(j_lo, j_hi)
            } else {
                (j_lo + j_hi) / 2.0
            };
            if let Some(u) = up {
                pieces[u].clip_bot = pieces[u].clip_bot.min(j + seam);
            }
            if let Some(l) = lo {
                pieces[l].clip_top = pieces[l].clip_top.max(j - seam);
            }
        }

        // Unclipped pieces stay exact glyphs; clipped ones become cut polygons.
        let mut items: Vec<Item> = Vec::new();
        let emit = |ch: char, y: f32, ink: (f32, f32), clip: (f32, f32), items: &mut Vec<Item>| {
            if clip.0 <= ink.0 + 1e-3 && clip.1 >= ink.1 - 1e-3 {
                items.push(Item::Glyph {
                    ch,
                    face,
                    em: eff_em,
                    x: pad,
                    y,
                    ssty: 0,
                });
                return;
            }
            for poly in self.f.glyph_polys(face, ch, eff_em, eff_em, pad, y) {
                let cut = clip_poly_y(&poly, clip.0, clip.1);
                if cut.len() >= 3 {
                    items.push(Item::Poly(cut));
                }
            }
        };
        for p in &pieces {
            emit(
                p.ch,
                p.y,
                (p.top, p.bot),
                (p.clip_top, p.clip_bot),
                &mut items,
            );
        }
        let (ea, ed, _, _) = self.exact_ink(face, r.ext, eff_em);
        let e_h = (ea + ed).max(1.0);
        let step = (e_h - ov).max(1.0);
        for (from, to) in fills {
            let mut t = from;
            while t < to - 1e-3 {
                emit(r.ext, t + ea, (t, t + e_h), (from, to), &mut items);
                t += step;
            }
        }

        let width = [Some(r.ext), r.top, r.mid, r.bot]
            .into_iter()
            .flatten()
            .map(|ch| self.f.advance(face, ch, eff_em))
            .fold(0.0f32, f32::max);

        Some(LBox {
            hits: Vec::new(),
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
        let sup_y = sup
            .as_ref()
            .map(|s| (-actual_asc + s.asc + 0.04 * em).min(-shift_up));

        // Position subscript near bottom of bar:
        let mut sub_y = sub
            .as_ref()
            .map(|s| (actual_desc - s.desc - 0.04 * em).max(shift_down));

        // Ensure minimum gap between sub and sup:
        if let (Some(s1), Some(s2), Some(sy), Some(py)) = (&sub, &sup, sub_y, sup_y) {
            let gap = (sy - s1.asc) - (py + s2.desc);
            if gap < gap_min {
                sub_y = Some(sy + (gap_min - gap));
            }
        }

        let mut out = LBox {
            hits: Vec::new(),
            w: script_x
                + sub
                    .as_ref()
                    .map_or(0.0, |s| s.w)
                    .max(sup.as_ref().map_or(0.0, |s| s.w)),
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
        Layout {
            f: fonts(),
            depth: 0,
        }
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
            println!(
                "DELIMITER '{c}': auto-calculated min_piece_h = {:.2} px ({:.3} em)",
                min_span,
                min_span / em
            );
        }
    }

    #[test]
    fn tall_delimiters_piece_assembly_and_vertical_stretch() {
        let l = layout();
        let em = 24.0;
        for c in ['(', ')', '[', ']', '{', '}', '|', '‖'] {
            // Very short span (0.6em): below min_assembly_span for all curved delimiters,
            // so even brackets use Tier 1 outline scaling.
            let short_span = 0.6 * em;
            let asc_s = short_span / 2.0 + AXIS * em;
            let desc_s = short_span / 2.0 - AXIS * em;
            let b_s = l
                .stretchy_delimiter(c, asc_s, desc_s, em)
                .expect("delimiter supported");
            assert_eq!(b_s.asc, asc_s);
            assert_eq!(b_s.desc, desc_s);
            if c != '[' && c != ']' {
                assert_eq!(b_s.items.len(), 1);
            }
            assert!(
                b_s.items
                    .iter()
                    .all(|it| matches!(it, Item::Poly(_) | Item::Contours(_))),
                "{c}: short should be Poly or Contours"
            );
            if c == '‖' {
                assert!(matches!(
                    b_s.items.first(),
                    Some(Item::Contours(cs)) if cs.len() == 2
                ));
            }

            // Tall span (6em): Tier 3 assembly. Unclipped end/mid pieces stay Glyph,
            // while the last tiled extender is cut to fit and emitted as a Poly.
            let tall_span = 6.0 * em;
            let asc_t = tall_span / 2.0 + AXIS * em;
            let desc_t = tall_span / 2.0 - AXIS * em;
            let b_t = l
                .stretchy_delimiter(c, asc_t, desc_t, em)
                .expect("delimiter supported");
            assert_eq!(b_t.asc, asc_t);
            assert_eq!(b_t.desc, desc_t);
            if c == '|' || c == '‖' {
                assert!(
                    b_t.items
                        .iter()
                        .all(|it| matches!(it, Item::Poly(_) | Item::Contours(_))),
                    "{c}: tall bar should be Poly or Contours"
                );
            } else {
                assert!(
                    b_t.items.iter().any(|it| matches!(it, Item::Glyph { .. })),
                    "{c}: tall should have Glyph pieces"
                );
                if c == '{' {
                    assert!(
                        b_t.items.len() > 3,
                        "curly brace at 6.0em should have extenders"
                    );
                }
            }
        }

        // For '{' across Tier 2 (1.6em..=3.2em): always 3 clipped pieces (no extenders),
        // and no piece contour extends outside the delimiter's vertical span.
        for k in [1.6f32, 2.0, 2.5, 3.0] {
            let span = k * em;
            let asc = span / 2.0 + AXIS * em;
            let desc = span / 2.0 - AXIS * em;
            let b = l
                .stretchy_delimiter('{', asc, desc, em)
                .expect("delimiter supported");
            assert_eq!(
                b.items.len(),
                3,
                "curly brace at {k}em should be 3 pieces without extenders"
            );
            for it in &b.items {
                if let Item::Poly(pts) = it {
                    for &(_, y) in pts {
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
        let l = layout();
        let em = 24.0;
        let asc = 1.4 * em;
        let desc = 0.9 * em;
        for c in ['⟦', '⟧'] {
            let b = l.stretchy_delimiter(c, asc, desc, em).unwrap();
            assert_eq!(
                b.items.len(),
                4,
                "{c}: must have top, bottom, and two stems"
            );
            let rects: Vec<(f32, f32, f32, f32)> = b
                .items
                .iter()
                .map(|it| match it {
                    Item::Poly(pts) => {
                        let (mut x0, mut x1, mut y0, mut y1) =
                            (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
                        for &(x, y) in pts {
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
            // Stems sit strictly between top bar bottom and bottom bar top, and have horizontal gap.
            let top_bot = rects[0].3;
            let bot_top = rects[1].1;
            let (s1, s2) = (rects[2], rects[3]);
            assert!((s1.1 - top_bot).abs() < 1e-3);
            assert!((s1.3 - bot_top).abs() < 1e-3);
            assert!(s2.0 - s1.2 > 0.05 * em, "hollow gap between vertical bars");
        }
    }

    #[test]
    fn evaluation_bar_layout_and_limits() {
        let mut l = layout();
        let em = 24.0;
        let ctx = MathContext::DISPLAY;
        let sub = MathNode::leaf("mn", "0");
        let sup = MathNode::leaf("mn", "1");

        let b = l
            .evaluation_bar(Some(&sub), Some(&sup), 1.5 * em, 1.2 * em, em, ctx)
            .unwrap();
        assert!(b.w > 0.4 * em);
        assert!(b.asc >= 1.5 * em);
        assert!(b.desc >= 1.2 * em);
        // Verify bar contours and limits are present
        assert!(b
            .items
            .iter()
            .any(|it| matches!(it, Item::Poly(_) | Item::Contours(_))));
        assert!(b
            .items
            .iter()
            .any(|it| matches!(it, Item::Glyph { ch: '0', .. })));
        assert!(b
            .items
            .iter()
            .any(|it| matches!(it, Item::Glyph { ch: '1', .. })));
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
        assert!(
            near(top, 560.0) && near(bot, 558.0),
            "⎨ stems = {top}, {bot}"
        );
        let (top, _) = l.piece_stem_lengths(face, '\u{23A9}', em, xmin, xmax);
        assert!(near(top, 646.0), "⎩ top stem = {top}");
    }
}
