use super::Grid;
use crate::cell::{Cell, CellAttrs, CellWidth};
use crate::error::{validate_printable_ascii, Result};

impl Grid {
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

    /// Writes a contiguous ASCII run into a single row using single-width cells.
    pub fn write_ascii_run(
        &mut self,
        row: usize,
        col: usize,
        bytes: &[u8],
        attrs: CellAttrs,
    ) -> Result<()> {
        if bytes.is_empty() {
            return Ok(());
        }

        validate_printable_ascii(bytes)?;

        if row >= self.rows() || col >= self.cols() || col + bytes.len() > self.cols() {
            return Err(self.invalid_position(row, col));
        }

        let start = row * self.cols() + col;
        let end = start + bytes.len();
        let mut requires_cleanup = false;

        for offset in 0..bytes.len() {
            if self.cells[start + offset].width != CellWidth::Single {
                requires_cleanup = true;
                break;
            }
        }

        if requires_cleanup {
            for offset in 0..bytes.len() {
                self.clear_wide_span_at(row, col + offset);
            }
        }

        for (cell, &byte) in self.cells[start..end].iter_mut().zip(bytes) {
            *cell = Cell {
                character: char::from(byte),
                width: CellWidth::Single,
                attrs,
            };
        }

        self.damage.mark_range(row, col, col + bytes.len() - 1);
        Ok(())
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

    pub(super) fn clear_wide_span_at(&mut self, row: usize, col: usize) {
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

    pub(super) fn normalize_row(&mut self, row: usize) {
        let cols = self.cols();
        if row >= self.rows() || cols == 0 {
            return;
        }

        let start = row * cols;
        for col in 0..cols {
            let index = start + col;
            match self.cells[index].width {
                CellWidth::Single => {}
                CellWidth::Continuation => {
                    let has_leader = col > 0 && self.cells[index - 1].width == CellWidth::Double;
                    if !has_leader {
                        self.cells[index] = Cell::default();
                    }
                }
                CellWidth::Double => {
                    if col + 1 >= cols {
                        self.cells[index].width = CellWidth::Single;
                    } else {
                        self.cells[index + 1] = Cell::continuation(self.cells[index].attrs);
                    }
                }
            }
        }
    }
}
