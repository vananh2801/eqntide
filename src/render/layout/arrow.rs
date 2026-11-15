//! Labeled arrow templates (MathType `tmARROW`): a stretchy arrow with text
//! above and/or below it.
//!
//! The arrow is assembled from font glyphs (arrowheads `\u{2192}`, `\u{2190}`
//! and horizontal extender line `\u{23AF}`) so that its typography, arrowheads,
//! and stroke weight match MathType and vector arrows identically.
//!
//! [`template_of`] mirrors the cases that `mtef::encode` turns into
//! `extensible_arrow`, so the preview picture matches the MathType object.

use super::*;

/// Which way a stretchy arrow points.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum ArrowDir {
    Right,
    Left,
    Both,
    Other(char),
}

/// A recognised arrow template: the arrow plus the text above / below it.
pub(super) struct ArrowTemplate<'a> {
    pub(super) dir: ArrowDir,
    pub(super) top: Option<&'a MathNode>,
    pub(super) bottom: Option<&'a MathNode>,
}

/// Direction of a single-line arrow character (spacing or combining form).
pub(super) fn arrow_dir(c: char) -> Option<ArrowDir> {
    match c {
        '\u{2192}' | '\u{27F6}' | '\u{20D7}' => Some(ArrowDir::Right),
        '\u{2190}' | '\u{27F5}' | '\u{20D6}' => Some(ArrowDir::Left),
        '\u{2194}' | '\u{27F7}' => Some(ArrowDir::Both),
        c @ ('⇒' | '⇐' | '⇔' | '⇀' | '↼' | '⇌' | '⇋' | '↦' | '⇄' | '⇆') => {
            Some(ArrowDir::Other(c))
        }
        _ => None,
    }
}

/// The single-line arrow held by a `<mo>` node, if it is one.
fn mo_arrow(n: &MathNode) -> Option<ArrowDir> {
    if n.tag != "mo" {
        return None;
    }
    n.text.trim().chars().next().and_then(arrow_dir)
}

/// Recognise the `munder` / `mover` / `munderover` shapes that MathType draws
/// as a labeled arrow:
///
/// 1. an arrow with a label over and/or under it (`A →(x) B`);
/// 2. `↔` / `←` over some text: the arrow is drawn above, the text below it;
/// 3. any arrow under some text (not a limit): the text is drawn above the arrow.
///
/// `→` over text stays a vector accent (`draw_accent`), as in MathType.
pub(super) fn template_of(n: &MathNode) -> Option<ArrowTemplate<'_>> {
    if n.attr("accent") == Some("true") || n.attr("accentunder") == Some("true") {
        return None;
    }
    let base = n.children.first()?;
    let (under, over) = match n.tag.as_str() {
        "munder" => (n.children.get(1), None),
        "mover" => (None, n.children.get(1)),
        "munderover" => (n.children.get(1), n.children.get(2)),
        _ => return None,
    };
    let is_label = |x: Option<&MathNode>| x.is_none_or(|x| mo_arrow(x).is_none());

    // 1. the base is the arrow, the children are its labels
    if let Some(dir) = mo_arrow(base) {
        if (under.is_some() || over.is_some()) && is_label(under) && is_label(over) {
            return Some(ArrowTemplate {
                dir,
                top: over,
                bottom: under,
            });
        }
    }
    // 2. ↔ or ← drawn over the base text
    if let Some(dir) = over.and_then(mo_arrow).filter(|d| *d != ArrowDir::Right) {
        return Some(ArrowTemplate {
            dir,
            top: None,
            bottom: Some(base),
        });
    }
    // 3. an arrow drawn under the base text (limits such as `lim` keep their subscript)
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
    /// An arrow whose length follows its text, assembled from font glyphs, with
    /// `top` centred above and `bottom` centred below the shaft.
    pub(super) fn labeled_arrow(
        &mut self,
        dir: ArrowDir,
        top: Option<&MathNode>,
        bottom: Option<&MathNode>,
        em: f32,
        ctx: MathContext,
    ) -> Result<LBox> {
        let c = self.f.constants();
        let lim_ctx = ctx.for_limits();
        let lim_em = em * lim_ctx.scale(c);
        let over = match top {
            Some(o) => Some(self.node(o, lim_em, lim_ctx)?),
            None => None,
        };
        let under = match bottom {
            Some(u) => Some(self.node(u, lim_em, lim_ctx)?),
            None => None,
        };

        // Shaft: wide enough so labels clear arrowheads on both sides.
        let label_w = [over.as_ref().map(|o| o.w), under.as_ref().map(|u| u.w)]
            .into_iter()
            .flatten()
            .fold(0.0f32, f32::max);
        let len = (label_w + 0.72 * em).max(1.40 * em);

        let (head_asc, head_desc) = self.f.ink(Face::Symbol, '\u{2192}', em);
        let mut b = LBox {
            w: len,
            asc: head_asc,
            desc: head_desc,
            ..Default::default()
        };

        let k = em / 1000.0;
        let e_w = 322.0 * k; // advance of extender '\u{23AF}'
        let face = Face::Symbol;
        let y_glyph = 0.0; // on the baseline, the math glyphs' axis naturally sits on the math axis
        let y_axis = -c.axis_height * em;
        let st = c.fraction_rule_thickness * em;
        let mut arrow_top_y = y_axis - st / 2.0;
        let mut arrow_bot_y = y_axis + st / 2.0;

        match dir {
            ArrowDir::Other(ch) => {
                let natural = self.f.glyph_polys(Face::Symbol, ch, em, em, 0.0, 0.0);
                let (lo, hi, y_lo, y_hi) = natural.iter().flatten().fold(
                    (f32::MAX, f32::MIN, f32::MAX, f32::MIN),
                    |(lo, hi, ylo, yhi), p| (lo.min(p.0), hi.max(p.0), ylo.min(p.1), yhi.max(p.1)),
                );
                if hi > lo {
                    arrow_top_y = y_lo;
                    arrow_bot_y = y_hi;
                    b.asc = b.asc.max(-y_lo);
                    b.desc = b.desc.max(y_hi);
                    let contours: Vec<Vec<(f32, f32)>> = natural
                        .into_iter()
                        .map(|p| {
                            p.into_iter()
                                .map(|(x, y)| ((x - lo) * len / (hi - lo), y))
                                .collect()
                        })
                        .collect();
                    b.items.push(Item::Contours(contours));
                }
            }
            ArrowDir::Right => {
                // 1. Right arrowhead glyph: tip at `len`
                let arrow_x = len - 876.0 * k;
                b.items.push(Item::Glyph {
                    ch: '\u{2192}',
                    face,
                    em,
                    x: arrow_x,
                    y: y_glyph,
                    ssty: 0,
                });
                // 2. Extenders tiled to the left to reach x = 0
                // Arrowhead's own shaft reaches leftwards to arrow_x + 71.0 * k = len - 805.0 * k.
                let to_x = (arrow_x + 150.0 * k).max(0.0);
                if to_x > 0.0 {
                    let count = (to_x / e_w).ceil().max(1.0) as usize;
                    let step = if count > 1 {
                        (to_x - e_w) / (count - 1) as f32
                    } else {
                        0.0
                    };
                    for i in 0..count {
                        b.items.push(Item::Glyph {
                            ch: '\u{23AF}',
                            face,
                            em,
                            x: i as f32 * step,
                            y: y_glyph,
                            ssty: 0,
                        });
                    }
                }
            }
            ArrowDir::Left => {
                // 1. Left arrowhead glyph: tip at 0
                let arrow_x = -72.0 * k;
                b.items.push(Item::Glyph {
                    ch: '\u{2190}',
                    face,
                    em,
                    x: arrow_x,
                    y: y_glyph,
                    ssty: 0,
                });
                // 2. Extenders tiled to the right to reach x = len
                // Arrowhead's own shaft reaches rightwards to arrow_x + 877.0 * k = 805.0 * k.
                let from_x = (arrow_x + 877.0 * k - 150.0 * k).min(len - e_w);
                let to_x = len;
                let gap = to_x - from_x;
                if gap > 0.0 {
                    let count = (gap / e_w).ceil().max(1.0) as usize;
                    let step = if count > 1 {
                        (gap - e_w) / (count - 1) as f32
                    } else {
                        0.0
                    };
                    for i in 0..count {
                        b.items.push(Item::Glyph {
                            ch: '\u{23AF}',
                            face,
                            em,
                            x: from_x + i as f32 * step,
                            y: y_glyph,
                            ssty: 0,
                        });
                    }
                }
            }
            ArrowDir::Both => {
                // 1. Left arrowhead glyph: tip at 0
                let left_x = -72.0 * k;
                b.items.push(Item::Glyph {
                    ch: '\u{2190}',
                    face,
                    em,
                    x: left_x,
                    y: y_glyph,
                    ssty: 0,
                });
                // 2. Right arrowhead glyph: tip at len
                let right_x = len - 876.0 * k;
                b.items.push(Item::Glyph {
                    ch: '\u{2192}',
                    face,
                    em,
                    x: right_x,
                    y: y_glyph,
                    ssty: 0,
                });
                // 3. Extenders in the middle if there is a gap between shafts
                let from_x = left_x + 877.0 * k - 100.0 * k;
                let to_x = right_x + 71.0 * k + 100.0 * k;
                let gap = to_x - from_x;
                if gap > e_w {
                    let count = (gap / e_w).ceil().max(1.0) as usize;
                    let step = if count > 1 {
                        (gap - e_w) / (count - 1) as f32
                    } else {
                        0.0
                    };
                    for i in 0..count {
                        b.items.push(Item::Glyph {
                            ch: '\u{23AF}',
                            face,
                            em,
                            x: from_x + i as f32 * step,
                            y: y_glyph,
                            ssty: 0,
                        });
                    }
                }
            }
        }

        let gap = 0.10 * em;
        if let Some(o) = &over {
            b.place(o, (len - o.w) / 2.0, arrow_top_y - gap - o.desc);
        }
        if let Some(u) = &under {
            b.place(u, (len - u.w) / 2.0, arrow_bot_y + gap + u.asc);
        }
        b.w = len;
        Ok(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_labeled_arrow_glyph_assembly() {
        let mut l = Layout {
            f: fonts(),
            depth: 0,
        };
        let em = 12.0 * 96.0 / 72.0;
        let ctx = MathContext::DISPLAY;

        let node_top =
            MathNode::parse("<math><mover><mo>→</mo><mi>yields</mi></mover></math>").unwrap();
        let b = l.node(&node_top.children[0], em, ctx).unwrap();
        assert!(b.w > 0.0);
        let glyphs: Vec<char> = b
            .items
            .iter()
            .filter_map(|it| match it {
                Item::Glyph { ch, .. } => Some(*ch),
                _ => None,
            })
            .collect();
        assert!(
            glyphs.contains(&'\u{2192}'),
            "must contain right arrow glyph"
        );
        assert!(glyphs.contains(&'\u{23AF}'), "must contain extender glyph");

        // Bidirectional arrow with label below
        let node_under =
            MathNode::parse("<math><munder><mo>↔</mo><mi>x</mi></munder></math>").unwrap();
        let b2 = l.node(&node_under.children[0], em, ctx).unwrap();
        let glyphs2: Vec<char> = b2
            .items
            .iter()
            .filter_map(|it| match it {
                Item::Glyph { ch, .. } => Some(*ch),
                _ => None,
            })
            .collect();
        assert!(
            glyphs2.contains(&'\u{2190}'),
            "must contain left arrow glyph"
        );
        assert!(
            glyphs2.contains(&'\u{2192}'),
            "must contain right arrow glyph"
        );

        // Render preview images for visual inspection
        let math_full = MathNode::parse(
            "<math><mtable><mtr><mtd><mi>A</mi><mover><mo>→</mo><mi>yields</mi></mover><mi>B</mi></mtd></mtr>\
            <mtr><mtd><mi>A</mi><munder><mo>↔</mo><mi>x</mi></munder><mi>B</mi></mtd></mtr></mtable></math>"
        ).unwrap();
        let preview = crate::render::render_preview(&math_full, 12.0).unwrap();
        assert!(!preview.png.is_empty(), "preview PNG must not be empty");
    }

    #[test]
    fn test_equilibrium_arrow_labels_clear_harpoons() {
        let mut l = Layout {
            f: fonts(),
            depth: 0,
        };
        let em = 24.0;
        let node =
            MathNode::parse("<math><munderover><mo>⇌</mo><mi>g</mi><mi>f</mi></munderover></math>")
                .unwrap();
        let b = l.node(&node.children[0], em, MathContext::DISPLAY).unwrap();
        let (mut arrow_top, mut arrow_bot) = (f32::MAX, f32::MIN);
        let mut top_label_y = None;
        let mut bot_label_y = None;
        for it in &b.items {
            match it {
                Item::Poly(pts) => {
                    for &(_, y) in pts {
                        arrow_top = arrow_top.min(y);
                        arrow_bot = arrow_bot.max(y);
                    }
                }
                Item::Contours(cs) => {
                    for pts in cs {
                        for &(_, y) in pts {
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
}
