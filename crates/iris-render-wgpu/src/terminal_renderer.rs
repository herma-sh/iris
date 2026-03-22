use std::cell::Cell as FlagCell;

use iris_core::cursor::Cursor;
use iris_core::damage::{DamageRegion, ScrollDelta};
use iris_core::grid::Grid;
use iris_core::terminal::Terminal;

use crate::error::Result;
use crate::font::{FontRasterizer, FontRasterizerConfig};
use crate::pipeline::{PresentPipeline, PresentUniforms};
use crate::renderer::Renderer;
use crate::surface::RendererSurface;
use crate::text_renderer::{TextRenderer, TextRendererConfig};
use crate::texture::{TextureSurface, TextureSurfaceConfig, TextureSurfaceSize};
use crate::theme::Theme;
use crate::TextUniforms;

/// Configuration for the higher-level terminal renderer integration.
#[derive(Clone, Debug, Default)]
pub struct TerminalRendererConfig {
    /// Stateful text-renderer configuration.
    pub text: TextRendererConfig,
    /// System font rasterizer configuration.
    pub font_rasterizer: FontRasterizerConfig,
}

/// Renderer-owned terminal draw state that prepares text and cursor output from
/// `iris-core` terminal state into a cached frame surface.
pub struct TerminalRenderer {
    text_renderer: TextRenderer,
    text_config: TextRendererConfig,
    font_rasterizer: FontRasterizer,
    font_rasterizer_config: FontRasterizerConfig,
    frame_surface: TextureSurface,
    scroll_surface: TextureSurface,
    present_pipeline: PresentPipeline,
    present_uniform_buffer: wgpu::Buffer,
    requested_uniforms: TextUniforms,
    present_uniforms_dirty: FlagCell<bool>,
    present_bind_group: wgpu::BindGroup,
    present_uniform_bind_group: wgpu::BindGroup,
    full_redraw_damage: Vec<DamageRegion>,
    previous_cursor: Option<Cursor>,
    frame_initialized: bool,
}

impl TerminalRenderer {
    /// Creates a terminal renderer for the provided render-target format.
    pub fn new(
        renderer: &Renderer,
        format: wgpu::TextureFormat,
        config: TerminalRendererConfig,
    ) -> Result<Self> {
        let TerminalRendererConfig {
            text,
            font_rasterizer,
        } = config;
        let requested_uniforms = text.uniforms;
        let frame_surface = create_frame_surface(renderer, format, requested_uniforms);
        let scroll_surface = create_scroll_surface(renderer, format, requested_uniforms);
        let mut text_config = text;
        text_config.uniforms = frame_uniforms_for_requested(requested_uniforms);
        let text_renderer =
            TextRenderer::new(renderer, frame_surface.format(), text_config.clone())?;
        let font_rasterizer_config = font_rasterizer;
        let font_rasterizer = FontRasterizer::new(font_rasterizer_config.clone())?;
        let present_pipeline = renderer.create_present_pipeline(format);
        let present_uniform_buffer = present_pipeline.create_uniform_buffer(renderer.device());
        let present_uniform_bind_group =
            present_pipeline.create_uniform_bind_group(renderer.device(), &present_uniform_buffer);
        let present_bind_group =
            present_pipeline.create_texture_bind_group(renderer.device(), &frame_surface);

        let terminal_renderer = Self {
            text_renderer,
            text_config,
            font_rasterizer,
            font_rasterizer_config,
            frame_surface,
            scroll_surface,
            present_pipeline,
            present_uniform_buffer,
            requested_uniforms,
            present_uniforms_dirty: FlagCell::new(true),
            present_bind_group,
            present_uniform_bind_group,
            full_redraw_damage: Vec::with_capacity(4),
            previous_cursor: None,
            frame_initialized: false,
        };
        terminal_renderer.write_present_uniforms(renderer);

        Ok(terminal_renderer)
    }

    /// Returns the active theme used for text, cursor, and clear color.
    #[must_use]
    pub const fn theme(&self) -> &Theme {
        self.text_renderer.theme()
    }

    /// Replaces the active renderer theme.
    pub fn set_theme(&mut self, theme: Theme) {
        self.text_renderer.set_theme(theme.clone());
        self.text_config.theme = theme;
        self.present_uniforms_dirty.set(true);
        self.invalidate_cached_frame();
    }

    /// Returns the current text uniforms.
    #[must_use]
    pub const fn uniforms(&self) -> TextUniforms {
        self.requested_uniforms
    }

    /// Returns the cached frame-surface size.
    #[must_use]
    pub const fn frame_surface_size(&self) -> TextureSurfaceSize {
        self.frame_surface.size()
    }

    /// Updates the viewport and cell metrics written to the uniform buffer.
    pub fn set_uniforms(&mut self, renderer: &Renderer, uniforms: TextUniforms) {
        let frame_uniforms = frame_uniforms_for_requested(uniforms);
        let frame_uniforms_changed = self.text_renderer.uniforms() != frame_uniforms;
        self.requested_uniforms = uniforms;
        self.present_uniforms_dirty.set(true);
        self.text_renderer.set_uniforms(renderer, frame_uniforms);
        self.text_config.uniforms = frame_uniforms;
        self.resize_frame_surface(renderer, uniforms);
        if frame_uniforms_changed {
            self.invalidate_cached_frame();
        }
    }

    /// Returns the configured system font size in pixels.
    #[must_use]
    pub const fn font_size_px(&self) -> f32 {
        self.font_rasterizer.font_size_px()
    }

    /// Updates the system font size used for glyph rasterization.
    ///
    /// Changing the rasterizer size rebuilds renderer-owned glyph resources so
    /// cached atlas entries cannot be reused across different font scales.
    pub fn set_font_size_px(&mut self, renderer: &Renderer, font_size_px: f32) -> Result<()> {
        if self.font_rasterizer_config.font_size_px.to_bits() == font_size_px.to_bits() {
            return Ok(());
        }

        let mut font_rasterizer_config = self.font_rasterizer_config.clone();
        font_rasterizer_config.font_size_px = font_size_px;
        let font_rasterizer = FontRasterizer::new(font_rasterizer_config.clone())?;

        let mut text_config = self.text_config.clone();
        text_config.theme = self.theme().clone();
        text_config.uniforms = frame_uniforms_for_requested(self.requested_uniforms);
        let text_renderer =
            TextRenderer::new(renderer, self.frame_surface.format(), text_config.clone())?;

        self.text_renderer = text_renderer;
        self.text_config = text_config;
        self.font_rasterizer = font_rasterizer;
        self.font_rasterizer_config = font_rasterizer_config;
        self.present_uniforms_dirty.set(true);
        self.invalidate_cached_frame();

        Ok(())
    }

    /// Returns the number of prepared text instances.
    #[must_use]
    pub const fn instance_count(&self) -> usize {
        self.text_renderer.instance_count()
    }

    /// Returns the number of prepared cursor instances.
    #[must_use]
    pub const fn cursor_instance_count(&self) -> usize {
        self.text_renderer.cursor_instance_count()
    }

    /// Prepares a full visible terminal frame from `iris-core` terminal state.
    pub fn prepare_terminal(&mut self, renderer: &Renderer, terminal: &Terminal) -> Result<()> {
        self.prepare_grid_and_cursor(renderer, &terminal.grid, terminal.cursor)
    }

    /// Applies an incremental terminal update into the cached frame using the
    /// terminal's accumulated damage plus cursor old/new regions.
    pub fn update_terminal(&mut self, renderer: &Renderer, terminal: &mut Terminal) -> Result<()> {
        if !self.frame_initialized {
            let result = self.prepare_grid_and_cursor(renderer, &terminal.grid, terminal.cursor);
            if result.is_ok() {
                let _ = terminal.take_scroll_delta();
                let _ = terminal.take_damage();
            }
            return result;
        }

        let scroll_delta = terminal.take_scroll_delta();
        let mut damage = terminal.take_damage();
        let original_damage_len = damage.len();
        let result = self.update_grid_and_cursor_internal(
            renderer,
            &terminal.grid,
            &mut damage,
            scroll_delta,
            terminal.cursor,
        );
        self.full_redraw_damage = damage;

        match result {
            Ok(()) => Ok(()),
            Err(error) => {
                self.invalidate_cached_frame();
                terminal.restore_scroll_delta(scroll_delta);
                terminal.restore_damage(&self.full_redraw_damage[..original_damage_len]);
                Err(error)
            }
        }
    }

    /// Prepares a full visible frame from explicit grid and cursor state.
    pub fn prepare_grid_and_cursor(
        &mut self,
        renderer: &Renderer,
        grid: &Grid,
        cursor: Cursor,
    ) -> Result<()> {
        self.rebuild_full_redraw_damage(grid);
        self.text_renderer.prepare_grid_with_font_rasterizer(
            renderer,
            grid,
            &self.full_redraw_damage,
            &mut self.font_rasterizer,
        )?;
        self.text_renderer.prepare_cursor(renderer, grid, cursor)?;
        self.text_renderer
            .render_to_texture_surface(renderer, &self.frame_surface);
        self.previous_cursor = Some(cursor);
        self.frame_initialized = true;
        Ok(())
    }

    /// Applies an incremental grid and cursor update into the cached frame.
    pub fn update_grid_and_cursor(
        &mut self,
        renderer: &Renderer,
        grid: &Grid,
        damage: &[DamageRegion],
        scroll_delta: Option<ScrollDelta>,
        cursor: Cursor,
    ) -> Result<()> {
        if !self.frame_initialized {
            return self.prepare_grid_and_cursor(renderer, grid, cursor);
        }

        let mut full_redraw_damage = std::mem::take(&mut self.full_redraw_damage);
        full_redraw_damage.clear();
        full_redraw_damage.extend_from_slice(damage);
        let result = self.update_grid_and_cursor_internal(
            renderer,
            grid,
            &mut full_redraw_damage,
            scroll_delta,
            cursor,
        );
        self.full_redraw_damage = full_redraw_damage;
        result
    }

    fn update_grid_and_cursor_internal(
        &mut self,
        renderer: &Renderer,
        grid: &Grid,
        damage: &mut Vec<DamageRegion>,
        scroll_delta: Option<ScrollDelta>,
        cursor: Cursor,
    ) -> Result<()> {
        let original_damage_len = damage.len();
        let cursor_changed = self.previous_cursor != Some(cursor);
        let normalized_scroll = normalized_scroll_delta(scroll_delta, grid);
        if normalized_scroll.is_none() && damage.is_empty() && !cursor_changed {
            return Ok(());
        }

        let mut shifted_retained_frame = false;
        if let Some(scroll_delta) = normalized_scroll {
            shifted_retained_frame = true;
            if is_full_grid_scroll_delta(scroll_delta, grid) {
                self.shift_retained_frame_for_scroll(renderer, scroll_delta);
            } else {
                self.shift_retained_frame_for_partial_scroll(renderer, scroll_delta);
            }
        }

        if cursor_changed || shifted_retained_frame {
            self.push_cursor_damage_pair(damage, grid, self.previous_cursor, Some(cursor));
        }
        let should_prepare_cursor = if cursor_changed || shifted_retained_frame {
            true
        } else {
            self.cursor_damage_region(grid, Some(cursor))
                .is_some_and(|region| {
                    damage_overlaps_region(&damage[..original_damage_len], region)
                })
        };

        if damage.is_empty() {
            // Keep cursor state current even when no redraw work is required.
            self.previous_cursor = Some(cursor);
            return Ok(());
        }

        self.text_renderer
            .prepare_grid_update_with_font_rasterizer(
                renderer,
                grid,
                damage,
                &mut self.font_rasterizer,
            )?;
        if should_prepare_cursor {
            self.text_renderer.prepare_cursor(renderer, grid, cursor)?;
        }
        self.text_renderer
            .render_to_texture_surface_with_load(renderer, &self.frame_surface);
        self.previous_cursor = Some(cursor);
        Ok(())
    }

    /// Renders the cached frame into an off-screen texture surface.
    pub fn render_to_texture_surface(&self, renderer: &Renderer, surface: &TextureSurface) {
        self.write_present_uniforms(renderer);
        renderer.draw_present_pipeline_to_texture_surface(
            &self.present_pipeline,
            &self.present_uniform_bind_group,
            &self.present_bind_group,
            surface,
        );
    }

    /// Renders the cached frame into the next presentation frame.
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
                    label: Some("iris-render-wgpu-terminal-renderer-surface-encoder"),
                });
        self.write_present_uniforms(renderer);
        self.present_pipeline.render(
            &mut encoder,
            &view,
            &self.present_uniform_bind_group,
            &self.present_bind_group,
        );
        renderer.queue().submit(std::iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }

    fn rebuild_full_redraw_damage(&mut self, grid: &Grid) {
        self.full_redraw_damage.clear();
        if grid.rows() == 0 || grid.cols() == 0 {
            return;
        }

        self.full_redraw_damage.push(DamageRegion::new(
            0,
            grid.rows().saturating_sub(1),
            0,
            grid.cols().saturating_sub(1),
        ));
    }

    fn push_cursor_damage_pair(
        &self,
        damage: &mut Vec<DamageRegion>,
        grid: &Grid,
        previous_cursor: Option<Cursor>,
        current_cursor: Option<Cursor>,
    ) {
        if previous_cursor == current_cursor {
            if let Some(region) = self.cursor_damage_region(grid, current_cursor) {
                damage.push(region);
            }
            return;
        }

        let previous_region = self.cursor_damage_region(grid, previous_cursor);
        let current_region = self.cursor_damage_region(grid, current_cursor);

        if let Some(region) = previous_region {
            damage.push(region);
        }
        if let Some(region) = current_region {
            if Some(region) != previous_region {
                damage.push(region);
            }
        }
    }

    fn cursor_damage_region(&self, grid: &Grid, cursor: Option<Cursor>) -> Option<DamageRegion> {
        crate::cursor::cursor_damage_region(cursor?, grid)
    }

    fn resize_frame_surface(&mut self, renderer: &Renderer, uniforms: TextUniforms) {
        let next_size = frame_surface_size_for_uniforms(uniforms);
        if next_size == self.frame_surface.size() {
            return;
        }

        self.frame_surface =
            create_frame_surface(renderer, self.present_pipeline.format(), uniforms);
        self.scroll_surface =
            create_scroll_surface(renderer, self.present_pipeline.format(), uniforms);
        self.present_bind_group = self
            .present_pipeline
            .create_texture_bind_group(renderer.device(), &self.frame_surface);
        self.present_uniforms_dirty.set(true);
        self.invalidate_cached_frame();
    }

    fn write_present_uniforms(&self, renderer: &Renderer) {
        if !self.present_uniforms_dirty.get() {
            return;
        }

        renderer.queue().write_buffer(
            &self.present_uniform_buffer,
            0,
            present_uniforms_for_requested(
                self.requested_uniforms,
                self.theme(),
                self.frame_surface.size(),
            )
            .as_bytes(),
        );
        self.present_uniforms_dirty.set(false);
    }

    fn invalidate_cached_frame(&mut self) {
        self.frame_initialized = false;
        self.previous_cursor = None;
    }

    fn shift_retained_frame_for_scroll(&mut self, renderer: &Renderer, scroll_delta: ScrollDelta) {
        let Some((source_y, destination_y, copy_height)) = scroll_copy_region(
            self.requested_uniforms,
            self.frame_surface.size(),
            scroll_delta,
        ) else {
            return;
        };

        let mut encoder =
            renderer
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("iris-render-wgpu-terminal-renderer-scroll-shift"),
                });
        let surface_size = self.frame_surface.size();
        let full_extent = wgpu::Extent3d {
            width: surface_size.width,
            height: surface_size.height,
            depth_or_array_layers: 1,
        };
        encoder.copy_texture_to_texture(
            wgpu::ImageCopyTexture {
                texture: self.frame_surface.texture(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyTexture {
                texture: self.scroll_surface.texture(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            full_extent,
        );
        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("iris-render-wgpu-terminal-renderer-scroll-clear-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: self.frame_surface.view(),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.theme().background.to_wgpu_color()),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        encoder.copy_texture_to_texture(
            wgpu::ImageCopyTexture {
                texture: self.scroll_surface.texture(),
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: source_y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyTexture {
                texture: self.frame_surface.texture(),
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: destination_y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: surface_size.width,
                height: copy_height,
                depth_or_array_layers: 1,
            },
        );
        renderer.queue().submit(std::iter::once(encoder.finish()));
    }

    fn shift_retained_frame_for_partial_scroll(
        &mut self,
        renderer: &Renderer,
        scroll_delta: ScrollDelta,
    ) {
        let Some((region_top_y, region_bottom_y, source_y, destination_y, copy_height)) =
            partial_scroll_copy_region(
                self.requested_uniforms,
                self.frame_surface.size(),
                scroll_delta,
            )
        else {
            return;
        };

        let mut encoder =
            renderer
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("iris-render-wgpu-terminal-renderer-partial-scroll-shift"),
                });
        let surface_size = self.frame_surface.size();
        let full_extent = wgpu::Extent3d {
            width: surface_size.width,
            height: surface_size.height,
            depth_or_array_layers: 1,
        };
        encoder.copy_texture_to_texture(
            wgpu::ImageCopyTexture {
                texture: self.frame_surface.texture(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyTexture {
                texture: self.scroll_surface.texture(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            full_extent,
        );
        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("iris-render-wgpu-terminal-renderer-partial-scroll-clear-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: self.frame_surface.view(),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.theme().background.to_wgpu_color()),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        if region_top_y > 0 {
            encoder.copy_texture_to_texture(
                wgpu::ImageCopyTexture {
                    texture: self.scroll_surface.texture(),
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::ImageCopyTexture {
                    texture: self.frame_surface.texture(),
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::Extent3d {
                    width: surface_size.width,
                    height: region_top_y,
                    depth_or_array_layers: 1,
                },
            );
        }
        let below_region_height = surface_size.height.saturating_sub(region_bottom_y);
        if below_region_height > 0 {
            encoder.copy_texture_to_texture(
                wgpu::ImageCopyTexture {
                    texture: self.scroll_surface.texture(),
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: region_bottom_y,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::ImageCopyTexture {
                    texture: self.frame_surface.texture(),
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: region_bottom_y,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::Extent3d {
                    width: surface_size.width,
                    height: below_region_height,
                    depth_or_array_layers: 1,
                },
            );
        }
        if copy_height > 0 {
            encoder.copy_texture_to_texture(
                wgpu::ImageCopyTexture {
                    texture: self.scroll_surface.texture(),
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: source_y,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::ImageCopyTexture {
                    texture: self.frame_surface.texture(),
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: destination_y,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::Extent3d {
                    width: surface_size.width,
                    height: copy_height,
                    depth_or_array_layers: 1,
                },
            );
        }
        renderer.queue().submit(std::iter::once(encoder.finish()));
    }
}

fn create_frame_surface(
    renderer: &Renderer,
    format: wgpu::TextureFormat,
    uniforms: TextUniforms,
) -> TextureSurface {
    renderer
        .create_texture_surface(TextureSurfaceConfig {
            size: frame_surface_size_for_uniforms(uniforms),
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::TEXTURE_BINDING,
        })
        .expect("internal frame-surface config should remain valid")
}

fn create_scroll_surface(
    renderer: &Renderer,
    format: wgpu::TextureFormat,
    uniforms: TextUniforms,
) -> TextureSurface {
    renderer
        .create_texture_surface(TextureSurfaceConfig {
            size: frame_surface_size_for_uniforms(uniforms),
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
        })
        .expect("internal scroll-surface config should remain valid")
}

fn frame_surface_size_for_uniforms(uniforms: TextUniforms) -> TextureSurfaceSize {
    let viewport_size = viewport_surface_size_for_uniforms(uniforms);
    let vertical_padding = frame_vertical_padding_pixels(uniforms);
    TextureSurfaceSize {
        width: viewport_size.width,
        height: viewport_size
            .height
            .saturating_add(vertical_padding.saturating_mul(2)),
    }
}

fn frame_uniforms_for_requested(uniforms: TextUniforms) -> TextUniforms {
    let frame_size = frame_surface_size_for_uniforms(uniforms);
    let vertical_padding = frame_vertical_padding_pixels(uniforms) as f32;
    TextUniforms::new(
        [frame_size.width as f32, frame_size.height as f32],
        uniforms.cell_size,
        vertical_padding,
    )
}

fn present_uniforms_for_requested(
    uniforms: TextUniforms,
    theme: &Theme,
    frame_surface_size: TextureSurfaceSize,
) -> PresentUniforms {
    PresentUniforms::new(
        [
            frame_surface_size.width as f32,
            frame_surface_size.height as f32,
        ],
        [0.0, frame_vertical_padding_pixels(uniforms) as f32],
        uniforms.scroll_offset,
        theme.background.to_f32_array(),
    )
}

fn viewport_surface_size_for_uniforms(uniforms: TextUniforms) -> TextureSurfaceSize {
    TextureSurfaceSize {
        width: normalized_surface_dimension(uniforms.resolution[0]),
        height: normalized_surface_dimension(uniforms.resolution[1]),
    }
}

fn frame_vertical_padding_pixels(uniforms: TextUniforms) -> u32 {
    viewport_surface_size_for_uniforms(uniforms).height
}

fn normalized_scroll_delta(scroll_delta: Option<ScrollDelta>, grid: &Grid) -> Option<ScrollDelta> {
    let scroll_delta = scroll_delta?;
    if grid.rows() == 0 || scroll_delta.lines == 0 {
        return None;
    }
    if scroll_delta.top > scroll_delta.bottom || scroll_delta.bottom >= grid.rows() {
        return None;
    }

    Some(scroll_delta)
}

fn damage_overlaps_region(damage: &[DamageRegion], region: DamageRegion) -> bool {
    damage.iter().any(|candidate| {
        candidate.start_row <= region.end_row
            && candidate.end_row >= region.start_row
            && candidate.start_col <= region.end_col
            && candidate.end_col >= region.start_col
    })
}

fn is_full_grid_scroll_delta(scroll_delta: ScrollDelta, grid: &Grid) -> bool {
    let Some(full_grid_bottom) = grid.rows().checked_sub(1) else {
        return false;
    };
    scroll_delta.top == 0 && scroll_delta.bottom == full_grid_bottom
}

fn partial_scroll_copy_region(
    uniforms: TextUniforms,
    frame_surface_size: TextureSurfaceSize,
    scroll_delta: ScrollDelta,
) -> Option<(u32, u32, u32, u32, u32)> {
    let viewport_size = viewport_surface_size_for_uniforms(uniforms);
    if viewport_size.height == 0 || frame_surface_size.width == 0 {
        return None;
    }

    let cell_height = normalized_surface_dimension(uniforms.cell_size[1]);
    if cell_height == 0 {
        return None;
    }

    let top_row = u32::try_from(scroll_delta.top).unwrap_or(u32::MAX);
    let bottom_row_exclusive =
        u32::try_from(scroll_delta.bottom.saturating_add(1)).unwrap_or(u32::MAX);
    let region_top_in_viewport = top_row
        .saturating_mul(cell_height)
        .min(viewport_size.height);
    let region_bottom_in_viewport = bottom_row_exclusive
        .saturating_mul(cell_height)
        .min(viewport_size.height);
    if region_top_in_viewport >= region_bottom_in_viewport {
        return None;
    }

    let vertical_padding = frame_vertical_padding_pixels(uniforms);
    let region_top_y = vertical_padding.saturating_add(region_top_in_viewport);
    let region_bottom_y = vertical_padding.saturating_add(region_bottom_in_viewport);
    let region_height = region_bottom_y.saturating_sub(region_top_y);
    let shift_pixels = scroll_delta
        .lines
        .unsigned_abs()
        .saturating_mul(cell_height);
    if shift_pixels == 0 || shift_pixels > region_height {
        return None;
    }

    let copy_height = region_height.saturating_sub(shift_pixels);
    let (source_y, destination_y) = match scroll_delta.lines {
        lines if lines > 0 => (region_top_y.saturating_add(shift_pixels), region_top_y),
        lines if lines < 0 => (region_top_y, region_top_y.saturating_add(shift_pixels)),
        _ => return None,
    };

    if region_bottom_y > frame_surface_size.height {
        return None;
    }

    Some((
        region_top_y,
        region_bottom_y,
        source_y,
        destination_y,
        copy_height,
    ))
}

fn scroll_copy_region(
    uniforms: TextUniforms,
    frame_surface_size: TextureSurfaceSize,
    scroll_delta: ScrollDelta,
) -> Option<(u32, u32, u32)> {
    let viewport_size = viewport_surface_size_for_uniforms(uniforms);
    if viewport_size.height == 0 || frame_surface_size.width == 0 {
        return None;
    }

    let vertical_padding = frame_vertical_padding_pixels(uniforms);
    let shift_pixels = scroll_delta
        .lines
        .unsigned_abs()
        .saturating_mul(normalized_surface_dimension(uniforms.cell_size[1]));
    if shift_pixels == 0 || shift_pixels > viewport_size.height {
        return None;
    }

    match scroll_delta.lines {
        lines if lines > 0 => Some((
            vertical_padding,
            vertical_padding.saturating_sub(shift_pixels),
            viewport_size.height,
        )),
        lines if lines < 0 => Some((
            vertical_padding,
            vertical_padding.saturating_add(shift_pixels),
            viewport_size.height,
        )),
        _ => None,
    }
}

fn normalized_surface_dimension(dimension: f32) -> u32 {
    if !dimension.is_finite() || dimension <= 0.0 {
        tracing::warn!(
            ?dimension,
            "invalid terminal frame dimension normalized to a 1px fallback"
        );
        1
    } else {
        dimension.round().max(1.0) as u32
    }
}

#[cfg(test)]
#[path = "test/terminal_renderer/tests.rs"]
mod tests;
