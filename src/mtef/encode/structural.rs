//! Structural MathML encoders for MTEF templates, piles, and matrices.

use crate::error::Result;
use crate::mathml::MathNode;
use crate::mtef::{
    boxed, long_division, matrix, pile, presubscript, presubsuperscript, presuperscript,
    strike_through, subscript, subsuperscript, superscript, HALIGN_CENTER, HALIGN_LEFT,
    HALIGN_RIGHT, TV_ST_DOWN, TV_ST_HORIZ, TV_ST_UP, VALIGN_CENTER_BASELINE,
};

use super::MathmlEncoder;

impl MathmlEncoder {
    pub(super) fn encode_mmultiscripts(&mut self, node: &MathNode) -> Result<Vec<u8>> {
        let base = self.nth(node, 0)?;
        if let Some(prescripts_pos) = node.children.iter().position(|c| c.tag == "mprescripts") {
            let presub = match node.children.get(prescripts_pos + 1) {
                Some(node) if node.tag != "none" => self.dispatch(node)?,
                _ => Vec::new(),
            };
            let presup = match node.children.get(prescripts_pos + 2) {
                Some(node) if node.tag != "none" => self.dispatch(node)?,
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
                    Some(node) if node.tag != "none" => self.dispatch(node)?,
                    _ => Vec::new(),
                };
                let post_sup = match node.children.get(2) {
                    Some(node) if prescripts_pos > 2 && node.tag != "none" => {
                        self.dispatch(node)?
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
            Some(node) if node.tag != "none" => self.dispatch(node)?,
            _ => Vec::new(),
        };
        let sup = match node.children.get(2) {
            Some(node) if node.tag != "none" => self.dispatch(node)?,
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
        let rounded = notation.contains("roundedbox");
        let all_sides = notation.contains("box") || rounded;
        let actuarial = notation.contains("actuarial");
        Ok(boxed(
            &inner,
            rounded,
            all_sides || notation.contains("left"),
            all_sides || actuarial || notation.contains("right"),
            all_sides || actuarial || notation.contains("top"),
            all_sides || notation.contains("bottom"),
        ))
    }

    pub(super) fn encode_mtable(&mut self, node: &MathNode) -> Result<Vec<u8>> {
        let nrows = node.children.len();
        if nrows == 0 {
            return Ok(Vec::new());
        }
        let ncols = node
            .children
            .iter()
            .map(|mtr| mtr.children.len())
            .max()
            .unwrap_or(0);
        if ncols <= 1 {
            if nrows == 2 {
                let row1_cell = node.children[1]
                    .children
                    .first()
                    .unwrap_or(&node.children[1]);
                if row1_cell.children.len() == 1
                    && row1_cell.children[0].tag == "menclose"
                    && row1_cell.children[0]
                        .attr("notation")
                        .unwrap_or("")
                        .contains("longdiv")
                {
                    let row0_cell = node.children[0]
                        .children
                        .first()
                        .unwrap_or(&node.children[0]);
                    let quotient = self.encode_children(row0_cell)?;
                    let dividend = self.encode_children(&row1_cell.children[0])?;
                    return Ok(long_division(
                        &dividend,
                        (!quotient.is_empty()).then_some(quotient.as_slice()),
                    ));
                }
            }
            let mut encoded_rows = Vec::with_capacity(nrows);
            for mtr in &node.children {
                encoded_rows.push(match mtr.children.first() {
                    Some(mtd) => self.encode_children(mtd)?,
                    None => self.encode_children(mtr)?,
                });
            }
            if nrows == 1 {
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
        let mut encoded_cells = Vec::with_capacity(nrows * ncols);
        for mtr in &node.children {
            for col_idx in 0..ncols {
                encoded_cells.push(match mtr.children.get(col_idx) {
                    Some(mtd) => self.encode_children(mtd)?,
                    None => Vec::new(),
                });
            }
        }
        let cell_refs: Vec<&[u8]> = encoded_cells.iter().map(Vec::as_slice).collect();
        let h_just = match node.attr("columnalign") {
            Some("left") => HALIGN_LEFT,
            Some("right") => HALIGN_RIGHT,
            _ => HALIGN_CENTER,
        };
        Ok(matrix(
            VALIGN_CENTER_BASELINE,
            h_just,
            VALIGN_CENTER_BASELINE,
            nrows.min(u8::MAX as usize) as u8,
            ncols.min(u8::MAX as usize) as u8,
            &cell_refs,
        ))
    }
}
