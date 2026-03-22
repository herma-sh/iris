use std::cell::Cell as FlagCell;

use iris_core::cursor::Cursor;
use iris_core::damage::{DamageRegion, ScrollDelta};
use iris_core::grid::Grid;
use iris_core::terminal::Terminal;

use crate::cursor::CursorInstance;
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
        let cursor_damage_overlap = self
            .cursor_damage_region(grid, Some(cursor))
            .is_some_and(|region| damage_overlaps_region(&damage[..original_damage_len], region));
        let should_prepare_cursor =
            cursor_changed || shifted_retained_frame || cursor_damage_overlap;

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
        let cursor = cursor?;
        let instance = CursorInstance::from_cursor(cursor, grid, self.theme())
            .ok()
            .flatten()?;
        let row = instance.grid_position[1] as usize;
        let start_col = instance.grid_position[0] as usize;
        let end_col = start_col.saturating_add(instance.extent[0].ceil().max(1.0) as usize - 1);
        Some(DamageRegion::new(row, row, start_col, end_col))
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
mod tests {
    use iris_core::cell::{Cell, CellAttrs, Color};
    use iris_core::damage::{DamageRegion, ScrollDelta};
    use iris_core::parser::Action;
    use iris_core::terminal::Terminal;

    use super::{
        partial_scroll_copy_region, scroll_copy_region, TerminalRenderer, TerminalRendererConfig,
    };
    use crate::error::Error;
    use crate::font::FontRasterizerConfig;
    use crate::renderer::{Renderer, RendererConfig};
    use crate::texture::{TextureSurfaceConfig, TextureSurfaceSize};
    use crate::theme::{Theme, ThemeColor};
    use crate::TextUniforms;

    fn cell_region_has_non_background(
        pixels: &[u8],
        surface_size: TextureSurfaceSize,
        cell_origin: (usize, usize),
        cell_size: (usize, usize),
        background: [u8; 4],
    ) -> bool {
        let row_stride = surface_size.width as usize * background.len();
        let start_x = cell_origin.0 * cell_size.0;
        let start_y = cell_origin.1 * cell_size.1;

        for row in start_y..start_y + cell_size.1 {
            for col in start_x..start_x + cell_size.0 {
                let offset = row * row_stride + col * background.len();
                if pixels[offset..offset + background.len()] != background {
                    return true;
                }
            }
        }

        false
    }

    fn cell_region_matches_background(
        pixels: &[u8],
        surface_size: TextureSurfaceSize,
        cell_origin: (usize, usize),
        cell_size: (usize, usize),
        background: [u8; 4],
    ) -> bool {
        let row_stride = surface_size.width as usize * background.len();
        let start_x = cell_origin.0 * cell_size.0;
        let start_y = cell_origin.1 * cell_size.1;

        for row in start_y..start_y + cell_size.1 {
            for col in start_x..start_x + cell_size.0 {
                let offset = row * row_stride + col * background.len();
                if pixels[offset..offset + background.len()] != background {
                    return false;
                }
            }
        }

        true
    }

    fn pixel_at(
        pixels: &[u8],
        surface_size: TextureSurfaceSize,
        position: (usize, usize),
    ) -> [u8; 4] {
        let row_stride = surface_size.width as usize * 4;
        let offset = position.1 * row_stride + position.0 * 4;
        [
            pixels[offset],
            pixels[offset + 1],
            pixels[offset + 2],
            pixels[offset + 3],
        ]
    }

    fn band_matches_background(
        pixels: &[u8],
        surface_size: TextureSurfaceSize,
        row_range: std::ops::Range<usize>,
        background: [u8; 4],
    ) -> bool {
        let row_stride = surface_size.width as usize * background.len();
        for row in row_range {
            for col in 0..surface_size.width as usize {
                let offset = row * row_stride + col * background.len();
                if pixels[offset..offset + background.len()] != background {
                    return false;
                }
            }
        }

        true
    }

    #[test]
    fn terminal_renderer_prepares_and_renders_terminal_state() {
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
        let mut terminal_renderer = match TerminalRenderer::new(
            &renderer,
            surface.format(),
            TerminalRendererConfig {
                text: crate::text_renderer::TextRendererConfig {
                    theme: Theme {
                        background: ThemeColor::rgb(0x00, 0x00, 0x00),
                        cursor: ThemeColor::rgb(0xff, 0x00, 0x00),
                        ..Theme::default()
                    },
                    uniforms: TextUniforms::new([32.0, 16.0], [16.0, 16.0], 0.0),
                    ..Default::default()
                },
                ..Default::default()
            },
        ) {
            Ok(renderer) => renderer,
            Err(crate::error::Error::NoUsableSystemFont) => return,
            Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
        };
        let mut terminal = Terminal::new(1, 2).expect("terminal should be created");
        terminal
            .write_char('A')
            .expect("terminal write should succeed");

        terminal_renderer
            .prepare_terminal(&renderer, &terminal)
            .expect("terminal frame should prepare");
        terminal_renderer.render_to_texture_surface(&renderer, &surface);

        let pixels = crate::test_support::read_texture_surface(&renderer, &surface);
        let background = crate::test_support::bgra_pixel(terminal_renderer.theme().background);
        assert_eq!(terminal_renderer.instance_count(), 1);
        assert_eq!(terminal_renderer.cursor_instance_count(), 1);
        assert!(
            pixels
                .chunks_exact(background.len())
                .any(|pixel| pixel != background),
            "prepared terminal state should render text and cursor pixels"
        );
    }

    #[test]
    fn terminal_renderer_updates_cached_frame_for_cursor_only_changes() {
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
        let mut terminal_renderer = match TerminalRenderer::new(
            &renderer,
            surface.format(),
            TerminalRendererConfig {
                text: crate::text_renderer::TextRendererConfig {
                    theme: Theme {
                        background: ThemeColor::rgb(0xff, 0x00, 0x00),
                        cursor: ThemeColor::rgb(0xff, 0xff, 0xff),
                        ..Theme::default()
                    },
                    uniforms: TextUniforms::new([32.0, 16.0], [16.0, 16.0], 0.0),
                    ..Default::default()
                },
                ..Default::default()
            },
        ) {
            Ok(renderer) => renderer,
            Err(crate::error::Error::NoUsableSystemFont) => return,
            Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
        };
        let mut terminal = Terminal::new(1, 2).expect("terminal should be created");
        terminal
            .write_char('A')
            .expect("terminal write should succeed");

        terminal_renderer
            .prepare_terminal(&renderer, &terminal)
            .expect("initial terminal frame should prepare");
        terminal_renderer.render_to_texture_surface(&renderer, &surface);
        assert_eq!(terminal_renderer.instance_count(), 1);
        let initial_pixels = crate::test_support::read_texture_surface(&renderer, &surface);
        let background = crate::test_support::bgra_pixel(terminal_renderer.theme().background);
        assert!(
            cell_region_has_non_background(
                &initial_pixels,
                surface.size(),
                (1, 0),
                (16, 16),
                background,
            ),
            "the old cursor cell should be visible before the cursor moves"
        );

        terminal.cursor.move_to(0, 0);

        terminal_renderer
            .update_terminal(&renderer, &mut terminal)
            .expect("cursor-only update should refresh the cached frame");
        terminal_renderer.render_to_texture_surface(&renderer, &surface);

        let cached_pixels =
            crate::test_support::read_texture_surface(&renderer, &terminal_renderer.frame_surface);
        let pixels = crate::test_support::read_texture_surface(&renderer, &surface);
        assert!(
            terminal_renderer.full_redraw_damage.len() >= 2,
            "cursor movement should include old/new cursor repaint regions"
        );
        assert_eq!(terminal_renderer.instance_count(), 2);
        assert_eq!(terminal_renderer.cursor_instance_count(), 1);
        assert_eq!(
            pixel_at(
                &cached_pixels,
                terminal_renderer.frame_surface.size(),
                (24, 24)
            ),
            background,
            "cursor-only updates should clear the old cursor cell in the cached frame"
        );
        assert_eq!(
            pixel_at(&pixels, surface.size(), (24, 8)),
            background,
            "cursor-only updates should clear the old cursor cell back to the background"
        );
        assert!(
            cell_region_has_non_background(&pixels, surface.size(), (0, 0), (16, 16), background),
            "cursor-only updates should preserve visible text and the new cursor position"
        );
    }

    #[test]
    fn terminal_renderer_skips_noop_cursor_redraw_when_nothing_changed() {
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
        let mut terminal_renderer = match TerminalRenderer::new(
            &renderer,
            surface.format(),
            TerminalRendererConfig {
                text: crate::text_renderer::TextRendererConfig {
                    uniforms: TextUniforms::new([48.0, 16.0], [16.0, 16.0], 0.0),
                    ..Default::default()
                },
                ..Default::default()
            },
        ) {
            Ok(renderer) => renderer,
            Err(crate::error::Error::NoUsableSystemFont) => return,
            Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
        };
        let mut terminal = Terminal::new(1, 3).expect("terminal should be created");
        terminal
            .write_ascii_run(b"ab")
            .expect("terminal write should succeed");

        terminal_renderer
            .prepare_terminal(&renderer, &terminal)
            .expect("initial terminal frame should prepare");
        let initial_instances = terminal_renderer.instance_count();
        assert_eq!(
            initial_instances, 2,
            "full prepare should encode visible non-blank cells"
        );

        // Drain pending terminal state so the next update is a true no-op.
        let _ = terminal.take_damage();
        let _ = terminal.take_scroll_delta();

        terminal_renderer
            .update_terminal(&renderer, &mut terminal)
            .expect("no-op update should succeed");

        assert_eq!(
            terminal_renderer.instance_count(),
            initial_instances,
            "no-op updates should keep previous prepared instances instead of forcing cursor-cell redraw"
        );
    }

    #[test]
    fn terminal_renderer_repaints_cell_damage_without_forcing_cursor_damage() {
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
        let mut terminal_renderer = match TerminalRenderer::new(
            &renderer,
            surface.format(),
            TerminalRendererConfig {
                text: crate::text_renderer::TextRendererConfig {
                    uniforms: TextUniforms::new([48.0, 16.0], [16.0, 16.0], 0.0),
                    ..Default::default()
                },
                ..Default::default()
            },
        ) {
            Ok(renderer) => renderer,
            Err(crate::error::Error::NoUsableSystemFont) => return,
            Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
        };
        let mut terminal = Terminal::new(1, 3).expect("terminal should be created");
        terminal
            .write_ascii_run(b"ab")
            .expect("terminal write should succeed");
        terminal.cursor.move_to(0, 2);

        terminal_renderer
            .prepare_terminal(&renderer, &terminal)
            .expect("initial terminal frame should prepare");
        let _ = terminal.take_damage();
        let _ = terminal.take_scroll_delta();

        terminal
            .grid
            .write(0, 0, Cell::new('x'))
            .expect("cell write should mark damage");
        let cursor_before = terminal.cursor;

        terminal_renderer
            .update_terminal(&renderer, &mut terminal)
            .expect("cell-only update should succeed");
        terminal_renderer.render_to_texture_surface(&renderer, &surface);

        let pixels = crate::test_support::read_texture_surface(&renderer, &surface);
        let background = crate::test_support::bgra_pixel(terminal_renderer.theme().background);
        assert_eq!(terminal.cursor, cursor_before);
        assert_eq!(
            terminal_renderer.instance_count(),
            1,
            "cell-only updates should not force cursor damage when cursor and scroll state are unchanged"
        );
        assert!(
            cell_region_has_non_background(&pixels, surface.size(), (0, 0), (16, 16), background),
            "the changed text cell should be repainted"
        );
        assert!(
            cell_region_has_non_background(&pixels, surface.size(), (2, 0), (16, 16), background),
            "the cursor overlay should remain visible at the unchanged cursor position"
        );
    }

    #[test]
    fn terminal_renderer_rebuilds_cursor_overlay_when_damage_overlaps_cursor_cell() {
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
        let cursor_theme = Theme {
            background: ThemeColor::rgb(0x00, 0x00, 0x00),
            cursor: ThemeColor::rgb(0x00, 0xff, 0x00),
            ..Theme::default()
        };
        let mut terminal_renderer = match TerminalRenderer::new(
            &renderer,
            surface.format(),
            TerminalRendererConfig {
                text: crate::text_renderer::TextRendererConfig {
                    theme: cursor_theme.clone(),
                    uniforms: TextUniforms::new([48.0, 16.0], [16.0, 16.0], 0.0),
                    ..Default::default()
                },
                ..Default::default()
            },
        ) {
            Ok(renderer) => renderer,
            Err(crate::error::Error::NoUsableSystemFont) => return,
            Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
        };
        let mut terminal = Terminal::new(1, 3).expect("terminal should be created");
        terminal
            .grid
            .write(0, 0, Cell::new('a'))
            .expect("seed cell should be written");
        terminal.cursor.move_to(0, 0);

        terminal_renderer
            .prepare_terminal(&renderer, &terminal)
            .expect("initial frame should prepare");
        let _ = terminal.take_damage();
        let _ = terminal.take_scroll_delta();

        terminal
            .grid
            .write(
                0,
                0,
                Cell {
                    character: 'A',
                    width: iris_core::cell::CellWidth::Double,
                    attrs: CellAttrs::default(),
                },
            )
            .expect("wide replacement cell should be written");

        terminal_renderer
            .update_terminal(&renderer, &mut terminal)
            .expect("overlapping cell damage should update cursor overlay");
        terminal_renderer.render_to_texture_surface(&renderer, &surface);

        let pixels = crate::test_support::read_texture_surface(&renderer, &surface);
        let cursor_color = crate::test_support::bgra_pixel(cursor_theme.cursor);
        assert_eq!(
            pixel_at(&pixels, surface.size(), (24, 8)),
            cursor_color,
            "cursor overlay should refresh when overlapping damage changes the cursor cell span"
        );
    }

    #[test]
    fn terminal_renderer_scroll_only_updates_still_include_cursor_repaint() {
        let _gpu_test_lock = crate::test_support::gpu_test_lock();
        let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
            Ok(renderer) => renderer,
            Err(crate::error::Error::NoAdapter) => return,
            Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
        };
        let surface = renderer
            .create_texture_surface(TextureSurfaceConfig::new(
                TextureSurfaceSize::new(16, 48).expect("surface dimensions are valid"),
            ))
            .expect("texture surface should be created");
        let mut terminal_renderer = match TerminalRenderer::new(
            &renderer,
            surface.format(),
            TerminalRendererConfig {
                text: crate::text_renderer::TextRendererConfig {
                    uniforms: TextUniforms::new([16.0, 48.0], [16.0, 16.0], 0.0),
                    ..Default::default()
                },
                ..Default::default()
            },
        ) {
            Ok(renderer) => renderer,
            Err(crate::error::Error::NoUsableSystemFont) => return,
            Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
        };
        let mut terminal = Terminal::new(3, 1).expect("terminal should be created");

        terminal_renderer
            .prepare_terminal(&renderer, &terminal)
            .expect("initial frame should prepare");
        let _ = terminal.take_damage();
        let _ = terminal.take_scroll_delta();
        let cursor_before = terminal.cursor;

        terminal
            .apply_action(Action::ScrollUp(1))
            .expect("scroll up should succeed");
        // Exercise the scroll-delta path directly without additional damage.
        let _ = terminal.take_damage();

        terminal_renderer
            .update_terminal(&renderer, &mut terminal)
            .expect("scroll-only update should succeed");
        terminal_renderer.render_to_texture_surface(&renderer, &surface);

        let pixels = crate::test_support::read_texture_surface(&renderer, &surface);
        let background = crate::test_support::bgra_pixel(terminal_renderer.theme().background);
        assert_eq!(terminal.cursor, cursor_before);
        assert_eq!(
            terminal_renderer.full_redraw_damage.len(),
            1,
            "scroll-only updates with an unchanged cursor should deduplicate cursor damage regions"
        );
        assert_eq!(terminal_renderer.cursor_instance_count(), 1);
        assert!(
            cell_region_has_non_background(
                &pixels,
                surface.size(),
                (cursor_before.position.col, cursor_before.position.row),
                (16, 16),
                background,
            ),
            "scroll-only updates should keep cursor pixels visible after retained-frame shifting"
        );
    }

    #[test]
    fn damage_overlaps_region_detects_intersections() {
        let damage = [DamageRegion::new(0, 0, 0, 2), DamageRegion::new(2, 3, 4, 5)];

        assert!(super::damage_overlaps_region(
            &damage,
            DamageRegion::new(0, 0, 2, 4)
        ));
        assert!(super::damage_overlaps_region(
            &damage,
            DamageRegion::new(3, 3, 4, 4)
        ));
        assert!(!super::damage_overlaps_region(
            &damage,
            DamageRegion::new(1, 1, 0, 1)
        ));
        assert!(!super::damage_overlaps_region(
            &damage,
            DamageRegion::new(4, 4, 4, 5)
        ));
    }

    #[test]
    fn terminal_renderer_clears_removed_text_during_damage_updates() {
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
        let mut terminal_renderer = match TerminalRenderer::new(
            &renderer,
            surface.format(),
            TerminalRendererConfig {
                text: crate::text_renderer::TextRendererConfig {
                    theme: Theme {
                        background: ThemeColor::rgb(0xff, 0x00, 0x00),
                        ..Theme::default()
                    },
                    uniforms: TextUniforms::new([32.0, 16.0], [16.0, 16.0], 0.0),
                    ..Default::default()
                },
                ..Default::default()
            },
        ) {
            Ok(renderer) => renderer,
            Err(crate::error::Error::NoUsableSystemFont) => return,
            Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
        };
        let mut terminal = Terminal::new(1, 2).expect("terminal should be created");
        terminal
            .write_char('A')
            .expect("terminal write should succeed");

        terminal_renderer
            .prepare_terminal(&renderer, &terminal)
            .expect("initial terminal frame should prepare");
        terminal
            .grid
            .write(0, 0, iris_core::cell::Cell::default())
            .expect("cell clear should mark damage");
        terminal.cursor.visible = false;

        terminal_renderer
            .update_terminal(&renderer, &mut terminal)
            .expect("blank-cell damage should refresh the cached frame");
        terminal_renderer.render_to_texture_surface(&renderer, &surface);

        let pixels = crate::test_support::read_texture_surface(&renderer, &surface);
        let background = crate::test_support::bgra_pixel(terminal_renderer.theme().background);
        assert!(
            pixels
                .chunks_exact(background.len())
                .all(|pixel| pixel == background),
            "blank default cells should clear previously rendered glyphs back to the background"
        );
    }

    #[test]
    fn terminal_renderer_updates_theme_and_uniform_state() {
        let _gpu_test_lock = crate::test_support::gpu_test_lock();
        let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
            Ok(renderer) => renderer,
            Err(crate::error::Error::NoAdapter) => return,
            Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
        };
        let mut terminal_renderer = match TerminalRenderer::new(
            &renderer,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            TerminalRendererConfig::default(),
        ) {
            Ok(renderer) => renderer,
            Err(crate::error::Error::NoUsableSystemFont) => return,
            Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
        };
        let theme = Theme {
            background: ThemeColor::rgb(0x12, 0x34, 0x56),
            ..Theme::default()
        };
        let uniforms = TextUniforms::new([800.0, 600.0], [10.0, 20.0], 4.0);

        terminal_renderer.set_theme(theme.clone());
        terminal_renderer.set_uniforms(&renderer, uniforms);

        assert_eq!(terminal_renderer.theme(), &theme);
        assert_eq!(terminal_renderer.uniforms(), uniforms);
        assert_eq!(terminal_renderer.font_size_px(), 14.0);
        assert_eq!(
            terminal_renderer.frame_surface_size(),
            TextureSurfaceSize::new(800, 1800).expect("surface dimensions are valid")
        );
    }

    #[test]
    fn terminal_renderer_updates_font_size_and_rebuilds_cached_resources() {
        let _gpu_test_lock = crate::test_support::gpu_test_lock();
        let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
            Ok(renderer) => renderer,
            Err(crate::error::Error::NoAdapter) => return,
            Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
        };
        let mut terminal_renderer = match TerminalRenderer::new(
            &renderer,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            TerminalRendererConfig::default(),
        ) {
            Ok(renderer) => renderer,
            Err(crate::error::Error::NoUsableSystemFont) => return,
            Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
        };
        let mut terminal = Terminal::new(1, 1).expect("terminal should be created");
        terminal
            .write_char('A')
            .expect("terminal write should succeed");

        terminal_renderer
            .prepare_terminal(&renderer, &terminal)
            .expect("initial terminal frame should prepare");
        let _ = terminal.take_damage();
        let _ = terminal.take_scroll_delta();
        terminal_renderer
            .set_font_size_px(&renderer, 18.0)
            .expect("font size update should succeed");
        terminal_renderer
            .update_terminal(&renderer, &mut terminal)
            .expect("font size change should trigger a full cached-frame rebuild");

        assert_eq!(terminal_renderer.font_size_px(), 18.0);
    }

    #[test]
    fn terminal_renderer_rejects_invalid_font_size_updates() {
        let _gpu_test_lock = crate::test_support::gpu_test_lock();
        let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
            Ok(renderer) => renderer,
            Err(crate::error::Error::NoAdapter) => return,
            Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
        };
        let mut terminal_renderer = match TerminalRenderer::new(
            &renderer,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            TerminalRendererConfig::default(),
        ) {
            Ok(renderer) => renderer,
            Err(crate::error::Error::NoUsableSystemFont) => return,
            Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
        };

        let result = terminal_renderer.set_font_size_px(&renderer, 0.0);

        assert!(matches!(result, Err(Error::InvalidFontSize { size: 0.0 })));
        assert_eq!(terminal_renderer.font_size_px(), 14.0);
    }

    #[test]
    fn terminal_renderer_invalidates_cached_frame_when_cell_metrics_change() {
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
        let mut terminal_renderer = match TerminalRenderer::new(
            &renderer,
            surface.format(),
            TerminalRendererConfig {
                text: crate::text_renderer::TextRendererConfig {
                    theme: Theme {
                        background: ThemeColor::rgb(0x00, 0x00, 0x00),
                        cursor: ThemeColor::rgb(0xff, 0x00, 0x00),
                        ..Theme::default()
                    },
                    uniforms: TextUniforms::new([32.0, 16.0], [16.0, 16.0], 0.0),
                    ..Default::default()
                },
                ..Default::default()
            },
        ) {
            Ok(renderer) => renderer,
            Err(crate::error::Error::NoUsableSystemFont) => return,
            Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
        };
        let mut terminal = Terminal::new(1, 2).expect("terminal should be created");
        terminal
            .write_char('A')
            .expect("terminal write should succeed");

        terminal_renderer
            .prepare_terminal(&renderer, &terminal)
            .expect("initial terminal frame should prepare");
        terminal_renderer.render_to_texture_surface(&renderer, &surface);

        let background = crate::test_support::bgra_pixel(terminal_renderer.theme().background);
        let initial_pixels = crate::test_support::read_texture_surface(&renderer, &surface);
        assert_ne!(
            pixel_at(&initial_pixels, surface.size(), (24, 8)),
            background,
            "the initial cursor should occupy the second 16px cell"
        );

        terminal_renderer
            .set_uniforms(&renderer, TextUniforms::new([32.0, 16.0], [8.0, 16.0], 0.0));
        terminal_renderer
            .update_terminal(&renderer, &mut terminal)
            .expect("metric changes should force a full cached-frame rebuild");
        terminal_renderer.render_to_texture_surface(&renderer, &surface);

        let updated_pixels = crate::test_support::read_texture_surface(&renderer, &surface);
        assert_eq!(
            pixel_at(&updated_pixels, surface.size(), (24, 8)),
            background,
            "metric changes should clear stale pixels from the old cursor span"
        );
        assert_ne!(
            pixel_at(&updated_pixels, surface.size(), (12, 8)),
            background,
            "metric changes should redraw the cursor at the new cell width"
        );
    }

    #[test]
    fn scroll_copy_region_returns_none_for_excessive_shift() {
        let uniforms = TextUniforms::new([32.0, 32.0], [16.0, 16.0], 0.0);
        let frame_size = TextureSurfaceSize::new(32, 96).expect("frame size is valid");
        let delta = ScrollDelta::new(0, 1, 100);

        assert!(scroll_copy_region(uniforms, frame_size, delta).is_none());
    }

    #[test]
    fn scroll_copy_region_returns_none_for_zero_width_frame() {
        let uniforms = TextUniforms::new([32.0, 32.0], [16.0, 16.0], 0.0);
        let delta = ScrollDelta::new(0, 1, 1);

        assert!(scroll_copy_region(
            uniforms,
            TextureSurfaceSize {
                width: 0,
                height: 96,
            },
            delta,
        )
        .is_none());
    }

    #[test]
    fn partial_scroll_copy_region_returns_expected_shift_for_middle_band() {
        let uniforms = TextUniforms::new([16.0, 48.0], [16.0, 16.0], 0.0);
        let frame_size = TextureSurfaceSize::new(16, 144).expect("frame size is valid");
        let delta = ScrollDelta::new(1, 2, 1);

        let (region_top, region_bottom, source_y, destination_y, copy_height) =
            partial_scroll_copy_region(uniforms, frame_size, delta)
                .expect("partial scroll copy region should be generated");

        assert_eq!(region_top, 64);
        assert_eq!(region_bottom, 96);
        assert_eq!(source_y, 80);
        assert_eq!(destination_y, 64);
        assert_eq!(copy_height, 16);
    }

    #[test]
    fn terminal_renderer_applies_scroll_offset_during_presentation() {
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
        let mut terminal_renderer = match TerminalRenderer::new(
            &renderer,
            surface.format(),
            TerminalRendererConfig {
                text: crate::text_renderer::TextRendererConfig {
                    theme: Theme {
                        background: ThemeColor::rgb(0xff, 0x00, 0x00),
                        ..Theme::default()
                    },
                    uniforms: TextUniforms::new([32.0, 16.0], [16.0, 16.0], 8.0),
                    ..Default::default()
                },
                ..Default::default()
            },
        ) {
            Ok(renderer) => renderer,
            Err(crate::error::Error::NoUsableSystemFont) => return,
            Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
        };
        let mut terminal = Terminal::new(1, 2).expect("terminal should be created");
        terminal
            .write_char('A')
            .expect("terminal write should succeed");
        terminal.cursor.visible = false;

        terminal_renderer
            .prepare_terminal(&renderer, &terminal)
            .expect("terminal frame should prepare");
        terminal_renderer.render_to_texture_surface(&renderer, &surface);

        let background = crate::test_support::bgra_pixel(terminal_renderer.theme().background);
        let cached_pixels =
            crate::test_support::read_texture_surface(&renderer, &terminal_renderer.frame_surface);
        let pixels = crate::test_support::read_texture_surface(&renderer, &surface);

        assert!(
            cell_region_has_non_background(
                &cached_pixels,
                terminal_renderer.frame_surface.size(),
                (0, 1),
                (16, 16),
                background,
            ),
            "the cached frame should still contain the unshifted glyph content"
        );
        assert!(
            cell_region_matches_background(
                &cached_pixels,
                terminal_renderer.frame_surface.size(),
                (0, 0),
                (16, 16),
                background,
            ),
            "the cached frame should reserve a blank top overscan row"
        );
        assert!(
            band_matches_background(&pixels, surface.size(), 0..8, background),
            "positive presentation scroll offset should reveal background at the top edge"
        );
        assert!(
            cell_region_has_non_background(&pixels, surface.size(), (0, 0), (16, 16), background),
            "the shifted presentation should still contain visible glyph pixels"
        );
    }

    #[test]
    fn terminal_renderer_retains_offscreen_rows_for_smooth_scroll_presentation() {
        let _gpu_test_lock = crate::test_support::gpu_test_lock();
        let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
            Ok(renderer) => renderer,
            Err(crate::error::Error::NoAdapter) => return,
            Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
        };
        let surface = renderer
            .create_texture_surface(TextureSurfaceConfig::new(
                TextureSurfaceSize::new(16, 32).expect("surface dimensions are valid"),
            ))
            .expect("texture surface should be created");
        let mut terminal_renderer = match TerminalRenderer::new(
            &renderer,
            surface.format(),
            TerminalRendererConfig {
                text: crate::text_renderer::TextRendererConfig {
                    theme: Theme {
                        background: ThemeColor::rgb(0x00, 0x00, 0x00),
                        ..Theme::default()
                    },
                    uniforms: TextUniforms::new([16.0, 32.0], [16.0, 16.0], 0.0),
                    ..Default::default()
                },
                ..Default::default()
            },
        ) {
            Ok(renderer) => renderer,
            Err(crate::error::Error::NoUsableSystemFont) => return,
            Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
        };
        let mut terminal = Terminal::new(2, 1).expect("terminal should be created");
        terminal.cursor.visible = false;
        terminal
            .grid
            .write(
                0,
                0,
                iris_core::cell::Cell::with_attrs(
                    ' ',
                    iris_core::cell::CellAttrs {
                        bg: iris_core::cell::Color::Rgb {
                            r: 0xff,
                            g: 0x00,
                            b: 0x00,
                        },
                        ..Default::default()
                    },
                ),
            )
            .expect("top styled cell should be written");
        terminal
            .grid
            .write(
                1,
                0,
                iris_core::cell::Cell::with_attrs(
                    ' ',
                    iris_core::cell::CellAttrs {
                        bg: iris_core::cell::Color::Rgb {
                            r: 0x00,
                            g: 0xff,
                            b: 0x00,
                        },
                        ..Default::default()
                    },
                ),
            )
            .expect("bottom styled cell should be written");

        terminal_renderer
            .prepare_terminal(&renderer, &terminal)
            .expect("initial terminal frame should prepare");
        terminal
            .apply_action(iris_core::parser::Action::ScrollUp(1))
            .expect("scroll up should succeed");
        terminal
            .grid
            .write(
                1,
                0,
                iris_core::cell::Cell::with_attrs(
                    ' ',
                    iris_core::cell::CellAttrs {
                        bg: iris_core::cell::Color::Rgb {
                            r: 0x00,
                            g: 0x00,
                            b: 0xff,
                        },
                        ..Default::default()
                    },
                ),
            )
            .expect("new bottom styled cell should be written");
        terminal_renderer.set_uniforms(
            &renderer,
            TextUniforms::new([16.0, 32.0], [16.0, 16.0], 16.0),
        );
        terminal_renderer
            .update_terminal(&renderer, &mut terminal)
            .expect("scroll update should preserve overscan rows");
        terminal_renderer.render_to_texture_surface(&renderer, &surface);

        let smooth_scroll_pixels = crate::test_support::read_texture_surface(&renderer, &surface);
        assert_eq!(
            pixel_at(&smooth_scroll_pixels, surface.size(), (8, 8)),
            [0x00, 0x00, 0xff, 0xff],
            "positive scroll offset should keep the old top row in the overscan band"
        );
        assert_eq!(
            pixel_at(&smooth_scroll_pixels, surface.size(), (8, 24)),
            [0x00, 0xff, 0x00, 0xff],
            "positive scroll offset should keep the old bottom row visible during the transition"
        );

        terminal_renderer.set_uniforms(
            &renderer,
            TextUniforms::new([16.0, 32.0], [16.0, 16.0], 0.0),
        );
        terminal_renderer.render_to_texture_surface(&renderer, &surface);

        let settled_pixels = crate::test_support::read_texture_surface(&renderer, &surface);
        assert_eq!(
            pixel_at(&settled_pixels, surface.size(), (8, 8)),
            [0x00, 0xff, 0x00, 0xff],
            "settling the scroll offset should reveal the new top row"
        );
        assert_eq!(
            pixel_at(&settled_pixels, surface.size(), (8, 24)),
            [0xff, 0x00, 0x00, 0xff],
            "settling the scroll offset should reveal the newly exposed bottom row"
        );
    }

    #[test]
    fn terminal_renderer_retains_full_viewport_overscan_for_multi_line_scrolls() {
        let _gpu_test_lock = crate::test_support::gpu_test_lock();
        let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
            Ok(renderer) => renderer,
            Err(crate::error::Error::NoAdapter) => return,
            Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
        };
        let surface = renderer
            .create_texture_surface(TextureSurfaceConfig::new(
                TextureSurfaceSize::new(16, 48).expect("surface dimensions are valid"),
            ))
            .expect("texture surface should be created");
        let mut terminal_renderer = match TerminalRenderer::new(
            &renderer,
            surface.format(),
            TerminalRendererConfig {
                text: crate::text_renderer::TextRendererConfig {
                    theme: Theme {
                        background: ThemeColor::rgb(0x00, 0x00, 0x00),
                        ..Theme::default()
                    },
                    uniforms: TextUniforms::new([16.0, 48.0], [16.0, 16.0], 0.0),
                    ..Default::default()
                },
                ..Default::default()
            },
        ) {
            Ok(renderer) => renderer,
            Err(crate::error::Error::NoUsableSystemFont) => return,
            Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
        };
        let mut terminal = Terminal::new(3, 1).expect("terminal should be created");
        terminal.cursor.visible = false;
        for (row, color) in [
            (0, (0xff, 0x00, 0x00)),
            (1, (0x00, 0xff, 0x00)),
            (2, (0x00, 0x00, 0xff)),
        ] {
            terminal
                .grid
                .write(
                    row,
                    0,
                    iris_core::cell::Cell::with_attrs(
                        ' ',
                        iris_core::cell::CellAttrs {
                            bg: iris_core::cell::Color::Rgb {
                                r: color.0,
                                g: color.1,
                                b: color.2,
                            },
                            ..Default::default()
                        },
                    ),
                )
                .expect("styled cell should be written");
        }

        terminal_renderer
            .prepare_terminal(&renderer, &terminal)
            .expect("initial terminal frame should prepare");
        terminal
            .apply_action(iris_core::parser::Action::ScrollUp(2))
            .expect("scroll up should succeed");
        for (row, color) in [(1, (0xff, 0xff, 0x00)), (2, (0x00, 0xff, 0xff))] {
            terminal
                .grid
                .write(
                    row,
                    0,
                    iris_core::cell::Cell::with_attrs(
                        ' ',
                        iris_core::cell::CellAttrs {
                            bg: iris_core::cell::Color::Rgb {
                                r: color.0,
                                g: color.1,
                                b: color.2,
                            },
                            ..Default::default()
                        },
                    ),
                )
                .expect("new styled cell should be written");
        }

        terminal_renderer.set_uniforms(
            &renderer,
            TextUniforms::new([16.0, 48.0], [16.0, 16.0], 32.0),
        );
        terminal_renderer
            .update_terminal(&renderer, &mut terminal)
            .expect("multi-line scroll update should preserve overscan rows");
        terminal_renderer.render_to_texture_surface(&renderer, &surface);

        let transition_pixels = crate::test_support::read_texture_surface(&renderer, &surface);
        assert_eq!(
            pixel_at(&transition_pixels, surface.size(), (8, 8)),
            [0x00, 0x00, 0xff, 0xff]
        );
        assert_eq!(
            pixel_at(&transition_pixels, surface.size(), (8, 24)),
            [0x00, 0xff, 0x00, 0xff]
        );
        assert_eq!(
            pixel_at(&transition_pixels, surface.size(), (8, 40)),
            [0xff, 0x00, 0x00, 0xff]
        );

        terminal_renderer.set_uniforms(
            &renderer,
            TextUniforms::new([16.0, 48.0], [16.0, 16.0], 0.0),
        );
        terminal_renderer.render_to_texture_surface(&renderer, &surface);

        let settled_pixels = crate::test_support::read_texture_surface(&renderer, &surface);
        assert_eq!(
            pixel_at(&settled_pixels, surface.size(), (8, 8)),
            [0xff, 0x00, 0x00, 0xff]
        );
        assert_eq!(
            pixel_at(&settled_pixels, surface.size(), (8, 24)),
            [0x00, 0xff, 0xff, 0xff]
        );
        assert_eq!(
            pixel_at(&settled_pixels, surface.size(), (8, 40)),
            [0xff, 0xff, 0x00, 0xff]
        );
    }

    #[test]
    fn terminal_renderer_renders_partial_scroll_regions_correctly() {
        let _gpu_test_lock = crate::test_support::gpu_test_lock();
        let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
            Ok(renderer) => renderer,
            Err(crate::error::Error::NoAdapter) => return,
            Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
        };
        let surface = renderer
            .create_texture_surface(TextureSurfaceConfig::new(
                TextureSurfaceSize::new(16, 48).expect("surface dimensions are valid"),
            ))
            .expect("texture surface should be created");
        let mut terminal_renderer = match TerminalRenderer::new(
            &renderer,
            surface.format(),
            TerminalRendererConfig {
                text: crate::text_renderer::TextRendererConfig {
                    theme: Theme {
                        background: ThemeColor::rgb(0x00, 0x00, 0x00),
                        ..Theme::default()
                    },
                    uniforms: TextUniforms::new([16.0, 48.0], [16.0, 16.0], 0.0),
                    ..Default::default()
                },
                ..Default::default()
            },
        ) {
            Ok(renderer) => renderer,
            Err(crate::error::Error::NoUsableSystemFont) => return,
            Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
        };
        let mut terminal = Terminal::new(3, 1).expect("terminal should be created");
        terminal.cursor.visible = false;
        terminal
            .grid
            .write(
                0,
                0,
                Cell::with_attrs(
                    ' ',
                    CellAttrs {
                        bg: Color::Rgb {
                            r: 0xff,
                            g: 0x00,
                            b: 0x00,
                        },
                        ..CellAttrs::default()
                    },
                ),
            )
            .expect("top row should be written");
        terminal
            .grid
            .write(
                1,
                0,
                Cell::with_attrs(
                    ' ',
                    CellAttrs {
                        bg: Color::Rgb {
                            r: 0x00,
                            g: 0xff,
                            b: 0x00,
                        },
                        ..CellAttrs::default()
                    },
                ),
            )
            .expect("middle row should be written");
        terminal
            .grid
            .write(
                2,
                0,
                Cell::with_attrs(
                    ' ',
                    CellAttrs {
                        bg: Color::Rgb {
                            r: 0x00,
                            g: 0x00,
                            b: 0xff,
                        },
                        ..CellAttrs::default()
                    },
                ),
            )
            .expect("bottom row should be written");

        terminal_renderer
            .prepare_terminal(&renderer, &terminal)
            .expect("initial frame should prepare");
        terminal_renderer.render_to_texture_surface(&renderer, &surface);

        let red = crate::test_support::bgra_pixel(ThemeColor::rgb(0xff, 0x00, 0x00));
        let green = crate::test_support::bgra_pixel(ThemeColor::rgb(0x00, 0xff, 0x00));
        let blue = crate::test_support::bgra_pixel(ThemeColor::rgb(0x00, 0x00, 0xff));
        let background = crate::test_support::bgra_pixel(terminal_renderer.theme().background);

        let initial_pixels = crate::test_support::read_texture_surface(&renderer, &surface);
        assert_eq!(pixel_at(&initial_pixels, surface.size(), (8, 8)), red);
        assert_eq!(pixel_at(&initial_pixels, surface.size(), (8, 24)), green);
        assert_eq!(pixel_at(&initial_pixels, surface.size(), (8, 40)), blue);

        let _ = terminal.take_damage();
        let _ = terminal.take_scroll_delta();
        terminal
            .apply_action(Action::SetScrollRegion { top: 2, bottom: 3 })
            .expect("scroll region should be set");
        terminal
            .apply_action(Action::ScrollUp(1))
            .expect("scroll up should succeed");

        terminal_renderer
            .update_terminal(&renderer, &mut terminal)
            .expect("scroll-region update should render");
        terminal_renderer.render_to_texture_surface(&renderer, &surface);

        let updated_pixels = crate::test_support::read_texture_surface(&renderer, &surface);
        assert_eq!(
            pixel_at(&updated_pixels, surface.size(), (8, 8)),
            red,
            "rows outside the scroll region should remain unchanged"
        );
        assert_eq!(
            pixel_at(&updated_pixels, surface.size(), (8, 24)),
            blue,
            "scrolling should move the previous bottom row into the middle row"
        );
        assert_eq!(
            pixel_at(&updated_pixels, surface.size(), (8, 40)),
            background,
            "scrolling should clear the exposed bottom row to the theme background"
        );
    }

    #[test]
    fn terminal_renderer_applies_partial_scroll_delta_without_damage_repaint() {
        let _gpu_test_lock = crate::test_support::gpu_test_lock();
        let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
            Ok(renderer) => renderer,
            Err(crate::error::Error::NoAdapter) => return,
            Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
        };
        let surface = renderer
            .create_texture_surface(TextureSurfaceConfig::new(
                TextureSurfaceSize::new(16, 48).expect("surface dimensions are valid"),
            ))
            .expect("texture surface should be created");
        let mut terminal_renderer = match TerminalRenderer::new(
            &renderer,
            surface.format(),
            TerminalRendererConfig {
                text: crate::text_renderer::TextRendererConfig {
                    theme: Theme {
                        background: ThemeColor::rgb(0x00, 0x00, 0x00),
                        ..Theme::default()
                    },
                    uniforms: TextUniforms::new([16.0, 48.0], [16.0, 16.0], 0.0),
                    ..Default::default()
                },
                ..Default::default()
            },
        ) {
            Ok(renderer) => renderer,
            Err(crate::error::Error::NoUsableSystemFont) => return,
            Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
        };
        let mut terminal = Terminal::new(3, 1).expect("terminal should be created");
        terminal.cursor.visible = false;
        for (row, color) in [
            (0, (0xff, 0x00, 0x00)),
            (1, (0x00, 0xff, 0x00)),
            (2, (0x00, 0x00, 0xff)),
        ] {
            terminal
                .grid
                .write(
                    row,
                    0,
                    Cell::with_attrs(
                        ' ',
                        CellAttrs {
                            bg: Color::Rgb {
                                r: color.0,
                                g: color.1,
                                b: color.2,
                            },
                            ..CellAttrs::default()
                        },
                    ),
                )
                .expect("styled row should be written");
        }

        terminal_renderer
            .prepare_terminal(&renderer, &terminal)
            .expect("initial frame should prepare");
        terminal_renderer.render_to_texture_surface(&renderer, &surface);

        let red = crate::test_support::bgra_pixel(ThemeColor::rgb(0xff, 0x00, 0x00));
        let blue = crate::test_support::bgra_pixel(ThemeColor::rgb(0x00, 0x00, 0xff));
        let background = crate::test_support::bgra_pixel(terminal_renderer.theme().background);

        let _ = terminal.take_damage();
        let _ = terminal.take_scroll_delta();
        terminal
            .apply_action(Action::SetScrollRegion { top: 2, bottom: 3 })
            .expect("scroll region should be set");
        terminal
            .apply_action(Action::ScrollUp(1))
            .expect("scroll up should succeed");

        // Force update_terminal to rely on scroll-delta shifting for this pass.
        let _ = terminal.take_damage();

        terminal_renderer
            .update_terminal(&renderer, &mut terminal)
            .expect("scroll-region delta should shift the retained frame");
        terminal_renderer.render_to_texture_surface(&renderer, &surface);

        let updated_pixels = crate::test_support::read_texture_surface(&renderer, &surface);
        assert_eq!(pixel_at(&updated_pixels, surface.size(), (8, 8)), red);
        assert_eq!(pixel_at(&updated_pixels, surface.size(), (8, 24)), blue);
        assert_eq!(
            pixel_at(&updated_pixels, surface.size(), (8, 40)),
            background
        );
    }

    #[test]
    fn terminal_renderer_invalidates_cached_frame_on_theme_change() {
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
        let mut terminal_renderer = match TerminalRenderer::new(
            &renderer,
            surface.format(),
            TerminalRendererConfig {
                text: crate::text_renderer::TextRendererConfig {
                    theme: Theme {
                        background: ThemeColor::rgb(0x00, 0x00, 0x00),
                        ..Theme::default()
                    },
                    uniforms: TextUniforms::new([32.0, 16.0], [16.0, 16.0], 0.0),
                    ..Default::default()
                },
                ..Default::default()
            },
        ) {
            Ok(renderer) => renderer,
            Err(crate::error::Error::NoUsableSystemFont) => return,
            Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
        };
        let mut terminal = Terminal::new(1, 2).expect("terminal should be created");
        terminal
            .write_char('A')
            .expect("terminal write should succeed");
        terminal.cursor.visible = false;

        terminal_renderer
            .prepare_terminal(&renderer, &terminal)
            .expect("initial terminal frame should prepare");
        terminal_renderer.render_to_texture_surface(&renderer, &surface);
        let initial_pixels = crate::test_support::read_texture_surface(&renderer, &surface);
        let initial_background =
            crate::test_support::bgra_pixel(terminal_renderer.theme().background);
        assert!(
            cell_region_matches_background(
                &initial_pixels,
                surface.size(),
                (1, 0),
                (16, 16),
                initial_background,
            ),
            "the blank trailing cell should use the initial theme background"
        );

        terminal_renderer.set_theme(Theme {
            background: ThemeColor::rgb(0xff, 0x00, 0x00),
            ..Theme::default()
        });
        terminal_renderer
            .update_terminal(&renderer, &mut terminal)
            .expect("theme invalidation should force a redraw on the next update");
        terminal_renderer.render_to_texture_surface(&renderer, &surface);

        let updated_pixels = crate::test_support::read_texture_surface(&renderer, &surface);
        let updated_background =
            crate::test_support::bgra_pixel(terminal_renderer.theme().background);
        assert_ne!(initial_background, updated_background);
        assert!(
            cell_region_matches_background(
                &updated_pixels,
                surface.size(),
                (1, 0),
                (16, 16),
                updated_background,
            ),
            "theme changes should invalidate the cached frame and redraw blank cells"
        );
    }

    #[test]
    fn terminal_renderer_marks_present_uniforms_dirty_only_when_inputs_change() {
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
        let mut terminal_renderer = match TerminalRenderer::new(
            &renderer,
            surface.format(),
            TerminalRendererConfig {
                text: crate::text_renderer::TextRendererConfig {
                    uniforms: TextUniforms::new([32.0, 16.0], [16.0, 16.0], 0.0),
                    ..Default::default()
                },
                ..Default::default()
            },
        ) {
            Ok(renderer) => renderer,
            Err(crate::error::Error::NoUsableSystemFont) => return,
            Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
        };

        assert!(
            !terminal_renderer.present_uniforms_dirty.get(),
            "constructor should flush the initial present uniforms"
        );

        terminal_renderer.render_to_texture_surface(&renderer, &surface);
        assert!(
            !terminal_renderer.present_uniforms_dirty.get(),
            "rendering without input changes should not dirty present uniforms"
        );

        terminal_renderer.set_theme(Theme {
            background: ThemeColor::rgb(0x10, 0x20, 0x30),
            ..Theme::default()
        });
        assert!(
            terminal_renderer.present_uniforms_dirty.get(),
            "theme changes should mark present uniforms dirty"
        );
        terminal_renderer.render_to_texture_surface(&renderer, &surface);
        assert!(
            !terminal_renderer.present_uniforms_dirty.get(),
            "rendering should flush dirty present uniforms"
        );

        terminal_renderer.set_uniforms(
            &renderer,
            TextUniforms::new([48.0, 16.0], [16.0, 16.0], 0.0),
        );
        assert!(
            terminal_renderer.present_uniforms_dirty.get(),
            "uniform changes should mark present uniforms dirty"
        );
        terminal_renderer.render_to_texture_surface(&renderer, &surface);
        assert!(
            !terminal_renderer.present_uniforms_dirty.get(),
            "rendering should clear the dirty flag after the GPU write"
        );
    }

    #[test]
    fn terminal_renderer_handles_update_before_prepare() {
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
        let mut terminal_renderer = match TerminalRenderer::new(
            &renderer,
            surface.format(),
            TerminalRendererConfig {
                text: crate::text_renderer::TextRendererConfig {
                    uniforms: TextUniforms::new([32.0, 16.0], [16.0, 16.0], 0.0),
                    ..Default::default()
                },
                ..Default::default()
            },
        ) {
            Ok(renderer) => renderer,
            Err(crate::error::Error::NoUsableSystemFont) => return,
            Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
        };
        let mut terminal = Terminal::new(1, 2).expect("terminal should be created");
        terminal
            .write_char('A')
            .expect("terminal write should succeed");

        terminal_renderer
            .update_terminal(&renderer, &mut terminal)
            .expect("update should fall back to a full redraw before any prepare call");
        terminal_renderer.render_to_texture_surface(&renderer, &surface);

        let pixels = crate::test_support::read_texture_surface(&renderer, &surface);
        let background = crate::test_support::bgra_pixel(terminal_renderer.theme().background);
        assert_eq!(terminal_renderer.instance_count(), 1);
        assert!(
            cell_region_has_non_background(&pixels, surface.size(), (0, 0), (16, 16), background),
            "the first incremental update should produce a visible cached frame"
        );
    }

    #[test]
    fn terminal_renderer_restores_terminal_damage_when_incremental_update_fails() {
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
        let mut terminal_renderer = match TerminalRenderer::new(
            &renderer,
            surface.format(),
            TerminalRendererConfig {
                text: crate::text_renderer::TextRendererConfig {
                    uniforms: TextUniforms::new([32.0, 16.0], [16.0, 16.0], 0.0),
                    ..Default::default()
                },
                font_rasterizer: FontRasterizerConfig {
                    font_size_px: 4096.0,
                    ..Default::default()
                },
            },
        ) {
            Ok(renderer) => renderer,
            Err(crate::error::Error::NoUsableSystemFont) => return,
            Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
        };
        let mut terminal = Terminal::new(1, 2).expect("terminal should be created");
        terminal.cursor.visible = false;

        terminal_renderer
            .prepare_terminal(&renderer, &terminal)
            .expect("blank terminal frame should prepare");
        terminal
            .write_char('A')
            .expect("terminal write should succeed");

        let result = terminal_renderer.update_terminal(&renderer, &mut terminal);

        assert!(matches!(
            result,
            Err(Error::GlyphRasterizationFailed { .. })
        ));
        assert_eq!(
            terminal.take_damage(),
            vec![iris_core::damage::DamageRegion::new(0, 0, 0, 0)],
            "failed incremental updates should restore terminal damage"
        );
    }

    #[test]
    fn terminal_renderer_invalidates_cached_frame_when_scroll_shift_update_fails() {
        let _gpu_test_lock = crate::test_support::gpu_test_lock();
        let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
            Ok(renderer) => renderer,
            Err(crate::error::Error::NoAdapter) => return,
            Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
        };
        let surface = renderer
            .create_texture_surface(TextureSurfaceConfig::new(
                TextureSurfaceSize::new(16, 48).expect("surface dimensions are valid"),
            ))
            .expect("texture surface should be created");
        let mut terminal_renderer = match TerminalRenderer::new(
            &renderer,
            surface.format(),
            TerminalRendererConfig {
                text: crate::text_renderer::TextRendererConfig {
                    uniforms: TextUniforms::new([16.0, 48.0], [16.0, 16.0], 0.0),
                    ..Default::default()
                },
                font_rasterizer: FontRasterizerConfig {
                    font_size_px: 4096.0,
                    ..Default::default()
                },
            },
        ) {
            Ok(renderer) => renderer,
            Err(crate::error::Error::NoUsableSystemFont) => return,
            Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
        };
        let mut terminal = Terminal::new(3, 1).expect("terminal should be created");
        terminal.cursor.visible = false;

        terminal_renderer
            .prepare_terminal(&renderer, &terminal)
            .expect("blank terminal frame should prepare");

        let _ = terminal.take_damage();
        let _ = terminal.take_scroll_delta();
        terminal
            .apply_action(Action::SetScrollRegion { top: 2, bottom: 3 })
            .expect("scroll region should be set");
        terminal
            .apply_action(Action::ScrollUp(1))
            .expect("scroll up should succeed");
        terminal.move_cursor(1, 0);
        terminal
            .write_char('A')
            .expect("terminal write should succeed");

        let result = terminal_renderer.update_terminal(&renderer, &mut terminal);

        assert!(matches!(
            result,
            Err(Error::GlyphRasterizationFailed { .. })
        ));
        assert!(
            !terminal_renderer.frame_initialized,
            "incremental failures after scroll shifts should invalidate the cached frame"
        );
        assert_eq!(
            terminal.take_scroll_delta(),
            Some(ScrollDelta::new(1, 2, 1)),
            "failed incremental updates should restore consumed scroll deltas"
        );
    }
}
