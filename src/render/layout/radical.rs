//! Radical templates: square roots and n-th roots using OpenType MATH variants and assembly.

use super::*;

impl Layout {
    /// Lay out a square root or n-th root box enclosing `radicand`.
    pub(super) fn radical(
        &self,
        radicand: &LBox,
        degree: Option<&LBox>,
        em: f32,
        ctx: MathContext,
    ) -> LBox {
        let constants = self.font_set.constants();
        let upem = 1000.0;
        let thick = (constants.radical_rule_thickness * em).max(1.0);
        let gap = if ctx.is_display() {
            constants.radical_display_style_vertical_gap * em
        } else {
            constants.radical_vertical_gap * em
        };

        let h_target = radicand.asc + radicand.desc + gap + thick;
        let target_font_units = (h_target / em * upem).ceil() as u16;

        let info = self.font_set.radical_info();
        let chosen_variant = info
            .variants
            .iter()
            .find(|(_, adv)| *adv >= target_font_units);

        let mut layout_box = LBox::default();

        // 1. Determine horizontal degree offset and radical sign starting x (right-bottom anchored).
        let (crook_units, hook_font_y): (f32, f32) = match chosen_variant {
            Some(&(1657, _)) => (440.0, 430.0),
            Some(&(1658, _)) => (550.0, 620.0),
            Some(&(1659, _)) => (620.0, 825.0),
            Some(&(1660, _)) => (620.0, 1010.0),
            Some(&(_gid, _)) => (620.0, 1010.0),
            None => (720.0, 1675.0),
        };

        let (x_deg, x_rad) = if let Some(deg_box) = degree {
            let tuck_x = (crook_units / upem) * em;
            let pad_left = 0.05 * em;
            let shift = (pad_left + deg_box.w - tuck_x).max(0.0);
            let rad_x = shift;
            let deg_x = rad_x + tuck_x - deg_box.w;
            (Some(deg_x), rad_x)
        } else {
            (None, 0.0)
        };

        // 2. Lay out radical sign by choosing a variant glyph or assembling from parts.
        let (tip_x, rule_top_y, radical_bot_y, crook_base_y) = if let Some(&(variant_gid, _adv)) =
            chosen_variant
        {
            let (_x_min, y_min, x_max, y_max) = self
                .font_set
                .math_glyph_bbox(variant_gid)
                .unwrap_or((18, -265, 829, 922));
            let glyph_h = (y_max - y_min) as f32 / upem * em;
            let excess = (glyph_h - h_target).max(0.0);
            let actual_gap = gap + excess / 2.0;
            let top_y = -(radicand.asc + actual_gap + thick);
            let base_y = top_y + (y_max as f32 / upem * em);
            let tip = x_rad + (x_max as f32 / upem * em);

            let polys = self
                .font_set
                .math_glyph_polys(variant_gid, em, em, x_rad, base_y);
            if !polys.is_empty() {
                layout_box.items.push(Item::Contours(polys));
            }

            (tip, top_y, top_y + glyph_h, base_y)
        } else if let Some(ref assembly) = info.assembly {
            // Assemble from STIX Two Math parts: bottom hook, extenders, and top hook.
            let part_bot = assembly.parts.first();
            let part_ext = assembly.parts.iter().find(|part| part.is_extender);
            let part_top = assembly.parts.last();

            if let (Some(bot), Some(ext), Some(top)) = (part_bot, part_ext, part_top) {
                let min_overlap = assembly.min_connector_overlap as f32;
                let max_overlap =
                    (bot.end_connector.min(ext.start_connector) as f32).max(min_overlap);

                let adv_bot = bot.full_advance as f32;
                let adv_ext = ext.full_advance as f32;
                let adv_top = top.full_advance as f32;

                let mut ext_count = 0usize;
                loop {
                    let joints = (ext_count + 1) as f32;
                    let max_h =
                        adv_bot + (ext_count as f32) * adv_ext + adv_top - joints * min_overlap;
                    if max_h >= target_font_units as f32 || ext_count >= 30 {
                        break;
                    }
                    ext_count += 1;
                }

                let joints = (ext_count + 1) as f32;
                let total_raw = adv_bot + (ext_count as f32) * adv_ext + adv_top;
                let overlap = ((total_raw - target_font_units as f32) / joints)
                    .clamp(min_overlap, max_overlap);
                let assembled_units = total_raw - joints * overlap;
                let glyph_h = assembled_units / upem * em;
                let excess = (glyph_h - h_target).max(0.0);
                let actual_gap = gap + excess / 2.0;
                let top_y = -(radicand.asc + actual_gap + thick);

                let (_top_x_min, _top_y_min, top_x_max, top_y_max) = self
                    .font_set
                    .math_glyph_bbox(top.glyph_id)
                    .unwrap_or((849, 0, 1243, 641));
                let tip = x_rad + (top_x_max as f32 / upem * em);

                let y_top_base = top_y + (top_y_max as f32 / upem * em);
                let top_polys =
                    self.font_set
                        .math_glyph_polys(top.glyph_id, em, em, x_rad, y_top_base);
                if !top_polys.is_empty() {
                    layout_box.items.push(Item::Contours(top_polys));
                }

                let (_ext_x_min, _ext_y_min, _ext_x_max, ext_y_max) = self
                    .font_set
                    .math_glyph_bbox(ext.glyph_id)
                    .unwrap_or((849, 0, 916, 650));
                let mut current_y_base = y_top_base;
                for _ in 0..ext_count {
                    current_y_base += (ext_y_max as f32 - overlap) / upem * em;
                    let ext_polys =
                        self.font_set
                            .math_glyph_polys(ext.glyph_id, em, em, x_rad, current_y_base);
                    if !ext_polys.is_empty() {
                        layout_box.items.push(Item::Contours(ext_polys));
                    }
                }

                let (_bot_x_min, _bot_y_min, _bot_x_max, bot_y_max) = self
                    .font_set
                    .math_glyph_bbox(bot.glyph_id)
                    .unwrap_or((86, -5, 916, 1899));
                let y_bot_base = current_y_base + (bot_y_max as f32 - overlap) / upem * em;
                let bot_polys =
                    self.font_set
                        .math_glyph_polys(bot.glyph_id, em, em, x_rad, y_bot_base);
                if !bot_polys.is_empty() {
                    layout_box.items.push(Item::Contours(bot_polys));
                }

                (tip, top_y, top_y + glyph_h, y_bot_base)
            } else {
                let &(variant_gid, _) = info.variants.last().unwrap_or(&(1657, 1188));
                let (_, _, x_max, y_max) = self
                    .font_set
                    .math_glyph_bbox(variant_gid)
                    .unwrap_or((18, -265, 829, 922));
                let glyph_h = h_target;
                let top_y = -(radicand.asc + gap + thick);
                let base_y = top_y + (y_max as f32 / upem * em);
                let tip = x_rad + (x_max as f32 / upem * em);
                let polys = self
                    .font_set
                    .math_glyph_polys(variant_gid, em, em, x_rad, base_y);
                if !polys.is_empty() {
                    layout_box.items.push(Item::Contours(polys));
                }
                (tip, top_y, top_y + glyph_h, base_y)
            }
        } else {
            let &(variant_gid, _) = info.variants.first().unwrap_or(&(1657, 1188));
            let (_, _, x_max, y_max) = self
                .font_set
                .math_glyph_bbox(variant_gid)
                .unwrap_or((18, -265, 829, 922));
            let glyph_h = h_target;
            let top_y = -(radicand.asc + gap + thick);
            let base_y = top_y + (y_max as f32 / upem * em);
            let tip = x_rad + (x_max as f32 / upem * em);
            let polys = self
                .font_set
                .math_glyph_polys(variant_gid, em, em, x_rad, base_y);
            if !polys.is_empty() {
                layout_box.items.push(Item::Contours(polys));
            }
            (tip, top_y, top_y + glyph_h, base_y)
        };

        // 3. Draw vinculum (horizontal rule across radicand).
        let vinculum_x = tip_x;
        let vinculum_w = radicand.w + 0.08 * em;
        layout_box
            .items
            .push(Item::Poly(rect(vinculum_x, rule_top_y, vinculum_w, thick)));

        // 4. Place radicand.
        let radicand_x = tip_x + 0.04 * em;
        layout_box.place(radicand, radicand_x, 0.0);

        // 5. Place degree if present (right-bottom aligned in the crook).
        if let (Some(deg_box), Some(deg_x)) = (degree, x_deg) {
            let deg_bottom_y = crook_base_y - (hook_font_y / upem) * em;
            let deg_y = deg_bottom_y - deg_box.desc;
            layout_box.place(deg_box, deg_x, deg_y);
            layout_box.asc = layout_box.asc.max(deg_box.asc - deg_y);
            layout_box.desc = layout_box.desc.max(deg_box.desc + deg_y);
        }

        layout_box.asc = layout_box
            .asc
            .max(-rule_top_y + constants.radical_extra_ascender * em);
        layout_box.desc = layout_box.desc.max(radical_bot_y.max(radicand.desc));
        layout_box.w = vinculum_x + vinculum_w;
        layout_box
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_radical_variants_and_degrees() {
        let mut layout = Layout {
            font_set: fonts(),
            depth: 0,
        };
        let em = 20.0;

        let node_sqrt_x = MathNode::parse("<math><msqrt><mi>x</mi></msqrt></math>").unwrap();
        let box_sqrt_x = layout.node(&node_sqrt_x, em, MathContext::DISPLAY).unwrap();
        assert!(box_sqrt_x.asc > 0.0);
        assert!(box_sqrt_x.w > 0.5 * em);

        let node_sqrt_frac =
            MathNode::parse("<math><msqrt><mfrac><mn>1</mn><mn>2</mn></mfrac></msqrt></math>")
                .unwrap();
        let box_sqrt_frac = layout
            .node(&node_sqrt_frac, em, MathContext::DISPLAY)
            .unwrap();
        assert!(
            box_sqrt_frac.asc > box_sqrt_x.asc,
            "Fraction under sqrt should produce taller radical"
        );

        let node_root3 =
            MathNode::parse("<math><mroot><mi>x</mi><mn>3</mn></mroot></math>").unwrap();
        let box_root3 = layout.node(&node_root3, em, MathContext::DISPLAY).unwrap();
        let has_three = box_root3
            .items
            .iter()
            .any(|item| matches!(item, Item::Glyph { ch: '3', .. }));
        assert!(
            has_three,
            "Degree '3' must be rendered inside the radical nook"
        );

        let node_root_np = MathNode::parse(
            "<math><mroot><mi>x</mi><mrow><mi>n</mi><mo>·</mo><mi>p</mi></mrow></mroot></math>",
        )
        .unwrap();
        let box_root_np = layout
            .node(&node_root_np, em, MathContext::DISPLAY)
            .unwrap();
        assert!(
            box_root_np.w > box_sqrt_x.w,
            "Wide degree expands width to accommodate index"
        );
    }

    #[test]
    fn test_radical_assembly_for_very_tall_contents() {
        let mut layout = Layout {
            font_set: fonts(),
            depth: 0,
        };
        let em = 20.0;

        let xml = "<math><msqrt><mtable>\
            <mtr><mtd><mn>1</mn></mtd></mtr>\
            <mtr><mtd><mn>2</mn></mtd></mtr>\
            <mtr><mtd><mn>3</mn></mtd></mtr>\
            <mtr><mtd><mn>4</mn></mtd></mtr>\
            <mtr><mtd><mn>5</mn></mtd></mtr>\
        </mtable></msqrt></math>";
        let node_tall = MathNode::parse(xml).unwrap();
        let box_tall = layout.node(&node_tall, em, MathContext::DISPLAY).unwrap();

        assert!(
            box_tall.asc + box_tall.desc > 4.0 * em,
            "Assembly should span over 4 em"
        );
    }
}
