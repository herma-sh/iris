use crate::cell::Cell;
use crate::grid::{Grid, GridSize};
use crate::selection::{Selection, SelectionEngine, SelectionKind, SelectionState};

fn grid_from_lines(lines: &[&str], cols: usize) -> Grid {
    let mut grid = Grid::new(GridSize {
        rows: lines.len(),
        cols,
    })
    .expect("grid dimensions should be valid");

    for (row, line) in lines.iter().enumerate() {
        for (col, character) in line.chars().enumerate() {
            if col >= cols {
                break;
            }
            grid.write(row, col, Cell::new(character))
                .expect("cell write should succeed");
        }
    }

    grid
}

#[test]
fn selection_contains_simple_ranges() {
    let mut selection = Selection::new(1, 4, SelectionKind::Simple);
    selection.extend(3, 2);

    assert!(selection.contains(1, 4));
    assert!(selection.contains(2, 0));
    assert!(selection.contains(3, 2));
    assert!(!selection.contains(0, 4));
    assert!(!selection.contains(3, 5));
}

#[test]
fn selection_contains_block_ranges() {
    let mut selection = Selection::new(4, 7, SelectionKind::Block);
    selection.extend(2, 3);

    assert!(selection.contains(2, 3));
    assert!(selection.contains(3, 5));
    assert!(selection.contains(4, 7));
    assert!(!selection.contains(1, 3));
    assert!(!selection.contains(2, 8));
}

#[test]
fn selection_row_bounds_clamp_to_grid_columns() {
    let mut selection = Selection::new(0, 2, SelectionKind::Simple);
    selection.extend(1, 99);

    assert_eq!(selection.row_bounds(0, 4), Some((2, 3)));
    assert_eq!(selection.row_bounds(1, 4), Some((0, 3)));
    assert_eq!(selection.row_bounds(2, 4), None);
}

#[test]
fn selection_engine_tracks_lifecycle() {
    let mut engine = SelectionEngine::new();
    assert!(!engine.is_selecting());
    assert!(!engine.has_selection());

    engine.start(0, 0, SelectionKind::Simple);
    assert!(engine.is_selecting());
    assert!(!engine.has_selection());

    engine.extend(0, 3);
    engine.complete();
    assert!(!engine.is_selecting());
    assert!(engine.has_selection());

    engine.cancel();
    assert!(!engine.is_selecting());
    assert!(!engine.has_selection());
}

#[test]
fn selection_engine_select_word_extracts_word_text() {
    let grid = grid_from_lines(&["hello world"], 16);
    let mut engine = SelectionEngine::new();

    engine.select_word(&grid, 0, 7);

    assert_eq!(
        engine.selection().map(|selection| selection.kind),
        Some(SelectionKind::Word)
    );
    assert_eq!(engine.selected_text(&grid).as_deref(), Some("world"));
}

#[test]
fn selection_engine_select_line_extracts_line_text() {
    let grid = grid_from_lines(&["first", "second"], 8);
    let mut engine = SelectionEngine::new();

    engine.select_line(&grid, 1);

    assert_eq!(
        engine.selection().map(|selection| selection.kind),
        Some(SelectionKind::Line)
    );
    assert_eq!(engine.selected_text(&grid).as_deref(), Some("second  "));
    assert_eq!(engine.copy_text(&grid).as_deref(), Some("second  \n"));
}

#[test]
fn selection_engine_copy_text_preserves_non_line_selection_without_trailing_newline() {
    let grid = grid_from_lines(&["hello world"], 16);
    let mut engine = SelectionEngine::new();

    engine.select_word(&grid, 0, 6);

    assert_eq!(engine.copy_text(&grid).as_deref(), Some("world"));
}

#[test]
fn selection_engine_selected_text_for_block_selection() {
    let grid = grid_from_lines(&["abcd", "efgh", "ijkl"], 4);
    let mut engine = SelectionEngine::new();

    engine.start(0, 1, SelectionKind::Block);
    engine.extend(2, 2);
    engine.complete();

    assert_eq!(
        engine.selection().map(|selection| selection.state),
        Some(SelectionState::Complete)
    );
    assert_eq!(engine.selected_text(&grid).as_deref(), Some("bc\nfg\njk"));
}
