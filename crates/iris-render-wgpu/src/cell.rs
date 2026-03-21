use bytemuck::{Pod, Zeroable};
use iris_core::cell::{Cell, CellWidth};
use iris_core::damage::DamageRegion;
use iris_core::grid::Grid;

use crate::atlas::AtlasSize;
use crate::error::{Error, Result};
use crate::glyph::CachedGlyph;
use crate::theme::Theme;

const CELL_INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 7] = wgpu::vertex_attr_array![
    0 => Float32x2,
    1 => Float32x2,
    2 => Float32x2,
    3 => Float32x4,
    4 => Float32x4,
    5 => Float32,
    6 => Uint32
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

        Ok(Self {
            grid_position: [column as f32, row as f32],
            atlas_min: [
                region.x as f32 / atlas_width,
                region.y as f32 / atlas_height,
            ],
            atlas_max: [
                (region.x + region.width) as f32 / atlas_width,
                (region.y + region.height) as f32 / atlas_height,
            ],
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
/// Empty cells, continuation cells, and cells without a cached glyph are skipped.
pub fn encode_damage_instances<F>(
    instances: &mut Vec<CellInstance>,
    grid: &Grid,
    damage: &[DamageRegion],
    atlas_size: AtlasSize,
    theme: &Theme,
    mut resolve_glyph: F,
) -> Result<()>
where
    F: FnMut(Cell) -> Option<CachedGlyph>,
{
    instances.clear();
    let mut skipped_missing_glyphs = 0usize;

    for region in normalized_damage_regions(grid, damage) {
        let Some(row_cells) = grid.row(region.start_row) else {
            continue;
        };

        for (col_index, &cell) in row_cells
            .iter()
            .enumerate()
            .skip(region.start_col)
            .take(region.end_col - region.start_col + 1)
        {
            if cell.is_empty() || cell.width.columns() == 0 {
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
            damage_regions = damage.len(),
            "skipped text instance encoding for cells without cached glyphs"
        );
    }

    Ok(())
}

pub(crate) fn normalized_damage_regions(grid: &Grid, damage: &[DamageRegion]) -> Vec<DamageRegion> {
    if grid.rows() == 0 || grid.cols() == 0 || damage.is_empty() {
        return Vec::new();
    }

    let mut normalized = Vec::new();
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

            normalized.push(DamageRegion::new(row_index, row_index, start_col, end_col));
        }
    }

    normalized.sort_unstable_by_key(|region| (region.start_row, region.start_col, region.end_col));

    let mut merged: Vec<DamageRegion> = Vec::with_capacity(normalized.len());
    for region in normalized {
        match merged.last_mut() {
            Some(previous)
                if previous.start_row == region.start_row
                    && region.start_col <= previous.end_col.saturating_add(1) =>
            {
                previous.end_col = previous.end_col.max(region.end_col);
            }
            _ => merged.push(region),
        }
    }

    merged
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
mod tests {
    use std::mem::size_of;

    use iris_core::cell::{Cell, CellAttrs, CellFlags};
    use iris_core::damage::DamageRegion;
    use iris_core::grid::{Grid, GridSize};

    use super::{
        cell_instances_as_bytes, encode_damage_instances, normalized_damage_regions, CellColors,
        CellInstance, TextBuffers, TextUniforms,
    };
    use crate::atlas::{AtlasRegion, AtlasSize};
    use crate::error::Error;
    use crate::glyph::CachedGlyph;
    use crate::renderer::{Renderer, RendererConfig};
    use crate::theme::Theme;

    #[test]
    fn text_uniforms_store_viewport_and_cell_metrics() {
        let uniforms = TextUniforms::new([1280.0, 720.0], [9.0, 18.0], 24.0);

        assert_eq!(uniforms.resolution, [1280.0, 720.0]);
        assert_eq!(uniforms.cell_size, [9.0, 18.0]);
        assert_eq!(uniforms.scroll_offset, 24.0);
        assert_eq!(uniforms._padding, 0);
    }

    #[test]
    fn cell_instance_encodes_grid_position_uvs_and_style() {
        let cell = Cell::with_attrs(
            'x',
            CellAttrs {
                fg: iris_core::cell::Color::Default,
                bg: iris_core::cell::Color::Default,
                flags: CellFlags::BOLD | CellFlags::UNDERLINE,
            },
        );
        let instance = CellInstance::from_cell(
            cell,
            3,
            5,
            CachedGlyph::new(AtlasRegion {
                x: 16,
                y: 8,
                width: 8,
                height: 12,
            }),
            AtlasSize::new(64, 32).expect("atlas size is valid"),
            CellColors::new([1.0, 0.5, 0.0, 1.0], [0.0, 0.0, 0.0, 1.0]),
        )
        .expect("cell should encode into an instance");

        assert_eq!(instance.grid_position, [3.0, 5.0]);
        assert_eq!(instance.atlas_min, [0.25, 0.25]);
        assert_eq!(instance.atlas_max, [0.375, 0.625]);
        assert_eq!(instance.cell_span, 1.0);
        assert_eq!(
            instance.style_flags,
            u32::from((CellFlags::BOLD | CellFlags::UNDERLINE).bits())
        );
    }

    #[test]
    fn cell_instance_uses_double_width_span_for_wide_cells() {
        let cell = Cell::new('中');
        let instance = CellInstance::from_cell(
            cell,
            0,
            0,
            CachedGlyph::new(AtlasRegion {
                x: 0,
                y: 0,
                width: 12,
                height: 16,
            }),
            AtlasSize::new(64, 64).expect("atlas size is valid"),
            CellColors::new([1.0; 4], [0.0; 4]),
        )
        .expect("wide cell should encode into an instance");

        assert_eq!(instance.cell_span, 2.0);
    }

    #[test]
    fn cell_instance_rejects_continuation_cells() {
        let cell = Cell::continuation(CellAttrs::default());
        let result = CellInstance::from_cell(
            cell,
            1,
            1,
            CachedGlyph::new(AtlasRegion {
                x: 0,
                y: 0,
                width: 8,
                height: 16,
            }),
            AtlasSize::new(32, 32).expect("atlas size is valid"),
            CellColors::new([1.0; 4], [0.0; 4]),
        );

        assert!(matches!(result, Err(Error::ContinuationCellNotRenderable)));
    }

    #[test]
    fn cell_instance_bytes_cover_the_full_slice() {
        let instance = CellInstance::from_cell(
            Cell::new('a'),
            2,
            4,
            CachedGlyph::new(AtlasRegion {
                x: 4,
                y: 8,
                width: 8,
                height: 8,
            }),
            AtlasSize::new(32, 32).expect("atlas size is valid"),
            CellColors::new([1.0; 4], [0.0; 4]),
        )
        .expect("cell should encode into an instance");
        let instances = [instance, instance];
        let bytes = cell_instances_as_bytes(&instances);

        assert_eq!(bytes.len(), size_of::<CellInstance>() * instances.len());
    }

    #[test]
    fn text_uniform_bytes_cover_the_full_struct() {
        let uniforms = TextUniforms::new([640.0, 480.0], [8.0, 16.0], 12.0);

        assert_eq!(uniforms.as_bytes().len(), size_of::<TextUniforms>());
    }

    #[test]
    fn cell_instance_layout_matches_the_struct_layout() {
        let layout = CellInstance::layout();

        assert_eq!(
            layout.array_stride,
            size_of::<CellInstance>() as wgpu::BufferAddress
        );
        assert_eq!(layout.step_mode, wgpu::VertexStepMode::Instance);
        assert_eq!(layout.attributes.len(), 7);
        assert_eq!(layout.attributes[0].offset, 0);
        assert_eq!(layout.attributes[3].offset, 24);
        assert_eq!(layout.attributes[5].offset, 56);
        assert_eq!(layout.attributes[6].offset, 60);
        assert_eq!(layout.attributes[6].format, wgpu::VertexFormat::Uint32);
    }

    #[test]
    fn text_buffers_create_with_requested_capacity() {
        let _gpu_test_lock = crate::test_support::gpu_test_lock();
        let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
            Ok(renderer) => renderer,
            Err(Error::NoAdapter) => return,
            Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
        };

        let buffers =
            TextBuffers::new(renderer.device(), 8).expect("text buffers should be created");

        assert_eq!(buffers.instance_capacity(), 8);
        assert_eq!(buffers.instance_count(), 0);
    }

    #[test]
    fn text_buffers_clamp_zero_capacity_to_one_instance() {
        let _gpu_test_lock = crate::test_support::gpu_test_lock();
        let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
            Ok(renderer) => renderer,
            Err(Error::NoAdapter) => return,
            Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
        };

        let buffers =
            TextBuffers::new(renderer.device(), 0).expect("text buffers should be created");

        assert_eq!(buffers.instance_capacity(), 1);
        assert_eq!(buffers.instance_count(), 0);
    }

    #[test]
    fn text_buffers_grow_when_more_instances_are_uploaded() {
        let _gpu_test_lock = crate::test_support::gpu_test_lock();
        let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
            Ok(renderer) => renderer,
            Err(Error::NoAdapter) => return,
            Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
        };
        let mut buffers =
            TextBuffers::new(renderer.device(), 1).expect("text buffers should be created");
        let instance = CellInstance::from_cell(
            Cell::new('a'),
            0,
            0,
            CachedGlyph::new(AtlasRegion {
                x: 0,
                y: 0,
                width: 8,
                height: 16,
            }),
            AtlasSize::new(32, 32).expect("atlas size is valid"),
            CellColors::new([1.0; 4], [0.0; 4]),
        )
        .expect("cell should encode into an instance");

        buffers
            .write_instances(renderer.device(), renderer.queue(), &[instance, instance])
            .expect("instance upload should succeed");

        assert_eq!(buffers.instance_count(), 2);
        assert!(buffers.instance_capacity() >= 2);
        assert!(buffers.instance_capacity().is_power_of_two());
    }

    #[test]
    fn text_buffers_accept_empty_instance_uploads() {
        let _gpu_test_lock = crate::test_support::gpu_test_lock();
        let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
            Ok(renderer) => renderer,
            Err(Error::NoAdapter) => return,
            Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
        };
        let mut buffers =
            TextBuffers::new(renderer.device(), 4).expect("text buffers should be created");

        buffers
            .write_instances(renderer.device(), renderer.queue(), &[])
            .expect("empty instance upload should succeed");

        assert_eq!(buffers.instance_capacity(), 4);
        assert_eq!(buffers.instance_count(), 0);
    }

    #[test]
    fn text_buffers_accept_uniform_updates() {
        let _gpu_test_lock = crate::test_support::gpu_test_lock();
        let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
            Ok(renderer) => renderer,
            Err(Error::NoAdapter) => return,
            Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
        };
        let buffers =
            TextBuffers::new(renderer.device(), 1).expect("text buffers should be created");

        buffers.write_uniforms(
            renderer.queue(),
            &TextUniforms::new([800.0, 600.0], [9.0, 18.0], 32.0),
        );
    }

    #[test]
    fn text_buffers_reject_unrepresentable_instance_capacity() {
        let _gpu_test_lock = crate::test_support::gpu_test_lock();
        let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
            Ok(renderer) => renderer,
            Err(Error::NoAdapter) => return,
            Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
        };

        let result = TextBuffers::new(renderer.device(), usize::MAX);

        assert!(matches!(
            result,
            Err(Error::TextInstanceBufferTooLarge {
                capacity: usize::MAX,
            })
        ));
    }

    #[test]
    fn encode_damage_instances_collects_the_requested_cells() {
        let mut grid = Grid::new(GridSize { rows: 2, cols: 4 }).expect("grid should be created");
        grid.write(0, 0, Cell::new('a'))
            .expect("first cell should be written");
        grid.write(
            0,
            1,
            Cell::with_attrs(
                'b',
                CellAttrs {
                    fg: iris_core::cell::Color::Ansi(2),
                    bg: iris_core::cell::Color::Ansi(4),
                    flags: CellFlags::BOLD,
                },
            ),
        )
        .expect("second cell should be written");
        grid.write(1, 0, Cell::new('c'))
            .expect("third cell should be written");

        let atlas_size = AtlasSize::new(64, 64).expect("atlas size is valid");
        let mut instances = Vec::new();
        encode_damage_instances(
            &mut instances,
            &grid,
            &[DamageRegion::new(0, 0, 0, 1)],
            atlas_size,
            &Theme::default(),
            |cell| {
                Some(CachedGlyph::new(match cell.character {
                    'a' => AtlasRegion {
                        x: 0,
                        y: 0,
                        width: 8,
                        height: 16,
                    },
                    'b' => AtlasRegion {
                        x: 8,
                        y: 0,
                        width: 8,
                        height: 16,
                    },
                    _ => AtlasRegion {
                        x: 16,
                        y: 0,
                        width: 8,
                        height: 16,
                    },
                }))
            },
        )
        .expect("damage should encode");

        assert_eq!(instances.len(), 2);
        assert_eq!(instances[0].grid_position, [0.0, 0.0]);
        assert_eq!(instances[1].grid_position, [1.0, 0.0]);
        assert_eq!(
            instances[1].fg_color,
            Theme::default().ansi[2].to_f32_array()
        );
        assert_eq!(
            instances[1].bg_color,
            Theme::default().ansi[4].to_f32_array()
        );
    }

    #[test]
    fn encode_damage_instances_skips_empty_continuation_and_missing_glyph_cells() {
        let mut grid = Grid::new(GridSize { rows: 1, cols: 4 }).expect("grid should be created");
        grid.write(0, 0, Cell::new('a'))
            .expect("cell should be written");
        grid.write(0, 1, Cell::default())
            .expect("blank cell should be written");
        grid.write(0, 2, Cell::continuation(CellAttrs::default()))
            .expect("continuation cell should be written");
        grid.write(0, 3, Cell::new('z'))
            .expect("final cell should be written");

        let mut instances = Vec::new();
        encode_damage_instances(
            &mut instances,
            &grid,
            &[DamageRegion::new(0, 0, 0, 3)],
            AtlasSize::new(32, 32).expect("atlas size is valid"),
            &Theme::default(),
            |cell| {
                (cell.character == 'a').then_some(CachedGlyph::new(AtlasRegion {
                    x: 0,
                    y: 0,
                    width: 8,
                    height: 16,
                }))
            },
        )
        .expect("damage should encode");

        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].grid_position, [0.0, 0.0]);
    }

    #[test]
    fn encode_damage_instances_reuses_the_output_buffer() {
        let mut grid = Grid::new(GridSize { rows: 1, cols: 2 }).expect("grid should be created");
        grid.write(0, 0, Cell::new('a'))
            .expect("cell should be written");
        let atlas_size = AtlasSize::new(32, 32).expect("atlas size is valid");
        let mut instances = vec![CellInstance::from_cell(
            Cell::new('x'),
            0,
            0,
            CachedGlyph::new(AtlasRegion {
                x: 0,
                y: 0,
                width: 8,
                height: 16,
            }),
            atlas_size,
            CellColors::new([1.0; 4], [0.0; 4]),
        )
        .expect("seed instance should encode")];

        encode_damage_instances(
            &mut instances,
            &grid,
            &[DamageRegion::new(0, 0, 0, 0)],
            atlas_size,
            &Theme::default(),
            |_| {
                Some(CachedGlyph::new(AtlasRegion {
                    x: 8,
                    y: 0,
                    width: 8,
                    height: 16,
                }))
            },
        )
        .expect("damage should encode");

        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].grid_position, [0.0, 0.0]);
    }

    #[test]
    fn encode_damage_instances_coalesces_overlapping_damage() {
        let mut grid = Grid::new(GridSize { rows: 1, cols: 3 }).expect("grid should be created");
        grid.write(0, 0, Cell::new('a'))
            .expect("first cell should be written");
        grid.write(0, 1, Cell::new('b'))
            .expect("second cell should be written");
        grid.write(0, 2, Cell::new('c'))
            .expect("third cell should be written");
        let mut instances = Vec::new();

        encode_damage_instances(
            &mut instances,
            &grid,
            &[DamageRegion::new(0, 0, 0, 1), DamageRegion::new(0, 0, 1, 2)],
            AtlasSize::new(32, 32).expect("atlas size is valid"),
            &Theme::default(),
            |_| {
                Some(CachedGlyph::new(AtlasRegion {
                    x: 0,
                    y: 0,
                    width: 8,
                    height: 16,
                }))
            },
        )
        .expect("damage should encode");

        assert_eq!(instances.len(), 3);
        assert_eq!(instances[0].grid_position, [0.0, 0.0]);
        assert_eq!(instances[1].grid_position, [1.0, 0.0]);
        assert_eq!(instances[2].grid_position, [2.0, 0.0]);
    }

    #[test]
    fn encode_damage_instances_handles_empty_damage_and_zero_sized_grids() {
        let grid = Grid::new(GridSize { rows: 0, cols: 0 }).expect("grid should be created");
        let mut instances = vec![CellInstance::from_cell(
            Cell::new('x'),
            0,
            0,
            CachedGlyph::new(AtlasRegion {
                x: 0,
                y: 0,
                width: 8,
                height: 16,
            }),
            AtlasSize::new(32, 32).expect("atlas size is valid"),
            CellColors::new([1.0; 4], [0.0; 4]),
        )
        .expect("seed instance should encode")];

        encode_damage_instances(
            &mut instances,
            &grid,
            &[],
            AtlasSize::new(32, 32).expect("atlas size is valid"),
            &Theme::default(),
            |_| None,
        )
        .expect("empty damage should encode");

        assert!(instances.is_empty());
    }

    #[test]
    fn encode_damage_instances_clamps_out_of_bounds_damage_regions() {
        let mut grid = Grid::new(GridSize { rows: 1, cols: 2 }).expect("grid should be created");
        grid.write(0, 0, Cell::new('a'))
            .expect("first cell should be written");
        grid.write(0, 1, Cell::new('b'))
            .expect("second cell should be written");
        let mut instances = Vec::new();

        encode_damage_instances(
            &mut instances,
            &grid,
            &[DamageRegion::new(0, 4, 0, 4)],
            AtlasSize::new(32, 32).expect("atlas size is valid"),
            &Theme::default(),
            |_| {
                Some(CachedGlyph::new(AtlasRegion {
                    x: 0,
                    y: 0,
                    width: 8,
                    height: 16,
                }))
            },
        )
        .expect("out-of-bounds damage should clamp");

        assert_eq!(instances.len(), 2);
    }

    #[test]
    fn normalized_damage_regions_include_wide_cell_lead_for_continuation_damage() {
        let mut grid = Grid::new(GridSize { rows: 1, cols: 2 }).expect("grid should be created");
        grid.write(0, 0, Cell::new('中'))
            .expect("wide cell should be written");

        let normalized = normalized_damage_regions(&grid, &[DamageRegion::new(0, 0, 1, 1)]);
        assert_eq!(normalized, vec![DamageRegion::new(0, 0, 0, 1)]);

        let mut instances = Vec::new();
        encode_damage_instances(
            &mut instances,
            &grid,
            &[DamageRegion::new(0, 0, 1, 1)],
            AtlasSize::new(32, 32).expect("atlas size is valid"),
            &Theme::default(),
            |cell| {
                (cell.character == '中').then_some(CachedGlyph::new(AtlasRegion {
                    x: 0,
                    y: 0,
                    width: 16,
                    height: 16,
                }))
            },
        )
        .expect("continuation damage should still encode the lead cell");

        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].grid_position, [0.0, 0.0]);
        assert_eq!(instances[0].cell_span, 2.0);
    }
}
