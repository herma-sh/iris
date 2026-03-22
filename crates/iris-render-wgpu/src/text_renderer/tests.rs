
use std::sync::atomic::{AtomicUsize, Ordering};

use iris_core::cell::Cell;
use iris_core::cursor::{Cursor, CursorStyle};
use iris_core::damage::DamageRegion;
use iris_core::grid::{Grid, GridSize};

use super::{
    expand_damage_regions_for_ligature_context, glyph_key_for_cell, operator_ligature_replacement,
    TextRenderer, TextRendererConfig,
};
use crate::glyph::{GlyphPlacement, RasterizedGlyph};
use crate::renderer::{Renderer, RendererConfig};
use crate::texture::{TextureSurfaceConfig, TextureSurfaceSize};
use crate::theme::{Theme, ThemeColor};
use crate::FontRasterizer;
use crate::FontRasterizerConfig;

fn pixel_at(pixels: &[u8], surface_size: TextureSurfaceSize, position: (usize, usize)) -> [u8; 4] {
    let row_stride = surface_size.width as usize * 4;
    let offset = position.1 * row_stride + position.0 * 4;
    [
        pixels[offset],
        pixels[offset + 1],
        pixels[offset + 2],
        pixels[offset + 3],
    ]
}

#[test]
fn glyph_key_for_cell_tracks_shape_relevant_state() {
    let regular = glyph_key_for_cell(Cell::new('a'));
    let bold = glyph_key_for_cell(Cell::with_attrs(
        'a',
        iris_core::cell::CellAttrs {
            flags: iris_core::cell::CellFlags::BOLD,
            ..Default::default()
        },
    ));
    let underlined = glyph_key_for_cell(Cell::with_attrs(
        'a',
        iris_core::cell::CellAttrs {
            flags: iris_core::cell::CellFlags::UNDERLINE,
            ..Default::default()
        },
    ));

    assert_ne!(regular, bold);
    assert_eq!(regular, underlined);
}

#[test]
fn operator_ligature_replacement_maps_supported_sequences() {
    assert_eq!(
        operator_ligature_replacement('<', Some('-'), Some('>')),
        Some(('\u{2194}', 3))
    );
    assert_eq!(
        operator_ligature_replacement('<', Some('='), Some('>')),
        Some(('\u{21D4}', 3))
    );
    assert_eq!(
        operator_ligature_replacement('=', Some('='), Some('=')),
        Some(('\u{2261}', 3))
    );
    assert_eq!(
        operator_ligature_replacement('!', Some('='), Some('=')),
        Some(('\u{2262}', 3))
    );
    assert_eq!(
        operator_ligature_replacement('-', Some('>'), Some('=')),
        Some(('\u{2192}', 2))
    );
    assert_eq!(
        operator_ligature_replacement('<', Some('-'), Some('=')),
        Some(('\u{2190}', 2))
    );
    assert_eq!(
        operator_ligature_replacement('=', Some('>'), Some('=')),
        Some(('\u{21D2}', 2))
    );
    assert_eq!(
        operator_ligature_replacement('<', Some('='), Some('-')),
        Some(('\u{2264}', 2))
    );
    assert_eq!(
        operator_ligature_replacement('>', Some('='), Some('-')),
        Some(('\u{2265}', 2))
    );
    assert_eq!(
        operator_ligature_replacement('!', Some('='), Some('-')),
        Some(('\u{2260}', 2))
    );
    assert_eq!(operator_ligature_replacement('x', Some('y'), None), None);
}

#[test]
fn ligature_context_damage_expands_columns_for_boundary_sequences() {
    let mut grid = Grid::new(GridSize { rows: 4, cols: 6 }).expect("grid should be created");
    grid.write(0, 2, Cell::new('-'))
        .expect("operator cell should be written");
    grid.write(0, 3, Cell::new('>'))
        .expect("operator cell should be written");
    grid.write(3, 0, Cell::new('x'))
        .expect("non-operator cell should be written");

    let damage = [
        DamageRegion::new(0, 0, 2, 2),
        DamageRegion::new(3, 3, 0, 0),
        DamageRegion::new(3, 3, 8, 9),
    ];
    let mut expanded = Vec::new();
    expand_damage_regions_for_ligature_context(&grid, &damage, &mut expanded);

    assert_eq!(
        expanded,
        vec![
            DamageRegion::new(0, 0, 0, 4),
            DamageRegion::new(3, 3, 0, 0),
            DamageRegion::new(3, 3, 8, 9),
        ]
    );

    let zero_width_grid = Grid::new(GridSize { rows: 1, cols: 0 }).expect("grid should be created");
    expand_damage_regions_for_ligature_context(&zero_width_grid, &damage, &mut expanded);
    assert!(expanded.is_empty());
}

#[test]
fn text_renderer_reuses_cached_glyphs_across_damage_updates() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(64, 32).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let mut text_renderer = TextRenderer::new(
        &renderer,
        surface.format(),
        TextRendererConfig {
            uniforms: crate::cell::TextUniforms::new([64.0, 32.0], [8.0, 16.0], 0.0),
            theme: Theme {
                background: ThemeColor::rgb(0x10, 0x20, 0x30),
                ..Theme::default()
            },
            ..TextRendererConfig::default()
        },
    )
    .expect("text renderer should be created");
    let mut grid = Grid::new(GridSize { rows: 1, cols: 2 }).expect("grid should be created");
    grid.write(0, 0, Cell::new('a'))
        .expect("first cell should be written");
    grid.write(0, 1, Cell::new('b'))
        .expect("second cell should be written");
    let damage = [DamageRegion::new(0, 0, 0, 1)];
    let rasterized_count = AtomicUsize::new(0);

    text_renderer
        .prepare_grid(&renderer, &grid, &damage, |cell| {
            rasterized_count.fetch_add(1, Ordering::Relaxed);
            Ok(Some(test_glyph_for(cell)))
        })
        .expect("first grid prepare should succeed");
    text_renderer
        .prepare_grid(&renderer, &grid, &damage, |cell| {
            rasterized_count.fetch_add(1, Ordering::Relaxed);
            Ok(Some(test_glyph_for(cell)))
        })
        .expect("second grid prepare should reuse cached glyphs");

    assert_eq!(rasterized_count.load(Ordering::Relaxed), 2);
    assert_eq!(text_renderer.instance_count(), 2);

    text_renderer.render_to_texture_surface(&renderer, &surface);

    let pixels = crate::test_support::read_texture_surface(&renderer, &surface);
    let background = crate::test_support::bgra_pixel(text_renderer.theme().background);
    assert!(
        pixels
            .chunks_exact(background.len())
            .any(|pixel| pixel != background),
        "rendered glyph pixels should differ from the themed background clear color"
    );
}

#[test]
fn text_renderer_clears_to_theme_background_without_instances() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(16, 16).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let theme = Theme {
        background: ThemeColor::rgb(0xff, 0x00, 0x00),
        ..Theme::default()
    };
    let text_renderer = TextRenderer::new(
        &renderer,
        surface.format(),
        TextRendererConfig {
            theme: theme.clone(),
            ..TextRendererConfig::default()
        },
    )
    .expect("text renderer should be created");

    text_renderer.render_to_texture_surface(&renderer, &surface);

    let pixels = crate::test_support::read_texture_surface(&renderer, &surface);
    let background = crate::test_support::bgra_pixel(theme.background);
    assert!(
        pixels
            .chunks_exact(background.len())
            .all(|pixel| pixel == background),
        "an empty text pass should clear to the themed background color"
    );
}

#[test]
fn text_renderer_populates_wide_cell_glyphs_when_damage_starts_on_continuation() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(32, 16).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let mut text_renderer = TextRenderer::new(
        &renderer,
        surface.format(),
        TextRendererConfig {
            uniforms: crate::cell::TextUniforms::new([32.0, 16.0], [16.0, 16.0], 0.0),
            ..TextRendererConfig::default()
        },
    )
    .expect("text renderer should be created");
    let mut grid = Grid::new(GridSize { rows: 1, cols: 2 }).expect("grid should be created");
    grid.write(0, 0, Cell::new('中'))
        .expect("wide cell should be written");
    let rasterized_count = AtomicUsize::new(0);

    text_renderer
        .prepare_grid(&renderer, &grid, &[DamageRegion::new(0, 0, 1, 1)], |cell| {
            rasterized_count.fetch_add(1, Ordering::Relaxed);
            Ok(Some(test_glyph_for(cell)))
        })
        .expect("continuation damage should still rasterize the lead glyph");

    assert_eq!(rasterized_count.load(Ordering::Relaxed), 1);
    assert_eq!(text_renderer.instance_count(), 1);

    text_renderer.render_to_texture_surface(&renderer, &surface);

    let pixels = crate::test_support::read_texture_surface(&renderer, &surface);
    let background = crate::test_support::bgra_pixel(text_renderer.theme().background);
    assert!(
        pixels
            .chunks_exact(background.len())
            .any(|pixel| pixel != background),
        "continuation damage should render the lead glyph"
    );
}

#[test]
fn text_renderer_draws_styled_blank_cells() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(16, 16).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let theme = Theme {
        background: ThemeColor::rgb(0x00, 0x00, 0x00),
        ..Theme::default()
    };
    let mut text_renderer = TextRenderer::new(
        &renderer,
        surface.format(),
        TextRendererConfig {
            theme: theme.clone(),
            uniforms: crate::cell::TextUniforms::new([16.0, 16.0], [16.0, 16.0], 0.0),
            ..TextRendererConfig::default()
        },
    )
    .expect("text renderer should be created");
    let mut grid = Grid::new(GridSize { rows: 1, cols: 1 }).expect("grid should be created");
    grid.write(
        0,
        0,
        Cell::with_attrs(
            ' ',
            iris_core::cell::CellAttrs {
                bg: iris_core::cell::Color::Ansi(1),
                ..Default::default()
            },
        ),
    )
    .expect("styled blank cell should be written");

    text_renderer
        .prepare_grid(&renderer, &grid, &[DamageRegion::new(0, 0, 0, 0)], |_| {
            Ok(Some(RasterizedGlyph::new(1, 1, vec![0])))
        })
        .expect("styled blank cell should prepare");
    text_renderer.render_to_texture_surface(&renderer, &surface);

    let pixels = crate::test_support::read_texture_surface(&renderer, &surface);
    let background = crate::test_support::bgra_pixel(theme.background);
    assert!(
        pixels
            .chunks_exact(background.len())
            .any(|pixel| pixel != background),
        "styled blank cells should draw their own background color"
    );
}

#[test]
fn text_renderer_applies_glyph_placement_offsets_inside_cells() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(16, 16).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let mut text_renderer = TextRenderer::new(
        &renderer,
        surface.format(),
        TextRendererConfig {
            theme: Theme {
                background: ThemeColor::rgb(0xff, 0x00, 0x00),
                ..Theme::default()
            },
            uniforms: crate::cell::TextUniforms::new([16.0, 16.0], [16.0, 16.0], 0.0),
            ..TextRendererConfig::default()
        },
    )
    .expect("text renderer should be created");
    let mut grid = Grid::new(GridSize { rows: 1, cols: 1 }).expect("grid should be created");
    grid.write(0, 0, Cell::new('A'))
        .expect("cell should be written");

    text_renderer
        .prepare_grid(&renderer, &grid, &[DamageRegion::new(0, 0, 0, 0)], |_| {
            Ok(Some(RasterizedGlyph::new_with_placement(
                4,
                4,
                vec![255; 16],
                GlyphPlacement {
                    left_px: 6,
                    top_px: 8,
                },
            )))
        })
        .expect("placed glyph should prepare");
    text_renderer.render_to_texture_surface(&renderer, &surface);

    let pixels = crate::test_support::read_texture_surface(&renderer, &surface);
    let background = crate::test_support::bgra_pixel(text_renderer.theme().background);
    assert_eq!(
        pixel_at(&pixels, surface.size(), (2, 2)),
        background,
        "pixels outside the glyph placement should preserve cell background"
    );
    assert_ne!(
        pixel_at(&pixels, surface.size(), (7, 10)),
        background,
        "pixels inside the glyph placement should draw glyph foreground"
    );
}

#[test]
fn text_renderer_can_prepare_grid_with_system_font_rasterizer() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(64, 32).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let mut font_rasterizer = match FontRasterizer::new(FontRasterizerConfig::default()) {
        Ok(font_rasterizer) => font_rasterizer,
        Err(crate::error::Error::NoUsableSystemFont) => return,
        Err(error) => panic!("font rasterizer failed unexpectedly: {error}"),
    };
    let mut text_renderer = TextRenderer::new(
        &renderer,
        surface.format(),
        TextRendererConfig {
            uniforms: crate::cell::TextUniforms::new([64.0, 32.0], [8.0, 16.0], 0.0),
            ..TextRendererConfig::default()
        },
    )
    .expect("text renderer should be created");
    let mut grid = Grid::new(GridSize { rows: 1, cols: 1 }).expect("grid should be created");
    grid.write(0, 0, Cell::new('A'))
        .expect("ASCII cell should be written");

    text_renderer
        .prepare_grid_with_font_rasterizer(
            &renderer,
            &grid,
            &[DamageRegion::new(0, 0, 0, 0)],
            &mut font_rasterizer,
        )
        .expect("system font rasterizer should prepare grid text");
    text_renderer.render_to_texture_surface(&renderer, &surface);

    let pixels = crate::test_support::read_texture_surface(&renderer, &surface);
    let background = crate::test_support::bgra_pixel(text_renderer.theme().background);
    assert!(
        pixels
            .chunks_exact(background.len())
            .any(|pixel| pixel != background),
        "system font rasterization should produce visible text pixels"
    );
}

#[test]
fn text_renderer_applies_operator_ligature_substitution_when_supported() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(32, 16).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let mut font_rasterizer = match FontRasterizer::new(FontRasterizerConfig::default()) {
        Ok(font_rasterizer) => font_rasterizer,
        Err(crate::error::Error::NoUsableSystemFont) => return,
        Err(error) => panic!("font rasterizer failed unexpectedly: {error}"),
    };
    let replacement_supported = font_rasterizer
        .rasterize_cell(Cell::new('\u{2192}'))
        .is_ok();
    let mut text_renderer = TextRenderer::new(
        &renderer,
        surface.format(),
        TextRendererConfig {
            uniforms: crate::cell::TextUniforms::new([32.0, 16.0], [16.0, 16.0], 0.0),
            ..TextRendererConfig::default()
        },
    )
    .expect("text renderer should be created");
    let mut grid = Grid::new(GridSize { rows: 1, cols: 2 }).expect("grid should be created");
    grid.write(0, 0, Cell::new('-'))
        .expect("left operator cell should be written");
    grid.write(0, 1, Cell::new('>'))
        .expect("right operator cell should be written");

    text_renderer
        .prepare_grid_with_font_rasterizer(
            &renderer,
            &grid,
            &[DamageRegion::new(0, 0, 0, 0)],
            &mut font_rasterizer,
        )
        .expect("operator pair should prepare");

    if replacement_supported {
        assert_eq!(text_renderer.instance_count(), 1);
        assert_eq!(text_renderer.instances[0].cell_span, 2.0);
    } else {
        assert_eq!(text_renderer.instance_count(), 2);
    }
}

#[test]
fn text_renderer_applies_three_character_operator_ligatures_when_supported() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(48, 16).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let mut font_rasterizer = match FontRasterizer::new(FontRasterizerConfig::default()) {
        Ok(font_rasterizer) => font_rasterizer,
        Err(crate::error::Error::NoUsableSystemFont) => return,
        Err(error) => panic!("font rasterizer failed unexpectedly: {error}"),
    };
    let replacement_supported = font_rasterizer
        .rasterize_cell(Cell::new('\u{21D4}'))
        .is_ok();
    let mut text_renderer = TextRenderer::new(
        &renderer,
        surface.format(),
        TextRendererConfig {
            uniforms: crate::cell::TextUniforms::new([48.0, 16.0], [16.0, 16.0], 0.0),
            ..TextRendererConfig::default()
        },
    )
    .expect("text renderer should be created");
    let mut grid = Grid::new(GridSize { rows: 1, cols: 3 }).expect("grid should be created");
    grid.write(0, 0, Cell::new('<'))
        .expect("left operator cell should be written");
    grid.write(0, 1, Cell::new('='))
        .expect("middle operator cell should be written");
    grid.write(0, 2, Cell::new('>'))
        .expect("right operator cell should be written");

    text_renderer
        .prepare_grid_with_font_rasterizer(
            &renderer,
            &grid,
            &[DamageRegion::new(0, 0, 1, 1)],
            &mut font_rasterizer,
        )
        .expect("operator sequence should prepare");

    if replacement_supported {
        assert_eq!(text_renderer.instance_count(), 1);
        assert_eq!(text_renderer.instances[0].cell_span, 3.0);
    } else {
        assert_eq!(text_renderer.instance_count(), 3);
    }
}

#[test]
fn text_renderer_draws_block_cursor_overlay() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(16, 16).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let mut text_renderer = TextRenderer::new(
        &renderer,
        surface.format(),
        TextRendererConfig {
            theme: Theme {
                background: ThemeColor::rgb(0x00, 0x00, 0x00),
                cursor: ThemeColor::rgb(0xff, 0x00, 0x00),
                ..Theme::default()
            },
            uniforms: crate::cell::TextUniforms::new([16.0, 16.0], [16.0, 16.0], 0.0),
            ..TextRendererConfig::default()
        },
    )
    .expect("text renderer should be created");
    let grid = Grid::new(GridSize { rows: 1, cols: 1 }).expect("grid should be created");

    text_renderer
        .prepare_cursor(&renderer, &grid, Cursor::new())
        .expect("cursor should prepare");
    text_renderer.render_to_texture_surface(&renderer, &surface);

    let pixels = crate::test_support::read_texture_surface(&renderer, &surface);
    let background = crate::test_support::bgra_pixel(text_renderer.theme().background);
    assert_eq!(text_renderer.cursor_instance_count(), 1);
    assert!(
        pixels
            .chunks_exact(background.len())
            .any(|pixel| pixel != background),
        "block cursor should draw pixels over the background"
    );
}

#[test]
fn text_renderer_draws_underline_and_bar_cursor_shapes() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let grid = Grid::new(GridSize { rows: 1, cols: 1 }).expect("grid should be created");
    let mut text_renderer = TextRenderer::new(
        &renderer,
        wgpu::TextureFormat::Bgra8UnormSrgb,
        TextRendererConfig {
            theme: Theme {
                background: ThemeColor::rgb(0x00, 0x00, 0x00),
                cursor: ThemeColor::rgb(0x00, 0xff, 0x00),
                ..Theme::default()
            },
            uniforms: crate::cell::TextUniforms::new([16.0, 16.0], [16.0, 16.0], 0.0),
            ..TextRendererConfig::default()
        },
    )
    .expect("text renderer should be created");

    for style in [CursorStyle::Underline, CursorStyle::Bar] {
        let surface = renderer
            .create_texture_surface(TextureSurfaceConfig::new(
                TextureSurfaceSize::new(16, 16).expect("surface dimensions are valid"),
            ))
            .expect("texture surface should be created");
        let mut cursor = Cursor::new();
        cursor.style = style;

        text_renderer
            .prepare_cursor(&renderer, &grid, cursor)
            .expect("cursor should prepare");
        text_renderer.render_to_texture_surface(&renderer, &surface);

        let pixels = crate::test_support::read_texture_surface(&renderer, &surface);
        let background = crate::test_support::bgra_pixel(text_renderer.theme().background);
        assert!(
            pixels
                .chunks_exact(background.len())
                .any(|pixel| pixel != background),
            "cursor style should render visible pixels"
        );
    }
}

#[test]
fn text_renderer_skips_hidden_cursor_overlay() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(16, 16).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let mut text_renderer = TextRenderer::new(
        &renderer,
        surface.format(),
        TextRendererConfig {
            theme: Theme {
                background: ThemeColor::rgb(0xff, 0x00, 0x00),
                ..Theme::default()
            },
            ..TextRendererConfig::default()
        },
    )
    .expect("text renderer should be created");
    let grid = Grid::new(GridSize { rows: 1, cols: 1 }).expect("grid should be created");
    let mut cursor = Cursor::new();
    cursor.visible = false;

    text_renderer
        .prepare_cursor(&renderer, &grid, cursor)
        .expect("hidden cursor should prepare");
    text_renderer.render_to_texture_surface(&renderer, &surface);

    let pixels = crate::test_support::read_texture_surface(&renderer, &surface);
    let background = crate::test_support::bgra_pixel(text_renderer.theme().background);
    assert_eq!(text_renderer.cursor_instance_count(), 0);
    assert!(
        pixels
            .chunks_exact(background.len())
            .all(|pixel| pixel == background),
        "hidden cursor should not draw over the background"
    );
}

#[test]
fn text_renderer_handles_empty_damage_gracefully() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(16, 16).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let mut text_renderer =
        TextRenderer::new(&renderer, surface.format(), TextRendererConfig::default())
            .expect("text renderer should be created");
    let mut grid = Grid::new(GridSize { rows: 1, cols: 1 }).expect("grid should be created");
    grid.write(0, 0, Cell::new('A'))
        .expect("cell should be written");

    text_renderer
        .prepare_grid(&renderer, &grid, &[], |_| Ok(None))
        .expect("empty damage should be accepted");

    assert_eq!(text_renderer.instance_count(), 0);
}

#[test]
fn text_renderer_clears_tracked_instances_when_prepare_fails() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(32, 16).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let mut text_renderer = TextRenderer::new(
        &renderer,
        surface.format(),
        TextRendererConfig {
            uniforms: crate::cell::TextUniforms::new([32.0, 16.0], [16.0, 16.0], 0.0),
            ..TextRendererConfig::default()
        },
    )
    .expect("text renderer should be created");
    let mut grid = Grid::new(GridSize { rows: 1, cols: 1 }).expect("grid should be created");
    grid.write(0, 0, Cell::new('A'))
        .expect("cell should be written");

    text_renderer
        .prepare_grid(&renderer, &grid, &[DamageRegion::new(0, 0, 0, 0)], |_| {
            Ok(Some(RasterizedGlyph::new(4, 8, vec![255; 32])))
        })
        .expect("initial prepare should succeed");
    assert_eq!(text_renderer.instance_count(), 1);

    let mut next_grid = Grid::new(GridSize { rows: 1, cols: 1 }).expect("grid should be created");
    next_grid
        .write(0, 0, Cell::new('B'))
        .expect("replacement cell should be written");

    let result = text_renderer.prepare_grid(
        &renderer,
        &next_grid,
        &[DamageRegion::new(0, 0, 0, 0)],
        |_| {
            Err(crate::error::Error::GlyphRasterizationFailed {
                reason: "forced failure".to_string(),
            })
        },
    );

    assert!(matches!(
        result,
        Err(crate::error::Error::GlyphRasterizationFailed { .. })
    ));
    assert_eq!(text_renderer.instance_count(), 0);
}

#[test]
fn text_renderer_returns_error_when_atlas_exhausts_space() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(16, 16).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let mut text_renderer = TextRenderer::new(
        &renderer,
        surface.format(),
        TextRendererConfig {
            atlas: crate::atlas::AtlasConfig::new(
                crate::atlas::AtlasSize::new(4, 4).expect("atlas size is valid"),
            ),
            ..TextRendererConfig::default()
        },
    )
    .expect("text renderer should be created");
    let mut grid = Grid::new(GridSize { rows: 1, cols: 2 }).expect("grid should be created");
    grid.write(0, 0, Cell::new('A'))
        .expect("first cell should be written");
    grid.write(0, 1, Cell::new('B'))
        .expect("second cell should be written");

    let result =
        text_renderer.prepare_grid(&renderer, &grid, &[DamageRegion::new(0, 0, 0, 1)], |cell| {
            Ok(Some(match cell.character {
                'A' => RasterizedGlyph::new(4, 4, vec![255; 16]),
                _ => RasterizedGlyph::new(1, 1, vec![255; 1]),
            }))
        });

    assert!(matches!(
        result,
        Err(crate::error::Error::AtlasFull {
            width: 1,
            height: 1
        })
    ));
    assert_eq!(text_renderer.instance_count(), 0);
}

fn test_glyph_for(cell: Cell) -> RasterizedGlyph {
    let (width, height) = if cell.is_wide() { (8, 8) } else { (4, 8) };
    RasterizedGlyph::new(width, height, vec![255; (width * height) as usize])
}
