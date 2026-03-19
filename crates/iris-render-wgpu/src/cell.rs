use bytemuck::{Pod, Zeroable};
use iris_core::cell::Cell;

use crate::atlas::AtlasSize;
use crate::error::{Error, Result};
use crate::glyph::CachedGlyph;

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
}

/// Returns the raw bytes for a contiguous cell-instance slice.
#[must_use]
pub fn cell_instances_as_bytes(instances: &[CellInstance]) -> &[u8] {
    bytemuck::cast_slice(instances)
}

#[cfg(test)]
mod tests {
    use std::mem::size_of;

    use iris_core::cell::{Cell, CellAttrs, CellFlags};

    use super::{cell_instances_as_bytes, CellColors, CellInstance, TextUniforms};
    use crate::atlas::{AtlasRegion, AtlasSize};
    use crate::error::Error;
    use crate::glyph::CachedGlyph;

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
}
