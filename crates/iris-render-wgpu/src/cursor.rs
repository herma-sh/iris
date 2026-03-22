use bytemuck::{Pod, Zeroable};
use iris_core::cell::CellWidth;
use iris_core::cursor::{Cursor, CursorStyle};
use iris_core::damage::DamageRegion;
use iris_core::grid::Grid;

use crate::error::{Error, Result};
use crate::theme::Theme;

const CURSOR_INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
    0 => Float32x2,
    1 => Float32x2,
    2 => Float32x2,
    3 => Float32x4
];

const UNDERLINE_HEIGHT: f32 = 0.12;
const BAR_WIDTH: f32 = 0.12;

/// GPU-ready cursor overlay instance.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct CursorInstance {
    /// Cursor origin in grid coordinates.
    pub grid_position: [f32; 2],
    /// In-cell offset relative to the cursor origin.
    pub offset: [f32; 2],
    /// Cursor size in cell units.
    pub extent: [f32; 2],
    /// Cursor RGBA color.
    pub color: [f32; 4],
}

impl CursorInstance {
    /// Creates a cursor instance from core cursor state.
    pub fn from_cursor(cursor: Cursor, grid: &Grid, theme: &Theme) -> Result<Option<Self>> {
        if !cursor.visible || grid.rows() == 0 || grid.cols() == 0 {
            return Ok(None);
        }

        let row = cursor.position.row.min(grid.rows().saturating_sub(1));
        let mut col = cursor.position.col.min(grid.cols().saturating_sub(1));
        let span = normalize_cursor_span(grid, row, &mut col);

        let row_u32 =
            u32::try_from(row).map_err(|_| Error::GridCoordinateOutOfRange { row, col })?;
        let col_u32 =
            u32::try_from(col).map_err(|_| Error::GridCoordinateOutOfRange { row, col })?;
        let (offset, extent) = cursor_geometry(cursor.style, span);

        Ok(Some(Self {
            grid_position: [col_u32 as f32, row_u32 as f32],
            offset,
            extent,
            color: theme.cursor.to_f32_array(),
        }))
    }

    /// Returns the vertex-buffer layout used when binding cursor instances.
    #[must_use]
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &CURSOR_INSTANCE_ATTRIBUTES,
        }
    }
}

/// Single-cursor GPU buffers used by the cursor overlay pass.
#[derive(Debug)]
pub struct CursorBuffers {
    instance_buffer: wgpu::Buffer,
    instance_count: usize,
}

impl CursorBuffers {
    /// Creates cursor buffers sized for a single cursor instance.
    #[must_use]
    pub fn new(device: &wgpu::Device) -> Self {
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("iris-render-wgpu-cursor-instance"),
            size: std::mem::size_of::<CursorInstance>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            instance_buffer,
            instance_count: 0,
        }
    }

    /// Returns the current instance buffer.
    #[must_use]
    pub const fn instance_buffer(&self) -> &wgpu::Buffer {
        &self.instance_buffer
    }

    /// Returns the number of active cursor instances.
    #[must_use]
    pub const fn instance_count(&self) -> usize {
        self.instance_count
    }

    /// Writes or clears the active cursor instance.
    pub fn write_instance(&mut self, queue: &wgpu::Queue, instance: Option<&CursorInstance>) {
        if let Some(instance) = instance {
            queue.write_buffer(&self.instance_buffer, 0, bytemuck::bytes_of(instance));
            self.instance_count = 1;
        } else {
            self.instance_count = 0;
        }
    }
}

fn normalize_cursor_span(grid: &Grid, row: usize, col: &mut usize) -> f32 {
    let Some(cell) = grid.cell(row, *col).copied() else {
        return 1.0;
    };

    match cell.width {
        CellWidth::Double => {
            if *col + 1 < grid.cols() {
                2.0
            } else {
                1.0
            }
        }
        CellWidth::Continuation if *col > 0 => {
            if matches!(
                grid.cell(row, *col - 1).map(|lead| lead.width),
                Some(CellWidth::Double)
            ) {
                *col -= 1;
                2.0
            } else {
                1.0
            }
        }
        _ => 1.0,
    }
}

/// Computes the visible cursor damage region in grid coordinates.
#[must_use]
pub(crate) fn cursor_damage_region(cursor: Cursor, grid: &Grid) -> Option<DamageRegion> {
    if !cursor.visible || grid.rows() == 0 || grid.cols() == 0 {
        return None;
    }

    let row = cursor.position.row.min(grid.rows().saturating_sub(1));
    let mut col = cursor.position.col.min(grid.cols().saturating_sub(1));
    let span = normalize_cursor_span(grid, row, &mut col);
    let (_, extent) = cursor_geometry(cursor.style, span);
    let end_col = col.saturating_add(extent[0].ceil().max(1.0) as usize - 1);
    Some(DamageRegion::new(row, row, col, end_col))
}

fn cursor_geometry(style: CursorStyle, span: f32) -> ([f32; 2], [f32; 2]) {
    match style {
        CursorStyle::Block => ([0.0, 0.0], [span, 1.0]),
        CursorStyle::Underline => ([0.0, 1.0 - UNDERLINE_HEIGHT], [span, UNDERLINE_HEIGHT]),
        CursorStyle::Bar => ([0.0, 0.0], [BAR_WIDTH, 1.0]),
    }
}

#[cfg(test)]
mod tests {
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
}
