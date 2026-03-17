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
    /// Creates a Terminal with the given visible dimensions.
    ///
    /// # Examples
    ///
    /// ```
    /// let _term = Terminal::new(3, 4);
    /// ```
    #[must_use]
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            grid: Grid::new(GridSize { rows, cols }),
            cursor: Cursor::new(),
            modes: TerminalModes::new(),
            attrs: CellAttrs::default(),
            saved_cursor: None,
        }
    }

    /// Writes a printable character at the current cursor position and advances the cursor,
    /// respecting the grid bounds and the terminal's wrap mode.
    ///
    /// If the terminal has no rows or columns, this is a no-op. The character is written
    /// with the terminal's current attributes. The cursor advances by the character's
    /// display width; if the advance would move past the end of the row and wrap mode
    /// is enabled, the cursor moves to column 0 of the next line (triggering scrolling
    /// if at the bottom).
    ///
    /// # Errors
    ///
    /// Returns the underlying grid write error if writing the cell fails.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut t = Terminal::new(1, 2);
    /// t.write_char('A').unwrap();
    /// assert_eq!(t.cursor.position.col, 1);
    /// ```
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

    /// Handle a single ASCII control byte by performing its associated terminal action.
    ///
    /// Recognizes the following control codes:
    /// - `0x08` — backspace
    /// - `0x09` — tab
    /// - `0x0A..=0x0C` — line feed
    /// - `0x0D` — carriage return
    /// Other byte values are ignored.
    ///
    /// # Parameters
    ///
    /// - `byte`: ASCII control code to execute.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut term = Terminal::new(2, 4);
    /// // Tab control (0x09) is handled and returns Ok(())
    /// assert!(term.execute_control(0x09).is_ok());
    /// ```
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

    /// Move the cursor to an absolute position within the visible grid.
    ///
    /// If the grid has zero rows or columns, the cursor is placed at (0, 0).
    /// Otherwise the provided `row` and `col` are treated as zero-based indices and
    /// clamped to the last valid row and column of the grid.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut term = Terminal::new(2, 3);
    /// term.move_cursor(1, 2);
    /// assert_eq!((term.cursor.row, term.cursor.col), (1, 2));
    ///
    /// // out-of-bounds values are clamped
    /// term.move_cursor(10, 10);
    /// assert_eq!((term.cursor.row, term.cursor.col), (1, 2));
    /// ```
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

    /// Resize the terminal's visible grid and ensure the cursor remains within the new bounds.
    ///
    /// After resizing, the cursor position is clamped to the grid dimensions so it does not lie
    /// outside the visible area.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut t = Terminal::new(8, 8);
    /// t.move_cursor(7, 7);
    /// t.resize(2, 2).unwrap();
    /// assert_eq!(t.cursor.position.row, 1);
    /// assert_eq!(t.cursor.position.col, 1);
    /// ```
    pub fn resize(&mut self, rows: usize, cols: usize) -> Result<()> {
        self.grid.resize(GridSize { rows, cols })?;
        self.move_cursor(self.cursor.position.row, self.cursor.position.col);
        Ok(())
    }

    /// Saves the current cursor state so it can be restored later.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut term = Terminal::new(3, 4);
    /// term.move_cursor(1, 2);
    /// term.save_cursor();
    /// term.move_cursor(0, 0);
    /// term.restore_cursor();
    /// assert_eq!((term.cursor.row, term.cursor.col), (1, 2));
    /// ```
    pub fn save_cursor(&mut self) {
        self.saved_cursor = Some(self.cursor.save());
    }

    /// Restores the previously saved cursor state if one exists.
    ///
    /// If a cursor state was saved with `save_cursor`, this restores the terminal's cursor
    /// to that saved state; otherwise it has no effect.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut term = Terminal::new(3, 4);
    /// term.move_cursor(1, 2);
    /// term.save_cursor();
    /// term.move_cursor(0, 0);
    /// term.restore_cursor();
    /// assert_eq!((term.cursor.row, term.cursor.col), (1, 2));
    /// ```
    pub fn restore_cursor(&mut self) {
        if let Some(saved_cursor) = self.saved_cursor {
            self.cursor.restore(saved_cursor);
        }
    }

    /// Retrieve and clear the current damage regions recorded by the grid.
    ///
    /// Returns the list of `DamageRegion` entries that were pending; the grid's damage list is emptied.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut t = Terminal::new(1, 1);
    /// let damages = t.take_damage();
    /// assert!(damages.is_empty());
    /// ```
    pub fn take_damage(&mut self) -> Vec<DamageRegion> {
        self.grid.take_damage()
    }

    /// Moves the terminal cursor one column to the left.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut term = Terminal::new(1, 3);
    /// term.move_cursor(0, 1);
    /// term.execute_control(0x08).unwrap(); // triggers backspace
    /// assert_eq!(term.cursor.col, 0);
    /// ```
    fn backspace(&mut self) {
        self.cursor.move_left(1);
    }

    /// Moves the cursor to the start of the current line (sets column to 0).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let mut term = Terminal::new(3, 4);
    /// term.move_cursor(1, 5);
    /// // internal carriage_return moves cursor column to 0
    /// term.carriage_return();
    /// assert_eq!(term.cursor.position.col, 0);
    /// ```
    fn carriage_return(&mut self) {
        self.cursor.position.col = 0;
    }

    /// Advances the terminal to the next line, scrolling the grid if the cursor is at the bottom.
    ///
    /// If the grid has zero rows this is a no-op. When the cursor is on the last row the visible
    /// contents are scrolled up by one line; otherwise the cursor row is incremented. If newline
    /// mode is enabled the cursor column is reset to the start of the line after moving.
    ///
    /// # Examples
    ///
    /// ```
    /// use iris_core::terminal::Terminal;
    ///
    /// let mut t = Terminal::new(2, 4);
    /// // move cursor to the bottom row then perform a line feed via the control byte 0x0A
    /// t.move_cursor(1, 0);
    /// t.execute_control(0x0A).unwrap(); // invokes line_feed internally
    /// // after line_feed the cursor will be at row 1 (either moved down or preserved after scroll)
    /// assert!(t.cursor.position.row <= 1);
    /// ```
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

    /// Advances the cursor to the next tab stop within the current row.
    ///
    /// If the grid has zero columns this is a no-op. The cursor column is set to
    /// the next multiple of `TAB_WIDTH` after the current column, clamped to the
    /// last column index.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut t = Terminal::new(1, 8);
    /// t.cursor.position.col = 2;
    /// t.tab();
    /// assert_eq!(t.cursor.position.col, 4);
    ///
    /// t.cursor.position.col = 7;
    /// t.tab();
    /// assert_eq!(t.cursor.position.col, 7);
    /// ```
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
        let mut terminal = Terminal::new(3, 4);
        terminal.write_char('A').unwrap();
        assert_eq!(
            terminal.grid.cell(0, 0).map(|cell| cell.character),
            Some('A')
        );
        assert_eq!(terminal.cursor.position.col, 1);
    }

    #[test]
    fn terminal_line_feed_scrolls_at_bottom() {
        let mut terminal = Terminal::new(2, 4);
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
        let mut terminal = Terminal::new(8, 8);
        terminal.move_cursor(7, 7);
        terminal.resize(2, 2).unwrap();
        assert_eq!(terminal.cursor.position.row, 1);
        assert_eq!(terminal.cursor.position.col, 1);
    }
}
