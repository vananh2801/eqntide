//! Tall delimiters (parentheses, brackets, braces, floors and ceilings) that
//! stretch to match the exact height of their contents.
//!
//! Delimiters are drawn in one of three tiers:
//! 1. Tier 1 (Small span < min_piece_h): When the actual height is small (e.g. single-line
//!    expressions), the single complete glyph is scaled vertically (`em_y`) while keeping
//!    horizontal scale at normal `em`. This avoids unnatural piece joins.
//! 2. Tier 2 (Moderate span, specifically for curly braces `{`, `}`): Assembled from exactly
//!    3 font glyph pieces (top hook, middle cusp, bottom hook) WITHOUT any `\u{23AA}` extenders.
//!    The pieces overlap along their straight stems, with the allocated span clamped to at least
//!    `h_3piece_min` so the straight stem overlap never crosses into the curved cusp (preventing
//!    discontinuous '|' bar artifacts).
//! 3. Tier 3 (Tall span): The pieces are placed at top, center, and bottom, with `\u{23AA}`
//!    extenders tiled seamlessly into the positive stem gaps.

use super::*;

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

impl Layout {
    /// Calculate the minimum delimiter height (`span`) at which font glyph pieces can
    /// begin to be assembled.
    ///
    /// - For delimiters without middle cusp (parentheses, brackets): end pieces meet end-to-end at `h_top + h_bot`.
    /// - For curly braces (with middle cusp): 3-piece assembly (head, middle, tail) begins at `1.75 * em` (e.g. fractions, 2-line systems).
    fn min_assembly_span(&self, face: Face, r: &Pieces, em: f32) -> f32 {
        let piece_h = |ch: Option<char>| -> f32 {
            ch.map_or(0.0, |c| {
                let (a, d) = self.f.ink(face, c, em);
                a + d
            })
        };

        let h_top = piece_h(r.top);
        let h_bot = piece_h(r.bot);

        match r.mid {
            Some(_) => 1.75 * em,
            None => h_top + h_bot,
        }
    }

    /// Build the delimiter `c` so that it spans exactly `asc` above and `desc`
    /// below the baseline.
    pub(super) fn stretchy_delimiter(&self, c: char, asc: f32, desc: f32, em: f32) -> Option<LBox> {
        let r = recipe(c)?;
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

        let piece_h = |ch: Option<char>| -> f32 {
            ch.map_or(0.0, |c| {
                let (a, d) = self.f.ink(face, c, eff_em);
                a + d
            })
        };

        let h_top = piece_h(r.top);
        let h_bot = piece_h(r.bot);
        let h_mid = piece_h(r.mid);
        let h_3piece_natural = h_top + h_mid + h_bot;
        // The straight stem of STIX Two Math brace pieces extends ~645 font units.
        // We allow up to 580 font units of safe overlap along the straight stems without touching the cusp:
        let max_safe_overlap = 580.0 / 1000.0 * eff_em;
        let h_3piece_min = h_3piece_natural - 2.0 * max_safe_overlap;

        let min_piece_h = self.min_assembly_span(face, &r, em);

        // Tier 1: When span < min_piece_h:
        // Use vertical-only outline scaling of the complete glyph to avoid overlapping piece stems.
        if span < min_piece_h {
            let natural: Vec<Vec<(f32, f32)>> = self.f.glyph_polys(face, c, em, em, 0.0, 0.0);
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
            let polys = self.f.glyph_polys(face, c, em_x, em_y, pad, y);
            let w = self.f.advance(face, c, em_x) + 2.0 * pad;

            return Some(LBox {
                w,
                asc,
                desc,
                items: polys.into_iter().map(Item::Poly).collect(),
            });
        }

        // Tier 2 & Tier 3:
        // For curly braces:
        // - Tier 2 (min_piece_h <= span < h_3piece_natural): Assembled from exactly 3 pieces (head, middle, tail)
        //   WITHOUT extenders. The span is clamped to at least h_3piece_min so the straight stems overlap
        //   safely without their flat end caps protruding into the curved cusp.
        // - Tier 3 (span >= h_3piece_natural): Assembled from 3 pieces + extenders tiled into the gaps.
        let actual_span = if r.mid.is_some() {
            span.max(h_3piece_min)
        } else {
            span
        };

        let centre = (top_y + bot_y) / 2.0;
        let top_y = centre - actual_span / 2.0;
        let bot_y = centre + actual_span / 2.0;
        let asc = -top_y;
        let desc = bot_y;

        let mut items: Vec<Item> = Vec::new();
        let push = |ch: char, y: f32, items: &mut Vec<Item>| {
            items.push(Item::Glyph { ch, face, em: eff_em, x: 0.0, y, ssty: 0 });
        };
        let ink = |ch: char| self.f.ink(face, ch, eff_em);

        if r.mid.is_some() {
            let k = eff_em / 1000.0;

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
            let (a_mid, _d_mid) = ink(mid_ch);
            let y_mid = centre + 973.0 * k;
            push(mid_ch, y_mid, &mut items);
            let mid_start_y = y_mid - a_mid;
            let mid_end_y = y_mid; // because yMin = 0

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
        for c in ['(', ')', '[', ']', '{', '}'] {
            let r = recipe(c).unwrap();
            let min_span = l.min_assembly_span(face, &r, em);
            println!("DELIMITER '{c}': auto-calculated min_piece_h = {:.2} px ({:.3} em)", min_span, min_span / em);
        }
    }

    #[test]
    fn tall_delimiters_piece_assembly_and_vertical_stretch() {
        let l = layout();
        let em = 24.0;
        for c in ['(', ')', '[', ']', '{', '}'] {
            // Short span (< min_piece_h): uses vertical outline scaling (Item::Poly)
            let short_span = 1.0 * em;
            let asc_s = short_span / 2.0 + AXIS * em;
            let desc_s = short_span / 2.0 - AXIS * em;
            let b_s = l.stretchy_delimiter(c, asc_s, desc_s, em).expect("delimiter supported");
            assert_eq!(b_s.asc, asc_s);
            assert_eq!(b_s.desc, desc_s);
            assert!(b_s.items.iter().all(|it| matches!(it, Item::Poly(_))), "{c}: short should be Poly");

            // Tall span (>= 5.0 * em): uses piece assembly with extenders
            let tall_span = 6.0 * em;
            let asc_t = tall_span / 2.0 + AXIS * em;
            let desc_t = tall_span / 2.0 - AXIS * em;
            let b_t = l.stretchy_delimiter(c, asc_t, desc_t, em).expect("delimiter supported");
            assert_eq!(b_t.asc, asc_t);
            assert_eq!(b_t.desc, desc_t);
            assert!(b_t.items.iter().all(|it| matches!(it, Item::Glyph { .. })), "{c}: tall should be Glyph");
            if c == '{' {
                // Must have extenders (\u{23AA})
                assert!(b_t.items.len() > 3, "curly brace at 6.0em should have extenders");
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
    fn unsupported_characters_return_none() {
        let l = layout();
        let em = 24.0;
        assert!(l.stretchy_delimiter('|', 1.0, 1.0, em).is_none());
        assert!(l.stretchy_delimiter('x', 1.0, 1.0, em).is_none());
    }
}
