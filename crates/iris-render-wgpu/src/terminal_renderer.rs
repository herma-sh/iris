use iris_core::cursor::Cursor;
use iris_core::damage::DamageRegion;
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
    font_rasterizer: FontRasterizer,
    frame_surface: TextureSurface,
    present_pipeline: PresentPipeline,
    present_uniform_buffer: wgpu::Buffer,
    requested_uniforms: TextUniforms,
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
            mut text,
            font_rasterizer,
        } = config;
        let requested_uniforms = text.uniforms;
        let frame_surface = create_frame_surface(renderer, format, requested_uniforms);
        text.uniforms = frame_uniforms_for_requested(requested_uniforms);
        let text_renderer = TextRenderer::new(renderer, frame_surface.format(), text)?;
        let font_rasterizer = FontRasterizer::new(font_rasterizer)?;
        let present_pipeline = renderer.create_present_pipeline(format);
        let present_uniform_buffer = present_pipeline.create_uniform_buffer(renderer.device());
        let present_uniform_bind_group =
            present_pipeline.create_uniform_bind_group(renderer.device(), &present_uniform_buffer);
        let present_bind_group =
            present_pipeline.create_texture_bind_group(renderer.device(), &frame_surface);

        let terminal_renderer = Self {
            text_renderer,
            font_rasterizer,
            frame_surface,
            present_pipeline,
            present_uniform_buffer,
            requested_uniforms,
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
        self.text_renderer.set_theme(theme);
        self.frame_initialized = false;
        self.previous_cursor = None;
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
        self.requested_uniforms = uniforms;
        self.text_renderer
            .set_uniforms(renderer, frame_uniforms_for_requested(uniforms));
        self.resize_frame_surface(renderer, uniforms);
        self.write_present_uniforms(renderer);
    }

    /// Returns the configured system font size in pixels.
    #[must_use]
    pub const fn font_size_px(&self) -> f32 {
        self.font_rasterizer.font_size_px()
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
        let damage = terminal.take_damage();
        self.update_grid_and_cursor(renderer, &terminal.grid, &damage, terminal.cursor)
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
        cursor: Cursor,
    ) -> Result<()> {
        if !self.frame_initialized {
            return self.prepare_grid_and_cursor(renderer, grid, cursor);
        }

        self.full_redraw_damage.clear();
        self.full_redraw_damage.extend_from_slice(damage);
        self.push_cursor_damage(grid, self.previous_cursor);
        self.push_cursor_damage(grid, Some(cursor));

        if self.full_redraw_damage.is_empty() {
            self.previous_cursor = Some(cursor);
            return Ok(());
        }

        self.text_renderer
            .prepare_grid_update_with_font_rasterizer(
                renderer,
                grid,
                &self.full_redraw_damage,
                &mut self.font_rasterizer,
            )?;
        self.text_renderer.prepare_cursor(renderer, grid, cursor)?;
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

    fn push_cursor_damage(&mut self, grid: &Grid, cursor: Option<Cursor>) {
        let Some(cursor) = cursor else {
            return;
        };
        let Some(instance) = CursorInstance::from_cursor(cursor, grid, self.theme())
            .ok()
            .flatten()
        else {
            return;
        };
        let row = instance.grid_position[1] as usize;
        let start_col = instance.grid_position[0] as usize;
        let end_col = start_col.saturating_add(instance.extent[0].ceil().max(1.0) as usize - 1);
        self.full_redraw_damage
            .push(DamageRegion::new(row, row, start_col, end_col));
    }

    fn resize_frame_surface(&mut self, renderer: &Renderer, uniforms: TextUniforms) {
        let next_size = frame_surface_size_for_uniforms(uniforms);
        if next_size == self.frame_surface.size() {
            return;
        }

        self.frame_surface =
            create_frame_surface(renderer, self.present_pipeline.format(), uniforms);
        self.present_bind_group = self
            .present_pipeline
            .create_texture_bind_group(renderer.device(), &self.frame_surface);
        self.write_present_uniforms(renderer);
        self.frame_initialized = false;
        self.previous_cursor = None;
    }

    fn write_present_uniforms(&self, renderer: &Renderer) {
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
                | wgpu::TextureUsages::TEXTURE_BINDING,
        })
        .expect("internal frame-surface config should remain valid")
}

fn frame_surface_size_for_uniforms(uniforms: TextUniforms) -> TextureSurfaceSize {
    TextureSurfaceSize {
        width: normalized_surface_dimension(uniforms.resolution[0]),
        height: normalized_surface_dimension(uniforms.resolution[1]),
    }
}

fn frame_uniforms_for_requested(uniforms: TextUniforms) -> TextUniforms {
    TextUniforms::new(uniforms.resolution, uniforms.cell_size, 0.0)
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
        uniforms.scroll_offset,
        theme.background.to_f32_array(),
    )
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
    use iris_core::terminal::Terminal;

    use super::{TerminalRenderer, TerminalRendererConfig};
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
        assert_eq!(terminal_renderer.instance_count(), 2);
        assert_eq!(terminal_renderer.cursor_instance_count(), 1);
        assert_eq!(
            pixel_at(
                &cached_pixels,
                terminal_renderer.frame_surface.size(),
                (24, 8)
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
            TextureSurfaceSize::new(800, 600).expect("surface dimensions are valid")
        );
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
                (0, 0),
                (16, 16),
                background,
            ),
            "the cached frame should still contain the unshifted glyph content"
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
}
