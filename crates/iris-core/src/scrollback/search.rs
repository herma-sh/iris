use crate::cell::CellWidth;

use super::line::Line;

/// A single search match inside scrollback history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchResult {
    /// Monotonic line number assigned by scrollback ingestion.
    pub line_number: u64,
    /// Newest-first index in the current scrollback buffer.
    pub line_index: usize,
    /// Inclusive match start column in display cells.
    pub column: usize,
    /// Match length in display cells.
    pub length: usize,
}

impl SearchResult {
    /// Returns the first column immediately after the match.
    #[must_use]
    pub const fn end_column(&self) -> usize {
        self.column.saturating_add(self.length)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SearchUnit {
    character: char,
    display_col: usize,
    width: usize,
}

pub(crate) fn search_line(line: &Line, pattern: &str, case_sensitive: bool) -> Vec<(usize, usize)> {
    let pattern_chars: Vec<char> = pattern.chars().collect();
    if pattern_chars.is_empty() {
        return Vec::new();
    }

    let units = searchable_units(line);
    if units.len() < pattern_chars.len() {
        return Vec::new();
    }

    let mut matches = Vec::new();
    for start in 0..=(units.len() - pattern_chars.len()) {
        if is_match(&units, &pattern_chars, start, case_sensitive) {
            let length = units[start..(start + pattern_chars.len())]
                .iter()
                .fold(0usize, |acc, unit| acc.saturating_add(unit.width));
            matches.push((units[start].display_col, length));
        }
    }

    matches
}

fn searchable_units(line: &Line) -> Vec<SearchUnit> {
    let mut units = Vec::with_capacity(line.cells.len());
    let mut display_col = 0usize;

    for cell in &line.cells {
        let width = cell.width.columns();
        if cell.width != CellWidth::Continuation {
            units.push(SearchUnit {
                character: cell.character,
                display_col,
                width,
            });
        }
        display_col = display_col.saturating_add(width);
    }

    units
}

fn is_match(
    units: &[SearchUnit],
    pattern_chars: &[char],
    start: usize,
    case_sensitive: bool,
) -> bool {
    for (offset, pattern_char) in pattern_chars.iter().enumerate() {
        let haystack_char = units[start + offset].character;
        if case_sensitive {
            if haystack_char != *pattern_char {
                return false;
            }
            continue;
        }

        if !chars_equal_case_insensitive(haystack_char, *pattern_char) {
            return false;
        }
    }

    true
}

fn chars_equal_case_insensitive(left: char, right: char) -> bool {
    left == right || left.to_lowercase().eq(right.to_lowercase())
}
