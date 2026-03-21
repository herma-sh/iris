use iris_core::cell::{Cell, CellFlags};
use iris_core::damage::DamageRegion;
use iris_core::grid::Grid;

use crate::atlas::{AtlasConfig, AtlasSize};
use crate::cell::{
    cell_needs_rendering, normalized_damage_regions, CellInstance, TextBuffers, TextUniforms,
};
use crate::cursor::{CursorBuffers, CursorInstance};
use crate::error::Result;
use crate::font::FontRasterizer;
use crate::glyph::{GlyphCache, GlyphKey, RasterizedGlyph};
use crate::pipeline::{CursorPipeline, TextPipeline};
use crate::renderer::Renderer;
use crate::surface::RendererSurface;
use crate::texture::TextureSurface;
use crate::theme::Theme;

const GLYPH_STYLE_FLAGS: CellFlags = CellFlags::BOLD.union(CellFlags::ITALIC);

/// Configuration for a stateful atlas-backed text renderer.
#[derive(Clone, Debug)]
pub struct TextRendererConfig {
    /// Glyph-atlas configuration.
    pub atlas: AtlasConfig,
    /// Initial cell-instance upload capacity.
    pub initial_instance_capacity: usize,
    /// Initial viewport and cell metrics.
    pub uniforms: TextUniforms,
    /// Theme used for clear color and cell color resolution.
    pub theme: Theme,
}

impl Default for TextRendererConfig {
    fn default() -> Self {
        Self {
            atlas: AtlasConfig::new(AtlasSize {
                width: 1024,
                height: 1024,
            }),
            initial_instance_capacity: 1,
            uniforms: TextUniforms::new([1.0, 1.0], [1.0, 1.0], 0.0),
            theme: Theme::default(),
        }
    }
}

/// Stateful renderer-side text session built on the lower-level renderer helpers.
#[derive(Debug)]
pub struct TextRenderer {
    atlas: crate::atlas::GlyphAtlas,
    glyph_cache: GlyphCache,
    buffers: TextBuffers,
    pipeline: TextPipeline,
    uniform_bind_group: wgpu::BindGroup,
    cursor_buffers: CursorBuffers,
    cursor_pipeline: CursorPipeline,
    cursor_uniform_bind_group: wgpu::BindGroup,
    theme: Theme,
    uniforms: TextUniforms,
    instances: Vec<CellInstance>,
}

impl TextRenderer {
    /// Creates a stateful text renderer for the provided target format.
    pub fn new(
        renderer: &Renderer,
        format: wgpu::TextureFormat,
        config: TextRendererConfig,
    ) -> Result<Self> {
        let atlas = renderer.create_glyph_atlas(config.atlas)?;
        let glyph_cache = renderer.create_glyph_cache();
        let buffers = renderer.create_text_buffers(config.initial_instance_capacity)?;
        renderer.write_text_uniforms(&buffers, &config.uniforms);
        let pipeline = renderer.create_text_pipeline(format, &atlas);
        let uniform_bind_group = renderer.create_text_uniform_bind_group(&pipeline, &buffers);
        let cursor_buffers = renderer.create_cursor_buffers();
        let cursor_pipeline = renderer.create_cursor_pipeline(format);
        let cursor_uniform_bind_group =
            renderer.create_cursor_uniform_bind_group(&cursor_pipeline, &buffers);

        Ok(Self {
            atlas,
            glyph_cache,
            buffers,
            pipeline,
            uniform_bind_group,
            cursor_buffers,
            cursor_pipeline,
            cursor_uniform_bind_group,
            theme: config.theme,
            uniforms: config.uniforms,
            instances: Vec::with_capacity(config.initial_instance_capacity.max(1)),
        })
    }

    /// Returns the active theme.
    #[must_use]
    pub const fn theme(&self) -> &Theme {
        &self.theme
    }

    /// Replaces the active theme used for clears and cell colors.
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    /// Returns the current text uniforms.
    #[must_use]
    pub const fn uniforms(&self) -> TextUniforms {
        self.uniforms
    }

    /// Updates the viewport and cell metrics written to the uniform buffer.
    pub fn set_uniforms(&mut self, renderer: &Renderer, uniforms: TextUniforms) {
        renderer.write_text_uniforms(&self.buffers, &uniforms);
        self.uniforms = uniforms;
    }

    /// Returns the number of instances prepared for the next draw.
    #[must_use]
    pub const fn instance_count(&self) -> usize {
        self.buffers.instance_count()
    }

    /// Returns the number of prepared cursor instances.
    #[must_use]
    pub const fn cursor_instance_count(&self) -> usize {
        self.cursor_buffers.instance_count()
    }

    /// Populates missing glyphs and uploads the latest damaged-cell instances.
    pub fn prepare_grid<F>(
        &mut self,
        renderer: &Renderer,
        grid: &Grid,
        damage: &[DamageRegion],
        mut rasterize_glyph: F,
    ) -> Result<()>
    where
        F: FnMut(Cell) -> Result<Option<RasterizedGlyph>>,
    {
        self.instances.clear();
        self.buffers.clear_instances();
        self.populate_missing_glyphs(renderer, grid, damage, &mut rasterize_glyph)?;
        renderer.encode_text_instances_for_damage(
            &mut self.instances,
            grid,
            damage,
            &self.atlas,
            &self.theme,
            |cell| self.glyph_cache.get(glyph_key_for_cell(cell)).copied(),
        )?;
        renderer.write_text_instances(&mut self.buffers, &self.instances)
    }

    /// Populates and prepares damaged grid cells using the built-in system font rasterizer.
    pub fn prepare_grid_with_font_rasterizer(
        &mut self,
        renderer: &Renderer,
        grid: &Grid,
        damage: &[DamageRegion],
        font_rasterizer: &mut FontRasterizer,
    ) -> Result<()> {
        self.prepare_grid(renderer, grid, damage, |cell| {
            font_rasterizer.rasterize_cell(cell)
        })
    }

    /// Updates the cursor overlay from core cursor state.
    pub fn prepare_cursor(
        &mut self,
        renderer: &Renderer,
        grid: &Grid,
        cursor: iris_core::cursor::Cursor,
    ) -> Result<()> {
        let instance = CursorInstance::from_cursor(cursor, grid, &self.theme)?;
        self.cursor_buffers
            .write_instance(renderer.queue(), instance.as_ref());
        Ok(())
    }

    /// Renders the prepared text instances into an off-screen texture surface.
    pub fn render_to_texture_surface(&self, renderer: &Renderer, surface: &TextureSurface) {
        let mut encoder =
            renderer
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("iris-render-wgpu-text-renderer-texture-encoder"),
                });
        self.pipeline.render(
            &mut encoder,
            surface.view(),
            &self.uniform_bind_group,
            &self.atlas,
            &self.buffers,
            self.theme.background.to_wgpu_color(),
        );
        self.cursor_pipeline.render(
            &mut encoder,
            surface.view(),
            &self.cursor_uniform_bind_group,
            &self.cursor_buffers,
        );
        renderer.queue().submit(std::iter::once(encoder.finish()));
    }

    /// Renders the prepared text instances into the next presentation frame.
    pub fn render_to_surface(
        &self,
        renderer: &Renderer,
        surface: &RendererSurface<'_>,
    ) -> Result<()> {
        let frame = surface.current_texture()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder =
            renderer
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("iris-render-wgpu-text-renderer-surface-encoder"),
                });
        self.pipeline.render(
            &mut encoder,
            &view,
            &self.uniform_bind_group,
            &self.atlas,
            &self.buffers,
            self.theme.background.to_wgpu_color(),
        );
        self.cursor_pipeline.render(
            &mut encoder,
            &view,
            &self.cursor_uniform_bind_group,
            &self.cursor_buffers,
        );
        renderer.queue().submit(std::iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }

    fn populate_missing_glyphs<F>(
        &mut self,
        renderer: &Renderer,
        grid: &Grid,
        damage: &[DamageRegion],
        rasterize_glyph: &mut F,
    ) -> Result<()>
    where
        F: FnMut(Cell) -> Result<Option<RasterizedGlyph>>,
    {
        let mut skipped_missing_rasterization = 0usize;

        for region in normalized_damage_regions(grid, damage) {
            let Some(row_cells) = grid.row(region.start_row) else {
                continue;
            };

            for &cell in row_cells
                .iter()
                .skip(region.start_col)
                .take(region.end_col - region.start_col + 1)
            {
                if !cell_needs_rendering(cell) {
                    continue;
                }

                let key = glyph_key_for_cell(cell);
                if self.glyph_cache.contains(key) {
                    continue;
                }

                let Some(rasterized) = rasterize_glyph(cell)? else {
                    skipped_missing_rasterization += 1;
                    continue;
                };

                renderer.cache_glyph(
                    &mut self.glyph_cache,
                    &mut self.atlas,
                    key,
                    rasterized.as_bitmap(),
                )?;
            }
        }

        if skipped_missing_rasterization > 0 {
            tracing::debug!(
                skipped_missing_rasterization,
                "skipped glyph-cache population because rasterization returned no bitmap"
            );
        }

        Ok(())
    }
}

/// Creates a glyph cache key for the rendered glyph shape of a cell.
///
/// Bit layout:
/// - bits `0..=31`: Unicode scalar value
/// - bits `32..=47`: shape-affecting style flags (`BOLD | ITALIC`)
/// - bits `48..=63`: width tag (`0` continuation, `1` single-width, `2` double-width)
///
/// Decorations such as underline and strikethrough are intentionally excluded
/// because they do not change glyph rasterization.
fn glyph_key_for_cell(cell: Cell) -> GlyphKey {
    let style_bits = (cell.attrs.flags & GLYPH_STYLE_FLAGS).bits();
    let width_tag = match cell.width {
        iris_core::cell::CellWidth::Single => 1u64,
        iris_core::cell::CellWidth::Double => 2u64,
        iris_core::cell::CellWidth::Continuation => 0u64,
    };

    GlyphKey::new(
        u64::from(cell.character as u32) | (u64::from(style_bits) << 32) | (width_tag << 48),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use iris_core::cell::Cell;
    use iris_core::cursor::{Cursor, CursorStyle};
    use iris_core::damage::DamageRegion;
    use iris_core::grid::{Grid, GridSize};

    use super::{glyph_key_for_cell, TextRenderer, TextRendererConfig};
    use crate::glyph::RasterizedGlyph;
    use crate::renderer::{Renderer, RendererConfig};
    use crate::texture::{TextureSurfaceConfig, TextureSurfaceSize};
    use crate::theme::{Theme, ThemeColor};
    use crate::FontRasterizer;
    use crate::FontRasterizerConfig;

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

        let mut next_grid =
            Grid::new(GridSize { rows: 1, cols: 1 }).expect("grid should be created");
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

        let result = text_renderer.prepare_grid(
            &renderer,
            &grid,
            &[DamageRegion::new(0, 0, 0, 1)],
            |cell| {
                Ok(Some(match cell.character {
                    'A' => RasterizedGlyph::new(4, 4, vec![255; 16]),
                    _ => RasterizedGlyph::new(1, 1, vec![255; 1]),
                }))
            },
        );

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
}
