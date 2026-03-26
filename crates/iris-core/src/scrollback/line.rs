use std::mem::size_of;
use std::time::Instant;

use crate::cell::{Cell, CellWidth};

/// A single captured line stored in scrollback history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    /// Cells captured for this logical line.
    pub cells: Vec<Cell>,
    /// `true` when this line wraps onto the next visible row.
    pub wrapped: bool,
    /// Capture timestamp for observability and ordering.
    pub timestamp: Instant,
    /// Monotonic line number assigned by [`Scrollback`](crate::scrollback::Scrollback).
    pub number: u64,
}

impl Line {
    /// Creates a line from pre-built cells.
    #[must_use]
    pub fn new(cells: Vec<Cell>, wrapped: bool) -> Self {
        Self {
            cells,
            wrapped,
            timestamp: Instant::now(),
            number: 0,
        }
    }

    /// Creates a line from plain text using default cell attributes.
    #[must_use]
    pub fn from_text(text: &str, wrapped: bool) -> Self {
        let cells = text.chars().map(Cell::new).collect();
        Self::new(cells, wrapped)
    }

    /// Returns logical line text, excluding continuation cells.
    #[must_use]
    pub fn text(&self) -> String {
        self.cells
            .iter()
            .filter(|cell| cell.width != CellWidth::Continuation)
            .map(|cell| cell.character)
            .collect()
    }

    /// Returns the display width in columns.
    #[must_use]
    pub fn display_width(&self) -> usize {
        self.cells
            .iter()
            .fold(0usize, |acc, cell| acc.saturating_add(cell.width.columns()))
    }

    pub(crate) fn memory_size_bytes(&self) -> usize {
        self.cells.capacity().saturating_mul(size_of::<Cell>())
    }
}
