//! Structural MathML encoders for MTEF scripts, enclosures, piles, and matrices.

use crate::error::Result;
use crate::mathml::MathNode;
use crate::mtef::{
    boxed, long_division, matrix, pile, presubscript, presubsuperscript, presuperscript,
    strike_through, subscript, subsuperscript, superscript, HALIGN_CENTER, HALIGN_LEFT,
    HALIGN_RIGHT, TV_ST_DOWN, TV_ST_HORIZ, TV_ST_UP, VALIGN_CENTER_BASELINE,
};

use super::MathmlEncoder;

impl MathmlEncoder {
    /// Encode `<mmultiscripts>` including optional `<mprescripts/>` prescripts.
    pub(super) fn encode_mmultiscripts(&mut self, node: &MathNode) -> Result<Vec<u8>> {
        let base = self.nth(node, 0)?;
        if let Some(prescripts_pos) = node
            .children
            .iter()
            .position(|child| child.tag == "mprescripts")
        {
            let presub = match node.children.get(prescripts_pos + 1) {
                Some(child) if child.tag != "none" => self.dispatch(child)?,
                _ => Vec::new(),
            };
            let presup = match node.children.get(prescripts_pos + 2) {
                Some(child) if child.tag != "none" => self.dispatch(child)?,
                _ => Vec::new(),
            };
            let mut out = match (!presub.is_empty(), !presup.is_empty()) {
                (true, true) => presubsuperscript(&base, &presub, &presup),
                (true, false) => presubscript(&base, &presub),
                (false, true) => presuperscript(&base, &presup),
                (false, false) => base,
            };
            if prescripts_pos > 1 {
                let post_sub = match node.children.get(1) {
                    Some(child) if child.tag != "none" => self.dispatch(child)?,
                    _ => Vec::new(),
                };
                let post_sup = match node.children.get(2) {
                    Some(child) if prescripts_pos > 2 && child.tag != "none" => {
                        self.dispatch(child)?
                    }
                    _ => Vec::new(),
                };
                match (!post_sub.is_empty(), !post_sup.is_empty()) {
                    (true, true) => out.extend(subsuperscript(&post_sub, &post_sup)),
                    (true, false) => out.extend(subscript(&post_sub)),
                    (false, true) => out.extend(superscript(&post_sup)),
                    (false, false) => {}
                }
            }
            return Ok(out);
        }

        let mut out = base;
        let sub = match node.children.get(1) {
            Some(child) if child.tag != "none" => self.dispatch(child)?,
            _ => Vec::new(),
        };
        let sup = match node.children.get(2) {
            Some(child) if child.tag != "none" => self.dispatch(child)?,
            _ => Vec::new(),
        };
        match (!sub.is_empty(), !sup.is_empty()) {
            (true, true) => out.extend(subsuperscript(&sub, &sup)),
            (true, false) => out.extend(subscript(&sub)),
            (false, true) => out.extend(superscript(&sup)),
            (false, false) => {}
        }
        Ok(out)
    }

    /// Encode `<menclose>` into long-division, strike-through, or box templates.
    pub(super) fn encode_menclose(&mut self, node: &MathNode) -> Result<Vec<u8>> {
        let inner = self.encode_children(node)?;
        let notation = node.attr("notation").unwrap_or("box");
        if notation.contains("longdiv") {
            return Ok(long_division(&inner, None));
        }
        let mut strike_bits = 0u8;
        if notation.contains("horizontalstrike") {
            strike_bits |= TV_ST_HORIZ;
        }
        if notation.contains("updiagonalstrike") {
            strike_bits |= TV_ST_UP;
        }
        if notation.contains("downdiagonalstrike") {
            strike_bits |= TV_ST_DOWN;
        }
        if strike_bits != 0 {
            return Ok(strike_through(&inner, strike_bits));
        }
        let is_rounded = notation.contains("roundedbox");
        let has_all_sides = notation.contains("box") || is_rounded;
        let is_actuarial = notation.contains("actuarial");
        Ok(boxed(
            &inner,
            is_rounded,
            has_all_sides || notation.contains("left"),
            has_all_sides || is_actuarial || notation.contains("right"),
            has_all_sides || is_actuarial || notation.contains("top"),
            has_all_sides || notation.contains("bottom"),
        ))
    }

    /// Encode `<mtable>` into long-division (2x1 with `<menclose>`), `PILE`, or `MATRIX`.
    pub(super) fn encode_mtable(&mut self, node: &MathNode) -> Result<Vec<u8>> {
        let row_count = node.children.len();
        if row_count == 0 {
            return Ok(Vec::new());
        }
        let col_count = node
            .children
            .iter()
            .map(|row| row.children.len())
            .max()
            .unwrap_or(0);
        if col_count <= 1 {
            if row_count == 2 {
                let second_row_cell = node.children[1]
                    .children
                    .first()
                    .unwrap_or(&node.children[1]);
                if second_row_cell.children.len() == 1
                    && second_row_cell.children[0].tag == "menclose"
                    && second_row_cell.children[0]
                        .attr("notation")
                        .unwrap_or("")
                        .contains("longdiv")
                {
                    let first_row_cell = node.children[0]
                        .children
                        .first()
                        .unwrap_or(&node.children[0]);
                    let quotient = self.encode_children(first_row_cell)?;
                    let dividend = self.encode_children(&second_row_cell.children[0])?;
                    return Ok(long_division(
                        &dividend,
                        (!quotient.is_empty()).then_some(quotient.as_slice()),
                    ));
                }
            }
            let mut encoded_rows = Vec::with_capacity(row_count);
            for row in &node.children {
                encoded_rows.push(match row.children.first() {
                    Some(cell) => self.encode_children(cell)?,
                    None => self.encode_children(row)?,
                });
            }
            if row_count == 1 {
                return Ok(encoded_rows.pop().unwrap_or_default());
            }
            let row_refs: Vec<&[u8]> = encoded_rows.iter().map(Vec::as_slice).collect();
            let halign = match node.attr("columnalign") {
                Some("left") => HALIGN_LEFT,
                Some("right") => HALIGN_RIGHT,
                _ => HALIGN_CENTER,
            };
            return Ok(pile(halign, VALIGN_CENTER_BASELINE, &row_refs));
        }

        let mut encoded_cells = Vec::with_capacity(row_count * col_count);
        for row in &node.children {
            for col_idx in 0..col_count {
                encoded_cells.push(match row.children.get(col_idx) {
                    Some(cell) => self.encode_children(cell)?,
                    None => Vec::new(),
                });
            }
        }
        let cell_refs: Vec<&[u8]> = encoded_cells.iter().map(Vec::as_slice).collect();
        let halign = match node.attr("columnalign") {
            Some("left") => HALIGN_LEFT,
            Some("right") => HALIGN_RIGHT,
            _ => HALIGN_CENTER,
        };
        Ok(matrix(
            VALIGN_CENTER_BASELINE,
            halign,
            VALIGN_CENTER_BASELINE,
            row_count.min(u8::MAX as usize) as u8,
            col_count.min(u8::MAX as usize) as u8,
            &cell_refs,
        ))
    }
}
