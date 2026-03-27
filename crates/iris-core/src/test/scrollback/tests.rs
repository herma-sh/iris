use std::mem::size_of;
use std::time::Duration;

use crate::cell::{Cell, CellAttrs};
use crate::scrollback::{Line, Scrollback, ScrollbackConfig, SearchEngine};

fn line(text: &str) -> Line {
    Line::from_text(text, false)
}

fn line_with_exact_capacity(text: &str) -> Line {
    let char_count = text.chars().count();
    let mut cells = Vec::with_capacity(char_count);
    cells.extend(text.chars().map(Cell::new));
    Line::new(cells, false)
}

fn line_with_capacity(text: &str, capacity: usize) -> Line {
    let mut cells = Vec::with_capacity(capacity);
    cells.extend(text.chars().map(Cell::new));
    Line::new(cells, false)
}

#[test]
fn line_text_omits_continuation_cells() {
    let attrs = CellAttrs::default();
    let line = Line::new(
        vec![
            Cell::new('\u{4f60}'),
            Cell::continuation(attrs),
            Cell::new('x'),
        ],
        false,
    );

    assert_eq!(line.text(), "\u{4f60}x");
}

#[test]
fn line_display_width_counts_wide_cells() {
    let attrs = CellAttrs::default();
    let line = Line::new(
        vec![
            Cell::new('\u{4f60}'),
            Cell::continuation(attrs),
            Cell::new('x'),
        ],
        false,
    );

    assert_eq!(line.display_width(), 3);
}

#[test]
fn scrollback_push_assigns_monotonic_line_numbers() {
    let mut scrollback = Scrollback::new(ScrollbackConfig {
        max_lines: 3,
        max_memory_bytes: None,
    });

    scrollback.push(line("line-0"));
    scrollback.push(line("line-1"));
    scrollback.push(line("line-2"));
    scrollback.push(line("line-3"));

    assert_eq!(scrollback.total_lines_seen(), 4);
    assert_eq!(scrollback.len(), 3);
    assert_eq!(scrollback.newest(0).map(|line| line.number), Some(3));
    assert_eq!(scrollback.newest(2).map(|line| line.number), Some(1));
    assert!(scrollback.line_by_number(0).is_none());
    assert_eq!(
        scrollback.line_by_number(2).map(Line::text).as_deref(),
        Some("line-2")
    );
}

#[test]
fn scrollback_enforces_max_lines() {
    let mut scrollback = Scrollback::new(ScrollbackConfig {
        max_lines: 2,
        max_memory_bytes: None,
    });

    scrollback.push(line("first"));
    scrollback.push(line("second"));
    scrollback.push(line("third"));

    assert_eq!(scrollback.len(), 2);
    assert_eq!(
        scrollback.newest(0).map(Line::text).as_deref(),
        Some("third")
    );
    assert_eq!(
        scrollback.newest(1).map(Line::text).as_deref(),
        Some("second")
    );
}

#[test]
fn scrollback_enforces_max_memory_bytes() {
    let bytes_per_line = 4 * size_of::<Cell>();
    let mut scrollback = Scrollback::new(ScrollbackConfig {
        max_lines: 8,
        max_memory_bytes: Some(bytes_per_line * 2),
    });

    scrollback.push(line_with_exact_capacity("aaaa"));
    scrollback.push(line_with_exact_capacity("bbbb"));
    scrollback.push(line_with_exact_capacity("cccc"));

    assert_eq!(scrollback.len(), 2);
    assert_eq!(
        scrollback.newest(0).map(Line::text).as_deref(),
        Some("cccc")
    );
    assert_eq!(
        scrollback.newest(1).map(Line::text).as_deref(),
        Some("bbbb")
    );
    assert!(scrollback.memory_bytes() <= bytes_per_line * 2);
}

#[test]
fn scrollback_skips_line_that_cannot_fit_memory_cap() {
    let bytes_per_line = 4 * size_of::<Cell>();
    let mut scrollback = Scrollback::new(ScrollbackConfig {
        max_lines: 8,
        max_memory_bytes: Some(bytes_per_line.saturating_sub(1)),
    });

    scrollback.push(line_with_exact_capacity("aaaa"));

    assert!(scrollback.is_empty());
    assert_eq!(scrollback.total_lines_seen(), 1);
    assert_eq!(scrollback.memory_bytes(), 0);
}

#[test]
fn scrollback_iterators_return_expected_order() {
    let mut scrollback = Scrollback::new(ScrollbackConfig::default());
    scrollback.push(line("one"));
    scrollback.push(line("two"));
    scrollback.push(line("three"));

    let newest: Vec<String> = scrollback.iter_newest_first().map(Line::text).collect();
    let oldest: Vec<String> = scrollback.iter_oldest_first().map(Line::text).collect();

    assert_eq!(newest, vec!["three", "two", "one"]);
    assert_eq!(oldest, vec!["one", "two", "three"]);
}

#[test]
fn scrollback_search_returns_newest_first_matches() {
    let mut scrollback = Scrollback::new(ScrollbackConfig::default());
    scrollback.push(line("alpha beta alpha"));
    scrollback.push(line("nothing here"));
    scrollback.push(line("ALPHA alpha"));

    let results = scrollback.search("alpha", true);

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].line_number, 2);
    assert_eq!(results[0].line_index, 0);
    assert_eq!(results[0].column, 6);
    assert_eq!(results[0].length, 5);
    assert_eq!(results[1].line_number, 0);
    assert_eq!(results[1].line_index, 2);
    assert_eq!(results[1].column, 0);
    assert_eq!(results[2].line_number, 0);
    assert_eq!(results[2].line_index, 2);
    assert_eq!(results[2].column, 11);
}

#[test]
fn scrollback_search_case_insensitive_matches_uppercase() {
    let mut scrollback = Scrollback::new(ScrollbackConfig::default());
    scrollback.push(line("alpha beta alpha"));
    scrollback.push(line("ALPHA alpha"));

    let results = scrollback.search("alpha", false);

    assert_eq!(results.len(), 4);
    assert_eq!(results[0].line_number, 1);
    assert_eq!(results[0].column, 0);
    assert_eq!(results[1].line_number, 1);
    assert_eq!(results[1].column, 6);
    assert_eq!(results[2].line_number, 0);
    assert_eq!(results[2].column, 0);
    assert_eq!(results[3].line_number, 0);
    assert_eq!(results[3].column, 11);
}

#[test]
fn scrollback_search_reports_display_columns_with_wide_cells() {
    let attrs = CellAttrs::default();
    let mut scrollback = Scrollback::new(ScrollbackConfig::default());
    scrollback.push(Line::new(
        vec![
            Cell::new('\u{4f60}'),
            Cell::continuation(attrs),
            Cell::new('a'),
        ],
        false,
    ));

    let results = scrollback.search("a", true);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].column, 2);
    assert_eq!(results[0].length, 1);
    assert_eq!(results[0].end_column(), 3);
}

#[test]
fn scrollback_search_empty_pattern_returns_no_results() {
    let mut scrollback = Scrollback::new(ScrollbackConfig::default());
    scrollback.push(line("anything"));

    assert!(scrollback.search("", true).is_empty());
}

#[test]
fn scrollback_clear_drops_retained_lines_but_keeps_total_seen_counter() {
    let mut scrollback = Scrollback::new(ScrollbackConfig::default());
    scrollback.push(line("one"));
    scrollback.push(line("two"));
    let seen_before_clear = scrollback.total_lines_seen();

    scrollback.clear();

    assert_eq!(scrollback.len(), 0);
    assert_eq!(scrollback.memory_bytes(), 0);
    assert_eq!(scrollback.total_lines_seen(), seen_before_clear);
}

#[test]
fn search_engine_forward_and_backward_find_adjacent_matches() {
    let mut scrollback = Scrollback::new(ScrollbackConfig::default());
    scrollback.push(line("alpha beta"));
    scrollback.push(line("middle"));
    scrollback.push(line("alpha gamma"));

    let mut engine = SearchEngine::new();
    engine.set_pattern("alpha");

    let forward = engine.search_forward(&scrollback, 0, 0).unwrap();
    assert_eq!(forward.line_number, 2);
    assert_eq!(forward.column, 0);
    assert_eq!(engine.current_match(), Some(1));

    let backward = engine.search_backward(&scrollback, 2, 0).unwrap();
    assert_eq!(backward.line_number, 0);
    assert_eq!(backward.column, 0);
    assert_eq!(engine.current_match(), Some(0));
}

#[test]
fn search_engine_wraps_when_enabled() {
    let mut scrollback = Scrollback::new(ScrollbackConfig::default());
    scrollback.push(line("one"));
    scrollback.push(line("two one"));

    let mut engine = SearchEngine::new();
    engine.set_pattern("one");
    let wrapped = engine
        .search_forward(&scrollback, u64::MAX, usize::MAX)
        .unwrap();
    assert_eq!(wrapped.line_number, 0);
    assert_eq!(engine.current_match(), Some(0));

    engine.set_wrap(false);
    assert!(engine
        .search_forward(&scrollback, u64::MAX, usize::MAX)
        .is_none());
}

#[test]
fn search_engine_whole_word_ignores_embedded_matches() {
    let mut scrollback = Scrollback::new(ScrollbackConfig::default());
    scrollback.push(line("helloworld world"));

    let mut engine = SearchEngine::new();
    engine.set_whole_word(true);
    engine.set_pattern("world");

    let results = engine.search(&scrollback);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].column, 11);
}

#[test]
fn search_engine_regex_matches_patterns() {
    let mut scrollback = Scrollback::new(ScrollbackConfig::default());
    scrollback.push(line("abc123xyz"));

    let mut engine = SearchEngine::new();
    engine.set_use_regex(true);
    engine.set_pattern(r"\d+");

    let results = engine.search(&scrollback);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].column, 3);
    assert_eq!(results[0].length, 3);
}

#[test]
fn search_engine_regex_honors_whole_word_boundaries() {
    let mut scrollback = Scrollback::new(ScrollbackConfig::default());
    scrollback.push(line("helloworld world WORLD"));

    let mut engine = SearchEngine::new();
    engine.set_use_regex(true);
    engine.set_whole_word(true);
    engine.set_pattern("world");

    let results = engine.search(&scrollback);
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].column, 11);
    assert_eq!(results[1].column, 17);
}

#[test]
fn scrollback_equality_ignores_line_timestamps() {
    let mut left = Scrollback::new(ScrollbackConfig::default());
    let mut right = Scrollback::new(ScrollbackConfig::default());

    let left_line = line("same");
    let mut right_line = line("same");
    right_line.timestamp = left_line.timestamp + Duration::from_secs(1);

    left.push(left_line);
    right.push(right_line);

    assert_eq!(left, right);
}

#[test]
fn scrollback_equality_ignores_allocation_accounting() {
    let mut left = Scrollback::new(ScrollbackConfig::default());
    let mut right = Scrollback::new(ScrollbackConfig::default());

    left.push(line_with_capacity("same", 4));
    right.push(line_with_capacity("same", 16));

    assert_ne!(left.memory_bytes(), right.memory_bytes());
    assert_eq!(left, right);
}

#[test]
fn search_engine_setters_noop_on_equal_values() {
    let mut scrollback = Scrollback::new(ScrollbackConfig::default());
    scrollback.push(line("alpha"));
    scrollback.push(line("beta"));
    scrollback.push(line("alpha"));

    let mut engine = SearchEngine::new();
    engine.set_pattern("alpha");
    let _ = engine.search_forward(&scrollback, 0, 0);
    let selected = engine.current_match();
    assert!(selected.is_some());

    engine.set_pattern("alpha");
    assert_eq!(engine.current_match(), selected);

    engine.set_whole_word(false);
    assert_eq!(engine.current_match(), selected);

    engine.set_wrap(true);
    assert_eq!(engine.current_match(), selected);
}
