//! Summation, product and set-theory big operators, plus the shared
//! `munder` / `mover` / `munderover` handler that also routes integrals,
//! accents and under-braces to their own modules.

use super::*;


impl Layout {
    /// A big operator glyph centred on the math axis.
    pub(super) fn big_glyph(&self, ch: char, face: Face, em: f32, scale: f32) -> LBox {
        let size = em * scale;
        let (a, d) = self.f.ink(face, ch, size);
        let y = (a - d) / 2.0 - AXIS * em;
        let mut b = LBox { w: self.f.advance(face, ch, size), ..Default::default() };
        b.items.push(Item::Glyph { ch, face, em: size, x: 0.0, y, ssty: 0 });
        b.asc = a - y;
        b.desc = d + y;
        b
    }

    pub(super) fn under_over(&mut self, n: &MathNode, em: f32, ctx: MathContext) -> Result<LBox> {
        let first = n.children.first();
        let base_char = first
            .filter(|b| b.tag == "mo")
            .and_then(|b| b.text.trim().chars().next());
        let c = self.f.constants();
        let lim_ctx = ctx.for_limits();
        let lim_em = em * lim_ctx.scale(c);
        let (under_n, over_n) = match n.tag.as_str() {
            "munder" => (n.children.get(1), None),
            "mover" => (None, n.children.get(1)),
            _ => (n.children.get(1), n.children.get(2)),
        };

        // Integral sign: limits sit above/below the sign (see `integral.rs`).
        if let Some(ch) = base_char.filter(|ch| is_integral(*ch)) {
            return self.integral_with_limits(ch, under_n, over_n, em, ctx);
        }

        // Labeled arrow (MathType `tmARROW`): stretchy arrow with text above / below (see `arrow.rs`).
        if let Some(t) = arrow::template_of(n) {
            return self.labeled_arrow(t.dir, t.top, t.bottom, em, ctx);
        }

        let base = match first {
            Some(f) => self.node(f, em, ctx)?,
            None => LBox::default(),
        };
        let big = base_char.map_or(false, is_big_op) || first.map_or(false, |f| f.tag == "mi" && is_function(f.text.trim()));

        // Accent over a normal base.
        let accent = over_n
            .filter(|o| o.tag == "mo" && !big)
            .and_then(|o| o.text.trim().chars().next())
            .filter(|c| is_accent(*c));
        let brace_under = under_n.filter(|u| u.tag == "mo" && u.text.trim() == "\u{23DF}").is_some();
        let bar_under = under_n
            .filter(|u| u.tag == "mo" && !big)
            .and_then(|u| u.text.trim().chars().next())
            .map_or(false, |c| c == '_' || c == '\u{0332}' || c == '\u{005F}' || c == '\u{00AF}' || c == '\u{203E}');

        let under = match under_n {
            Some(u) if !brace_under && !bar_under => Some(self.node(u, lim_em, lim_ctx)?),
            _ => None,
        };
        let over = match over_n {
            Some(o) if accent.is_none() => Some(self.node(o, lim_em, lim_ctx)?),
            _ => None,
        };

        let gap = if big { 0.10 * em } else { 0.12 * em };
        let w = [Some(base.w), under.as_ref().map(|u| u.w), over.as_ref().map(|o| o.w)]
            .into_iter()
            .flatten()
            .fold(0.0f32, f32::max);
        let mut b = LBox::default();
        b.place(&base, (w - base.w) / 2.0, 0.0);
        if let Some(o) = &over {
            b.place(o, (w - o.w) / 2.0, -(base.asc + gap + o.desc));
        }
        if let Some(u) = &under {
            b.place(u, (w - u.w) / 2.0, base.desc + gap + u.asc);
        }
        if let Some(c) = accent {
            self.draw_accent(&mut b, c, w, (w - base.w) / 2.0, base.asc, em);
        }
        if brace_under {
            self.draw_under_brace(&mut b, w, base.desc, em);
        }
        if bar_under {
            let st = 0.045 * em;
            let y = (base.desc + 0.06 * em).max(0.10 * em);
            let x0 = (w - base.w) / 2.0;
            b.items.push(Item::Poly(rect(x0, y, base.w, st)));
            b.desc = b.desc.max(y + st + 0.04 * em);
        }
        b.w = w;
        Ok(b)
    }
}
