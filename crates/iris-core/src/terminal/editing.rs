use crate::cell::{Cell, CellAttrs, CellFlags};
use crate::error::Result;
use crate::parser::GraphicsRendition;

use super::Terminal;

impl Terminal {
    pub(super) fn erase_display(&mut self, mode: u16) -> Result<()> {
        if self.grid.rows() == 0 || self.grid.cols() == 0 {
            return Ok(());
        }

        let row = self
            .cursor
            .position
            .row
            .min(self.grid.rows().saturating_sub(1));
        let col = self
            .cursor
            .position
            .col
            .min(self.grid.cols().saturating_sub(1));

        match mode {
            0 => {
                self.clear_row_range(row, col, self.grid.cols().saturating_sub(1))?;
                for clear_row in (row + 1)..self.grid.rows() {
                    self.grid.clear_row(clear_row)?;
                }
            }
            1 => {
                for clear_row in 0..row {
                    self.grid.clear_row(clear_row)?;
                }
                self.clear_row_range(row, 0, col)?;
            }
            2 | 3 => self.grid.clear(),
            _ => {}
        }

        Ok(())
    }

    pub(super) fn erase_line_mode(&mut self, mode: u16) -> Result<()> {
        if self.grid.rows() == 0 || self.grid.cols() == 0 {
            return Ok(());
        }

        let row = self
            .cursor
            .position
            .row
            .min(self.grid.rows().saturating_sub(1));
        let col = self
            .cursor
            .position
            .col
            .min(self.grid.cols().saturating_sub(1));

        match mode {
            0 => self.clear_row_range(row, col, self.grid.cols().saturating_sub(1))?,
            1 => self.clear_row_range(row, 0, col)?,
            2 => self.grid.clear_row(row)?,
            _ => {}
        }

        Ok(())
    }

    pub(super) fn erase_characters(&mut self, count: u16) -> Result<()> {
        if self.grid.rows() == 0 || self.grid.cols() == 0 {
            return Ok(());
        }

        let row = self
            .cursor
            .position
            .row
            .min(self.grid.rows().saturating_sub(1));
        let start = self
            .cursor
            .position
            .col
            .min(self.grid.cols().saturating_sub(1));
        let end = start
            .saturating_add(usize::from(count.max(1)))
            .saturating_sub(1)
            .min(self.grid.cols().saturating_sub(1));
        self.clear_row_range(row, start, end)
    }

    pub(super) fn insert_characters(&mut self, count: u16) -> Result<()> {
        if self.grid.rows() == 0 || self.grid.cols() == 0 {
            return Ok(());
        }

        let row = self
            .cursor
            .position
            .row
            .min(self.grid.rows().saturating_sub(1));
        let col = self
            .cursor
            .position
            .col
            .min(self.grid.cols().saturating_sub(1));
        self.grid
            .insert_blank_cells(row, col, usize::from(count.max(1)))
    }

    pub(super) fn delete_characters(&mut self, count: u16) -> Result<()> {
        if self.grid.rows() == 0 || self.grid.cols() == 0 {
            return Ok(());
        }

        let row = self
            .cursor
            .position
            .row
            .min(self.grid.rows().saturating_sub(1));
        let col = self
            .cursor
            .position
            .col
            .min(self.grid.cols().saturating_sub(1));
        self.grid.delete_cells(row, col, usize::from(count.max(1)))
    }

    pub(super) fn insert_lines(&mut self, count: u16) -> Result<()> {
        if self.grid.rows() == 0 {
            return Ok(());
        }

        let row = self.cursor.position.row;
        let (top, bottom) = self.active_scroll_region();
        if row < top || row > bottom {
            return Ok(());
        }

        self.grid
            .scroll_down_range(row, bottom, usize::from(count.max(1)))
    }

    pub(super) fn delete_lines(&mut self, count: u16) -> Result<()> {
        if self.grid.rows() == 0 {
            return Ok(());
        }

        let row = self.cursor.position.row;
        let (top, bottom) = self.active_scroll_region();
        if row < top || row > bottom {
            return Ok(());
        }

        self.grid
            .scroll_up_range(row, bottom, usize::from(count.max(1)))
    }

    fn clear_row_range(&mut self, row: usize, start_col: usize, end_col: usize) -> Result<()> {
        if start_col > end_col {
            return Ok(());
        }

        for col in start_col..=end_col {
            self.grid.write(row, col, Cell::default())?;
        }

        Ok(())
    }

    pub(super) fn apply_sgr(&mut self, renditions: &[GraphicsRendition]) {
        for rendition in renditions {
            match *rendition {
                GraphicsRendition::Reset => self.attrs = CellAttrs::default(),
                GraphicsRendition::Bold(enabled) => {
                    self.attrs.flags.set(CellFlags::BOLD, enabled);
                }
                GraphicsRendition::Dim(enabled) => {
                    self.attrs.flags.set(CellFlags::DIM, enabled);
                }
                GraphicsRendition::Italic(enabled) => {
                    self.attrs.flags.set(CellFlags::ITALIC, enabled);
                }
                GraphicsRendition::Underline(enabled) => {
                    self.attrs.flags.set(CellFlags::UNDERLINE, enabled);
                }
                GraphicsRendition::Blink(enabled) => {
                    self.attrs.flags.set(CellFlags::BLINK, enabled);
                }
                GraphicsRendition::Inverse(enabled) => {
                    self.attrs.flags.set(CellFlags::INVERSE, enabled);
                }
                GraphicsRendition::Hidden(enabled) => {
                    self.attrs.flags.set(CellFlags::HIDDEN, enabled);
                }
                GraphicsRendition::Strikethrough(enabled) => {
                    self.attrs.flags.set(CellFlags::STRIKETHROUGH, enabled);
                }
                GraphicsRendition::Foreground(color) => self.attrs.fg = color,
                GraphicsRendition::Background(color) => self.attrs.bg = color,
            }
        }
    }
}
