//! Matrix templates (`mtable`).

use super::*;

impl Layout {
    /// Lay out an `<mtable>` element with column widths, row heights, and alignment.
    pub(super) fn table(&mut self, node: &MathNode, em: f32, ctx: MathContext) -> Result<LBox> {
        let cell_ctx = if ctx.is_script() {
            ctx
        } else {
            MathContext::TEXT
        };
        let mut rows: Vec<Vec<LBox>> = Vec::new();
        for mtr in &node.children {
            let mut cells = Vec::new();
            for mtd in &mtr.children {
                let mut cell = self.row(&mtd.children, em, cell_ctx)?;
                cell.mark(mtd);
                cells.push(cell);
            }
            rows.push(cells);
        }
        if rows.is_empty() {
            return Ok(LBox::default());
        }
        let col_count = rows.iter().map(Vec::len).max().unwrap_or(0);
        let mut col_widths = vec![0.0f32; col_count];
        for row_cells in &rows {
            for (j, cell) in row_cells.iter().enumerate() {
                col_widths[j] = col_widths[j].max(cell.w);
            }
        }
        let col_gap = 1.0 * em;
        let row_gap = 0.35 * em;
        let heights: Vec<(f32, f32)> = rows
            .iter()
            .map(|row_cells| {
                let asc = row_cells
                    .iter()
                    .map(|cell| cell.asc)
                    .fold(0.0f32, f32::max)
                    .max(0.7 * em);
                let desc = row_cells
                    .iter()
                    .map(|cell| cell.desc)
                    .fold(0.0f32, f32::max)
                    .max(0.2 * em);
                (asc, desc)
            })
            .collect();
        let total_height: f32 = heights.iter().map(|(asc, desc)| asc + desc).sum::<f32>()
            + row_gap * (rows.len() - 1) as f32;
        let axis = self.font_set.constants().axis_height * em;
        let mut y = -axis - total_height / 2.0;
        let mut layout_box = LBox::default();
        for ((row_cells, (row_asc, row_desc)), source_row) in
            rows.iter().zip(&heights).zip(&node.children)
        {
            y += row_asc;
            let mut x = 0.0;
            for (j, cell) in row_cells.iter().enumerate() {
                let align = source_row
                    .children
                    .get(j)
                    .and_then(|cell_node| cell_node.attr("columnalign"))
                    .or_else(|| source_row.attr("columnalign"))
                    .or_else(|| node.attr("columnalign"));
                let align = align.and_then(|attr_val| {
                    attr_val
                        .split_whitespace()
                        .nth(j)
                        .or_else(|| attr_val.split_whitespace().last())
                });
                let dx = match align {
                    Some("left") => 0.0,
                    Some("right") => col_widths[j] - cell.w,
                    Some("center") => (col_widths[j] - cell.w) / 2.0,
                    _ if col_count == 1 => 0.0,
                    _ => (col_widths[j] - cell.w) / 2.0,
                };
                layout_box.place(cell, x + dx, y);
                x += col_widths[j] + col_gap;
            }
            y += row_desc + row_gap;
        }
        layout_box.w =
            col_widths.iter().sum::<f32>() + col_gap * (col_count.saturating_sub(1)) as f32;
        layout_box.asc = layout_box.asc.max(total_height / 2.0 + axis);
        layout_box.desc = layout_box.desc.max(total_height / 2.0 - axis);
        Ok(layout_box)
    }
}
