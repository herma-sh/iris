use crate::cursor::CursorPosition;

/// Selection kind.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SelectionKind {
    /// Character-by-character.
    #[default]
    Simple,
    /// Word-boundary selection.
    Word,
    /// Full-line selection.
    Line,
    /// Rectangular block selection.
    Block,
}

/// Selection lifecycle state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SelectionState {
    /// No active selection.
    #[default]
    None,
    /// Selection is in progress.
    Selecting,
    /// Selection is complete.
    Complete,
}

/// Selection anchor point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Anchor {
    /// Grid position for the anchor.
    pub position: CursorPosition,
    /// Selection mode tied to this anchor.
    pub kind: SelectionKind,
}

impl Anchor {
    /// Creates an anchor at the provided grid position.
    #[must_use]
    pub const fn new(row: usize, col: usize, kind: SelectionKind) -> Self {
        Self {
            position: CursorPosition::new(row, col),
            kind,
        }
    }
}

/// Selection range and state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    /// Start anchor.
    pub start: Anchor,
    /// End anchor.
    pub end: Anchor,
    /// Selection mode.
    pub kind: SelectionKind,
    /// Current lifecycle state.
    pub state: SelectionState,
}

impl Selection {
    /// Creates a new in-progress selection.
    #[must_use]
    pub const fn new(start_row: usize, start_col: usize, kind: SelectionKind) -> Self {
        let anchor = Anchor::new(start_row, start_col, kind);
        Self {
            start: anchor,
            end: anchor,
            kind,
            state: SelectionState::Selecting,
        }
    }

    /// Marks the selection as complete.
    pub fn complete(&mut self) {
        self.state = SelectionState::Complete;
    }

    /// Extends the selection endpoint.
    pub fn extend(&mut self, row: usize, col: usize) {
        self.end.position = CursorPosition::new(row, col);
    }

    /// Returns `true` when start and end are equal.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.start.position == self.end.position
    }

    /// Returns `true` if the given position is selected.
    #[must_use]
    pub fn contains(&self, row: usize, col: usize) -> bool {
        match self.kind {
            SelectionKind::Simple | SelectionKind::Word | SelectionKind::Line => {
                self.contains_linear(row, col)
            }
            SelectionKind::Block => self.contains_block(row, col),
        }
    }

    /// Returns selected column bounds for a row, clamped to the visible grid width.
    #[must_use]
    pub(crate) fn row_bounds(&self, row: usize, grid_cols: usize) -> Option<(usize, usize)> {
        if grid_cols == 0 {
            return None;
        }

        match self.kind {
            SelectionKind::Simple | SelectionKind::Word | SelectionKind::Line => {
                let (start, end) = self.ordered();
                if row < start.row || row > end.row {
                    return None;
                }

                let row_last_col = grid_cols.saturating_sub(1);
                let start_col = if row == start.row {
                    start.col.min(row_last_col)
                } else {
                    0
                };
                let end_col = if row == end.row {
                    end.col.min(row_last_col)
                } else {
                    row_last_col
                };

                (start_col <= end_col).then_some((start_col, end_col))
            }
            SelectionKind::Block => {
                let min_row = self.start.position.row.min(self.end.position.row);
                let max_row = self.start.position.row.max(self.end.position.row);
                if row < min_row || row > max_row {
                    return None;
                }

                let min_col = self.start.position.col.min(self.end.position.col);
                let max_col = self.start.position.col.max(self.end.position.col);
                let row_last_col = grid_cols.saturating_sub(1);
                let start_col = min_col.min(row_last_col);
                let end_col = max_col.min(row_last_col);

                (start_col <= end_col).then_some((start_col, end_col))
            }
        }
    }

    fn contains_linear(&self, row: usize, col: usize) -> bool {
        let (start, end) = self.ordered();
        if row < start.row || row > end.row {
            return false;
        }

        if start.row == end.row {
            return col >= start.col && col <= end.col;
        }
        if row == start.row {
            return col >= start.col;
        }
        if row == end.row {
            return col <= end.col;
        }

        true
    }

    fn contains_block(&self, row: usize, col: usize) -> bool {
        let min_row = self.start.position.row.min(self.end.position.row);
        let max_row = self.start.position.row.max(self.end.position.row);
        let min_col = self.start.position.col.min(self.end.position.col);
        let max_col = self.start.position.col.max(self.end.position.col);

        row >= min_row && row <= max_row && col >= min_col && col <= max_col
    }

    fn ordered(&self) -> (CursorPosition, CursorPosition) {
        let start = self.start.position;
        let end = self.end.position;

        if start.row < end.row {
            return (start, end);
        }
        if start.row > end.row {
            return (end, start);
        }
        if start.col <= end.col {
            (start, end)
        } else {
            (end, start)
        }
    }
}
