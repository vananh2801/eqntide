//! Tall delimiters (parentheses, brackets, braces, floors and ceilings) that
//! stretch to match the exact height of their contents.
//!
//! Delimiters are drawn in one of two modes:
//! 1. Piece assembly (`span >= pieces_h`): When the actual height exceeds the total
//!    height of the assembly pieces (top hook, middle cusp if any, and bottom hook),
//!    the delimiter is assembled from font glyph pieces (U+239B..U+23AD) with extenders
//!    tiled between them.
//! 2. Vertical scaling (`span < pieces_h`): When the actual height is smaller than the
//!    total height of the assembly pieces, the single complete glyph is scaled vertically
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

        // Total height of the fixed assembly pieces (top hook, middle cusp, bottom hook).
        let ink_h = |ch: Option<char>| ch.map_or(0.0, |ch| {
            let (a, d) = self.f.ink(face, ch, em);
            a + d
        });
        let pieces_h = ink_h(r.top) + ink_h(r.mid) + ink_h(r.bot);

        // Case 1: When actual height is smaller than the combined pieces height,
        // use vertical scaling on the single complete character so horizontal strokes
        // and serifs stay at normal font thickness without becoming abnormally thick.
        if span < pieces_h {
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

        // Case 2: When actual height >= total pieces height, assemble from font glyph pieces.
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

        // Tile the uncovered range [from, to] with extenders.
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
            // Short span (< pieces_h): uses vertical outline scaling (Item::Poly)
            let short_span = 1.5 * em;
            let asc_s = short_span / 2.0 + AXIS * em;
            let desc_s = short_span / 2.0 - AXIS * em;
            let b_s = l.stretchy_delimiter(c, asc_s, desc_s, em).expect("delimiter supported");
            assert_eq!(b_s.asc, asc_s);
            assert_eq!(b_s.desc, desc_s);
            assert!(b_s.items.iter().all(|it| matches!(it, Item::Poly(_))), "{c}: short should be Poly");

            // Tall span (>= pieces_h): uses piece assembly (Item::Glyph)
            let tall_span = 6.0 * em;
            let asc_t = tall_span / 2.0 + AXIS * em;
            let desc_t = tall_span / 2.0 - AXIS * em;
            let b_t = l.stretchy_delimiter(c, asc_t, desc_t, em).expect("delimiter supported");
            assert_eq!(b_t.asc, asc_t);
            assert_eq!(b_t.desc, desc_t);
            assert!(b_t.items.iter().all(|it| matches!(it, Item::Glyph { .. })), "{c}: tall should be Glyph");
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
