use super::{Selection, SelectionKind, SelectionState};
use crate::grid::Grid;

/// Stateful selection engine for terminal grids.
#[derive(Debug)]
pub struct SelectionEngine {
    selection: Option<Selection>,
    is_word_boundary: fn(char) -> bool,
}

impl Default for SelectionEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SelectionEngine {
    /// Creates an empty selection engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            selection: None,
            is_word_boundary,
        }
    }

    /// Returns the active selection, if any.
    #[must_use]
    pub const fn selection(&self) -> Option<&Selection> {
        self.selection.as_ref()
    }

    /// Returns `true` while selection dragging is active.
    #[must_use]
    pub fn is_selecting(&self) -> bool {
        self.selection
            .map(|selection| selection.state == SelectionState::Selecting)
            .unwrap_or(false)
    }

    /// Returns `true` when a completed selection exists.
    #[must_use]
    pub fn has_selection(&self) -> bool {
        self.selection
            .map(|selection| selection.state == SelectionState::Complete)
            .unwrap_or(false)
    }

    /// Starts a new selection at the provided position.
    pub fn start(&mut self, row: usize, col: usize, kind: SelectionKind) {
        self.selection = Some(Selection::new(row, col, kind));
    }

    /// Extends the current selection endpoint.
    pub fn extend(&mut self, row: usize, col: usize) {
        if let Some(selection) = &mut self.selection {
            selection.extend(row, col);
        }
    }

    /// Completes the current selection.
    pub fn complete(&mut self) {
        if let Some(selection) = &mut self.selection {
            selection.complete();
        }
    }

    /// Cancels the current selection.
    pub fn cancel(&mut self) {
        self.selection = None;
    }

    /// Selects the word at the given grid position.
    pub fn select_word(&mut self, grid: &Grid, row: usize, col: usize) {
        let Some(row_cells) = grid.row(row) else {
            self.selection = None;
            return;
        };
        if row_cells.is_empty() {
            self.selection = None;
            return;
        }

        let col = col.min(row_cells.len().saturating_sub(1));
        let selected = row_cells[col].character;

        let (start_col, end_col) = if (self.is_word_boundary)(selected) {
            (col, col)
        } else {
            let start_col = self.find_word_start(row_cells, col);
            let end_col = self.find_word_end(row_cells, col);
            (start_col, end_col)
        };

        let mut selection = Selection::new(row, start_col, SelectionKind::Word);
        selection.extend(row, end_col);
        selection.complete();
        self.selection = Some(selection);
    }

    /// Selects an entire row.
    pub fn select_line(&mut self, grid: &Grid, row: usize) {
        if row >= grid.rows() || grid.cols() == 0 {
            self.selection = None;
            return;
        }

        let mut selection = Selection::new(row, 0, SelectionKind::Line);
        selection.extend(row, grid.cols().saturating_sub(1));
        selection.complete();
        self.selection = Some(selection);
    }

    /// Returns selected text from the provided grid.
    #[must_use]
    pub fn selected_text(&self, grid: &Grid) -> Option<String> {
        let selection = self.selection?;
        if selection.state != SelectionState::Complete {
            return None;
        }

        let min_row = selection.start.position.row.min(selection.end.position.row);
        let max_row = selection.start.position.row.max(selection.end.position.row);

        let mut output = String::new();
        let mut wrote_any_row = false;

        for row in min_row..=max_row {
            let Some((start_col, end_col)) = selection.row_bounds(row, grid.cols()) else {
                continue;
            };
            let Some(cells) = grid.row(row) else {
                continue;
            };

            if wrote_any_row {
                output.push('\n');
            }
            wrote_any_row = true;

            for col in start_col..=end_col {
                if let Some(cell) = cells.get(col) {
                    output.push(cell.character);
                }
            }
        }

        wrote_any_row.then_some(output)
    }

    fn find_word_start(&self, row_cells: &[crate::cell::Cell], col: usize) -> usize {
        let mut start = col;
        while start > 0 && !(self.is_word_boundary)(row_cells[start - 1].character) {
            start = start.saturating_sub(1);
        }
        start
    }

    fn find_word_end(&self, row_cells: &[crate::cell::Cell], col: usize) -> usize {
        let mut end = col;
        while end + 1 < row_cells.len() && !(self.is_word_boundary)(row_cells[end + 1].character) {
            end = end.saturating_add(1);
        }
        end
    }
}

fn is_word_boundary(character: char) -> bool {
    character.is_whitespace() || character.is_ascii_punctuation()
}
