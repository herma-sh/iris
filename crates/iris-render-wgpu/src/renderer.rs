use crate::atlas::{AtlasConfig, GlyphAtlas};
use crate::cell::{encode_damage_instances, CellInstance, TextBuffers, TextUniforms};
use crate::cursor::CursorBuffers;
use crate::error::{Error, Result};
use crate::glyph::{CachedGlyph, GlyphBitmap, GlyphCache, GlyphKey, GlyphPlacement};
use crate::pipeline::{CursorPipeline, FullscreenPipeline, PresentPipeline, TextPipeline};
use crate::surface::{RendererSurface, SurfaceConfig, SurfaceSize};
use crate::texture::{TextureSurface, TextureSurfaceConfig};
use crate::theme::Theme;
use iris_core::damage::DamageRegion;
use iris_core::grid::Grid;

/// Bootstrap configuration for the GPU renderer.
#[derive(Clone, Debug)]
pub struct RendererConfig {
    /// Backends that may be used to create the renderer.
    pub backends: wgpu::Backends,
    /// Adapter selection preference.
    pub power_preference: wgpu::PowerPreference,
    /// Whether to force a fallback adapter.
    pub force_fallback_adapter: bool,
    /// Features that must be enabled on the requested device.
    pub required_features: wgpu::Features,
    /// Limits that must be supported by the requested device.
    pub required_limits: wgpu::Limits,
}

impl Default for RendererConfig {
    fn default() -> Self {
        Self {
            backends: wgpu::Backends::all(),
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
        }
    }
}

/// Owns the `wgpu` instance, adapter, device, and queue used for rendering.
#[derive(Debug)]
pub struct Renderer {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl Renderer {
    /// Creates a renderer with a headless adapter/device bootstrap.
    pub async fn new(config: RendererConfig) -> Result<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: config.backends,
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: config.power_preference,
                force_fallback_adapter: config.force_fallback_adapter,
                compatible_surface: None,
            })
            .await
            .ok_or(Error::NoAdapter)?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("iris-render-wgpu-device"),
                    required_features: config.required_features,
                    required_limits: config.required_limits,
                },
                None,
            )
            .await
            .map_err(|request_error| Error::RequestDevice {
                reason: request_error.to_string(),
            })?;

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
        })
    }

    /// Returns the `wgpu` instance used by the renderer.
    #[must_use]
    pub const fn instance(&self) -> &wgpu::Instance {
        &self.instance
    }

    /// Returns the selected adapter metadata.
    #[must_use]
    pub fn adapter_info(&self) -> wgpu::AdapterInfo {
        self.adapter.get_info()
    }

    /// Returns the selected adapter feature set.
    #[must_use]
    pub fn adapter_features(&self) -> wgpu::Features {
        self.adapter.features()
    }

    /// Returns the selected adapter limits.
    #[must_use]
    pub fn adapter_limits(&self) -> wgpu::Limits {
        self.adapter.limits()
    }

    /// Returns the initialized device.
    #[must_use]
    pub const fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Returns the initialized queue.
    #[must_use]
    pub const fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Allocates an off-screen texture render target.
    pub fn create_texture_surface(&self, config: TextureSurfaceConfig) -> Result<TextureSurface> {
        TextureSurface::new(&self.device, config)
    }

    /// Creates the glyph atlas texture used by later text rendering work.
    pub fn create_glyph_atlas(&self, config: AtlasConfig) -> Result<GlyphAtlas> {
        GlyphAtlas::new(&self.device, config)
    }

    /// Creates the CPU-side glyph cache used to track atlas residency.
    #[must_use]
    pub fn create_glyph_cache(&self) -> GlyphCache {
        GlyphCache::new()
    }

    /// Creates the uniform and instance buffers used by later text rendering work.
    pub fn create_text_buffers(&self, instance_capacity: usize) -> Result<TextBuffers> {
        TextBuffers::new(&self.device, instance_capacity)
    }

    /// Creates the single-instance cursor buffers used by the cursor overlay pass.
    #[must_use]
    pub fn create_cursor_buffers(&self) -> CursorBuffers {
        CursorBuffers::new(&self.device)
    }

    /// Caches a glyph bitmap in the provided atlas.
    pub fn cache_glyph(
        &self,
        cache: &mut GlyphCache,
        atlas: &mut GlyphAtlas,
        key: GlyphKey,
        bitmap: GlyphBitmap<'_>,
    ) -> Result<CachedGlyph> {
        cache.cache_glyph(atlas, &self.queue, key, bitmap)
    }

    /// Caches a glyph bitmap in the provided atlas with explicit placement offsets.
    pub fn cache_glyph_with_placement(
        &self,
        cache: &mut GlyphCache,
        atlas: &mut GlyphAtlas,
        key: GlyphKey,
        bitmap: GlyphBitmap<'_>,
        placement: GlyphPlacement,
    ) -> Result<CachedGlyph> {
        cache.cache_glyph_with_placement(atlas, &self.queue, key, bitmap, placement)
    }

    /// Uploads the latest text uniforms.
    pub fn write_text_uniforms(&self, buffers: &TextBuffers, uniforms: &TextUniforms) {
        buffers.write_uniforms(&self.queue, uniforms);
    }

    /// Uploads text instances, growing the instance buffer when required.
    pub fn write_text_instances(
        &self,
        buffers: &mut TextBuffers,
        instances: &[CellInstance],
    ) -> Result<()> {
        buffers.write_instances(&self.device, &self.queue, instances)
    }

    /// Encodes damaged grid cells into reusable text instances using cached glyphs.
    pub fn encode_text_instances_for_damage<F>(
        &self,
        instances: &mut Vec<CellInstance>,
        grid: &Grid,
        damage: &[DamageRegion],
        atlas: &GlyphAtlas,
        theme: &Theme,
        resolve_glyph: F,
    ) -> Result<()>
    where
        F: FnMut(iris_core::cell::Cell) -> Option<CachedGlyph>,
    {
        encode_damage_instances(instances, grid, damage, atlas.size(), theme, resolve_glyph)
    }

    /// Creates the temporary fullscreen pipeline used for renderer bootstrap.
    #[must_use]
    pub fn create_fullscreen_pipeline(&self, format: wgpu::TextureFormat) -> FullscreenPipeline {
        FullscreenPipeline::new(&self.device, format)
    }

    /// Creates the text pipeline used for atlas-backed cell rendering bootstrap.
    #[must_use]
    pub fn create_text_pipeline(
        &self,
        format: wgpu::TextureFormat,
        atlas: &GlyphAtlas,
    ) -> TextPipeline {
        TextPipeline::new(&self.device, format, atlas)
    }

    /// Creates the uniform bind group used by the text pipeline.
    #[must_use]
    pub fn create_text_uniform_bind_group(
        &self,
        pipeline: &TextPipeline,
        buffers: &TextBuffers,
    ) -> wgpu::BindGroup {
        pipeline.create_uniform_bind_group(&self.device, buffers)
    }

    /// Creates the cursor overlay pipeline used by the renderer.
    #[must_use]
    pub fn create_cursor_pipeline(&self, format: wgpu::TextureFormat) -> CursorPipeline {
        CursorPipeline::new(&self.device, format)
    }

    /// Creates the fullscreen textured presentation pipeline used for cached frames.
    #[must_use]
    pub fn create_present_pipeline(&self, format: wgpu::TextureFormat) -> PresentPipeline {
        PresentPipeline::new(&self.device, format)
    }

    /// Creates the uniform bind group used by the cursor pipeline.
    #[must_use]
    pub fn create_cursor_uniform_bind_group(
        &self,
        pipeline: &CursorPipeline,
        buffers: &TextBuffers,
    ) -> wgpu::BindGroup {
        pipeline.create_uniform_bind_group(&self.device, buffers)
    }

    /// Creates and configures a presentation surface for a window target.
    pub fn create_surface<'window>(
        &self,
        target: impl Into<wgpu::SurfaceTarget<'window>>,
        config: SurfaceConfig,
    ) -> Result<RendererSurface<'window>> {
        let surface = self
            .instance
            .create_surface(target)
            .map_err(|create_surface_error| Error::CreateSurface {
                reason: create_surface_error.to_string(),
            })?;

        RendererSurface::new(surface, &self.adapter, &self.device, config)
    }

    /// Reconfigures an existing presentation surface for a new size.
    pub fn resize_surface(
        &self,
        surface: &mut RendererSurface<'_>,
        size: SurfaceSize,
    ) -> Result<()> {
        surface.resize(&self.device, size)
    }

    /// Draws the bootstrap fullscreen pipeline into an off-screen texture surface.
    pub fn draw_fullscreen_pipeline_to_texture_surface(
        &self,
        pipeline: &FullscreenPipeline,
        surface: &TextureSurface,
    ) {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("iris-render-wgpu-fullscreen-encoder"),
            });
        pipeline.render(&mut encoder, surface.view());
        self.queue.submit(std::iter::once(encoder.finish()));
    }

    /// Draws the text pipeline into an off-screen texture surface.
    pub fn draw_text_pipeline_to_texture_surface(
        &self,
        pipeline: &TextPipeline,
        uniform_bind_group: &wgpu::BindGroup,
        atlas: &GlyphAtlas,
        buffers: &TextBuffers,
        surface: &TextureSurface,
    ) {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("iris-render-wgpu-text-encoder"),
            });
        pipeline.render(
            &mut encoder,
            surface.view(),
            uniform_bind_group,
            atlas,
            buffers,
            wgpu::Color::BLACK,
        );
        self.queue.submit(std::iter::once(encoder.finish()));
    }

    /// Clears an off-screen render target to the provided color.
    pub fn clear_texture_surface(&self, surface: &TextureSurface, color: wgpu::Color) {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("iris-render-wgpu-clear"),
            });

        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("iris-render-wgpu-clear-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface.view(),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }

    /// Draws a cached frame texture into an off-screen render target.
    pub fn draw_present_pipeline_to_texture_surface(
        &self,
        pipeline: &PresentPipeline,
        uniform_bind_group: &wgpu::BindGroup,
        texture_bind_group: &wgpu::BindGroup,
        surface: &TextureSurface,
    ) {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("iris-render-wgpu-present-texture-encoder"),
            });
        pipeline.render(
            &mut encoder,
            surface.view(),
            uniform_bind_group,
            texture_bind_group,
        );
        self.queue.submit(std::iter::once(encoder.finish()));
    }
}

#[cfg(test)]
#[path = "test/renderer/tests.rs"]
mod tests;
