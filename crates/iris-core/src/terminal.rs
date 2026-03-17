use crate::cell::{Cell, CellAttrs};
use crate::cursor::{Cursor, SavedCursor};
use crate::damage::DamageRegion;
use crate::error::Result;
use crate::grid::{Grid, GridSize};
use crate::modes::TerminalModes;
use crate::utils::TAB_WIDTH;

/// The visible terminal state used by Iris core.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Terminal {
    /// Visible grid contents.
    pub grid: Grid,
    /// Cursor state.
    pub cursor: Cursor,
    /// Terminal modes.
    pub modes: TerminalModes,
    /// Active attributes used for printed text.
    pub attrs: CellAttrs,
    saved_cursor: Option<SavedCursor>,
}

impl Terminal {
    /// Creates a terminal with the provided visible dimensions.
    pub fn new(rows: usize, cols: usize) -> Result<Self> {
        Ok(Self {
            grid: Grid::new(GridSize { rows, cols })?,
            cursor: Cursor::new(),
            modes: TerminalModes::new(),
            attrs: CellAttrs::default(),
            saved_cursor: None,
        })
    }

    /// Writes a printable character at the cursor and advances the cursor.
    pub fn write_char(&mut self, character: char) -> Result<()> {
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
        let cell = Cell::with_attrs(character, self.attrs);
        let width = cell.width.columns();
        self.grid.write(row, col, cell)?;

        if col + width < self.grid.cols() {
            self.cursor.position.col = col + width;
        } else if self.modes.wrap {
            self.cursor.position.col = 0;
            self.line_feed()?;
        }

        Ok(())
    }

    /// Executes a single control character.
    pub fn execute_control(&mut self, byte: u8) -> Result<()> {
        match byte {
            0x08 => self.backspace(),
            0x09 => self.tab(),
            0x0a..=0x0c => self.line_feed()?,
            0x0d => self.carriage_return(),
            _ => {}
        }

        Ok(())
    }

    /// Moves the cursor to an absolute position inside the visible grid.
    pub fn move_cursor(&mut self, row: usize, col: usize) {
        if self.grid.rows() == 0 || self.grid.cols() == 0 {
            self.cursor.move_to(0, 0);
            return;
        }

        self.cursor.move_to(
            row.min(self.grid.rows().saturating_sub(1)),
            col.min(self.grid.cols().saturating_sub(1)),
        );
    }

    /// Resizes the terminal grid and clamps the cursor to the new bounds.
    pub fn resize(&mut self, rows: usize, cols: usize) -> Result<()> {
        self.grid.resize(GridSize { rows, cols })?;
        self.move_cursor(self.cursor.position.row, self.cursor.position.col);
        Ok(())
    }

    /// Saves the current cursor state.
    pub fn save_cursor(&mut self) {
        self.saved_cursor = Some(self.cursor.save());
    }

    /// Restores the saved cursor state when present.
    pub fn restore_cursor(&mut self) {
        if let Some(saved_cursor) = self.saved_cursor {
            self.cursor.restore(saved_cursor);
            self.move_cursor(self.cursor.position.row, self.cursor.position.col);
        }
    }

    /// Returns and clears the current damage list.
    pub fn take_damage(&mut self) -> Vec<DamageRegion> {
        self.grid.take_damage()
    }

    fn backspace(&mut self) {
        self.cursor.move_left(1);
    }

    fn carriage_return(&mut self) {
        self.cursor.position.col = 0;
    }

    fn line_feed(&mut self) -> Result<()> {
        if self.grid.rows() == 0 {
            return Ok(());
        }

        if self.cursor.position.row + 1 >= self.grid.rows() {
            self.grid.scroll_up(1);
        } else {
            self.cursor.move_down(1, self.grid.rows());
        }

        if self.modes.newline {
            self.carriage_return();
        }

        Ok(())
    }

    fn tab(&mut self) {
        let cols = self.grid.cols();
        if cols == 0 {
            return;
        }

        let current = self.cursor.position.col;
        let next_tab_stop = ((current / TAB_WIDTH) + 1) * TAB_WIDTH;
        self.cursor.position.col = next_tab_stop.min(cols.saturating_sub(1));
    }
}

#[cfg(test)]
mod tests {
    use super::Terminal;

    #[test]
    fn terminal_write_advances_cursor() {
        let mut terminal = Terminal::new(3, 4).unwrap();
        terminal.write_char('A').unwrap();
        assert_eq!(
            terminal.grid.cell(0, 0).map(|cell| cell.character),
            Some('A')
        );
        assert_eq!(terminal.cursor.position.col, 1);
    }

    #[test]
    fn terminal_line_feed_scrolls_at_bottom() {
        let mut terminal = Terminal::new(2, 4).unwrap();
        terminal.move_cursor(1, 0);
        terminal.write_char('Z').unwrap();
        terminal.execute_control(0x0a).unwrap();
        terminal.write_char('Q').unwrap();

        assert_eq!(
            terminal.grid.cell(0, 0).map(|cell| cell.character),
            Some('Z')
        );
        assert_eq!(
            terminal.grid.cell(1, 1).map(|cell| cell.character),
            Some('Q')
        );
    }

    #[test]
    fn terminal_resize_clamps_cursor() {
        let mut terminal = Terminal::new(8, 8).unwrap();
        terminal.move_cursor(7, 7);
        terminal.resize(2, 2).unwrap();
        assert_eq!(terminal.cursor.position.row, 1);
        assert_eq!(terminal.cursor.position.col, 1);
    }

    #[test]
    fn terminal_restore_cursor_clamps_after_resize() {
        let mut terminal = Terminal::new(8, 8).unwrap();
        terminal.move_cursor(7, 7);
        terminal.save_cursor();
        terminal.resize(2, 2).unwrap();
        terminal.restore_cursor();
        assert_eq!(terminal.cursor.position.row, 1);
        assert_eq!(terminal.cursor.position.col, 1);
    }
}
