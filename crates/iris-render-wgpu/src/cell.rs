use bytemuck::{Pod, Zeroable};
use iris_core::cell::{Cell, CellAttrs, CellWidth};
use iris_core::damage::DamageRegion;
use iris_core::grid::Grid;

use crate::atlas::AtlasSize;
use crate::error::{Error, Result};
use crate::glyph::CachedGlyph;
use crate::theme::Theme;

const CELL_INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 9] = wgpu::vertex_attr_array![
    0 => Float32x2,
    1 => Float32x2,
    2 => Float32x2,
    3 => Float32x2,
    4 => Float32x2,
    5 => Float32x4,
    6 => Float32x4,
    7 => Float32,
    8 => Uint32
];

/// Resolved foreground and background colors used for text rendering.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellColors {
    /// Foreground RGBA color.
    pub fg: [f32; 4],
    /// Background RGBA color.
    pub bg: [f32; 4],
}

impl CellColors {
    /// Creates resolved cell colors for a text instance.
    #[must_use]
    pub const fn new(fg: [f32; 4], bg: [f32; 4]) -> Self {
        Self { fg, bg }
    }
}

/// GPU uniform data shared by the text renderer.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct TextUniforms {
    /// Viewport resolution in pixels.
    pub resolution: [f32; 2],
    /// Cell size in pixels.
    pub cell_size: [f32; 2],
    /// Vertical scroll offset in pixels.
    pub scroll_offset: f32,
    /// Explicit padding for stable WGSL layout.
    pub _padding: u32,
}

impl TextUniforms {
    /// Creates text-rendering uniforms for the provided viewport and cell size.
    #[must_use]
    pub const fn new(resolution: [f32; 2], cell_size: [f32; 2], scroll_offset: f32) -> Self {
        Self {
            resolution,
            cell_size,
            scroll_offset,
            _padding: 0,
        }
    }

    /// Returns the uniform payload as raw bytes for buffer uploads.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}

/// GPU-ready per-cell instance data for text rendering.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct CellInstance {
    /// Cell origin in grid coordinates.
    pub grid_position: [f32; 2],
    /// Glyph UV origin in atlas space.
    pub atlas_min: [f32; 2],
    /// Glyph UV extent in atlas space.
    pub atlas_max: [f32; 2],
    /// Glyph offset in pixels relative to the cell's top-left corner.
    pub glyph_offset: [f32; 2],
    /// Glyph bitmap extent in pixels.
    pub glyph_extent: [f32; 2],
    /// Resolved foreground color.
    pub fg_color: [f32; 4],
    /// Resolved background color.
    pub bg_color: [f32; 4],
    /// Number of grid columns occupied by the rendered cell.
    pub cell_span: f32,
    /// Packed style flags from the source terminal cell.
    pub style_flags: u32,
}

impl CellInstance {
    /// Encodes a renderable terminal cell and cached glyph into GPU instance data.
    pub fn from_cell(
        cell: Cell,
        column: u32,
        row: u32,
        glyph: CachedGlyph,
        atlas_size: AtlasSize,
        colors: CellColors,
    ) -> Result<Self> {
        if cell.width.columns() == 0 {
            return Err(Error::ContinuationCellNotRenderable);
        }

        let region = glyph.region();
        let atlas_width = atlas_size.width as f32;
        let atlas_height = atlas_size.height as f32;
        let atlas_min_x = (region.x as f32 + 0.5) / atlas_width;
        let atlas_min_y = (region.y as f32 + 0.5) / atlas_height;
        let atlas_max_x = (region.x as f32 + region.width as f32 - 0.5) / atlas_width;
        let atlas_max_y = (region.y as f32 + region.height as f32 - 0.5) / atlas_height;

        Ok(Self {
            grid_position: [column as f32, row as f32],
            atlas_min: [atlas_min_x, atlas_min_y],
            atlas_max: [atlas_max_x, atlas_max_y],
            glyph_offset: [
                glyph.placement().left_px as f32,
                glyph.placement().top_px as f32,
            ],
            glyph_extent: [region.width as f32, region.height as f32],
            fg_color: colors.fg,
            bg_color: colors.bg,
            cell_span: cell.width.columns() as f32,
            style_flags: u32::from(cell.attrs.flags.bits()),
        })
    }

    /// Returns the vertex-buffer layout used when binding cell instances.
    #[must_use]
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &CELL_INSTANCE_ATTRIBUTES,
        }
    }
}

/// Encodes visible damaged cells into GPU instances using the provided glyph resolver.
///
/// Blank cells with default attributes, continuation cells, and cells without a
/// cached glyph are skipped.
pub fn encode_damage_instances<F>(
    instances: &mut Vec<CellInstance>,
    grid: &Grid,
    damage: &[DamageRegion],
    atlas_size: AtlasSize,
    theme: &Theme,
    resolve_glyph: F,
) -> Result<()>
where
    F: FnMut(Cell) -> Option<CachedGlyph>,
{
    encode_damage_instances_with_options(
        instances,
        grid,
        damage,
        atlas_size,
        theme,
        resolve_glyph,
        false,
    )
}

pub(crate) fn encode_damage_instances_with_options<F>(
    instances: &mut Vec<CellInstance>,
    grid: &Grid,
    damage: &[DamageRegion],
    atlas_size: AtlasSize,
    theme: &Theme,
    resolve_glyph: F,
    include_default_blank_cells: bool,
) -> Result<()>
where
    F: FnMut(Cell) -> Option<CachedGlyph>,
{
    let mut normalized_damage = Vec::new();
    normalized_damage_regions_into(grid, damage, &mut normalized_damage);
    encode_normalized_damage_instances_with_options(
        instances,
        grid,
        &normalized_damage,
        atlas_size,
        theme,
        resolve_glyph,
        include_default_blank_cells,
    )
}

pub(crate) fn encode_normalized_damage_instances_with_options<F>(
    instances: &mut Vec<CellInstance>,
    grid: &Grid,
    normalized_damage: &[DamageRegion],
    atlas_size: AtlasSize,
    theme: &Theme,
    mut resolve_glyph: F,
    include_default_blank_cells: bool,
) -> Result<()>
where
    F: FnMut(Cell) -> Option<CachedGlyph>,
{
    instances.clear();
    let mut skipped_missing_glyphs = 0usize;

    for region in normalized_damage {
        let Some(row_cells) = grid.row(region.start_row) else {
            continue;
        };

        for (col_index, &cell) in row_cells
            .iter()
            .enumerate()
            .skip(region.start_col)
            .take(region.end_col - region.start_col + 1)
        {
            if !cell_needs_rendering_with_blank_default_cells(cell, include_default_blank_cells) {
                continue;
            }

            let Some(glyph) = resolve_glyph(cell) else {
                skipped_missing_glyphs += 1;
                continue;
            };

            let row_index = region.start_row;
            let row_u32 =
                u32::try_from(row_index).map_err(|_| Error::GridCoordinateOutOfRange {
                    row: row_index,
                    col: col_index,
                })?;
            let col_u32 =
                u32::try_from(col_index).map_err(|_| Error::GridCoordinateOutOfRange {
                    row: row_index,
                    col: col_index,
                })?;
            let colors = theme.resolve_cell_colors(cell.attrs);

            instances.push(CellInstance::from_cell(
                cell, col_u32, row_u32, glyph, atlas_size, colors,
            )?);
        }
    }

    if skipped_missing_glyphs > 0 {
        tracing::debug!(
            skipped_missing_glyphs,
            encoded_instances = instances.len(),
            damage_regions = normalized_damage.len(),
            "skipped text instance encoding for cells without cached glyphs"
        );
    }

    Ok(())
}

#[must_use]
pub(crate) fn cell_needs_rendering_with_blank_default_cells(
    cell: Cell,
    include_default_blank_cells: bool,
) -> bool {
    cell.width.columns() != 0
        && (include_default_blank_cells || !cell.is_empty() || cell.attrs != CellAttrs::default())
}

pub(crate) fn normalized_damage_regions_into(
    grid: &Grid,
    damage: &[DamageRegion],
    output: &mut Vec<DamageRegion>,
) {
    output.clear();
    if grid.rows() == 0 || grid.cols() == 0 || damage.is_empty() {
        return;
    }

    for region in damage {
        if region.start_row >= grid.rows() || region.start_col >= grid.cols() {
            continue;
        }

        let end_row = region.end_row.min(grid.rows().saturating_sub(1));
        let end_col = region.end_col.min(grid.cols().saturating_sub(1));
        if region.start_col > end_col {
            continue;
        }

        for row_index in region.start_row..=end_row {
            let mut start_col = region.start_col;
            if start_col > 0
                && matches!(
                    grid.cell(row_index, start_col).map(|cell| cell.width),
                    Some(CellWidth::Continuation)
                )
                && matches!(
                    grid.cell(row_index, start_col - 1).map(|cell| cell.width),
                    Some(CellWidth::Double)
                )
            {
                start_col -= 1;
            }

            output.push(DamageRegion::new(row_index, row_index, start_col, end_col));
        }
    }

    output.sort_unstable_by_key(|region| (region.start_row, region.start_col, region.end_col));

    let mut merged_len = 0usize;
    for read_index in 0..output.len() {
        let region = output[read_index];
        if merged_len > 0 {
            let previous = &mut output[merged_len - 1];
            if previous.start_row == region.start_row
                && region.start_col <= previous.end_col.saturating_add(1)
            {
                previous.end_col = previous.end_col.max(region.end_col);
                continue;
            }
        }

        output[merged_len] = region;
        merged_len += 1;
    }
    output.truncate(merged_len);
}

#[cfg(test)]
pub(crate) fn normalized_damage_regions(grid: &Grid, damage: &[DamageRegion]) -> Vec<DamageRegion> {
    let mut normalized = Vec::new();
    normalized_damage_regions_into(grid, damage, &mut normalized);
    normalized
}

/// Returns the raw bytes for a contiguous cell-instance slice.
#[must_use]
pub fn cell_instances_as_bytes(instances: &[CellInstance]) -> &[u8] {
    bytemuck::cast_slice(instances)
}

/// Uniform and instance buffers used by the text renderer.
#[derive(Debug)]
pub struct TextBuffers {
    uniform_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,
    instance_count: usize,
}

impl TextBuffers {
    /// Creates text-rendering buffers sized for the provided instance capacity.
    pub fn new(device: &wgpu::Device, instance_capacity: usize) -> Result<Self> {
        let instance_capacity = instance_capacity.max(1);
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("iris-render-wgpu-text-uniforms"),
            size: std::mem::size_of::<TextUniforms>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("iris-render-wgpu-text-instances"),
            size: instance_buffer_size(instance_capacity)?,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            uniform_buffer,
            instance_buffer,
            instance_capacity,
            instance_count: 0,
        })
    }

    /// Returns the uniform buffer.
    #[must_use]
    pub const fn uniform_buffer(&self) -> &wgpu::Buffer {
        &self.uniform_buffer
    }

    /// Returns the instance buffer.
    #[must_use]
    pub const fn instance_buffer(&self) -> &wgpu::Buffer {
        &self.instance_buffer
    }

    /// Returns the number of cells that currently fit in the instance buffer.
    #[must_use]
    pub const fn instance_capacity(&self) -> usize {
        self.instance_capacity
    }

    /// Returns the number of instances written in the latest upload.
    #[must_use]
    pub const fn instance_count(&self) -> usize {
        self.instance_count
    }

    /// Clears the tracked instance count without rewriting the GPU buffer.
    pub fn clear_instances(&mut self) {
        self.instance_count = 0;
    }

    /// Uploads the latest text uniforms.
    pub fn write_uniforms(&self, queue: &wgpu::Queue, uniforms: &TextUniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, uniforms.as_bytes());
    }

    /// Uploads text instances, growing the instance buffer when required.
    pub fn write_instances(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        instances: &[CellInstance],
    ) -> Result<()> {
        self.ensure_instance_capacity(device, instances.len())?;

        if !instances.is_empty() {
            queue.write_buffer(&self.instance_buffer, 0, cell_instances_as_bytes(instances));
        }

        self.instance_count = instances.len();
        Ok(())
    }

    fn ensure_instance_capacity(
        &mut self,
        device: &wgpu::Device,
        required_capacity: usize,
    ) -> Result<()> {
        if required_capacity <= self.instance_capacity {
            return Ok(());
        }

        let grown_capacity = required_capacity.checked_next_power_of_two().ok_or(
            Error::TextInstanceBufferTooLarge {
                capacity: required_capacity,
            },
        )?;
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("iris-render-wgpu-text-instances"),
            size: instance_buffer_size(grown_capacity)?,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        self.instance_buffer = instance_buffer;
        self.instance_capacity = grown_capacity;
        Ok(())
    }
}

fn instance_buffer_size(capacity: usize) -> Result<wgpu::BufferAddress> {
    let byte_len = capacity
        .checked_mul(std::mem::size_of::<CellInstance>())
        .ok_or(Error::TextInstanceBufferTooLarge { capacity })?;
    u64::try_from(byte_len).map_err(|_| Error::TextInstanceBufferTooLarge { capacity })
}

#[cfg(test)]
mod tests;
