use super::{Grid, GridSize};
use crate::cell::{Cell, CellWidth};
use crate::error::{Error, Result};

impl Grid {
    /// Resizes the grid, preserving the overlapping top-left content.
    pub fn resize(&mut self, new_size: GridSize) -> Result<()> {
        let cell_count =
            new_size
                .rows
                .checked_mul(new_size.cols)
                .ok_or_else(|| Error::ResizeFailed {
                    reason: "requested grid size overflows allocation".to_string(),
                })?;
        if new_size.cols == 0 {
            self.size = new_size;
            self.cells = vec![Cell::default(); cell_count];
            self.damage.resize(new_size.rows);
            return Ok(());
        }

        let mut new_cells = vec![Cell::default(); cell_count];
        let preserved_rows = self.rows().min(new_size.rows);
        let preserved_cols = self.cols().min(new_size.cols);

        for row in 0..preserved_rows {
            let old_start = row * self.cols();
            let new_start = row * new_size.cols;
            for col in 0..preserved_cols {
                let mut cell = self.cells[old_start + col];
                if cell.width == CellWidth::Continuation {
                    let has_leader =
                        col > 0 && self.cells[old_start + col - 1].width == CellWidth::Double;
                    if !has_leader {
                        cell = Cell::default();
                    }
                }
                if cell.width == CellWidth::Double && col + 1 >= new_size.cols {
                    cell.width = CellWidth::Single;
                }
                new_cells[new_start + col] = cell;
            }
        }

        self.size = new_size;
        self.cells = new_cells;
        self.damage.resize(new_size.rows);
        Ok(())
    }
}
