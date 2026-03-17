use std::ops::Index;

use crate::cell::{Cell, CellWidth};
use crate::damage::{DamageRegion, DamageTracker};
use crate::error::{Error, Result};

/// The dimensions of the visible terminal grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridSize {
    /// Number of rows in the visible grid.
    pub rows: usize,
    /// Number of columns in the visible grid.
    pub cols: usize,
}

impl Default for GridSize {
    fn default() -> Self {
        Self { rows: 24, cols: 80 }
    }
}

/// A pre-allocated visible grid of terminal cells.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grid {
    size: GridSize,
    cells: Vec<Cell>,
    damage: DamageTracker,
}

impl Grid {
    /// Creates a grid with the requested dimensions.
    pub fn new(size: GridSize) -> Result<Self> {
        let cell_count = size
            .rows
            .checked_mul(size.cols)
            .ok_or_else(|| Error::ResizeFailed {
                reason: "requested grid size overflows allocation".to_string(),
            })?;

        Ok(Self {
            size,
            cells: vec![Cell::default(); cell_count],
            damage: DamageTracker::new(size.rows),
        })
    }

    /// Returns the current grid size.
    #[must_use]
    pub const fn size(&self) -> GridSize {
        self.size
    }

    /// Returns the number of rows.
    #[must_use]
    pub const fn rows(&self) -> usize {
        self.size.rows
    }

    /// Returns the number of columns.
    #[must_use]
    pub const fn cols(&self) -> usize {
        self.size.cols
    }

    /// Returns a reference to the cell at the requested position.
    #[must_use]
    pub fn cell(&self, row: usize, col: usize) -> Option<&Cell> {
        self.index_of(row, col)
            .and_then(|index| self.cells.get(index))
    }

    /// Writes a cell to a visible position and records the damaged columns.
    pub fn write(&mut self, row: usize, col: usize, cell: Cell) -> Result<()> {
        let index = self.checked_index(row, col)?;
        self.clear_wide_span_at(row, col);

        let mut cell = cell;
        if cell.width == CellWidth::Double {
            if col + 1 < self.cols() {
                self.clear_wide_span_at(row, col + 1);
            } else {
                cell.width = CellWidth::Single;
            }
        }

        self.cells[index] = cell;
        self.damage.mark(row, col);

        if cell.width == CellWidth::Double && col + 1 < self.cols() {
            let continuation_index = self.checked_index(row, col + 1)?;
            self.cells[continuation_index] = Cell::continuation(cell.attrs);
            self.damage.mark(row, col + 1);
        }

        Ok(())
    }

    /// Returns a row slice when the row exists.
    #[must_use]
    pub fn row(&self, row: usize) -> Option<&[Cell]> {
        if row >= self.rows() {
            return None;
        }

        let start = row.saturating_mul(self.cols());
        let end = start + self.cols();
        self.cells.get(start..end)
    }

    /// Clears a single row.
    pub fn clear_row(&mut self, row: usize) -> Result<()> {
        if row >= self.rows() {
            return Err(self.invalid_position(row, 0));
        }

        let start = row.saturating_mul(self.cols());
        let end = start + self.cols();
        self.cells[start..end].fill(Cell::default());
        self.damage.mark_row(row, self.cols());
        Ok(())
    }

    /// Clears the entire visible grid.
    pub fn clear(&mut self) {
        self.cells.fill(Cell::default());
        self.damage.mark_all();
    }

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

    /// Returns and clears accumulated damage.
    pub fn take_damage(&mut self) -> Vec<DamageRegion> {
        self.damage.take(self.cols())
    }

    /// Marks the entire visible grid as damaged without modifying cell data.
    pub fn mark_all_damage(&mut self) {
        self.damage.mark_all();
    }

    fn clear_wide_span_at(&mut self, row: usize, col: usize) {
        if let Some(index) = self.index_of(row, col) {
            match self.cells[index].width {
                CellWidth::Double => {
                    self.cells[index] = Cell::default();
                    self.damage.mark(row, col);
                    if col + 1 < self.cols() {
                        let continuation_index = index + 1;
                        self.cells[continuation_index] = Cell::default();
                        self.damage.mark(row, col + 1);
                    }
                }
                CellWidth::Continuation => {
                    self.cells[index] = Cell::default();
                    self.damage.mark(row, col);
                    if col > 0 {
                        let previous_index = index - 1;
                        if self.cells[previous_index].width == CellWidth::Double {
                            self.cells[previous_index] = Cell::default();
                            self.damage.mark(row, col - 1);
                        }
                    }
                }
                CellWidth::Single => {}
            }
        }
    }

    fn checked_index(&self, row: usize, col: usize) -> Result<usize> {
        self.index_of(row, col)
            .ok_or_else(|| self.invalid_position(row, col))
    }

    fn index_of(&self, row: usize, col: usize) -> Option<usize> {
        if row < self.rows() && col < self.cols() {
            Some(row * self.cols() + col)
        } else {
            None
        }
    }

    fn invalid_position(&self, row: usize, col: usize) -> Error {
        Error::InvalidPosition {
            row,
            col,
            rows: self.rows(),
            cols: self.cols(),
        }
    }
}

impl Index<usize> for Grid {
    type Output = [Cell];

    fn index(&self, row: usize) -> &Self::Output {
        let start = row * self.cols();
        let end = start + self.cols();
        &self.cells[start..end]
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::{Cell, CellWidth, Grid, GridSize};
    use crate::damage::DamageRegion;

    #[test]
    fn grid_write_updates_damage() {
        let mut grid = Grid::new(GridSize { rows: 3, cols: 4 }).unwrap();
        grid.write(1, 2, Cell::new('A')).unwrap();
        assert_eq!(grid.cell(1, 2), Some(&Cell::new('A')));
        assert_eq!(grid.take_damage(), vec![DamageRegion::new(1, 1, 2, 2)]);
    }

    #[test]
    fn grid_scroll_moves_content() {
        let mut grid = Grid::new(GridSize { rows: 3, cols: 4 }).unwrap();
        grid.write(0, 0, Cell::new('A')).unwrap();
        grid.write(1, 0, Cell::new('B')).unwrap();
        grid.write(2, 0, Cell::new('C')).unwrap();

        grid.scroll_up(1);

        assert_eq!(grid.cell(0, 0), Some(&Cell::new('B')));
        assert_eq!(grid.cell(1, 0), Some(&Cell::new('C')));
        assert_eq!(grid.cell(2, 0), Some(&Cell::default()));
    }

    #[test]
    fn grid_resize_preserves_content() {
        let mut grid = Grid::new(GridSize { rows: 2, cols: 2 }).unwrap();
        grid.write(0, 0, Cell::new('X')).unwrap();
        grid.write(1, 1, Cell::new('Y')).unwrap();

        grid.resize(GridSize { rows: 3, cols: 4 }).unwrap();

        assert_eq!(grid.cell(0, 0), Some(&Cell::new('X')));
        assert_eq!(grid.cell(1, 1), Some(&Cell::new('Y')));
        assert_eq!(grid.rows(), 3);
        assert_eq!(grid.cols(), 4);
    }

    #[test]
    fn grid_downgrades_wide_cells_at_right_edge() {
        let mut grid = Grid::new(GridSize { rows: 1, cols: 1 }).unwrap();
        grid.write(0, 0, Cell::new('中')).unwrap();
        assert_eq!(grid.cell(0, 0).unwrap().width, CellWidth::Single);
    }

    #[test]
    fn grid_clears_overwritten_wide_cell_spans() {
        let mut grid = Grid::new(GridSize { rows: 1, cols: 3 }).unwrap();
        grid.write(0, 1, Cell::new('中')).unwrap();
        grid.take_damage();

        grid.write(0, 0, Cell::new('中')).unwrap();

        assert_eq!(grid.cell(0, 0).unwrap().width, CellWidth::Double);
        assert_eq!(grid.cell(0, 1).unwrap().width, CellWidth::Continuation);
        assert_eq!(grid.cell(0, 2).unwrap(), &Cell::default());
        assert_eq!(grid.take_damage(), vec![DamageRegion::new(0, 0, 0, 2)]);
    }
}
