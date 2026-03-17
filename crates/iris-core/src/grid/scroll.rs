use super::Grid;
use crate::cell::Cell;
use crate::error::Result;

impl Grid {
    /// Scrolls the grid upward by the requested number of rows.
    pub fn scroll_up(&mut self, count: usize) {
        let rows = self.rows();
        let cols = self.cols();
        if rows == 0 || cols == 0 {
            return;
        }

        let shift = count.min(rows);
        if shift == 0 {
            return;
        }

        let shift_cells = shift.saturating_mul(cols);
        self.cells.copy_within(shift_cells.., 0);
        let clear_start = (rows - shift).saturating_mul(cols);
        self.cells[clear_start..].fill(Cell::default());
        self.damage.mark_all();
    }

    /// Scrolls the grid downward by the requested number of rows.
    pub fn scroll_down(&mut self, count: usize) {
        let rows = self.rows();
        let cols = self.cols();
        if rows == 0 || cols == 0 {
            return;
        }

        let shift = count.min(rows);
        if shift == 0 {
            return;
        }

        let visible_cells = (rows - shift).saturating_mul(cols);
        self.cells
            .copy_within(0..visible_cells, shift.saturating_mul(cols));
        let clear_end = shift.saturating_mul(cols);
        self.cells[..clear_end].fill(Cell::default());
        self.damage.mark_all();
    }

    /// Scrolls an inclusive row range upward by the requested number of rows.
    pub fn scroll_up_range(&mut self, top: usize, bottom: usize, count: usize) -> Result<()> {
        self.validate_row_range(top, bottom)?;

        let cols = self.cols();
        if cols == 0 {
            return Ok(());
        }

        let region_rows = bottom - top + 1;
        let shift = count.min(region_rows);
        if shift == 0 {
            return Ok(());
        }

        let top_start = top * cols;
        let bottom_end = (bottom + 1) * cols;

        if shift < region_rows {
            let source_start = (top + shift) * cols;
            self.cells.copy_within(source_start..bottom_end, top_start);
        }

        let clear_start = (bottom + 1 - shift) * cols;
        self.cells[clear_start..bottom_end].fill(Cell::default());

        for row in top..=bottom {
            self.damage.mark_row(row, cols);
        }

        Ok(())
    }

    /// Scrolls an inclusive row range downward by the requested number of rows.
    pub fn scroll_down_range(&mut self, top: usize, bottom: usize, count: usize) -> Result<()> {
        self.validate_row_range(top, bottom)?;

        let cols = self.cols();
        if cols == 0 {
            return Ok(());
        }

        let region_rows = bottom - top + 1;
        let shift = count.min(region_rows);
        if shift == 0 {
            return Ok(());
        }

        let top_start = top * cols;

        if shift < region_rows {
            let source_end = (bottom + 1 - shift) * cols;
            let destination_start = (top + shift) * cols;
            self.cells
                .copy_within(top_start..source_end, destination_start);
        }

        let clear_end = (top + shift) * cols;
        self.cells[top_start..clear_end].fill(Cell::default());

        for row in top..=bottom {
            self.damage.mark_row(row, cols);
        }

        Ok(())
    }

    /// Inserts blank cells in a row, shifting existing cells rightward.
    pub fn insert_blank_cells(&mut self, row: usize, col: usize, count: usize) -> Result<()> {
        let cols = self.cols();
        if cols == 0 {
            return Ok(());
        }
        let start = self.checked_index(row, col)?;

        let shift = count.min(cols.saturating_sub(col));
        if shift == 0 {
            return Ok(());
        }

        let row_end = (row + 1) * cols;
        if shift < cols - col {
            self.cells
                .copy_within(start..(row_end - shift), start + shift);
        }
        self.cells[start..(start + shift)].fill(Cell::default());
        self.normalize_row(row);
        self.damage.mark_row(row, cols);
        Ok(())
    }

    /// Deletes cells from a row, shifting trailing cells leftward.
    pub fn delete_cells(&mut self, row: usize, col: usize, count: usize) -> Result<()> {
        let cols = self.cols();
        if cols == 0 {
            return Ok(());
        }
        let start = self.checked_index(row, col)?;

        let shift = count.min(cols.saturating_sub(col));
        if shift == 0 {
            return Ok(());
        }

        let row_end = (row + 1) * cols;
        if shift < cols - col {
            self.cells.copy_within((start + shift)..row_end, start);
        }
        self.cells[(row_end - shift)..row_end].fill(Cell::default());
        self.normalize_row(row);
        self.damage.mark_row(row, cols);
        Ok(())
    }
}
