use std::collections::VecDeque;

use super::line::Line;
use super::search::{search_line, SearchResult};

/// Scrollback retention settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScrollbackConfig {
    /// Maximum number of lines to retain.
    pub max_lines: usize,
    /// Optional approximate memory cap in bytes.
    pub max_memory_bytes: Option<usize>,
}

impl Default for ScrollbackConfig {
    fn default() -> Self {
        Self {
            max_lines: 10_000,
            max_memory_bytes: None,
        }
    }
}

/// Ring buffer of terminal history lines.
#[derive(Clone, Debug)]
pub struct Scrollback {
    config: ScrollbackConfig,
    lines: VecDeque<Line>,
    total_lines_seen: u64,
    memory_bytes: usize,
}

impl Scrollback {
    /// Creates a new scrollback buffer.
    #[must_use]
    pub fn new(config: ScrollbackConfig) -> Self {
        Self {
            config,
            lines: VecDeque::new(),
            total_lines_seen: 0,
            memory_bytes: 0,
        }
    }

    /// Appends a line and applies retention limits.
    pub fn push(&mut self, mut line: Line) {
        line.number = self.total_lines_seen;
        self.total_lines_seen = self.total_lines_seen.saturating_add(1);

        if self.config.max_lines == 0 {
            return;
        }

        let line_size = line.memory_size_bytes();
        self.evict_for_line_count();

        if let Some(max_memory_bytes) = self.config.max_memory_bytes {
            self.evict_for_memory(line_size, max_memory_bytes);
            if line_size > max_memory_bytes {
                return;
            }
        }

        self.memory_bytes = self.memory_bytes.saturating_add(line_size);
        self.lines.push_back(line);
    }

    /// Returns the number of retained lines.
    #[must_use]
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Returns `true` when no lines are retained.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Returns a retained line by newest-first index (`0` is newest).
    #[must_use]
    pub fn newest(&self, index: usize) -> Option<&Line> {
        let len = self.lines.len();
        if index >= len {
            return None;
        }

        self.lines.get(len - 1 - index)
    }

    /// Returns a retained line by monotonic line number.
    #[must_use]
    pub fn line_by_number(&self, number: u64) -> Option<&Line> {
        let oldest_number = self.lines.front()?.number;
        let newest_number = self.lines.back()?.number;
        if number < oldest_number || number > newest_number {
            return None;
        }

        self.lines.iter().find(|line| line.number == number)
    }

    /// Iterates retained lines in newest-first order.
    pub fn iter_newest_first(&self) -> impl Iterator<Item = &Line> {
        self.lines.iter().rev()
    }

    /// Iterates retained lines in oldest-first order.
    pub fn iter_oldest_first(&self) -> impl Iterator<Item = &Line> {
        self.lines.iter()
    }

    /// Searches retained lines and returns matches in newest-first order.
    #[must_use]
    pub fn search(&self, pattern: &str, case_sensitive: bool) -> Vec<SearchResult> {
        if pattern.is_empty() {
            return Vec::new();
        }

        let mut results = Vec::new();
        for (line_index, line) in self.lines.iter().rev().enumerate() {
            for (column, length) in search_line(line, pattern, case_sensitive) {
                results.push(SearchResult {
                    line_number: line.number,
                    line_index,
                    column,
                    length,
                });
            }
        }

        results
    }

    /// Clears retained lines while preserving `total_lines_seen`.
    pub fn clear(&mut self) {
        self.lines.clear();
        self.memory_bytes = 0;
    }

    /// Returns approximate retained memory usage in bytes.
    #[must_use]
    pub const fn memory_bytes(&self) -> usize {
        self.memory_bytes
    }

    /// Returns the total number of ingested lines.
    #[must_use]
    pub const fn total_lines_seen(&self) -> u64 {
        self.total_lines_seen
    }

    fn evict_for_line_count(&mut self) {
        while self.lines.len() >= self.config.max_lines {
            if !self.evict_oldest() {
                break;
            }
        }
    }

    fn evict_for_memory(&mut self, new_line_size: usize, max_memory_bytes: usize) {
        while self.memory_bytes.saturating_add(new_line_size) > max_memory_bytes {
            if !self.evict_oldest() {
                break;
            }
        }
    }

    fn evict_oldest(&mut self) -> bool {
        let Some(oldest) = self.lines.pop_front() else {
            return false;
        };
        self.memory_bytes = self.memory_bytes.saturating_sub(oldest.memory_size_bytes());
        true
    }
}
