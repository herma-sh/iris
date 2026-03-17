/// The visible cursor position inside the grid.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CursorPosition {
    /// Zero-based row index.
    pub row: usize,
    /// Zero-based column index.
    pub col: usize,
}

impl CursorPosition {
    /// Constructs a `CursorPosition` at the given zero-based row and column.
    ///
    /// # Examples
    ///
    /// ```
    /// let p = CursorPosition::new(4, 9);
    /// assert_eq!(p.row, 4);
    /// assert_eq!(p.col, 9);
    /// ```
    pub const fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }
}

/// The configured cursor presentation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CursorStyle {
    /// Full cell block cursor.
    #[default]
    Block,
    /// Underline cursor.
    Underline,
    /// Vertical bar cursor.
    Bar,
}

/// Saved cursor state for DEC save/restore semantics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SavedCursor {
    /// Saved position.
    pub position: CursorPosition,
    /// Saved style.
    pub style: CursorStyle,
}

/// Cursor state tracked by the terminal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cursor {
    /// Current position.
    pub position: CursorPosition,
    /// Cursor style.
    pub style: CursorStyle,
    /// Visibility flag.
    pub visible: bool,
    /// Blink flag.
    pub blinking: bool,
}

impl Default for Cursor {
    /// Creates a cursor at the home position (row 0, column 0) with `Block` style, visible, and blinking.
    ///
    /// # Examples
    ///
    /// ```
    /// let d = Cursor::default();
    /// let n = Cursor::new();
    /// assert_eq!(d, n);
    /// ```
    fn default() -> Self {
        Self::new()
    }
}

impl Cursor {
    /// Creates a cursor at the home position (row 0, column 0) with `Block` style, visible and blinking.
    ///
    /// # Examples
    ///
    /// ```
    /// let c = Cursor::new();
    /// assert_eq!(c.position.row, 0);
    /// assert_eq!(c.position.col, 0);
    /// assert_eq!(c.style, CursorStyle::Block);
    /// assert!(c.visible && c.blinking);
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self {
            position: CursorPosition::new(0, 0),
            style: CursorStyle::Block,
            visible: true,
            blinking: true,
        }
    }

    /// Moves the cursor to the specified absolute position.
    ///
    /// Row and column are zero-based indices; the cursor's position is set to these values.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut c = Cursor::new();
    /// c.move_to(4, 9);
    /// assert_eq!(c.position, CursorPosition::new(4, 9));
    /// ```
    pub fn move_to(&mut self, row: usize, col: usize) {
        self.position = CursorPosition::new(row, col);
    }

    /// Moves the cursor upward, saturating at the first row.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut c = Cursor::new();
    /// c.move_to(5, 2);
    /// c.move_up(3);
    /// assert_eq!(c.position.row, 2);
    ///
    /// c.move_up(10); // saturates at 0, does not underflow
    /// assert_eq!(c.position.row, 0);
    /// ```
    pub fn move_up(&mut self, count: usize) {
        self.position.row = self.position.row.saturating_sub(count);
    }

    /// Moves the cursor down by `count` rows, clamping the row to the last visible index (`max_rows - 1`).
    ///
    /// # Examples
    ///
    /// ```
    /// let mut c = Cursor::new();
    /// c.move_down(5, 24);
    /// assert!(c.position.row <= 23);
    /// ```
    pub fn move_down(&mut self, count: usize, max_rows: usize) {
        self.position.row = (self.position.row + count).min(max_rows.saturating_sub(1));
    }

    /// Moves the cursor left by `count` columns, clamping at column zero.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut c = Cursor::new();
    /// c.move_to(0, 5);
    /// c.move_left(3);
    /// assert_eq!(c.position.col, 2);
    /// c.move_left(5);
    /// assert_eq!(c.position.col, 0);
    /// ```
    pub fn move_left(&mut self, count: usize) {
        self.position.col = self.position.col.saturating_sub(count);
    }

    /// Moves the cursor right by `count`, ensuring the column does not exceed the last visible column.
    ///
    /// `max_cols` is the total number of visible columns; the cursor column will be clamped to `max_cols - 1`.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut c = Cursor::new();
    /// c.move_right(5, 10);
    /// assert_eq!(c.position.col, 5);
    ///
    /// // does not move past the last column (9)
    /// c.move_right(10, 10);
    /// assert_eq!(c.position.col, 9);
    /// ```
    pub fn move_right(&mut self, count: usize, max_cols: usize) {
        self.position.col = (self.position.col + count).min(max_cols.saturating_sub(1));
    }

    /// Creates a snapshot of the cursor's current position and style.
    ///
    /// # Examples
    ///
    /// ```
    /// let cur = Cursor::new();
    /// let saved = cur.save();
    /// assert_eq!(saved.position, cur.position);
    /// assert_eq!(saved.style, cur.style);
    /// ```
    #[must_use]
    pub const fn save(self) -> SavedCursor {
        SavedCursor {
            position: self.position,
            style: self.style,
        }
    }

    /// Restores a previously saved cursor state.
    pub fn restore(&mut self, saved: SavedCursor) {
        self.position = saved.position;
        self.style = saved.style;
    }
}

#[cfg(test)]
mod tests {
    use super::Cursor;

    #[test]
    fn cursor_movement_respects_bounds() {
        let mut cursor = Cursor::new();
        cursor.move_right(100, 80);
        cursor.move_down(100, 24);
        assert_eq!(cursor.position.col, 79);
        assert_eq!(cursor.position.row, 23);

        cursor.move_left(200);
        cursor.move_up(200);
        assert_eq!(cursor.position.col, 0);
        assert_eq!(cursor.position.row, 0);
    }

    #[test]
    fn cursor_save_and_restore_round_trips() {
        let mut cursor = Cursor::new();
        cursor.move_to(4, 9);
        let saved = cursor.save();
        cursor.move_to(0, 0);
        cursor.restore(saved);
        assert_eq!(cursor.position.row, 4);
        assert_eq!(cursor.position.col, 9);
    }
}
