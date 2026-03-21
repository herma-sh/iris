use pretty_assertions::assert_eq;

use super::{Grid, GridSize};
use crate::cell::{Cell, CellWidth};
use crate::damage::DamageRegion;
use crate::error::Error;

#[test]
fn grid_write_updates_damage() {
    let mut grid = Grid::new(GridSize { rows: 3, cols: 4 }).unwrap();
    grid.write(1, 2, Cell::new('A')).unwrap();
    assert_eq!(grid.cell(1, 2), Some(&Cell::new('A')));
    assert_eq!(grid.take_damage(), vec![DamageRegion::new(1, 1, 2, 2)]);
}

#[test]
fn grid_write_ascii_run_updates_a_contiguous_damage_range() {
    let mut grid = Grid::new(GridSize { rows: 1, cols: 5 }).unwrap();
    grid.write_ascii_run(0, 1, b"abc", crate::cell::CellAttrs::default())
        .unwrap();

    assert_eq!(grid.cell(0, 1), Some(&Cell::new('a')));
    assert_eq!(grid.cell(0, 2), Some(&Cell::new('b')));
    assert_eq!(grid.cell(0, 3), Some(&Cell::new('c')));
    assert_eq!(grid.take_damage(), vec![DamageRegion::new(0, 0, 1, 3)]);
}

#[test]
fn grid_write_ascii_run_clears_existing_wide_cells() {
    let mut grid = Grid::new(GridSize { rows: 1, cols: 4 }).unwrap();
    grid.write(0, 1, Cell::new('\u{4e2d}')).unwrap();
    grid.take_damage();

    grid.write_ascii_run(0, 1, b"xy", crate::cell::CellAttrs::default())
        .unwrap();

    assert_eq!(grid.cell(0, 0), Some(&Cell::default()));
    assert_eq!(grid.cell(0, 1), Some(&Cell::new('x')));
    assert_eq!(grid.cell(0, 2), Some(&Cell::new('y')));
    assert_eq!(grid.take_damage(), vec![DamageRegion::new(0, 0, 1, 2)]);
}

#[test]
fn grid_write_ascii_run_rejects_non_printable_bytes() {
    let mut grid = Grid::new(GridSize { rows: 1, cols: 4 }).unwrap();

    let error = grid
        .write_ascii_run(0, 1, b"a\n", crate::cell::CellAttrs::default())
        .unwrap_err();

    assert_eq!(error, Error::InvalidAsciiRun { byte: b'\n' });
    assert_eq!(grid.cell(0, 1), Some(&Cell::default()));
    assert!(grid.take_damage().is_empty());
}

#[test]
fn grid_write_ascii_run_rejects_utf8_bytes() {
    let mut grid = Grid::new(GridSize { rows: 1, cols: 4 }).unwrap();

    let error = grid
        .write_ascii_run(0, 0, &[0xc3], crate::cell::CellAttrs::default())
        .unwrap_err();

    assert_eq!(error, Error::InvalidAsciiRun { byte: 0xc3 });
    assert_eq!(grid.cell(0, 0), Some(&Cell::default()));
    assert!(grid.take_damage().is_empty());
}

#[test]
fn grid_scroll_moves_content() {
    let mut grid = Grid::new(GridSize { rows: 3, cols: 4 }).unwrap();
    grid.write(0, 0, Cell::new('A')).unwrap();
    grid.write(1, 0, Cell::new('B')).unwrap();
    grid.write(2, 0, Cell::new('C')).unwrap();

    grid.scroll_up(1);

    assert_eq!(grid.cell(0, 0), Some(&Cell::new('B')));
    assert_eq!(grid.cell(1, 0), Some(&Cell::new('C')));
    assert_eq!(grid.cell(2, 0), Some(&Cell::default()));
}

#[test]
fn grid_resize_preserves_content() {
    let mut grid = Grid::new(GridSize { rows: 2, cols: 2 }).unwrap();
    grid.write(0, 0, Cell::new('X')).unwrap();
    grid.write(1, 1, Cell::new('Y')).unwrap();

    grid.resize(GridSize { rows: 3, cols: 4 }).unwrap();

    assert_eq!(grid.cell(0, 0), Some(&Cell::new('X')));
    assert_eq!(grid.cell(1, 1), Some(&Cell::new('Y')));
    assert_eq!(grid.rows(), 3);
    assert_eq!(grid.cols(), 4);
}

#[test]
fn grid_scroll_up_range_preserves_outside_rows() {
    let mut grid = Grid::new(GridSize { rows: 4, cols: 1 }).unwrap();
    grid.write(0, 0, Cell::new('A')).unwrap();
    grid.write(1, 0, Cell::new('B')).unwrap();
    grid.write(2, 0, Cell::new('C')).unwrap();
    grid.write(3, 0, Cell::new('D')).unwrap();

    grid.scroll_up_range(1, 3, 1).unwrap();

    assert_eq!(grid.cell(0, 0), Some(&Cell::new('A')));
    assert_eq!(grid.cell(1, 0), Some(&Cell::new('C')));
    assert_eq!(grid.cell(2, 0), Some(&Cell::new('D')));
    assert_eq!(grid.cell(3, 0), Some(&Cell::default()));
}

#[test]
fn grid_scroll_down_range_preserves_outside_rows() {
    let mut grid = Grid::new(GridSize { rows: 4, cols: 1 }).unwrap();
    grid.write(0, 0, Cell::new('A')).unwrap();
    grid.write(1, 0, Cell::new('B')).unwrap();
    grid.write(2, 0, Cell::new('C')).unwrap();
    grid.write(3, 0, Cell::new('D')).unwrap();

    grid.scroll_down_range(1, 3, 1).unwrap();

    assert_eq!(grid.cell(0, 0), Some(&Cell::new('A')));
    assert_eq!(grid.cell(1, 0), Some(&Cell::default()));
    assert_eq!(grid.cell(2, 0), Some(&Cell::new('B')));
    assert_eq!(grid.cell(3, 0), Some(&Cell::new('C')));
}

#[test]
fn grid_scroll_up_range_rejects_invalid_row_ranges() {
    let mut grid = Grid::new(GridSize { rows: 4, cols: 1 }).unwrap();

    assert_eq!(
        grid.scroll_up_range(3, 2, 1),
        Err(Error::InvalidPosition {
            row: 3,
            col: 0,
            rows: 4,
            cols: 1,
        })
    );
    assert_eq!(
        grid.scroll_up_range(0, 4, 1),
        Err(Error::InvalidPosition {
            row: 4,
            col: 0,
            rows: 4,
            cols: 1,
        })
    );
}

#[test]
fn grid_scroll_down_range_rejects_invalid_row_ranges() {
    let mut grid = Grid::new(GridSize { rows: 4, cols: 1 }).unwrap();

    assert_eq!(
        grid.scroll_down_range(3, 2, 1),
        Err(Error::InvalidPosition {
            row: 3,
            col: 0,
            rows: 4,
            cols: 1,
        })
    );
    assert_eq!(
        grid.scroll_down_range(0, 4, 1),
        Err(Error::InvalidPosition {
            row: 4,
            col: 0,
            rows: 4,
            cols: 1,
        })
    );
}

#[test]
fn grid_insert_blank_cells_shifts_row_contents() {
    let mut grid = Grid::new(GridSize { rows: 1, cols: 5 }).unwrap();
    grid.write(0, 0, Cell::new('A')).unwrap();
    grid.write(0, 1, Cell::new('B')).unwrap();
    grid.write(0, 2, Cell::new('C')).unwrap();
    grid.write(0, 3, Cell::new('D')).unwrap();

    grid.insert_blank_cells(0, 1, 2).unwrap();

    assert_eq!(grid.cell(0, 0), Some(&Cell::new('A')));
    assert_eq!(grid.cell(0, 1), Some(&Cell::default()));
    assert_eq!(grid.cell(0, 2), Some(&Cell::default()));
    assert_eq!(grid.cell(0, 3), Some(&Cell::new('B')));
    assert_eq!(grid.cell(0, 4), Some(&Cell::new('C')));
}

#[test]
fn grid_delete_cells_shifts_row_contents_left() {
    let mut grid = Grid::new(GridSize { rows: 1, cols: 5 }).unwrap();
    grid.write(0, 0, Cell::new('A')).unwrap();
    grid.write(0, 1, Cell::new('B')).unwrap();
    grid.write(0, 2, Cell::new('C')).unwrap();
    grid.write(0, 3, Cell::new('D')).unwrap();

    grid.delete_cells(0, 1, 2).unwrap();

    assert_eq!(grid.cell(0, 0), Some(&Cell::new('A')));
    assert_eq!(grid.cell(0, 1), Some(&Cell::new('D')));
    assert_eq!(grid.cell(0, 2), Some(&Cell::default()));
    assert_eq!(grid.cell(0, 3), Some(&Cell::default()));
    assert_eq!(grid.cell(0, 4), Some(&Cell::default()));
}

#[test]
fn grid_downgrades_wide_cells_at_right_edge() {
    let mut grid = Grid::new(GridSize { rows: 1, cols: 1 }).unwrap();
    grid.write(0, 0, Cell::new('中')).unwrap();
    assert_eq!(grid.cell(0, 0).unwrap().width, CellWidth::Single);
}

#[test]
fn grid_clears_overwritten_wide_cell_spans() {
    let mut grid = Grid::new(GridSize { rows: 1, cols: 3 }).unwrap();
    grid.write(0, 1, Cell::new('中')).unwrap();
    grid.take_damage();

    grid.write(0, 0, Cell::new('中')).unwrap();

    assert_eq!(grid.cell(0, 0).unwrap().width, CellWidth::Double);
    assert_eq!(grid.cell(0, 1).unwrap().width, CellWidth::Continuation);
    assert_eq!(grid.cell(0, 2).unwrap(), &Cell::default());
    assert_eq!(grid.take_damage(), vec![DamageRegion::new(0, 0, 0, 2)]);
}

#[test]
fn grid_restore_damage_replays_drained_regions() {
    let mut grid = Grid::new(GridSize { rows: 2, cols: 3 }).unwrap();
    grid.write(1, 2, Cell::new('Z')).unwrap();

    let damage = grid.take_damage();
    assert_eq!(damage, vec![DamageRegion::new(1, 1, 2, 2)]);
    assert!(grid.take_damage().is_empty());

    grid.restore_damage(&damage);

    assert_eq!(grid.take_damage(), damage);
}

#[test]
fn grid_scroll_up_tracks_scroll_delta() {
    let mut grid = Grid::new(GridSize { rows: 3, cols: 2 }).unwrap();

    grid.scroll_up(1);

    assert_eq!(
        grid.take_scroll_delta(),
        Some(crate::damage::ScrollDelta::new(0, 2, 1))
    );
    assert_eq!(grid.take_scroll_delta(), None);
}

#[test]
fn grid_restore_scroll_delta_replays_drained_scrolls() {
    let mut grid = Grid::new(GridSize { rows: 3, cols: 2 }).unwrap();

    grid.scroll_down(1);

    let scroll_delta = grid.take_scroll_delta();
    assert_eq!(
        scroll_delta,
        Some(crate::damage::ScrollDelta::new(0, 2, -1))
    );
    assert_eq!(grid.take_scroll_delta(), None);

    grid.restore_scroll_delta(scroll_delta);

    assert_eq!(
        grid.take_scroll_delta(),
        Some(crate::damage::ScrollDelta::new(0, 2, -1))
    );
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "restore_scroll_delta called with an occupied pending scroll slot")]
fn grid_restore_scroll_delta_rejects_overwriting_existing() {
    let mut grid = Grid::new(GridSize { rows: 3, cols: 2 }).unwrap();
    grid.scroll_up(1);
    let prior = grid.take_scroll_delta();

    grid.scroll_up(2);
    grid.restore_scroll_delta(prior);
}

#[test]
fn grid_consecutive_same_region_scrolls_merge() {
    let mut grid = Grid::new(GridSize { rows: 5, cols: 2 }).unwrap();

    grid.scroll_up(1);
    grid.scroll_up(2);

    assert_eq!(
        grid.take_scroll_delta(),
        Some(crate::damage::ScrollDelta::new(0, 4, 3))
    );
}

#[test]
fn grid_opposite_direction_scrolls_replace_not_merge() {
    let mut grid = Grid::new(GridSize { rows: 3, cols: 2 }).unwrap();

    grid.scroll_up(1);
    grid.scroll_down(1);

    assert_eq!(
        grid.take_scroll_delta(),
        Some(crate::damage::ScrollDelta::new(0, 2, -1))
    );
}

#[test]
fn grid_scroll_up_range_records_partial_scroll_delta() {
    let mut grid = Grid::new(GridSize { rows: 5, cols: 2 }).unwrap();

    grid.scroll_up_range(1, 3, 1).unwrap();

    assert_eq!(
        grid.take_scroll_delta(),
        Some(crate::damage::ScrollDelta::new(1, 3, 1))
    );
}

#[test]
fn grid_scroll_down_range_records_partial_scroll_delta() {
    let mut grid = Grid::new(GridSize { rows: 5, cols: 2 }).unwrap();

    grid.scroll_down_range(1, 3, 1).unwrap();

    assert_eq!(
        grid.take_scroll_delta(),
        Some(crate::damage::ScrollDelta::new(1, 3, -1))
    );
}

#[test]
fn grid_restore_damage_and_scroll_delta_together() {
    let mut grid = Grid::new(GridSize { rows: 3, cols: 2 }).unwrap();
    grid.write(1, 1, Cell::new('X')).unwrap();
    grid.record_scroll(crate::damage::ScrollDelta::new(0, 2, 1));

    let damage = grid.take_damage();
    let scroll_delta = grid.take_scroll_delta();

    grid.restore_damage(&damage);
    grid.restore_scroll_delta(scroll_delta);

    assert_eq!(grid.take_damage(), damage);
    assert_eq!(grid.take_scroll_delta(), scroll_delta);
}

#[test]
fn grid_scroll_on_zero_size_grid_produces_no_delta() {
    let mut grid = Grid::new(GridSize { rows: 0, cols: 4 }).unwrap();

    grid.scroll_up(1);

    assert_eq!(grid.take_scroll_delta(), None);
}

#[test]
fn grid_consecutive_scroll_merging_saturates_at_max() {
    let mut grid = Grid::new(GridSize { rows: 2, cols: 2 }).unwrap();

    grid.record_scroll(crate::damage::ScrollDelta::new(0, 1, i32::MAX - 1));
    grid.record_scroll(crate::damage::ScrollDelta::new(0, 1, 5));

    assert_eq!(
        grid.take_scroll_delta(),
        Some(crate::damage::ScrollDelta::new(0, 1, i32::MAX))
    );
}

#[test]
fn grid_consecutive_scroll_merging_saturates_at_min() {
    let mut grid = Grid::new(GridSize { rows: 2, cols: 2 }).unwrap();

    grid.record_scroll(crate::damage::ScrollDelta::new(0, 1, i32::MIN + 1));
    grid.record_scroll(crate::damage::ScrollDelta::new(0, 1, -5));

    assert_eq!(
        grid.take_scroll_delta(),
        Some(crate::damage::ScrollDelta::new(0, 1, i32::MIN))
    );
}
