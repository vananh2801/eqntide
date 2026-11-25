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

/// Direction of a stretchy labeled arrow.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum ArrowDir {
    Right,
    Left,
    Both,
    Other(char),
}

/// Recognized arrow template together with its optional top and bottom labels.
pub(super) struct ArrowTemplate<'a> {
    pub(super) dir: ArrowDir,
    pub(super) top: Option<&'a MathNode>,
    pub(super) bottom: Option<&'a MathNode>,
}

/// Direction of a single-line arrow character (spacing or combining form).
pub(super) fn arrow_dir(ch: char) -> Option<ArrowDir> {
    match ch {
        '\u{2192}' | '\u{27F6}' | '\u{20D7}' => Some(ArrowDir::Right),
        '\u{2190}' | '\u{27F5}' | '\u{20D6}' => Some(ArrowDir::Left),
        '\u{2194}' | '\u{27F7}' => Some(ArrowDir::Both),
        other @ ('⇒' | '⇐' | '⇔' | '⇀' | '↼' | '⇌' | '⇋' | '↦' | '⇄' | '⇆') => {
            Some(ArrowDir::Other(other))
        }
        _ => None,
    }
}

/// Recognize the `munder` / `mover` / `munderover` shapes that MathType draws
/// as a labeled arrow:
///
/// 1. an arrow with a label over and/or under it (`A ->(x) B`);
/// 2. `↔` / `←` over some text: the arrow is drawn above, the text below it;
/// 3. any arrow under some text (not a limit): the text is drawn above the arrow.
///
/// `→` over text stays a vector accent (`draw_accent`), as in MathType.
pub(super) fn template_of(node: &MathNode) -> Option<ArrowTemplate<'_>> {
    if node.attr("accent") == Some("true") || node.attr("accentunder") == Some("true") {
        return None;
    }
    let base = node.children.first()?;
    let (under, over) = match node.tag.as_str() {
        "munder" => (node.children.get(1), None),
        "mover" => (None, node.children.get(1)),
        "munderover" => (node.children.get(1), node.children.get(2)),
        _ => return None,
    };
    let is_label =
        |label_node: Option<&MathNode>| label_node.is_none_or(|inner| mo_arrow(inner).is_none());

    // 1. Base is the arrow; children are its labels.
    if let Some(dir) = mo_arrow(base) {
        if (under.is_some() || over.is_some()) && is_label(under) && is_label(over) {
            return Some(ArrowTemplate {
                dir,
                top: over,
                bottom: under,
            });
        }
    }
    // 2. ↔ or ← drawn over the base text.
    if let Some(dir) = over
        .and_then(mo_arrow)
        .filter(|dir| *dir != ArrowDir::Right)
    {
        return Some(ArrowTemplate {
            dir,
            top: None,
            bottom: Some(base),
        });
    }
    // 3. Arrow drawn under the base text (limits such as `lim` keep their subscript).
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

fn mo_arrow(node: &MathNode) -> Option<ArrowDir> {
    if node.tag != "mo" {
        return None;
    }
    node.text.trim().chars().next().and_then(arrow_dir)
}

impl Layout {
    /// Lay out an arrow whose length follows its labels, assembled from font glyphs
    /// with `top` centered above and `bottom` centered below the shaft.
    pub(super) fn labeled_arrow(
        &mut self,
        dir: ArrowDir,
        top: Option<&MathNode>,
        bottom: Option<&MathNode>,
        em: f32,
        ctx: MathContext,
    ) -> Result<LBox> {
        let constants = self.font_set.constants();
        let lim_ctx = ctx.for_limits();
        let lim_em = em * lim_ctx.scale(constants);
        let over = match top {
            Some(over_node) => Some(self.node(over_node, lim_em, lim_ctx)?),
            None => None,
        };
        let under = match bottom {
            Some(under_node) => Some(self.node(under_node, lim_em, lim_ctx)?),
            None => None,
        };

        // Shaft: wide enough so labels clear arrowheads on both sides.
        let label_w = [
            over.as_ref().map(|over_box| over_box.w),
            under.as_ref().map(|under_box| under_box.w),
        ]
        .into_iter()
        .flatten()
        .fold(0.0f32, f32::max);
        let len = (label_w + 0.72 * em).max(1.40 * em);

        let (head_asc, head_desc) = self.font_set.ink(Face::Symbol, '\u{2192}', em);
        let mut layout_box = LBox {
            w: len,
            asc: head_asc,
            desc: head_desc,
            ..Default::default()
        };

        let unit = em / 1000.0;
        let ext_w = 322.0 * unit; // advance of extender '\u{23AF}'
        let face = Face::Symbol;
        let y_glyph = 0.0; // on the baseline, the math glyphs' axis sits on the math axis
        let y_axis = -constants.axis_height * em;
        let rule_thick = constants.fraction_rule_thickness * em;
        let mut arrow_top_y = y_axis - rule_thick / 2.0;
        let mut arrow_bot_y = y_axis + rule_thick / 2.0;

        match dir {
            ArrowDir::Other(ch) => {
                let natural = self
                    .font_set
                    .glyph_polys(Face::Symbol, ch, em, em, 0.0, 0.0);
                let (min_x, max_x, min_y, max_y) = natural.iter().flatten().fold(
                    (f32::MAX, f32::MIN, f32::MAX, f32::MIN),
                    |(x_lo, x_hi, y_lo, y_hi), point| {
                        (
                            x_lo.min(point.0),
                            x_hi.max(point.0),
                            y_lo.min(point.1),
                            y_hi.max(point.1),
                        )
                    },
                );
                if max_x > min_x {
                    arrow_top_y = min_y;
                    arrow_bot_y = max_y;
                    layout_box.asc = layout_box.asc.max(-min_y);
                    layout_box.desc = layout_box.desc.max(max_y);
                    let contours: Vec<Vec<(f32, f32)>> = natural
                        .into_iter()
                        .map(|poly| {
                            poly.into_iter()
                                .map(|(x, y)| ((x - min_x) * len / (max_x - min_x), y))
                                .collect()
                        })
                        .collect();
                    layout_box.items.push(Item::Contours(contours));
                }
            }
            ArrowDir::Right => {
                // 1. Right arrowhead glyph: tip at `len`.
                let arrow_x = len - 876.0 * unit;
                layout_box.items.push(Item::Glyph {
                    ch: '\u{2192}',
                    face,
                    em,
                    x: arrow_x,
                    y: y_glyph,
                    ssty: 0,
                });
                // 2. Extenders tiled left to reach x = 0.
                let to_x = (arrow_x + 150.0 * unit).max(0.0);
                if to_x > 0.0 {
                    let count = (to_x / ext_w).ceil().max(1.0) as usize;
                    let step = if count > 1 {
                        (to_x - ext_w) / (count - 1) as f32
                    } else {
                        0.0
                    };
                    for i in 0..count {
                        layout_box.items.push(Item::Glyph {
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
                // 1. Left arrowhead glyph: tip at 0.
                let arrow_x = -72.0 * unit;
                layout_box.items.push(Item::Glyph {
                    ch: '\u{2190}',
                    face,
                    em,
                    x: arrow_x,
                    y: y_glyph,
                    ssty: 0,
                });
                // 2. Extenders tiled right to reach x = len.
                let from_x = (arrow_x + 877.0 * unit - 150.0 * unit).min(len - ext_w);
                let gap = len - from_x;
                if gap > 0.0 {
                    let count = (gap / ext_w).ceil().max(1.0) as usize;
                    let step = if count > 1 {
                        (gap - ext_w) / (count - 1) as f32
                    } else {
                        0.0
                    };
                    for i in 0..count {
                        layout_box.items.push(Item::Glyph {
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
                // 1. Left arrowhead glyph: tip at 0.
                let left_x = -72.0 * unit;
                layout_box.items.push(Item::Glyph {
                    ch: '\u{2190}',
                    face,
                    em,
                    x: left_x,
                    y: y_glyph,
                    ssty: 0,
                });
                // 2. Right arrowhead glyph: tip at len.
                let right_x = len - 876.0 * unit;
                layout_box.items.push(Item::Glyph {
                    ch: '\u{2192}',
                    face,
                    em,
                    x: right_x,
                    y: y_glyph,
                    ssty: 0,
                });
                // 3. Extenders in the middle when a gap remains between shafts.
                let from_x = left_x + 877.0 * unit - 100.0 * unit;
                let to_x = right_x + 71.0 * unit + 100.0 * unit;
                let gap = to_x - from_x;
                if gap > ext_w {
                    let count = (gap / ext_w).ceil().max(1.0) as usize;
                    let step = if count > 1 {
                        (gap - ext_w) / (count - 1) as f32
                    } else {
                        0.0
                    };
                    for i in 0..count {
                        layout_box.items.push(Item::Glyph {
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
        if let Some(over_box) = &over {
            layout_box.place(
                over_box,
                (len - over_box.w) / 2.0,
                arrow_top_y - gap - over_box.desc,
            );
        }
        if let Some(under_box) = &under {
            layout_box.place(
                under_box,
                (len - under_box.w) / 2.0,
                arrow_bot_y + gap + under_box.asc,
            );
        }
        layout_box.w = len;
        Ok(layout_box)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_labeled_arrow_glyph_assembly() {
        let mut layout = Layout {
            font_set: fonts(),
            depth: 0,
        };
        let em = 12.0 * 96.0 / 72.0;
        let ctx = MathContext::DISPLAY;

        let node_top =
            MathNode::parse("<math><mover><mo>→</mo><mi>yields</mi></mover></math>").unwrap();
        let top_box = layout.node(&node_top.children[0], em, ctx).unwrap();
        assert!(top_box.w > 0.0);
        let glyphs: Vec<char> = top_box
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Glyph { ch, .. } => Some(*ch),
                _ => None,
            })
            .collect();
        assert!(
            glyphs.contains(&'\u{2192}'),
            "must contain right arrow glyph"
        );
        assert!(glyphs.contains(&'\u{23AF}'), "must contain extender glyph");

        // Bidirectional arrow with label below.
        let node_under =
            MathNode::parse("<math><munder><mo>↔</mo><mi>x</mi></munder></math>").unwrap();
        let under_box = layout.node(&node_under.children[0], em, ctx).unwrap();
        let glyphs_under: Vec<char> = under_box
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Glyph { ch, .. } => Some(*ch),
                _ => None,
            })
            .collect();
        assert!(
            glyphs_under.contains(&'\u{2190}'),
            "must contain left arrow glyph"
        );
        assert!(
            glyphs_under.contains(&'\u{2192}'),
            "must contain right arrow glyph"
        );

        let math_full = MathNode::parse(
            "<math><mtable><mtr><mtd><mi>A</mi><mover><mo>→</mo><mi>yields</mi></mover><mi>B</mi></mtd></mtr>\
            <mtr><mtd><mi>A</mi><munder><mo>↔</mo><mi>x</mi></munder><mi>B</mi></mtd></mtr></mtable></math>",
        )
        .unwrap();
        let preview = crate::render::render_preview(&math_full, 12.0).unwrap();
        assert!(!preview.png.is_empty(), "preview PNG must not be empty");
    }

    #[test]
    fn test_equilibrium_arrow_labels_clear_harpoons() {
        let mut layout = Layout {
            font_set: fonts(),
            depth: 0,
        };
        let em = 24.0;
        let node =
            MathNode::parse("<math><munderover><mo>⇌</mo><mi>g</mi><mi>f</mi></munderover></math>")
                .unwrap();
        let layout_box = layout
            .node(&node.children[0], em, MathContext::DISPLAY)
            .unwrap();
        let (mut arrow_top, mut arrow_bot) = (f32::MAX, f32::MIN);
        let mut top_label_y = None;
        let mut bot_label_y = None;
        for item in &layout_box.items {
            match item {
                Item::Poly(points) => {
                    for &(_, y) in points {
                        arrow_top = arrow_top.min(y);
                        arrow_bot = arrow_bot.max(y);
                    }
                }
                Item::Contours(contours) => {
                    for points in contours {
                        for &(_, y) in points {
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
