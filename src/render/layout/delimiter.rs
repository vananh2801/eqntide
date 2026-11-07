//! Tall delimiters (parentheses, brackets, braces, floors and ceilings) that
//! stretch to match the exact height of their contents.
//!
//! Delimiters are drawn in one of two modes:
//! 1. Piece assembly (`span >= min_piece_h`): When the actual height is sufficient
//!    for the font glyph pieces (U+239B..U+23AD) to fit without their curved hooks
//!    colliding or protruding past the delimiter bounds, the delimiter is assembled
//!    from pieces.
//!    - For curly braces (`{`, `}`): the 3 pieces (top hook, middle cusp, bottom hook)
//!      can assemble without `\u{23AA}` extenders down to `min_piece_h` by overlapping
//!      along their straight stems. For taller spans, `\u{23AA}` extenders are tiled in the gaps.
//!    - For parentheses and square brackets: the top and bottom hooks can assemble without
//!      extenders down to `min_piece_h` by overlapping along their straight stems. For taller
//!      spans, extenders are tiled between them.
//! 2. Vertical scaling (`span < min_piece_h`): When the actual height is too short for
//!    the pieces to fit, the single complete glyph is scaled vertically (`em_y`) to match
//!    the requested span, while keeping the horizontal axis (`em_x`) at the standard font size.
//!    This avoids uniform scaling so horizontal strokes and serifs do not become abnormally thick.

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

impl Layout {
    /// Calculate the minimum delimiter height (`span`) at which font glyph pieces can
    /// be assembled without protruding past the delimiter bounds or colliding along their features.
    ///
    /// This threshold is computed automatically from font glyph metrics and vector outlines:
    /// - Extender horizontal bounds `[stem_x0, stem_x1]` define the straight stem width.
    /// - Feature depths: the vertical heights of curved hooks, serifs, and middle cusps
    ///   where outline points extend horizontally outside the straight stem.
    /// - Bounds constraint: no individual piece may be taller than the allocated span.
    fn min_assembly_span(&self, face: Face, r: &Pieces, em: f32) -> f32 {
        let (stem_x0, stem_x1) = self.f.ink_x(face, r.ext, em);
        let stem_w = (stem_x1 - stem_x0).abs();
        let tol = (0.05 * stem_w).max(0.5);

        let piece_h = |ch: Option<char>| -> f32 {
            ch.map_or(0.0, |c| {
                let (a, d) = self.f.ink(face, c, em);
                a + d
            })
        };

        let h_top = piece_h(r.top);
        let h_bot = piece_h(r.bot);
        let h_mid = piece_h(r.mid);

        let feature_depth = |ch: Option<char>, is_top: bool| -> f32 {
            let Some(c) = ch else { return 0.0 };
            let Some(segs) = self.f.glyph_segments(face, c, em, 0.0, 0.0) else { return 0.0 };
            let mut y_min = f32::MAX;
            let mut y_max = f32::MIN;
            let mut out_y_min = f32::MAX;
            let mut out_y_max = f32::MIN;
            let mut has_out = false;

            let mut check_pt = |x: f32, y: f32| {
                y_min = y_min.min(y);
                y_max = y_max.max(y);
                if x < stem_x0 - tol || x > stem_x1 + tol {
                    out_y_min = out_y_min.min(y);
                    out_y_max = out_y_max.max(y);
                    has_out = true;
                }
            };

            for s in segs {
                match s {
                    Seg::Move(x, y) | Seg::Line(x, y) => check_pt(x, y),
                    Seg::Quad(cx, cy, x, y) => {
                        check_pt(cx, cy);
                        check_pt(x, y);
                    }
                    Seg::Cubic(c1x, c1y, c2x, c2y, x, y) => {
                        check_pt(c1x, c1y);
                        check_pt(c2x, c2y);
                        check_pt(x, y);
                    }
                    Seg::Close => {}
                }
            }
            if !has_out {
                return 0.0;
            }
            if is_top {
                (out_y_max - y_min).max(0.0)
            } else {
                (y_max - out_y_min).max(0.0)
            }
        };

        let depth_top = feature_depth(r.top, true);
        let depth_bot = feature_depth(r.bot, false);

        match r.mid {
            Some(m) => {
                let (reach_top, reach_bot) = if let Some(segs) = self.f.glyph_segments(face, m, em, 0.0, 0.0) {
                    let mut y_min = f32::MAX;
                    let mut y_max = f32::MIN;
                    let mut out_y_min = f32::MAX;
                    let mut out_y_max = f32::MIN;
                    let mut has_out = false;

                    let mut check_pt = |x: f32, y: f32| {
                        y_min = y_min.min(y);
                        y_max = y_max.max(y);
                        if x < stem_x0 - tol || x > stem_x1 + tol {
                            out_y_min = out_y_min.min(y);
                            out_y_max = out_y_max.max(y);
                            has_out = true;
                        }
                    };

                    for s in segs {
                        match s {
                            Seg::Move(x, y) | Seg::Line(x, y) => check_pt(x, y),
                            Seg::Quad(cx, cy, x, y) => {
                                check_pt(cx, cy);
                                check_pt(x, y);
                            }
                            Seg::Cubic(c1x, c1y, c2x, c2y, x, y) => {
                                check_pt(c1x, c1y);
                                check_pt(c2x, c2y);
                                check_pt(x, y);
                            }
                            Seg::Close => {}
                        }
                    }
                    if has_out {
                        let mid_centre = (y_min + y_max) / 2.0;
                        ((mid_centre - out_y_min).max(0.0), (out_y_max - mid_centre).max(0.0))
                    } else {
                        (0.0, 0.0)
                    }
                } else {
                    (0.0, 0.0)
                };

                let collision_top = 2.0 * (depth_top + reach_top);
                let collision_bot = 2.0 * (depth_bot + reach_bot);
                h_mid.max(h_top).max(h_bot).max(collision_top).max(collision_bot)
            }
            None => {
                // Without middle cusp (parentheses, brackets): end pieces meet end-to-end.
                // Overlapping them along straight stems would insert an unnatural straight
                // vertical bar '|' into parentheses for moderate spans (< h_top + h_bot).
                // Thus, piece assembly starts at h_top + h_bot.
                h_top + h_bot
            }
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

        // Automatically determine the minimum height where pieces can assemble cleanly:
        let min_piece_h = self.min_assembly_span(face, &r, em);

        // Case 1: When actual height is smaller than min_piece_h, the pieces would collide
        // or protrude past the delimiter bounds.
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

        // For '{': moderate span (e.g. 2.5 * em) assembles with exactly 3 pieces without extenders:
        let mod_span = 2.5 * em;
        let asc_m = mod_span / 2.0 + AXIS * em;
        let desc_m = mod_span / 2.0 - AXIS * em;
        let b_m = l.stretchy_delimiter('{', asc_m, desc_m, em).expect("delimiter supported");
        assert_eq!(b_m.items.len(), 3, "curly brace at 2.5em should have exactly 3 pieces without extenders");
    }

    #[test]
    fn unsupported_characters_return_none() {
        let l = layout();
        let em = 24.0;
        assert!(l.stretchy_delimiter('|', 1.0, 1.0, em).is_none());
        assert!(l.stretchy_delimiter('x', 1.0, 1.0, em).is_none());
    }
}
