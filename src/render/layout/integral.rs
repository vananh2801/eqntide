//! Integral templates: the sign is assembled from glyph pieces and its limits
//! can sit above/below it; explicit side limits use the script layout.

use super::*;

/// Glyph pieces that make up a tall integral sign.
struct AssemblyRecipe {
    top: char,
    ext: char,
    mid: Option<char>,
    bot: char,
}

impl Layout {
    /// Assemble an integral sign from glyph pieces (delimiters live in `delimiter.rs`).
    pub(super) fn integral_without_limits(&self, ch: char, target_h: f32, em: f32) -> Option<LBox> {
        let face = Face::Symbol;
        let recipe = match ch {
            '\u{222B}' | '\u{222C}' | '\u{222D}' | '\u{222E}' | '\u{222F}' | '\u{2230}'
            | '\u{2232}' | '\u{2233}' => AssemblyRecipe {
                top: '\u{2320}',
                ext: '\u{23AE}',
                mid: None,
                bot: '\u{2321}',
            },
            _ => return None,
        };
        let num_stems = match ch {
            '\u{222C}' | '\u{222F}' => 2,
            '\u{222D}' | '\u{2230}' => 3,
            _ => 1,
        };
        let ellipse_w = match ch {
            '\u{222E}' | '\u{2232}' | '\u{2233}' => 1.0,
            '\u{222F}' => 2.0,
            '\u{2230}' => 3.0,
            _ => 0.0,
        };

        // 1. Integrals: keep the proven sizing logic unchanged.
        let (eff_em, ext_top_count, ext_bot_count) = {
            let nominal_h0 = 2.00 * em;
            let (scale_factor, count) = if target_h < nominal_h0 {
                ((target_h / nominal_h0).max(0.60), 0)
            } else {
                (1.0, ((target_h - nominal_h0) / (0.80 * em)).ceil() as usize)
            };
            (em * scale_factor, count, 0)
        };

        // 2. Measure the pieces at the effective em size.
        let (asc_top, desc_top) = self.font_set.ink(face, recipe.top, eff_em);
        let (asc_bot, desc_bot) = self.font_set.ink(face, recipe.bot, eff_em);
        let (asc_ext, desc_ext) = self.font_set.ink(face, recipe.ext, eff_em);
        let (_, asc_mid, desc_mid) = match recipe.mid {
            Some(mid_ch) => {
                let (asc, desc) = self.font_set.ink(face, mid_ch, eff_em);
                (asc + desc, asc, desc)
            }
            None => (0.0, 0.0, 0.0),
        };

        let mut pieces: Vec<(char, f32, f32)> = Vec::new();
        pieces.push((recipe.top, asc_top, desc_top));
        for _ in 0..ext_top_count {
            pieces.push((recipe.ext, asc_ext, desc_ext));
        }
        if let Some(mid_ch) = recipe.mid {
            pieces.push((mid_ch, asc_mid, desc_mid));
        }
        for _ in 0..ext_bot_count {
            pieces.push((recipe.ext, asc_ext, desc_ext));
        }
        pieces.push((recipe.bot, asc_bot, desc_bot));

        let count = pieces.len();
        let overlap = 0.03 * eff_em;

        // 3. Compute the y offsets so the pieces join seamlessly.
        let mut rel_y = vec![0.0f32; count];
        for i in 1..count {
            rel_y[i] = rel_y[i - 1] + pieces[i - 1].2 + pieces[i].1 - overlap;
        }

        let top_ink = rel_y[0] - pieces[0].1;
        let bot_ink = rel_y[count - 1] + pieces[count - 1].2;
        let actual_total_h = bot_ink - top_ink;

        let y_mid = -AXIS * em;
        let shift = (y_mid - actual_total_h / 2.0) - top_ink;

        let stem_spacing = 0.45 * eff_em;
        let single_w = self.font_set.advance(face, recipe.top, eff_em);
        let total_w = (num_stems - 1) as f32 * stem_spacing + single_w;

        let mut layout_box = LBox {
            hits: Vec::new(),
            w: total_w,
            asc: -top_ink - shift,
            desc: bot_ink + shift,
            items: Vec::new(),
        };

        for stem_idx in 0..num_stems {
            let x = stem_idx as f32 * stem_spacing;
            for i in 0..count {
                let (piece_ch, _, _) = pieces[i];
                layout_box.items.push(Item::Glyph {
                    ch: piece_ch,
                    face,
                    em: eff_em,
                    x,
                    y: rel_y[i] + shift,
                    ssty: 0,
                });
            }
        }
        if ellipse_w != 0.0 {
            let (x_min, x_max) = self.font_set.ink_x(face, recipe.ext, eff_em);
            let stem_x_offset = (x_min + x_max) / 2.0;
            let center_x = (num_stems - 1) as f32 * stem_spacing / 2.0 + stem_x_offset;
            let radius_x = 0.3 * eff_em * ellipse_w;
            let radius_y = 0.3 * eff_em;
            let steps = 24;
            let ellipse_pts: Vec<(f32, f32)> = (0..=steps)
                .map(|i| {
                    let angle = i as f32 / steps as f32 * std::f32::consts::TAU;
                    (
                        center_x + radius_x * angle.cos(),
                        y_mid + radius_y * angle.sin(),
                    )
                })
                .collect();
            for quad in stroke(&ellipse_pts, 0.045 * eff_em) {
                layout_box.items.push(Item::Poly(quad));
            }
        }

        Some(layout_box)
    }

    /// Lay out an integral sign with its lower/upper limits placed above and below it.
    pub(super) fn integral_with_limits(
        &mut self,
        base_node: &MathNode,
        under_node: Option<&MathNode>,
        over_node: Option<&MathNode>,
        em: f32,
        ctx: MathContext,
    ) -> Result<LBox> {
        let constants = self.font_set.constants();
        let sub_ctx = ctx.for_subscript();
        let sup_ctx = ctx.for_superscript();
        let sub_em = em * sub_ctx.scale(constants);
        let sup_em = em * sup_ctx.scale(constants);

        // Reuse the integral assembly through the normal node path, retaining
        // the operator's hit region for caret/selection in the native editor.
        let base = self.node(base_node, em, ctx)?;
        let sub = match under_node {
            Some(node) => Some(self.node(node, sub_em, sub_ctx)?),
            None => None,
        };
        let sup = match over_node {
            Some(node) => Some(self.node(node, sup_em, sup_ctx)?),
            None => None,
        };

        let mut layout_box = LBox::default();
        let w_base = base.w;
        let w_sub = sub.as_ref().map_or(0.0, |sub_box| sub_box.w);
        let w_sup = sup.as_ref().map_or(0.0, |sup_box| sup_box.w);
        let w_center = w_base.max(w_sub).max(w_sup);

        let offset_x = 0.12 * em;
        layout_box.place(&base, offset_x + (w_center - w_base) / 2.0, 0.0);

        let gap = 0.08 * em;

        if let Some(sup_box) = &sup {
            let sup_x = offset_x + (w_center - w_sup) / 2.0 + 0.14 * em;
            layout_box.place(sup_box, sup_x, -(base.asc + gap + sup_box.desc));
        }

        if let Some(sub_box) = &sub {
            let sub_x = offset_x + (w_center - w_sub) / 2.0 - 0.14 * em;
            layout_box.place(sub_box, sub_x, base.desc + gap + sub_box.asc);
        }

        layout_box.w += 0.12 * em;
        Ok(layout_box)
    }
}
