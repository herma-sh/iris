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
#[path = "test/cursor/tests.rs"]
mod tests;
