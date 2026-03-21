use iris_core::cursor::Cursor;
use iris_core::damage::DamageRegion;
use iris_core::grid::Grid;
use iris_core::terminal::Terminal;

use crate::error::Result;
use crate::font::{FontRasterizer, FontRasterizerConfig};
use crate::pipeline::PresentPipeline;
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
    present_bind_group: wgpu::BindGroup,
    full_redraw_damage: Vec<DamageRegion>,
}

impl TerminalRenderer {
    /// Creates a terminal renderer for the provided render-target format.
    pub fn new(
        renderer: &Renderer,
        format: wgpu::TextureFormat,
        config: TerminalRendererConfig,
    ) -> Result<Self> {
        let frame_surface = create_frame_surface(renderer, format, config.text.uniforms);
        let text_renderer = TextRenderer::new(renderer, frame_surface.format(), config.text)?;
        let font_rasterizer = FontRasterizer::new(config.font_rasterizer)?;
        let present_pipeline = renderer.create_present_pipeline(format);
        let present_bind_group =
            present_pipeline.create_texture_bind_group(renderer.device(), &frame_surface);

        Ok(Self {
            text_renderer,
            font_rasterizer,
            frame_surface,
            present_pipeline,
            present_bind_group,
            full_redraw_damage: Vec::with_capacity(1),
        })
    }

    /// Returns the active theme used for text, cursor, and clear color.
    #[must_use]
    pub const fn theme(&self) -> &Theme {
        self.text_renderer.theme()
    }

    /// Replaces the active renderer theme.
    pub fn set_theme(&mut self, theme: Theme) {
        self.text_renderer.set_theme(theme);
    }

    /// Returns the current text uniforms.
    #[must_use]
    pub const fn uniforms(&self) -> TextUniforms {
        self.text_renderer.uniforms()
    }

    /// Returns the cached frame-surface size.
    #[must_use]
    pub const fn frame_surface_size(&self) -> TextureSurfaceSize {
        self.frame_surface.size()
    }

    /// Updates the viewport and cell metrics written to the uniform buffer.
    pub fn set_uniforms(&mut self, renderer: &Renderer, uniforms: TextUniforms) {
        self.text_renderer.set_uniforms(renderer, uniforms);
        self.resize_frame_surface(renderer, uniforms);
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
        Ok(())
    }

    /// Renders the cached frame into an off-screen texture surface.
    pub fn render_to_texture_surface(&self, renderer: &Renderer, surface: &TextureSurface) {
        let bind_group = self
            .present_pipeline
            .create_texture_bind_group(renderer.device(), &self.frame_surface);
        renderer.draw_present_pipeline_to_texture_surface(
            &self.present_pipeline,
            &bind_group,
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
        self.present_pipeline
            .render(&mut encoder, &view, &self.present_bind_group);
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

fn normalized_surface_dimension(dimension: f32) -> u32 {
    if !dimension.is_finite() || dimension <= 0.0 {
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
    fn terminal_renderer_redraws_the_full_frame_for_cursor_only_updates() {
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
            .prepare_terminal(&renderer, &terminal)
            .expect("initial terminal frame should prepare");
        assert_eq!(terminal_renderer.instance_count(), 1);

        let _ = terminal.take_damage();
        terminal.cursor.move_to(0, 0);

        terminal_renderer
            .prepare_terminal(&renderer, &terminal)
            .expect("cursor-only update should still prepare");
        terminal_renderer.render_to_texture_surface(&renderer, &surface);

        let pixels = crate::test_support::read_texture_surface(&renderer, &surface);
        let background = crate::test_support::bgra_pixel(terminal_renderer.theme().background);
        assert_eq!(terminal_renderer.instance_count(), 1);
        assert_eq!(terminal_renderer.cursor_instance_count(), 1);
        assert!(
            pixels
                .chunks_exact(background.len())
                .any(|pixel| pixel != background),
            "full-frame redraw should preserve text when only the cursor moves"
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
}
