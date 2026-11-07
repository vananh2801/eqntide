//! Labeled arrow templates (MathType `tmARROW`): a stretchy arrow with text
//! above and/or below it.
//!
//! The arrow is drawn as strokes (not as a font glyph) so that its length can
//! follow the width of its text, and it sits on the math axis between the
//! neighbouring symbols, exactly where MathType puts it. Only single-line
//! arrows are handled; double arrows (⇒ ⇐ ⇔) keep using the glyph path.
//!
//! [`template_of`] mirrors the cases that `mtef::encode` turns into
//! `extensible_arrow`, so the preview picture matches the MathType object.

use super::*;

/// Which way a stretchy arrow points.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ArrowDir {
    Right,
    Left,
    Both,
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
    let base = n.children.first()?;
    let (under, over) = match n.tag.as_str() {
        "munder" => (n.children.get(1), None),
        "mover" => (None, n.children.get(1)),
        "munderover" => (n.children.get(1), n.children.get(2)),
        _ => return None,
    };
    let is_label = |x: Option<&MathNode>| x.map_or(true, |x| mo_arrow(x).is_none());

    // 1. the base is the arrow, the children are its labels
    if let Some(dir) = mo_arrow(base) {
        if (under.is_some() || over.is_some()) && is_label(under) && is_label(over) {
            return Some(ArrowTemplate { dir, top: over, bottom: under });
        }
    }
    // 2. ↔ or ← drawn over the base text
    if let Some(dir) = over.and_then(mo_arrow).filter(|d| *d != ArrowDir::Right) {
        return Some(ArrowTemplate { dir, top: None, bottom: Some(base) });
    }
    // 3. an arrow drawn under the base text (limits such as `lim` keep their subscript)
    if let Some(dir) = under.and_then(mo_arrow) {
        if !base.text.contains("lim") {
            return Some(ArrowTemplate { dir, top: Some(base), bottom: None });
        }
    }
    None
}

impl Layout {
    /// An arrow whose length follows its text, with `top` centred above and
    /// `bottom` centred below the shaft.
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

        // Shaft: a little longer than the widest text, never shorter than a
        // plain arrow.
        let label_w = [over.as_ref().map(|o| o.w), under.as_ref().map(|u| u.w)]
            .into_iter()
            .flatten()
            .fold(0.0f32, f32::max);
        let len = (label_w + 0.6 * em).max(1.4 * em);

        let st = RULE * em;
        let y = -AXIS * em; // the shaft sits on the math axis
        let (head_len, head_half) = (0.14 * em, 0.08 * em);

        let mut b = LBox::default();
        b.w = len;
        b.asc = -y + head_half;
        b.desc = 0.0;

        let (l, r) = (0.0, len);
        let mut polys: Vec<Vec<(f32, f32)>> = Vec::new();
        // shaft, trimmed so it does not poke through the tips
        let trim = 0.5 * st;
        let (sl, sr) = (
            if dir == ArrowDir::Right { l } else { l + trim },
            if dir == ArrowDir::Left { r } else { r - trim },
        );
        polys.extend(stroke(&[(sl, y), (sr, y)], st));
        if dir != ArrowDir::Left {
            polys.extend(stroke(&[(r - head_len, y - head_half), (r, y), (r - head_len, y + head_half)], st));
        }
        if dir != ArrowDir::Right {
            polys.extend(stroke(&[(l + head_len, y - head_half), (l, y), (l + head_len, y + head_half)], st));
        }
        for p in polys {
            b.items.push(Item::Poly(p));
        }

        let gap = st / 2.0 + 0.12 * em;
        if let Some(o) = &over {
            b.place(o, (len - o.w) / 2.0, y - gap - o.desc);
        }
        if let Some(u) = &under {
            b.place(u, (len - u.w) / 2.0, y + gap + u.asc);
        }
        b.w = len;
        Ok(b)
    }
}
