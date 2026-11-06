//! Tall delimiters assembled from font glyph pieces (parentheses, brackets,
//! braces, floors and ceilings), the same technique the integral sign uses.
//!
//! STIX Two Math ships the TeX-style "extensible" pieces in the Miscellaneous
//! Technical block (U+239B..U+23AD): a top hook, a vertical extender, an optional
//! middle cusp (braces only) and a bottom hook. A delimiter of any height is
//! built by pinning the hooks to the top and bottom of the requested span and
//! tiling the gaps with extenders, so the result always matches the content.

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
    /// below the baseline. Returns `None` when `c` has no glyph recipe.
    pub(super) fn stretchy_delimiter(&self, c: char, asc: f32, desc: f32, em: f32) -> Option<LBox> {
        let r = recipe(c)?;
        let face = Face::Symbol;
        let span = asc + desc;
        let top_y = -asc;
        let bot_y = desc;

        // The hooks and the cusp have a fixed shape. If the requested span is
        // shorter than they are, shrink the whole assembly instead of overlapping.
        let ink_h = |ch: Option<char>, e: f32| ch.map_or(0.0, |ch| {
            let (a, d) = self.f.ink(face, ch, e);
            a + d
        });
        let fixed = ink_h(r.top, em) + ink_h(r.mid, em) + ink_h(r.bot, em);
        let pad = 0.05 * em;

        // Too short for the hooks to fit at full size: fall back to the ordinary
        // glyph, scaled uniformly so its ink exactly spans the requested height.
        if fixed > span {
            let (a0, d0) = self.f.ink(face, c, em);
            let h0 = a0 + d0;
            if h0 <= 0.0 {
                return None;
            }
            let eff_em = em * span / h0;
            let (a, d) = self.f.ink(face, c, eff_em);
            let centre = (top_y + bot_y) / 2.0;
            let y = centre + (a - d) / 2.0;
            let items = vec![Item::Glyph { ch: c, face, em: eff_em, x: pad, y }];
            return Some(LBox { w: self.f.advance(face, c, eff_em) + 2.0 * pad, asc, desc, items });
        }
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

        // Ink extents of a piece as (above-baseline, below-baseline).
        let ink = |ch: char| self.f.ink(face, ch, eff_em);

        // Pin the end pieces to the ends of the span.
        let mut gap_top = top_y; // first y not yet covered
        let mut gap_bot = bot_y; // last y not yet covered
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

        // Tile the uncovered range [from, to] with extenders. Extenders are
        // plain vertical strokes, so overlapping them (and the hooks) is invisible.
        let (ea, ed) = ink(r.ext);
        let e_h = (ea + ed).max(1.0);
        let fill = |from: f32, to: f32, items: &mut Vec<Item>| {
            // Overlap the neighbouring pieces slightly so rounding never leaves a hairline gap.
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
        Some(LBox { w: width + 2.0 * pad, asc, desc, items })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> Layout {
        Layout { f: fonts(), depth: 0 }
    }

    /// Vertical ink intervals of every glyph piece in the box.
    fn intervals(l: &Layout, b: &LBox) -> Vec<(f32, f32)> {
        let mut v: Vec<(f32, f32)> = b
            .items
            .iter()
            .filter_map(|it| match it {
                Item::Glyph { ch, face, em, y, .. } => {
                    let (a, d) = l.f.ink(*face, *ch, *em);
                    Some((y - a, y + d))
                }
                _ => None,
            })
            .collect();
        v.sort_by(|a, b| a.0.total_cmp(&b.0));
        v
    }

    #[test]
    fn tall_delimiters_are_gapless_and_span_the_request() {
        let l = layout();
        let em = 24.0;
        for c in ['(', ')', '[', ']', '{', '}'] {
            for span in [4.8 * em, 6.5 * em, 9.0 * em] {
                let b = l.stretchy_delimiter(c, span / 2.0 + AXIS * em, span / 2.0 - AXIS * em, em).unwrap();
                let iv = intervals(&l, &b);
                assert!(iv.len() >= 2, "{c}: expected several pieces");
                // Covers the requested span (to within font rounding) ...
                assert!((iv[0].0 + b.asc).abs() < 1.5, "{c}: top edge");
                assert!((iv.iter().map(|p| p.1).fold(f32::MIN, f32::max) - b.desc).abs() < 1.5, "{c}: bottom edge");
                // ... and consecutive pieces touch or overlap.
                let mut reach = iv[0].1;
                for p in &iv[1..] {
                    assert!(p.0 <= reach + 1.0, "{c} span {span}: gap of {} px", p.0 - reach);
                    reach = reach.max(p.1);
                }
            }
        }
    }

    #[test]
    fn braces_get_a_middle_piece_and_short_spans_fall_back_to_one_glyph() {
        let l = layout();
        let em = 24.0;
        let tall = l.stretchy_delimiter('{', 3.0 * em, 3.0 * em, em).unwrap();
        assert!(tall.items.iter().any(|it| matches!(it, Item::Glyph { ch: '\u{23A8}', .. })));
        let short = l.stretchy_delimiter('(', 0.9 * em, 0.9 * em, em).unwrap();
        assert_eq!(short.items.len(), 1);
        assert!(l.stretchy_delimiter('|', 1.0, 1.0, em).is_none());
    }
}
