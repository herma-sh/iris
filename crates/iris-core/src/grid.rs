use std::ops::{Index, IndexMut};

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
    /// Creates a GridSize with the default terminal dimensions of 24 rows and 80 columns.
    ///
    /// # Examples
    ///
    /// ```
    /// let size = GridSize::default();
    /// assert_eq!(size.rows, 24);
    /// assert_eq!(size.cols, 80);
    /// ```
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
    /// Constructs a new Grid pre-filled with default cells for the given size.
    ///
    /// # Examples
    ///
    /// ```
    /// let size = GridSize { rows: 3, cols: 4 };
    /// let grid = Grid::new(size);
    /// assert_eq!(grid.size(), size);
    /// assert_eq!(grid.rows(), 3);
    /// assert_eq!(grid.cols(), 4);
    /// ```
    #[must_use]
    pub fn new(size: GridSize) -> Self {
        let cell_count = size.rows.saturating_mul(size.cols);
        Self {
            size,
            cells: vec![Cell::default(); cell_count],
            damage: DamageTracker::new(size.rows),
        }
    }

    /// Return the grid's dimensions as a GridSize.
    ///
    /// # Examples
    ///
    /// ```
    /// let size = GridSize { rows: 2, cols: 3 };
    /// let grid = Grid::new(size);
    /// assert_eq!(grid.size(), size);
    /// ```
    #[must_use]
    pub const fn size(&self) -> GridSize {
        self.size
    }

    /// Get the current grid height in rows.
    ///
    /// # Returns
    ///
    /// The number of rows in the grid.
    ///
    /// # Examples
    ///
    /// ```
    /// let size = GridSize::default();
    /// let grid = Grid::new(size);
    /// let r = grid.rows();
    /// assert_eq!(r, size.rows);
    /// ```
    #[must_use]
    pub const fn rows(&self) -> usize {
        self.size.rows
    }

    /// Get the current number of columns in the grid.
    ///
    /// # Examples
    ///
    /// ```
    /// let g = Grid::new(GridSize { rows: 2, cols: 4 });
    /// assert_eq!(g.cols(), 4);
    /// ```
    ///
    /// Returns the number of columns.
    #[must_use]
    pub const fn cols(&self) -> usize {
        self.size.cols
    }

    /// Gets a reference to the cell at the specified row and column.
    ///
    /// Returns `Some(&Cell)` when the position is within the grid bounds, or `None` if the position is out of range.
    ///
    /// # Examples
    ///
    /// ```
    /// let size = GridSize { rows: 2, cols: 2 };
    /// let grid = Grid::new(size);
    /// assert!(grid.cell(0, 0).is_some());
    /// assert!(grid.cell(10, 10).is_none());
    /// ```
    pub fn cell(&self, row: usize, col: usize) -> Option<&Cell> {
        self.index_of(row, col)
            .and_then(|index| self.cells.get(index))
    }

    /// Get a mutable reference to the cell at the specified row and column.
    ///
    /// Returns `Some(&mut Cell)` when the position is within the grid bounds, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut grid = Grid::new(GridSize { rows: 2, cols: 2 });
    /// assert!(grid.cell_mut(0, 1).is_some());
    /// assert!(grid.cell_mut(2, 0).is_none());
    /// ```
    pub fn cell_mut(&mut self, row: usize, col: usize) -> Option<&mut Cell> {
        self.index_of(row, col)
            .and_then(|index| self.cells.get_mut(index))
    }

    /// Writes `cell` into the grid at the specified position and marks affected columns as damaged.
    ///
    /// The function clears any existing wide-character continuation that overlaps the target cell,
    /// stores the given `cell`, and marks its column in the damage tracker. If `cell` is double-width
    /// and there is room to the right, a continuation cell is written at `col + 1` and that column is
    /// also marked as damaged.
    ///
    /// # Errors
    ///
    /// Returns an error if the `(row, col)` (or the continuation position when applicable) is outside
    /// the grid bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut grid = Grid::new(GridSize { rows: 3, cols: 4 });
    /// let cell = Cell::with_char('好'); // double-width character
    /// grid.write(0, 0, cell).unwrap();
    /// // column 0 and 1 of row 0 are marked damaged and contain the cell + continuation
    /// ```
    pub fn write(&mut self, row: usize, col: usize, cell: Cell) -> Result<()> {
        let index = self.checked_index(row, col)?;
        self.clear_wide_span_at(row, col);

        self.cells[index] = cell;
        self.damage.mark(row, col);

        if cell.width == CellWidth::Double && col + 1 < self.cols() {
            let continuation_index = self.checked_index(row, col + 1)?;
            self.cells[continuation_index] = Cell::continuation(cell.attrs);
            self.damage.mark(row, col + 1);
        }

        Ok(())
    }

    /// Access the cells of a specific row as a slice.
    ///
    /// Returns `Some(&[Cell])` containing the row's cells if `row` is within bounds, or `None` if `row` is out of range.
    ///
    /// # Examples
    ///
    /// ```
    /// let size = GridSize { rows: 2, cols: 3 };
    /// let grid = Grid::new(size);
    /// if let Some(row) = grid.row(1) {
    ///     assert_eq!(row.len(), 3);
    /// } else {
    ///     panic!("expected row 1 to exist");
    /// }
    /// ```
    pub fn row(&self, row: usize) -> Option<&[Cell]> {
        if row >= self.rows() {
            return None;
        }

        let start = row.saturating_mul(self.cols());
        let end = start + self.cols();
        self.cells.get(start..end)
    }

    /// Clears all cells in the specified row and marks that row as damaged.
    ///
    /// Returns an error if `row` is outside the grid's bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut grid = Grid::new(GridSize { rows: 2, cols: 3 });
    /// // ensure row 0 is cleared
    /// grid.clear_row(0).unwrap();
    /// assert!(grid.row(0).unwrap().iter().all(|c| *c == Cell::default()));
    /// ```
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

    /// Clears all cells in the visible grid and marks the entire grid as damaged.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut grid = Grid::new(GridSize { rows: 2, cols: 3 });
    /// // write a non-default cell
    /// grid.write(0, 0, Cell::default()).unwrap();
    /// // clear the grid
    /// grid.clear();
    /// // every position becomes the default cell
    /// for r in 0..grid.rows() {
    ///     for c in 0..grid.cols() {
    ///         assert_eq!(grid.cell(r, c), Some(&Cell::default()));
    ///     }
    /// }
    /// ```
    pub fn clear(&mut self) {
        self.cells.fill(Cell::default());
        self.damage.mark_all();
    }

    /// Shifts the grid content upward by the specified number of rows, clearing the newly exposed bottom rows.
    ///
    /// If `count` is greater than the number of rows the grid contains, the grid is cleared.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut grid = Grid::new(GridSize { rows: 3, cols: 2 });
    /// // Populate or modify grid...
    /// grid.scroll_up(1);
    /// ```
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

    /// Scrolls the grid content downward by the given number of rows.
    ///
    /// Content is moved toward increasing row indices; newly exposed top rows are
    /// cleared and the grid's damage tracker is marked for a full redraw.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut grid = Grid::new(GridSize { rows: 2, cols: 2 });
    /// // scrolling an empty grid preserves dimensions and clears the newly exposed rows
    /// grid.scroll_down(1);
    /// assert_eq!(grid.rows(), 2);
    /// assert_eq!(grid.cols(), 2);
    /// ```
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

    /// Resize the grid to `new_size`, preserving the overlapping top-left region of cells.
    ///
    /// If `new_size` is larger than the current size, newly exposed cells are set to `Cell::default()`.
    /// The top-left min(rows, cols) rectangle from the old grid is copied into the new grid.
    ///
    /// # Errors
    ///
    /// Returns `Err(Error::ResizeFailed)` when the requested allocation size would overflow.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut grid = Grid::new(GridSize { rows: 2, cols: 2 });
    /// grid.write(0, 0, Cell::from('A')).unwrap();
    /// grid.write(1, 1, Cell::from('B')).unwrap();
    ///
    /// grid.resize(GridSize { rows: 3, cols: 4 }).unwrap();
    ///
    /// assert_eq!(grid.rows(), 3);
    /// assert_eq!(grid.cols(), 4);
    /// assert_eq!(grid.cell(0, 0).unwrap().c, 'A');
    /// assert_eq!(grid.cell(1, 1).unwrap().c, 'B');
    /// ```
    pub fn resize(&mut self, new_size: GridSize) -> Result<()> {
        if new_size.rows > usize::MAX / new_size.cols.max(1) {
            return Err(Error::ResizeFailed {
                reason: "requested grid size overflows allocation".to_string(),
            });
        }

        let mut new_cells = vec![Cell::default(); new_size.rows.saturating_mul(new_size.cols)];
        let preserved_rows = self.rows().min(new_size.rows);
        let preserved_cols = self.cols().min(new_size.cols);

        for row in 0..preserved_rows {
            let old_start = row * self.cols();
            let new_start = row * new_size.cols;
            let old_end = old_start + preserved_cols;
            let new_end = new_start + preserved_cols;
            new_cells[new_start..new_end].copy_from_slice(&self.cells[old_start..old_end]);
        }

        self.size = new_size;
        self.cells = new_cells;
        self.damage.resize(new_size.rows);
        Ok(())
    }

    /// Retrieve and clear the accumulated damage regions for the grid.
    ///
    /// The returned vector contains the damage regions recorded since the last
    /// call; after this call the internal damage tracker is cleared.
    ///
    /// # Examples
    ///
    /// ```
    /// let size = GridSize { rows: 2, cols: 2 };
    /// let mut grid = Grid::new(size);
    /// // cause some damage (e.g., clear marks the entire grid as damaged)
    /// grid.clear();
    /// let regions = grid.take_damage();
    /// assert!(!regions.is_empty());
    /// ```
    pub fn take_damage(&mut self) -> Vec<DamageRegion> {
        self.damage.take(self.cols())
    }

    /// Clears any double-width continuation cells that overlap the given position.
    ///
    /// If the cell at (row, col) is the leading half of a double-width cell, its
    /// right-side continuation (at column `col + 1`) is reset to `Cell::default()`.
    /// If the cell at (row, col) is the trailing half of a double-width cell, the
    /// leading half (at column `col - 1`) is reset to `Cell::default()`.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut grid = Grid::new(GridSize { rows: 1, cols: 2 });
    /// let mut wide = Cell::default();
    /// wide.width = CellWidth::Double;
    /// grid.write(0, 0, wide).unwrap();
    /// // The continuation cell at (0,1) exists after the write.
    /// grid.clear_wide_span_at(0, 0);
    /// assert_eq!(grid.cell(0, 1), Some(&Cell::default()));
    /// ```
    fn clear_wide_span_at(&mut self, row: usize, col: usize) {
        if let Some(index) = self.index_of(row, col) {
            if self.cells[index].width == CellWidth::Double && col + 1 < self.cols() {
                let continuation_index = index + 1;
                self.cells[continuation_index] = Cell::default();
            }
        }

        if col > 0 {
            if let Some(previous_index) = self.index_of(row, col - 1) {
                if self.cells[previous_index].width == CellWidth::Double {
                    self.cells[previous_index] = Cell::default();
                }
            }
        }
    }

    /// Compute the linear buffer index for the cell at (row, col) if the position is valid.
    ///
    /// # Errors
    ///
    /// Returns an `Error::InvalidPosition` when `row` or `col` are out of the grid bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// let size = GridSize { rows: 2, cols: 3 };
    /// let grid = Grid::new(size);
    /// let idx = grid.checked_index(1, 2).unwrap();
    /// assert_eq!(idx, 1 * 3 + 2);
    /// ```
    fn checked_index(&self, row: usize, col: usize) -> Result<usize> {
        self.index_of(row, col)
            .ok_or_else(|| self.invalid_position(row, col))
    }

    /// Compute the linear (row-major) buffer index for the cell at `(row, col)`.
    ///
    /// Returns `Some(index)` when `(row, col)` is inside the grid bounds, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// let size = GridSize { rows: 3, cols: 4 };
    /// let grid = Grid::new(size);
    /// assert_eq!(grid.index_of(0, 0), Some(0));
    /// assert_eq!(grid.index_of(1, 2), Some(1 * 4 + 2));
    /// assert_eq!(grid.index_of(3, 0), None); // row out of bounds
    /// ```
    fn index_of(&self, row: usize, col: usize) -> Option<usize> {
        if row < self.rows() && col < self.cols() {
            Some(row * self.cols() + col)
        } else {
            None
        }
    }

    /// Constructs an `Error::InvalidPosition` for the given coordinate using the grid's current bounds.
    ///
    ///
    /// # Returns
    ///
    /// An `Error::InvalidPosition` containing the attempted `row` and `col` and the grid's current `rows` and `cols`.
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

    /// Accesses the specified row as a slice of cells.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::{Grid, GridSize};
    ///
    /// let size = GridSize { rows: 2, cols: 3 };
    /// let grid = Grid::new(size);
    /// let row = &grid[0];
    /// assert_eq!(row.len(), 3);
    /// ```
    fn index(&self, row: usize) -> &Self::Output {
        let start = row * self.cols();
        let end = start + self.cols();
        &self.cells[start..end]
    }
}

impl IndexMut<usize> for Grid {
    /// Get a mutable slice of `Cell`s for the specified row.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut grid = Grid::new(GridSize { rows: 1, cols: 2 });
    /// let row = grid.index_mut(0);
    /// assert_eq!(row.len(), 2);
    /// ```
    fn index_mut(&mut self, row: usize) -> &mut Self::Output {
        let start = row * self.cols();
        let end = start + self.cols();
        &mut self.cells[start..end]
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::{Cell, Grid, GridSize};
    use crate::damage::DamageRegion;

    #[test]
    fn grid_write_updates_damage() {
        let mut grid = Grid::new(GridSize { rows: 3, cols: 4 });
        grid.write(1, 2, Cell::new('A')).unwrap();
        assert_eq!(grid.cell(1, 2), Some(&Cell::new('A')));
        assert_eq!(grid.take_damage(), vec![DamageRegion::new(1, 1, 2, 2)]);
    }

    #[test]
    fn grid_scroll_moves_content() {
        let mut grid = Grid::new(GridSize { rows: 3, cols: 4 });
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
        let mut grid = Grid::new(GridSize { rows: 2, cols: 2 });
        grid.write(0, 0, Cell::new('X')).unwrap();
        grid.write(1, 1, Cell::new('Y')).unwrap();

        grid.resize(GridSize { rows: 3, cols: 4 }).unwrap();

        assert_eq!(grid.cell(0, 0), Some(&Cell::new('X')));
        assert_eq!(grid.cell(1, 1), Some(&Cell::new('Y')));
        assert_eq!(grid.rows(), 3);
        assert_eq!(grid.cols(), 4);
    }
}
