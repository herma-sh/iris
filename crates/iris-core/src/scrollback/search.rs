use regex::{Regex, RegexBuilder};

use crate::cell::CellWidth;

use super::buffer::Scrollback;
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

/// Runtime search settings for scrollback queries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchConfig {
    /// Search pattern text.
    pub pattern: String,
    /// `true` for case-sensitive matching.
    pub case_sensitive: bool,
    /// `true` to treat `pattern` as a regex.
    pub use_regex: bool,
    /// `true` to require whole-word boundaries.
    pub whole_word: bool,
    /// `true` to wrap around at buffer edges for next/previous operations.
    pub wrap: bool,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            pattern: String::new(),
            case_sensitive: false,
            use_regex: false,
            whole_word: false,
            wrap: true,
        }
    }
}

/// Stateful scrollback search helper supporting next/previous navigation.
#[derive(Clone, Debug)]
pub struct SearchEngine {
    config: SearchConfig,
    compiled_regex: Option<Regex>,
    current_match: Option<usize>,
}

impl SearchEngine {
    /// Creates a search engine with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: SearchConfig::default(),
            compiled_regex: None,
            current_match: None,
        }
    }

    /// Returns the current search configuration.
    #[must_use]
    pub const fn config(&self) -> &SearchConfig {
        &self.config
    }

    /// Returns the selected result index from the latest next/previous lookup.
    #[must_use]
    pub const fn current_match(&self) -> Option<usize> {
        self.current_match
    }

    /// Sets the search pattern and refreshes compiled regex state.
    pub fn set_pattern<S: Into<String>>(&mut self, pattern: S) {
        let next_pattern = pattern.into();
        if self.config.pattern == next_pattern {
            return;
        }

        self.config.pattern = next_pattern;
        self.current_match = None;
        self.rebuild_regex();
    }

    /// Sets case sensitivity and refreshes compiled regex state.
    pub fn set_case_sensitive(&mut self, case_sensitive: bool) {
        if self.config.case_sensitive == case_sensitive {
            return;
        }

        self.config.case_sensitive = case_sensitive;
        self.current_match = None;
        self.rebuild_regex();
    }

    /// Enables or disables regex mode and refreshes compiled regex state.
    pub fn set_use_regex(&mut self, use_regex: bool) {
        if self.config.use_regex == use_regex {
            return;
        }

        self.config.use_regex = use_regex;
        self.current_match = None;
        self.rebuild_regex();
    }

    /// Enables or disables whole-word matching.
    pub fn set_whole_word(&mut self, whole_word: bool) {
        if self.config.whole_word == whole_word {
            return;
        }

        self.config.whole_word = whole_word;
        self.current_match = None;
    }

    /// Enables or disables wrap-around behavior for next/previous lookups.
    pub fn set_wrap(&mut self, wrap: bool) {
        if self.config.wrap == wrap {
            return;
        }

        self.config.wrap = wrap;
        self.current_match = None;
    }

    /// Returns all matches ordered from oldest to newest line.
    #[must_use]
    pub fn search(&self, scrollback: &Scrollback) -> Vec<SearchResult> {
        let pattern = self.config.pattern.as_str();
        if pattern.is_empty() {
            return Vec::new();
        }

        let retained_len = scrollback.len();
        if retained_len == 0 {
            return Vec::new();
        }

        let mut results = Vec::new();
        for (oldest_index, line) in scrollback.iter_oldest_first().enumerate() {
            let newest_index = retained_len - 1 - oldest_index;
            let line_matches = if self.config.use_regex {
                self.compiled_regex.as_ref().map_or_else(Vec::new, |regex| {
                    search_line_regex(line, regex, self.config.whole_word)
                })
            } else if self.config.whole_word {
                search_line_whole_word(line, pattern, self.config.case_sensitive)
            } else {
                search_line(line, pattern, self.config.case_sensitive)
            };

            for (column, length) in line_matches {
                results.push(SearchResult {
                    line_number: line.number,
                    line_index: newest_index,
                    column,
                    length,
                });
            }
        }

        results
    }

    /// Finds the next match after the provided position.
    #[must_use]
    pub fn search_forward(
        &mut self,
        scrollback: &Scrollback,
        start_line: u64,
        start_col: usize,
    ) -> Option<SearchResult> {
        let results = self.search(scrollback);

        for (index, result) in results.iter().enumerate() {
            let is_after_line = result.line_number > start_line;
            let is_after_column = result.line_number == start_line && result.column > start_col;
            if is_after_line || is_after_column {
                self.current_match = Some(index);
                return Some(result.clone());
            }
        }

        if self.config.wrap {
            if let Some(result) = results.first() {
                self.current_match = Some(0);
                return Some(result.clone());
            }
        }

        self.current_match = None;
        None
    }

    /// Finds the previous match before the provided position.
    #[must_use]
    pub fn search_backward(
        &mut self,
        scrollback: &Scrollback,
        start_line: u64,
        start_col: usize,
    ) -> Option<SearchResult> {
        let results = self.search(scrollback);

        for (index, result) in results.iter().enumerate().rev() {
            let is_before_line = result.line_number < start_line;
            let is_before_column = result.line_number == start_line && result.column < start_col;
            if is_before_line || is_before_column {
                self.current_match = Some(index);
                return Some(result.clone());
            }
        }

        if self.config.wrap {
            if let Some((index, result)) = results
                .len()
                .checked_sub(1)
                .and_then(|index| results.get(index).map(|result| (index, result)))
            {
                self.current_match = Some(index);
                return Some(result.clone());
            }
        }

        self.current_match = None;
        None
    }

    fn rebuild_regex(&mut self) {
        self.compiled_regex = if self.config.use_regex && !self.config.pattern.is_empty() {
            RegexBuilder::new(&self.config.pattern)
                .case_insensitive(!self.config.case_sensitive)
                .build()
                .ok()
        } else {
            None
        };
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
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
    find_char_slice_matches(&units, &pattern_chars, case_sensitive, false)
}

fn search_line_whole_word(line: &Line, pattern: &str, case_sensitive: bool) -> Vec<(usize, usize)> {
    let pattern_chars: Vec<char> = pattern.chars().collect();
    if pattern_chars.is_empty() {
        return Vec::new();
    }

    let units = searchable_units(line);
    find_char_slice_matches(&units, &pattern_chars, case_sensitive, true)
}

fn search_line_regex(line: &Line, regex: &Regex, whole_word: bool) -> Vec<(usize, usize)> {
    let units = searchable_units(line);
    if units.is_empty() {
        return Vec::new();
    }

    let searchable: String = units.iter().map(|unit| unit.character).collect();
    let mut results = Vec::new();

    for candidate in regex.find_iter(&searchable) {
        let start_char = searchable[..candidate.start()].chars().count();
        let end_char = searchable[..candidate.end()].chars().count();
        if end_char <= start_char || start_char >= units.len() || end_char > units.len() {
            continue;
        }
        if whole_word && !has_word_boundaries(&units, start_char, end_char) {
            continue;
        }

        let length = units[start_char..end_char]
            .iter()
            .fold(0usize, |acc, unit| acc.saturating_add(unit.width));
        results.push((units[start_char].display_col, length));
    }

    results
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

fn find_char_slice_matches(
    units: &[SearchUnit],
    pattern_chars: &[char],
    case_sensitive: bool,
    whole_word: bool,
) -> Vec<(usize, usize)> {
    if pattern_chars.is_empty() || units.len() < pattern_chars.len() {
        return Vec::new();
    }

    let mut matches = Vec::new();
    let last_start = units.len() - pattern_chars.len();
    for start in 0..=last_start {
        if !is_match(units, pattern_chars, start, case_sensitive) {
            continue;
        }

        let end = start + pattern_chars.len();
        if whole_word && !has_word_boundaries(units, start, end) {
            continue;
        }

        let length = units[start..end]
            .iter()
            .fold(0usize, |acc, unit| acc.saturating_add(unit.width));
        matches.push((units[start].display_col, length));
    }

    matches
}

fn has_word_boundaries(units: &[SearchUnit], start: usize, end: usize) -> bool {
    let starts_on_boundary =
        start == 0 || !units[start.saturating_sub(1)].character.is_alphanumeric();
    let ends_on_boundary = end >= units.len() || !units[end].character.is_alphanumeric();
    starts_on_boundary && ends_on_boundary
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
