//! Radical templates: square roots and n-th roots using OpenType MATH variants and assembly.

use super::*;

impl Layout {
    pub(super) fn radical(&self, c: &LBox, deg: Option<&LBox>, em: f32, ctx: MathContext) -> LBox {
        let consts = self.f.constants();
        let upem = 1000.0;
        let thick = (consts.radical_rule_thickness * em).max(1.0);
        let gap = if ctx.is_display() {
            consts.radical_display_style_vertical_gap * em
        } else {
            consts.radical_vertical_gap * em
        };

        let h_target = c.asc + c.desc + gap + thick;
        let target_font_units = (h_target / em * upem).ceil() as u16;

        let info = self.f.radical_info();
        let chosen_variant = info
            .variants
            .iter()
            .find(|(_, adv)| *adv >= target_font_units);

        let mut b = LBox::default();

        // 1. Determine horizontal degree offset and radical sign starting x (right-bottom anchored)
        let (crook_units, hook_font_y): (f32, f32) = match chosen_variant {
            Some(&(1657, _)) => (440.0, 430.0),
            Some(&(1658, _)) => (550.0, 620.0),
            Some(&(1659, _)) => (620.0, 825.0),
            Some(&(1660, _)) => (620.0, 1010.0),
            Some(&(_gid, _)) => (620.0, 1010.0),
            None => (720.0, 1675.0),
        };

        let (x_deg, x_rad) = if let Some(d) = deg {
            let tuck_x = (crook_units / upem) * em;
            let pad_left = 0.05 * em;
            let shift = (pad_left + d.w - tuck_x).max(0.0);
            let x_r = shift;
            let x_d = x_r + tuck_x - d.w;
            (Some(x_d), x_r)
        } else {
            (None, 0.0)
        };

        // 2. Lay out radical sign: either by choosing a variant glyph or assembling from parts
        let (tip_x, rule_top_y, radical_bot_y, crook_base_y) =
            if let Some(&(variant_gid, _adv)) = chosen_variant {
                let (_x_min, y_min, x_max, y_max) = self
                    .f
                    .math_glyph_bbox(variant_gid)
                    .unwrap_or((18, -265, 829, 922));
                let glyph_h = (y_max - y_min) as f32 / upem * em;
                let excess = (glyph_h - h_target).max(0.0);
                let actual_gap = gap + excess / 2.0;
                let r_top_y = -(c.asc + actual_gap + thick);
                let y_base = r_top_y + (y_max as f32 / upem * em);
                let t_x = x_rad + (x_max as f32 / upem * em);

                let polys = self.f.math_glyph_polys(variant_gid, em, em, x_rad, y_base);
                if !polys.is_empty() {
                    b.items.push(Item::Contours(polys));
                }

                let r_bot_y = r_top_y + glyph_h;
                (t_x, r_top_y, r_bot_y, y_base)
            } else if let Some(ref assembly) = info.assembly {
                // Assembly from STIX Two Math parts: Part 0 (bottom hook), Part 1 (extender), Part 2 (top)
                let part_bot = assembly.parts.first();
                let part_ext = assembly.parts.iter().find(|p| p.is_extender);
                let part_top = assembly.parts.last();

                if let (Some(bot), Some(ext), Some(top)) = (part_bot, part_ext, part_top) {
                    let min_o = assembly.min_connector_overlap as f32;
                    let max_o = (bot.end_connector.min(ext.start_connector) as f32).max(min_o);

                    // Determine number of extenders k needed
                    let adv_bot = bot.full_advance as f32;
                    let adv_ext = ext.full_advance as f32;
                    let adv_top = top.full_advance as f32;

                    let mut k = 0usize;
                    loop {
                        let joints = (k + 1) as f32;
                        let max_h = adv_bot + (k as f32) * adv_ext + adv_top - joints * min_o;
                        if max_h >= target_font_units as f32 || k >= 30 {
                            break;
                        }
                        k += 1;
                    }

                    let joints = (k + 1) as f32;
                    let total_raw = adv_bot + (k as f32) * adv_ext + adv_top;
                    // Calculate exact overlap o to achieve target height (clamped between min_o and max_o)
                    let o = ((total_raw - target_font_units as f32) / joints).clamp(min_o, max_o);
                    let assembled_units = total_raw - joints * o;
                    let glyph_h = assembled_units / upem * em;
                    let excess = (glyph_h - h_target).max(0.0);
                    let actual_gap = gap + excess / 2.0;
                    let r_top_y = -(c.asc + actual_gap + thick);

                    // Top part bbox: y_max = 641, x_max = 1243
                    let (_top_x_min, _top_y_min, top_x_max, top_y_max) = self
                        .f
                        .math_glyph_bbox(top.glyph_id)
                        .unwrap_or((849, 0, 1243, 641));
                    let t_x = x_rad + (top_x_max as f32 / upem * em);

                    // Place top part so its tip is at r_top_y
                    let y_top_base = r_top_y + (top_y_max as f32 / upem * em);
                    let top_polys =
                        self.f
                            .math_glyph_polys(top.glyph_id, em, em, x_rad, y_top_base);
                    if !top_polys.is_empty() {
                        b.items.push(Item::Contours(top_polys));
                    }

                    // Place k extenders below top part
                    let (_ext_x_min, _ext_y_min, _ext_x_max, ext_y_max) = self
                        .f
                        .math_glyph_bbox(ext.glyph_id)
                        .unwrap_or((849, 0, 916, 650));
                    let mut current_y_base = y_top_base;
                    for _ in 0..k {
                        current_y_base += (ext_y_max as f32 - o) / upem * em;
                        let ext_polys =
                            self.f
                                .math_glyph_polys(ext.glyph_id, em, em, x_rad, current_y_base);
                        if !ext_polys.is_empty() {
                            b.items.push(Item::Contours(ext_polys));
                        }
                    }

                    // Place bottom part below last extender
                    let (_bot_x_min, _bot_y_min, _bot_x_max, bot_y_max) = self
                        .f
                        .math_glyph_bbox(bot.glyph_id)
                        .unwrap_or((86, -5, 916, 1899));
                    let y_bot_base = current_y_base + (bot_y_max as f32 - o) / upem * em;
                    let bot_polys =
                        self.f
                            .math_glyph_polys(bot.glyph_id, em, em, x_rad, y_bot_base);
                    if !bot_polys.is_empty() {
                        b.items.push(Item::Contours(bot_polys));
                    }

                    let r_bot_y = r_top_y + glyph_h;
                    (t_x, r_top_y, r_bot_y, y_bot_base)
                } else {
                    // Fallback: draw largest variant
                    let &(variant_gid, _) = info.variants.last().unwrap_or(&(1657, 1188));
                    let (_, _, x_max, y_max) = self
                        .f
                        .math_glyph_bbox(variant_gid)
                        .unwrap_or((18, -265, 829, 922));
                    let glyph_h = h_target;
                    let r_top_y = -(c.asc + gap + thick);
                    let y_base = r_top_y + (y_max as f32 / upem * em);
                    let t_x = x_rad + (x_max as f32 / upem * em);
                    let polys = self.f.math_glyph_polys(variant_gid, em, em, x_rad, y_base);
                    if !polys.is_empty() {
                        b.items.push(Item::Contours(polys));
                    }
                    (t_x, r_top_y, r_top_y + glyph_h, y_base)
                }
            } else {
                // Minimal fallback
                let &(variant_gid, _) = info.variants.first().unwrap_or(&(1657, 1188));
                let (_, _, x_max, y_max) = self
                    .f
                    .math_glyph_bbox(variant_gid)
                    .unwrap_or((18, -265, 829, 922));
                let glyph_h = h_target;
                let r_top_y = -(c.asc + gap + thick);
                let y_base = r_top_y + (y_max as f32 / upem * em);
                let t_x = x_rad + (x_max as f32 / upem * em);
                let polys = self.f.math_glyph_polys(variant_gid, em, em, x_rad, y_base);
                if !polys.is_empty() {
                    b.items.push(Item::Contours(polys));
                }
                (t_x, r_top_y, r_top_y + glyph_h, y_base)
            };

        // 3. Draw vinculum (horizontal rule across radicand)
        let vinculum_x = tip_x;
        let vinculum_w = c.w + 0.08 * em;
        b.items
            .push(Item::Poly(rect(vinculum_x, rule_top_y, vinculum_w, thick)));

        // 4. Place radicand c
        let radicand_x = tip_x + 0.04 * em;
        b.place(c, radicand_x, 0.0);

        // 5. Place degree if present (right-bottom aligned)
        if let (Some(d), Some(x_d)) = (deg, x_deg) {
            let deg_bottom_y = crook_base_y - (hook_font_y / upem) * em;
            let deg_y = deg_bottom_y - d.desc;
            b.place(d, x_d, deg_y);
            b.asc = b.asc.max(d.asc - deg_y);
            b.desc = b.desc.max(d.desc + deg_y);
        }

        b.asc = b.asc.max(-rule_top_y + consts.radical_extra_ascender * em);
        b.desc = b.desc.max(radical_bot_y.max(c.desc));
        b.w = vinculum_x + vinculum_w;
        b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_radical_variants_and_degrees() {
        let mut l = Layout {
            f: fonts(),
            depth: 0,
        };
        let em = 20.0;

        // 1. Small radicand: \sqrt{x}
        let node_sqrt_x = MathNode::parse("<math><msqrt><mi>x</mi></msqrt></math>").unwrap();
        let b_sqrt_x = l.node(&node_sqrt_x, em, MathContext::DISPLAY).unwrap();
        assert!(b_sqrt_x.asc > 0.0);
        assert!(b_sqrt_x.w > 0.5 * em);

        // 2. Tall radicand: \sqrt{\frac{1}{2}} (fraction requires larger variant)
        let node_sqrt_frac =
            MathNode::parse("<math><msqrt><mfrac><mn>1</mn><mn>2</mn></mfrac></msqrt></math>")
                .unwrap();
        let b_sqrt_frac = l.node(&node_sqrt_frac, em, MathContext::DISPLAY).unwrap();
        assert!(
            b_sqrt_frac.asc > b_sqrt_x.asc,
            "Fraction under sqrt should produce taller radical"
        );

        // 3. N-th root: \sqrt[3]{x}
        let node_root3 =
            MathNode::parse("<math><mroot><mi>x</mi><mn>3</mn></mroot></math>").unwrap();
        let b_root3 = l.node(&node_root3, em, MathContext::DISPLAY).unwrap();
        // The degree '3' should be present
        let has_three = b_root3
            .items
            .iter()
            .any(|it| matches!(it, Item::Glyph { ch: '3', .. }));
        assert!(
            has_three,
            "Degree '3' must be rendered inside the radical nook"
        );

        // Multi-digit or wide degree should expand width horizontally: \sqrt[n \cdot p]{x}
        let node_root_np = MathNode::parse(
            "<math><mroot><mi>x</mi><mrow><mi>n</mi><mo>·</mo><mi>p</mi></mrow></mroot></math>",
        )
        .unwrap();
        let b_root_np = l.node(&node_root_np, em, MathContext::DISPLAY).unwrap();
        assert!(
            b_root_np.w > b_sqrt_x.w,
            "Wide degree expands width to accommodate index"
        );
    }

    #[test]
    fn test_radical_assembly_for_very_tall_contents() {
        let mut l = Layout {
            f: fonts(),
            depth: 0,
        };
        let em = 20.0;

        // Extremely tall radicand (e.g. 5-row tall table) exceeding variant 3 (2892 font units = 2.892 * em)
        let xml = "<math><msqrt><mtable>\
            <mtr><mtd><mn>1</mn></mtd></mtr>\
            <mtr><mtd><mn>2</mn></mtd></mtr>\
            <mtr><mtd><mn>3</mn></mtd></mtr>\
            <mtr><mtd><mn>4</mn></mtd></mtr>\
            <mtr><mtd><mn>5</mn></mtd></mtr>\
        </mtable></msqrt></math>";
        let node_tall = MathNode::parse(xml).unwrap();
        let b_tall = l.node(&node_tall, em, MathContext::DISPLAY).unwrap();

        // The assembled radical should enclose the entire tall matrix
        assert!(
            b_tall.asc + b_tall.desc > 4.0 * em,
            "Assembly should span over 4 em"
        );
    }
}
