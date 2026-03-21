use std::ops::Index;

use crate::cell::Cell;
use crate::damage::{DamageRegion, DamageTracker};
use crate::error::{Error, Result};

mod indexing;
mod resize;
mod scroll;
#[cfg(test)]
mod tests;
mod write;

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

    /// Returns and clears accumulated damage.
    pub fn take_damage(&mut self) -> Vec<DamageRegion> {
        self.damage.take(self.cols())
    }

    /// Restores previously drained visible damage regions back into the tracker.
    pub fn restore_damage(&mut self, damage: &[DamageRegion]) {
        if self.rows() == 0 || self.cols() == 0 {
            return;
        }

        let last_row = self.rows().saturating_sub(1);
        let last_col = self.cols().saturating_sub(1);

        for region in damage {
            if region.start_row > region.end_row
                || region.start_col > region.end_col
                || region.start_row > last_row
                || region.start_col > last_col
            {
                continue;
            }

            let end_row = region.end_row.min(last_row);
            let end_col = region.end_col.min(last_col);
            for row in region.start_row..=end_row {
                self.damage.mark_range(row, region.start_col, end_col);
            }
        }
    }

    /// Marks the entire visible grid as damaged without modifying cell data.
    pub fn mark_all_damage(&mut self) {
        self.damage.mark_all();
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
