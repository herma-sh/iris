use std::collections::{HashMap, HashSet};

use iris_core::cell::{Cell, CellFlags, CellWidth};
use iris_core::damage::DamageRegion;
use iris_core::grid::Grid;

use crate::atlas::{AtlasConfig, AtlasSize};
use crate::cell::{
    cell_needs_rendering_with_blank_default_cells,
    encode_normalized_damage_instances_with_options_and_selection, never_selected,
    normalized_damage_regions_into, CellInstance, EncodeInstancesOptions, TextBuffers,
    TextUniforms,
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

mod ligatures;
#[cfg(test)]
use ligatures::operator_ligature_replacement;
use ligatures::{expand_damage_regions_for_ligature_context, glyph_key_for_cell, LigatureOverride};

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
    rewritten_instances: Vec<CellInstance>,
    ligature_damage: Vec<DamageRegion>,
    normalized_damage: Vec<DamageRegion>,
    ligature_overrides: HashMap<(usize, usize), LigatureOverride>,
    ligature_followers: HashSet<(usize, usize)>,
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
            rewritten_instances: Vec::with_capacity(config.initial_instance_capacity.max(1)),
            ligature_damage: Vec::with_capacity(config.initial_instance_capacity.max(1)),
            normalized_damage: Vec::with_capacity(config.initial_instance_capacity.max(1)),
            ligature_overrides: HashMap::with_capacity(config.initial_instance_capacity.max(1)),
            ligature_followers: HashSet::with_capacity(config.initial_instance_capacity.max(1)),
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
        self.prepare_grid_internal(
            renderer,
            grid,
            damage,
            &mut rasterize_glyph,
            false,
            &never_selected,
        )
    }

    /// Populates and prepares damaged grid cells using the built-in system font rasterizer.
    pub fn prepare_grid_with_font_rasterizer(
        &mut self,
        renderer: &Renderer,
        grid: &Grid,
        damage: &[DamageRegion],
        font_rasterizer: &mut FontRasterizer,
    ) -> Result<()> {
        self.prepare_grid_with_font_rasterizer_with_selection(
            renderer,
            grid,
            damage,
            font_rasterizer,
            false,
            never_selected,
        )
    }

    /// Populates and prepares damaged grid cells for retained updates, keeping
    /// default blank cells so stale content can be cleared back to the theme background.
    pub fn prepare_grid_update_with_font_rasterizer(
        &mut self,
        renderer: &Renderer,
        grid: &Grid,
        damage: &[DamageRegion],
        font_rasterizer: &mut FontRasterizer,
    ) -> Result<()> {
        self.prepare_grid_with_font_rasterizer_with_selection(
            renderer,
            grid,
            damage,
            font_rasterizer,
            true,
            never_selected,
        )
    }

    pub(crate) fn prepare_grid_with_font_rasterizer_with_selection<S>(
        &mut self,
        renderer: &Renderer,
        grid: &Grid,
        damage: &[DamageRegion],
        font_rasterizer: &mut FontRasterizer,
        include_blank_default_cells: bool,
        is_selected: S,
    ) -> Result<()>
    where
        S: Fn(usize, usize) -> bool,
    {
        self.prepare_grid_with_font_rasterizer_internal(
            renderer,
            grid,
            damage,
            font_rasterizer,
            include_blank_default_cells,
            &is_selected,
        )
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
        self.render_to_texture_surface_internal(
            renderer,
            surface,
            wgpu::LoadOp::Clear(self.theme.background.to_wgpu_color()),
        );
    }

    pub(crate) fn render_to_texture_surface_with_load(
        &self,
        renderer: &Renderer,
        surface: &TextureSurface,
    ) {
        self.render_to_texture_surface_internal(renderer, surface, wgpu::LoadOp::Load);
    }

    fn render_to_texture_surface_internal(
        &self,
        renderer: &Renderer,
        surface: &TextureSurface,
        load_op: wgpu::LoadOp<wgpu::Color>,
    ) {
        let mut encoder =
            renderer
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("iris-render-wgpu-text-renderer-texture-encoder"),
                });
        self.pipeline.render_with_load_op(
            &mut encoder,
            surface.view(),
            &self.uniform_bind_group,
            &self.atlas,
            &self.buffers,
            load_op,
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
        rasterize_glyph: &mut F,
        include_default_blank_cells: bool,
    ) -> Result<()>
    where
        F: FnMut(Cell) -> Result<Option<RasterizedGlyph>>,
    {
        let mut skipped_missing_rasterization = 0usize;

        for region in &self.normalized_damage {
            let Some(row_cells) = grid.row(region.start_row) else {
                continue;
            };

            for &cell in row_cells
                .iter()
                .skip(region.start_col)
                .take(region.end_col - region.start_col + 1)
            {
                if !cell_needs_rendering_with_blank_default_cells(cell, include_default_blank_cells)
                {
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

                renderer.cache_glyph_with_placement(
                    &mut self.glyph_cache,
                    &mut self.atlas,
                    key,
                    rasterized.as_bitmap(),
                    rasterized.placement(),
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

    fn prepare_grid_internal<F, S>(
        &mut self,
        renderer: &Renderer,
        grid: &Grid,
        damage: &[DamageRegion],
        rasterize_glyph: &mut F,
        include_default_blank_cells: bool,
        is_selected: &S,
    ) -> Result<()>
    where
        F: FnMut(Cell) -> Result<Option<RasterizedGlyph>>,
        S: Fn(usize, usize) -> bool,
    {
        self.instances.clear();
        normalized_damage_regions_into(grid, damage, &mut self.normalized_damage);
        self.buffers.clear_instances();
        self.populate_missing_glyphs(renderer, grid, rasterize_glyph, include_default_blank_cells)?;
        let atlas_size = self.atlas.size();
        let theme = &self.theme;
        let normalized_damage = &self.normalized_damage;
        let glyph_cache = &self.glyph_cache;
        encode_normalized_damage_instances_with_options_and_selection(
            &mut self.instances,
            grid,
            normalized_damage,
            atlas_size,
            theme,
            |cell| glyph_cache.get(glyph_key_for_cell(cell)).copied(),
            EncodeInstancesOptions {
                include_default_blank_cells,
                is_selected,
            },
        )?;
        renderer.write_text_instances(&mut self.buffers, &self.instances)
    }

    fn prepare_grid_with_font_rasterizer_internal<S>(
        &mut self,
        renderer: &Renderer,
        grid: &Grid,
        damage: &[DamageRegion],
        font_rasterizer: &mut FontRasterizer,
        include_blank_default_cells: bool,
        is_selected: &S,
    ) -> Result<()>
    where
        S: Fn(usize, usize) -> bool,
    {
        let mut ligature_damage = std::mem::take(&mut self.ligature_damage);
        ligature_damage.clear();
        ligature_damage.reserve(damage.len());
        expand_damage_regions_for_ligature_context(grid, damage, &mut ligature_damage);

        let prepare_result = self.prepare_grid_internal(
            renderer,
            grid,
            &ligature_damage,
            &mut |cell| font_rasterizer.rasterize_cell(cell),
            include_blank_default_cells,
            is_selected,
        );
        self.ligature_damage = ligature_damage;

        prepare_result?;
        self.apply_operator_ligatures(renderer, grid, font_rasterizer, is_selected)
    }
}

#[cfg(test)]
#[path = "test/text_renderer/tests.rs"]
mod tests;
