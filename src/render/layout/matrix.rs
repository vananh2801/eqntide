//! Matrix templates (`mtable`).

use super::*;


impl Layout {
    pub(super) fn table(&mut self, n: &MathNode, em: f32, script: bool) -> Result<LBox> {
        let mut rows: Vec<Vec<LBox>> = Vec::new();
        for mtr in &n.children {
            let mut cells = Vec::new();
            for mtd in &mtr.children {
                cells.push(self.row(&mtd.children, em, script)?);
            }
            rows.push(cells);
        }
        if rows.is_empty() {
            return Ok(LBox::default());
        }
        let ncols = rows.iter().map(Vec::len).max().unwrap_or(0);
        let mut col_w = vec![0.0f32; ncols];
        for r in &rows {
            for (j, c) in r.iter().enumerate() {
                col_w[j] = col_w[j].max(c.w);
            }
        }
        let left_align = ncols == 1;
        let col_gap = 1.0 * em;
        let row_gap = 0.35 * em;
        let heights: Vec<(f32, f32)> = rows
            .iter()
            .map(|r| {
                let a = r.iter().map(|c| c.asc).fold(0.0f32, f32::max).max(0.7 * em);
                let d = r.iter().map(|c| c.desc).fold(0.0f32, f32::max).max(0.2 * em);
                (a, d)
            })
            .collect();
        let total: f32 = heights.iter().map(|(a, d)| a + d).sum::<f32>() + row_gap * (rows.len() - 1) as f32;
        let mut y = -AXIS * em - total / 2.0;
        let mut b = LBox::default();
        for (r, (a, d)) in rows.iter().zip(&heights) {
            y += a;
            let mut x = 0.0;
            for (j, c) in r.iter().enumerate() {
                let dx = if left_align { 0.0 } else { (col_w[j] - c.w) / 2.0 };
                b.place(c, x + dx, y);
                x += col_w[j] + col_gap;
            }
            y += d + row_gap;
        }
        b.w = col_w.iter().sum::<f32>() + col_gap * (ncols.saturating_sub(1)) as f32;
        b.asc = b.asc.max(total / 2.0 + AXIS * em);
        b.desc = b.desc.max(total / 2.0 - AXIS * em);
        Ok(b)
    }
}
