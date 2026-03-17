/// The visible cursor position inside the grid.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CursorPosition {
    /// Zero-based row index.
    pub row: usize,
    /// Zero-based column index.
    pub col: usize,
}

impl CursorPosition {
    /// Creates a new cursor position.
    #[must_use]
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
    fn default() -> Self {
        Self::new()
    }
}

impl Cursor {
    /// Creates a cursor at the home position.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            position: CursorPosition::new(0, 0),
            style: CursorStyle::Block,
            visible: true,
            blinking: true,
        }
    }

    /// Moves the cursor to an absolute position.
    pub fn move_to(&mut self, row: usize, col: usize) {
        self.position = CursorPosition::new(row, col);
    }

    /// Moves the cursor upward, saturating at the first row.
    pub fn move_up(&mut self, count: usize) {
        self.position.row = self.position.row.saturating_sub(count);
    }

    /// Moves the cursor downward, clamping to the visible grid.
    pub fn move_down(&mut self, count: usize, max_rows: usize) {
        self.position.row = self
            .position
            .row
            .saturating_add(count)
            .min(max_rows.saturating_sub(1));
    }

    /// Moves the cursor left, saturating at column zero.
    pub fn move_left(&mut self, count: usize) {
        self.position.col = self.position.col.saturating_sub(count);
    }

    /// Moves the cursor right, clamping to the visible grid.
    pub fn move_right(&mut self, count: usize, max_cols: usize) {
        self.position.col = self
            .position
            .col
            .saturating_add(count)
            .min(max_cols.saturating_sub(1));
    }

    /// Captures the current cursor state.
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
