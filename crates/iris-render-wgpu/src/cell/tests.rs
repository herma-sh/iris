
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
    assert_eq!(instance.atlas_min, [0.2578125, 0.265625]);
    assert_eq!(instance.atlas_max, [0.3671875, 0.609375]);
    assert_eq!(instance.glyph_offset, [0.0, 0.0]);
    assert_eq!(instance.glyph_extent, [8.0, 12.0]);
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
fn cell_instance_tracks_cached_glyph_pixel_offsets() {
    let cell = Cell::new('A');
    let instance = CellInstance::from_cell(
        cell,
        0,
        0,
        CachedGlyph::with_placement(
            AtlasRegion {
                x: 4,
                y: 4,
                width: 6,
                height: 10,
            },
            crate::glyph::GlyphPlacement {
                left_px: -1,
                top_px: 3,
            },
        ),
        AtlasSize::new(32, 32).expect("atlas size is valid"),
        CellColors::new([1.0; 4], [0.0; 4]),
    )
    .expect("cell should encode into an instance");

    assert_eq!(instance.glyph_offset, [-1.0, 3.0]);
    assert_eq!(instance.glyph_extent, [6.0, 10.0]);
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
    assert_eq!(layout.attributes.len(), 9);
    assert_eq!(layout.attributes[0].offset, 0);
    assert_eq!(layout.attributes[3].offset, 24);
    assert_eq!(layout.attributes[5].offset, 40);
    assert_eq!(layout.attributes[7].offset, 72);
    assert_eq!(layout.attributes[8].offset, 76);
    assert_eq!(layout.attributes[8].format, wgpu::VertexFormat::Uint32);
}

#[test]
fn text_buffers_create_with_requested_capacity() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };

    let buffers = TextBuffers::new(renderer.device(), 8).expect("text buffers should be created");

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

    let buffers = TextBuffers::new(renderer.device(), 0).expect("text buffers should be created");

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
    let buffers = TextBuffers::new(renderer.device(), 1).expect("text buffers should be created");

    buffers.write_uniforms(
        renderer.queue(),
        &TextUniforms::new([800.0, 600.0], [9.0, 18.0], 32.0),
    );
}

#[test]
fn text_buffers_can_clear_tracked_instances() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let mut buffers =
        TextBuffers::new(renderer.device(), 2).expect("text buffers should be created");
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
        .write_instances(renderer.device(), renderer.queue(), &[instance])
        .expect("instance upload should succeed");
    assert_eq!(buffers.instance_count(), 1);

    buffers.clear_instances();

    assert_eq!(buffers.instance_count(), 0);
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
fn encode_damage_instances_keeps_blank_cells_with_non_default_attributes() {
    let mut grid = Grid::new(GridSize { rows: 1, cols: 1 }).expect("grid should be created");
    grid.write(
        0,
        0,
        Cell::with_attrs(
            ' ',
            CellAttrs {
                bg: iris_core::cell::Color::Ansi(1),
                ..CellAttrs::default()
            },
        ),
    )
    .expect("styled blank cell should be written");

    let mut instances = Vec::new();
    encode_damage_instances(
        &mut instances,
        &grid,
        &[DamageRegion::new(0, 0, 0, 0)],
        AtlasSize::new(32, 32).expect("atlas size is valid"),
        &Theme::default(),
        |_| {
            Some(CachedGlyph::new(AtlasRegion {
                x: 0,
                y: 0,
                width: 1,
                height: 1,
            }))
        },
    )
    .expect("styled blank cells should encode");

    assert_eq!(instances.len(), 1);
    assert_eq!(
        instances[0].bg_color,
        Theme::default().ansi[1].to_f32_array()
    );
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
