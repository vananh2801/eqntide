//! Tall delimiters (parentheses, brackets, braces, floors and ceilings) that
//! stretch to match the exact height of their contents.
//!
//! Delimiters are drawn in one of two modes:
//! 1. Piece assembly (`span >= min_piece_h`): When the actual height is sufficient
//!    for the font glyph pieces (U+239B..U+23AD) to fit without their curved hooks
//!    colliding, the delimiter is assembled from pieces.
//!    - For curly braces (`{`, `}`): the 3 pieces (top hook, middle cusp, bottom hook)
//!      can assemble without `\u{23AA}` extenders down to ~1.85 em by overlapping along
//!      their straight stems. For taller spans (>= ~4.49 em), `\u{23AA}` extenders are
//!      tiled in the gaps.
//!    - For parentheses and square brackets: the top and bottom hooks can assemble without
//!      extenders down to ~1.20 em by overlapping along their straight stems. For taller
//!      spans (>= ~2.54 em), extenders are tiled between them.
//! 2. Vertical scaling (`span < min_piece_h`): When the actual height is too short even
//!    for the overlapping hooks to fit, the single complete glyph is scaled vertically
//!    (`em_y`) to match the requested span, while keeping the horizontal axis (`em_x`) at
//!    the standard font size. This avoids uniform scaling so horizontal strokes and serifs
//!    do not become abnormally thick.

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
    /// Build the delimiter `c` so that it spans exactly `asc` above and `desc`
    /// below the baseline.
    pub(super) fn stretchy_delimiter(&self, c: char, asc: f32, desc: f32, em: f32) -> Option<LBox> {
        let r = recipe(c)?;
        let face = Face::Symbol;
        let span = asc + desc;
        let top_y = -asc;
        let bot_y = desc;
        let pad = 0.05 * em;

        // Minimum height where the end pieces (and middle cusp if any) can overlap
        // along their straight stems without their curved hooks / cusps colliding:
        // - For '{' and '}': top hook curve is ~0.63 em, mid cusp curves are ~0.32 em each,
        //   bottom hook curve is ~0.56 em -> min span is (0.63 + 0.32) + (0.32 + 0.56) = ~1.83 em.
        //   Between 1.85 em and 4.49 em, the 3 pieces assemble directly without \u{23AA} extenders.
        // - For '(' and ')': hooks can overlap along straight stems down to ~1.20 em.
        // - For '[' and ']': hooks can overlap along straight stems down to ~1.15 em.
        let min_piece_h = match c {
            '{' | '}' => 1.85 * em,
            '(' | ')' => 1.20 * em,
            '[' | ']' | '⌈' | '⌉' | '⌊' | '⌋' => 1.15 * em,
            _ => 1.50 * em,
        };

        // Case 1: When actual height is smaller than min_piece_h, the pieces would collide.
        // Scale the single complete character vertically only (em_y = em * span / h0, em_x = em),
        // keeping horizontal stroke thickness at normal font size.
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

        // Case 2: When actual height >= min_piece_h, assemble from font glyph pieces:
        // - If span >= fixed (no overlap): extenders (\u{23AA}) are tiled into the gaps.
        // - If min_piece_h <= span < fixed: end pieces (and mid cusp) overlap along their
        //   straight vertical stems without needing extenders.
        let eff_em = em;
        let mut items: Vec<Item> = Vec::new();
        let push = |ch: char, y: f32, items: &mut Vec<Item>| {
            items.push(Item::Glyph { ch, face, em: eff_em, x: 0.0, y });
        };
        let width = [Some(r.ext), r.top, r.mid, r.bot]
            .into_iter()
            .flatten()
            .map(|ch| self.f.advance(face, ch, eff_em))
            .fold(0.0f32, f32::max);

        let ink = |ch: char| self.f.ink(face, ch, eff_em);

        // Pin the end pieces to the ends of the span.
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

        // Tile the uncovered range [from, to] with extenders if there is a positive gap.
        let (ea, ed) = ink(r.ext);
        let e_h = (ea + ed).max(1.0);
        let fill = |from: f32, to: f32, items: &mut Vec<Item>| {
            let ov = 0.02 * em;
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

        match r.mid {
            Some(m) => {
                let (a, d) = ink(m);
                let centre = (top_y + bot_y) / 2.0;
                let (m_top, m_bot) = (centre - (a + d) / 2.0, centre + (a + d) / 2.0);
                fill(gap_top, m_top, &mut items);
                push(m, m_top + a, &mut items);
                fill(m_bot, gap_bot, &mut items);
            }
            None => fill(gap_top, gap_bot, &mut items),
        }

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

            // Moderate span (e.g. 2.5 * em): for '{' this is the 3-piece assembly without extenders!
            let mod_span = 2.5 * em;
            let asc_m = mod_span / 2.0 + AXIS * em;
            let desc_m = mod_span / 2.0 - AXIS * em;
            let b_m = l.stretchy_delimiter(c, asc_m, desc_m, em).expect("delimiter supported");
            assert_eq!(b_m.asc, asc_m);
            assert_eq!(b_m.desc, desc_m);
            assert!(b_m.items.iter().all(|it| matches!(it, Item::Glyph { .. })), "{c}: mod should be Glyph");
            if c == '{' {
                // Exactly 3 pieces: top hook (\u{23A7}), mid cusp (\u{23A8}), bot hook (\u{23A9})
                assert_eq!(b_m.items.len(), 3, "curly brace at 2.5em should have exactly 3 pieces without extenders");
            }

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
    }

    #[test]
    fn unsupported_characters_return_none() {
        let l = layout();
        let em = 24.0;
        assert!(l.stretchy_delimiter('|', 1.0, 1.0, em).is_none());
        assert!(l.stretchy_delimiter('x', 1.0, 1.0, em).is_none());
    }
}
