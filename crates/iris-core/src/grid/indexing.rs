use super::Grid;
use crate::error::{Error, Result};

impl Grid {
    pub(super) fn checked_index(&self, row: usize, col: usize) -> Result<usize> {
        self.index_of(row, col)
            .ok_or_else(|| self.invalid_position(row, col))
    }

    pub(super) fn index_of(&self, row: usize, col: usize) -> Option<usize> {
        if row < self.rows() && col < self.cols() {
            Some(row * self.cols() + col)
        } else {
            None
        }
    }

    pub(super) fn invalid_position(&self, row: usize, col: usize) -> Error {
        Error::InvalidPosition {
            row,
            col,
            rows: self.rows(),
            cols: self.cols(),
        }
    }

    pub(super) fn validate_row_range(&self, top: usize, bottom: usize) -> Result<()> {
        if top > bottom {
            return Err(self.invalid_position(top, 0));
        }

        if bottom >= self.rows() {
            return Err(self.invalid_position(bottom, 0));
        }

        Ok(())
    }
}
