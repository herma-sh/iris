use iris_core::cell::Cell;
use iris_core::cursor::{Cursor, CursorStyle};
use iris_core::damage::DamageRegion;
use iris_core::grid::{Grid, GridSize};

use super::{cursor_damage_region, cursor_geometry, CursorBuffers, CursorInstance};
use crate::renderer::{Renderer, RendererConfig};
use crate::theme::Theme;

#[test]
fn cursor_geometry_matches_style_expectations() {
    assert_eq!(
        cursor_geometry(CursorStyle::Block, 1.0),
        ([0.0, 0.0], [1.0, 1.0])
    );
    assert_eq!(
        cursor_geometry(CursorStyle::Underline, 2.0),
        ([0.0, 0.88], [2.0, 0.12])
    );
    assert_eq!(
        cursor_geometry(CursorStyle::Bar, 1.0),
        ([0.0, 0.0], [0.12, 1.0])
    );
}

#[test]
fn cursor_instance_skips_hidden_or_empty_grids() {
    let mut cursor = Cursor::new();
    cursor.visible = false;
    let grid = Grid::new(GridSize { rows: 1, cols: 1 }).expect("grid should be created");

    assert!(
        CursorInstance::from_cursor(cursor, &grid, &Theme::default())
            .expect("hidden cursor should not error")
            .is_none()
    );

    let empty_grid = Grid::new(GridSize { rows: 0, cols: 0 }).expect("grid should be created");
    assert!(
        CursorInstance::from_cursor(Cursor::new(), &empty_grid, &Theme::default())
            .expect("empty grid cursor should not error")
            .is_none()
    );
}

#[test]
fn cursor_instance_tracks_style_and_color() {
    let mut cursor = Cursor::new();
    cursor.move_to(2, 3);
    cursor.style = CursorStyle::Underline;
    let grid = Grid::new(GridSize { rows: 4, cols: 5 }).expect("grid should be created");
    let instance = CursorInstance::from_cursor(cursor, &grid, &Theme::default())
        .expect("cursor instance should encode")
        .expect("cursor should remain visible");

    assert_eq!(instance.grid_position, [3.0, 2.0]);
    assert_eq!(instance.offset, [0.0, 0.88]);
    assert_eq!(instance.extent, [1.0, 0.12]);
    assert_eq!(instance.color, Theme::default().cursor.to_f32_array());
}

#[test]
fn cursor_instance_expands_to_cover_wide_cells() {
    let mut grid = Grid::new(GridSize { rows: 1, cols: 2 }).expect("grid should be created");
    grid.write(0, 0, Cell::new('中'))
        .expect("wide cell should be written");
    let mut cursor = Cursor::new();
    cursor.move_to(0, 1);

    let instance = CursorInstance::from_cursor(cursor, &grid, &Theme::default())
        .expect("wide cursor should encode")
        .expect("wide cursor should be visible");

    assert_eq!(instance.grid_position, [0.0, 0.0]);
    assert_eq!(instance.extent, [2.0, 1.0]);
}

#[test]
fn cursor_instance_falls_back_to_single_width_at_the_right_edge() {
    let mut grid = Grid::new(GridSize { rows: 1, cols: 2 }).expect("grid should be created");
    grid.write(0, 0, Cell::new('A'))
        .expect("leading cell should be written");
    grid.write(0, 1, Cell::new('\u{4E2D}'))
        .expect("right-edge wide cell should be normalized by the grid");
    let mut cursor = Cursor::new();
    cursor.move_to(0, 1);

    let instance = CursorInstance::from_cursor(cursor, &grid, &Theme::default())
        .expect("right-edge cursor should encode")
        .expect("cursor should be visible");

    assert_eq!(instance.grid_position, [1.0, 0.0]);
    assert_eq!(instance.extent, [1.0, 1.0]);
}

#[test]
fn cursor_instance_handles_continuation_at_column_zero() {
    let mut grid = Grid::new(GridSize { rows: 1, cols: 2 }).expect("grid should be created");
    grid.write(0, 0, Cell::continuation(Default::default()))
        .expect("orphan continuation should be written");

    let instance = CursorInstance::from_cursor(Cursor::new(), &grid, &Theme::default())
        .expect("orphan continuation should still encode")
        .expect("cursor should remain visible");

    assert_eq!(instance.grid_position, [0.0, 0.0]);
    assert_eq!(instance.extent, [1.0, 1.0]);
}

#[test]
fn cursor_damage_region_skips_hidden_and_empty_grids() {
    let mut hidden_cursor = Cursor::new();
    hidden_cursor.visible = false;
    let grid = Grid::new(GridSize { rows: 1, cols: 1 }).expect("grid should be created");
    assert_eq!(cursor_damage_region(hidden_cursor, &grid), None);

    let empty_grid = Grid::new(GridSize { rows: 0, cols: 0 }).expect("grid should be created");
    assert_eq!(cursor_damage_region(Cursor::new(), &empty_grid), None);
}

#[test]
fn cursor_damage_region_tracks_wide_cell_spans() {
    let mut grid = Grid::new(GridSize { rows: 1, cols: 2 }).expect("grid should be created");
    grid.write(0, 0, Cell::new('中'))
        .expect("wide cell should be written");
    let mut cursor = Cursor::new();
    cursor.move_to(0, 1);

    assert_eq!(
        cursor_damage_region(cursor, &grid),
        Some(DamageRegion::new(0, 0, 0, 1))
    );
}

#[test]
fn cursor_buffers_track_optional_instance_uploads() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let mut buffers = CursorBuffers::new(renderer.device());
    let instance = CursorInstance {
        grid_position: [1.0, 2.0],
        offset: [0.0, 0.0],
        extent: [1.0, 1.0],
        color: [1.0; 4],
    };

    buffers.write_instance(renderer.queue(), Some(&instance));
    assert_eq!(buffers.instance_count(), 1);

    buffers.write_instance(renderer.queue(), None);
    assert_eq!(buffers.instance_count(), 0);
}
