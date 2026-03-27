use std::mem::size_of;

use crate::cell::{Cell, CellAttrs};
use crate::scrollback::{Line, Scrollback, ScrollbackConfig};

fn line(text: &str) -> Line {
    Line::from_text(text, false)
}

fn line_with_exact_capacity(text: &str) -> Line {
    let char_count = text.chars().count();
    let mut cells = Vec::with_capacity(char_count);
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
