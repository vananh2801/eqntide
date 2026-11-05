//! Integral templates: the sign is assembled from glyph pieces and its limits
//! sit beside it.

use super::*;


/// Glyph pieces that make up a tall delimiter.
struct AssemblyRecipe {
    top: char,
    ext: char,
    mid: Option<char>,
    bot: char,
}

impl Layout {
    /// Generalized Glyph Assembly: Delimiters hug content tightly, Integrals remain untouched.
    pub(super) fn integral_without_limits(&self, c: char, target_h: f32, em: f32) -> Option<LBox> {

        let face = Face::Symbol;
        let recipe = match c {
            '\u{222B}' | '\u{222C}' | '\u{222D}' | '\u{222E}' | '\u{222F}' | '\u{2230}' => AssemblyRecipe {
                top: '\u{2320}',
                ext: '\u{23AE}',
                mid: None,
                bot: '\u{2321}'
            },
            _ => return None,
        };
        let num_stems = match c {
            '\u{222C}' | '\u{222F}' => 2,
            '\u{222D}' | '\u{2230}' => 3,
            _ => 1,
        };
        let ellipse_w = match c {
            '\u{222E}' => 1.0,
            '\u{222F}' => 2.0,
            '\u{2230}' => 3.0,
            _ => 0.0,
        };

        // 1. Integrals: keep the proven sizing logic unchanged
        let (eff_em, ext_top_count, ext_bot_count) = {
            let nominal_h0 = 2.00 * em;
            let (scale_factor, count) = if target_h < nominal_h0 {
                ((target_h / nominal_h0).max(0.60), 0)
            } else {
                (1.0, ((target_h - nominal_h0) / (0.80 * em)).ceil() as usize)
            };
            (em * scale_factor, count, 0)
        };

        // 3. Measure the pieces at the effective em size
        let (a_t, d_t) = self.f.ink(face, recipe.top, eff_em);
        let (a_b, d_b) = self.f.ink(face, recipe.bot, eff_em);
        let (a_e, d_e) = self.f.ink(face, recipe.ext, eff_em);
        let (_, a_m, d_m) = match recipe.mid {
            Some(m) => {
                let (a, d) = self.f.ink(face, m, eff_em);
                (a + d, a, d)
            }
            None => (0.0, 0.0, 0.0),
        };

        let mut pieces: Vec<(char, f32, f32)> = Vec::new();
        pieces.push((recipe.top, a_t, d_t));
        for _ in 0..ext_top_count {
            pieces.push((recipe.ext, a_e, d_e));
        }
        if let Some(m) = recipe.mid {
            pieces.push((m, a_m, d_m));
        }
        for _ in 0..ext_bot_count {
            pieces.push((recipe.ext, a_e, d_e));
        }
        pieces.push((recipe.bot, a_b, d_b));

        let n = pieces.len();
        let overlap = 1.5;

        // 4. Compute the Y offsets so the pieces join seamlessly
        let mut rel_y = vec![0.0f32; n];
        for i in 1..n {
            rel_y[i] = rel_y[i - 1] + pieces[i - 1].2 + pieces[i].1 - overlap;
        }

        let top_ink = rel_y[0] - pieces[0].1;
        let bot_ink = rel_y[n - 1] + pieces[n - 1].2;
        let actual_total_h = bot_ink - top_ink;

        let y_mid = -AXIS * em;
        let shift = (y_mid - actual_total_h / 2.0) - top_ink;

        let stem_spacing = 0.45 * eff_em;
        let single_w = self.f.advance(face, recipe.top, eff_em);
        let total_w = (num_stems - 1) as f32 * stem_spacing + single_w;

        let mut b = LBox {
            w: total_w,
            asc: -top_ink - shift,
            desc: bot_ink + shift,
            items: Vec::new(),
        };

        for s in 0..num_stems {
            let x = s as f32 * stem_spacing;
            for i in 0..n {
                let (ch, _, _) = pieces[i];
                b.items.push(Item::Glyph {
                    ch,
                    face,
                    em: eff_em,
                    x,
                    y: rel_y[i] + shift,
                });
            }
        }
        if ellipse_w != 0.0 {
            let (x_min, x_max) = self.f.ink_x(face, recipe.ext, eff_em);
            let stem_x_offset = (x_min + x_max) / 2.0;
            let xc = (num_stems - 1) as f32 * stem_spacing / 2.0 + stem_x_offset;
            let rx = 0.3 * eff_em * ellipse_w;
            let ry = 0.3 * eff_em;
            let n_pts = 24;
            let ellipse_pts: Vec<(f32, f32)> = (0..=n_pts).map(|i| {
                let a = i as f32 / n_pts as f32 * std::f32::consts::TAU;
                (xc + rx * a.cos(), y_mid + ry * a.sin())
            }).collect();
            for q in stroke(&ellipse_pts, 0.045 * eff_em) {
                b.items.push(Item::Poly(q));
            }
        }

        Some(b)
    }

    /// An integral sign with its lower/upper limits placed around it.
    pub(super) fn integral_with_limits(
        &mut self,
        c: char,
        under_n: Option<&MathNode>,
        over_n: Option<&MathNode>,
        em: f32,
    ) -> Result<LBox> {
        let sem = em * SCRIPT;
        // Use the hand-built glyph assembly
        let base = self.integral_without_limits(c, 1.45 * em, em).unwrap();
        let sub = match under_n { Some(u) => Some(self.node(u, sem, true)?), None => None };
        let sup = match over_n { Some(o) => Some(self.node(o, sem, true)?), None => None };

        let mut b = LBox::default();
        let w_base = base.w;
        let w_sub = sub.as_ref().map_or(0.0, |s| s.w);
        let w_sup = sup.as_ref().map_or(0.0, |s| s.w);
        let w_center = w_base.max(w_sub).max(w_sup);

        let offset_x = 0.12 * em;
        b.place(&base, offset_x + (w_center - w_base) / 2.0, 0.0);

        let gap = 0.08 * em;

        if let Some(o) = &sup {
            let ox = offset_x + (w_center - w_sup) / 2.0 + 0.14 * em;
            b.place(o, ox, -(base.asc + gap + o.desc));
        }

        if let Some(u) = &sub {
            let ux = offset_x + (w_center - w_sub) / 2.0 - 0.14 * em;
            b.place(u, ux, base.desc + gap + u.asc);
        }

        b.w = b.w + 0.12 * em;
        Ok(b)
    }
}
