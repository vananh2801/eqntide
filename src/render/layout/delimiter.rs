//! Tall delimiters (parentheses, brackets, braces, floors and ceilings) that
//! stretch to match the exact height of their contents.
//!
//! Rather than assembling delimiters from piecewise font extensions (which
//! produces artificial straight vertical segments in parentheses and clunky
//! proportions in curly braces for systems of equations), we scale the font's
//! natural glyph outline independently along the vertical axis (`em_y`) to match
//! the requested height, while keeping the horizontal axis (`em_x`) at or near
//! the base font size. This ensures the stroke thickness remains slender and
//! uniform with the surrounding equation text.

use super::*;

impl Layout {
    /// Build the delimiter `c` so that it spans exactly `asc` above and `desc`
    /// below the baseline.
    pub(super) fn stretchy_delimiter(&self, c: char, asc: f32, desc: f32, em: f32) -> Option<LBox> {
        if !matches!(c, '(' | ')' | '[' | ']' | '{' | '}' | '⌈' | '⌉' | '⌊' | '⌋') {
            return None;
        }

        let face = Face::Symbol;
        let span = asc + desc;
        let top_y = -asc;
        let bot_y = desc;

        let pad = 0.05 * em;

        let natural: Vec<Vec<(f32, f32)>> = self.f.glyph_polys(face, c, em, em, 0.0, 0.0);
        if natural.is_empty() {
            return None;
        }
        let (y_lo, y_hi) = natural.iter().flatten().fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p.1), hi.max(p.1)));
        let h0 = y_hi - y_lo;
        if h0 <= 0.0 {
            return None;
        }

        let stretch = (span / h0).max(1.0);
        let em_y = em * span / h0;

        // Scale horizontally only very slightly with height so tall delimiters
        // don't look completely squashed, while keeping the stroke width slender.
        let width_factor = match c {
            '(' | ')' => 0.12,
            '{' | '}' => 0.08,
            '[' | ']' | '⌈' | '⌉' | '⌊' | '⌋' => 0.05,
            _ => 0.08,
        };
        let em_x = em * (1.0 + width_factor * (stretch - 1.0).min(4.0));

        // Vertically centre the ink on the requested span.
        let centre = (top_y + bot_y) / 2.0;
        let y = centre - (y_lo + y_hi) / 2.0 * (em_y / em);
        let polys = self.f.glyph_polys(face, c, em_x, em_y, pad, y);
        let w = self.f.advance(face, c, em_x) + 2.0 * pad;

        Some(LBox {
            w,
            asc,
            desc,
            items: polys.into_iter().map(Item::Poly).collect(),
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
    fn tall_delimiters_span_the_requested_height() {
        let l = layout();
        let em = 24.0;
        for c in ['(', ')', '[', ']', '{', '}'] {
            for span in [1.5 * em, 2.5 * em, 4.0 * em, 6.0 * em] {
                let asc = span / 2.0 + AXIS * em;
                let desc = span / 2.0 - AXIS * em;
                let b = l.stretchy_delimiter(c, asc, desc, em).expect("delimiter supported");
                assert_eq!(b.asc, asc);
                assert_eq!(b.desc, desc);
                assert!(!b.items.is_empty(), "must produce rendered polygons");
                for it in &b.items {
                    assert!(matches!(it, Item::Poly(_)));
                }
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
