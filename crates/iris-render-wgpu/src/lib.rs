//! GPU-backed renderer bootstrap for Iris.
//!
//! This crate currently establishes `wgpu` device initialization and
//! testable off-screen render targets. Text rasterization, grid batching, and
//! on-screen rendering land in follow-up changes.

pub mod atlas;
pub mod cell;
pub mod error;
pub mod font;
pub mod glyph;
pub mod pipeline;
pub mod renderer;
pub mod surface;
pub mod text_renderer;
pub mod texture;
pub mod theme;

pub use atlas::{AtlasConfig, AtlasRegion, AtlasSize, GlyphAtlas};
pub use cell::{cell_instances_as_bytes, CellColors, CellInstance, TextBuffers, TextUniforms};
pub use error::{Error, Result};
pub use font::{FontRasterizer, FontRasterizerConfig};
pub use glyph::{CachedGlyph, GlyphBitmap, GlyphCache, GlyphKey, RasterizedGlyph};
pub use pipeline::{FullscreenPipeline, TextPipeline};
pub use renderer::{Renderer, RendererConfig};
pub use surface::{RendererSurface, SurfaceConfig, SurfaceSize};
pub use text_renderer::{TextRenderer, TextRendererConfig};
pub use texture::{TextureSurface, TextureSurfaceConfig, TextureSurfaceSize};
pub use theme::{Theme, ThemeColor};

#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::{mpsc, Mutex, MutexGuard, OnceLock};

    use crate::{renderer::Renderer, texture::TextureSurface};

    pub(crate) fn gpu_test_lock() -> MutexGuard<'static, ()> {
        static GPU_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

        GPU_TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub(crate) fn read_texture_surface(renderer: &Renderer, surface: &TextureSurface) -> Vec<u8> {
        let width = surface.size().width as usize;
        let height = surface.size().height as usize;
        let bytes_per_pixel = 4usize;
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let alignment = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(alignment) * alignment;
        let copy_buffer_size = (padded_bytes_per_row * height) as wgpu::BufferAddress;
        let buffer = renderer.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("iris-render-wgpu-test-readback"),
            size: copy_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder =
            renderer
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("iris-render-wgpu-test-readback-encoder"),
                });

        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: surface.texture(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row as u32),
                    rows_per_image: Some(height as u32),
                },
            },
            wgpu::Extent3d {
                width: surface.size().width,
                height: surface.size().height,
                depth_or_array_layers: 1,
            },
        );

        renderer.queue().submit(std::iter::once(encoder.finish()));

        let slice = buffer.slice(..);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            sender
                .send(result)
                .expect("readback channel should remain open");
        });
        renderer.device().poll(wgpu::Maintain::Wait);
        receiver
            .recv()
            .expect("readback channel should receive a result")
            .expect("texture readback should succeed");

        let mapped = slice.get_mapped_range();
        let mut bytes = vec![0; unpadded_bytes_per_row * height];
        for row in 0..height {
            let source_offset = row * padded_bytes_per_row;
            let dest_offset = row * unpadded_bytes_per_row;
            bytes[dest_offset..dest_offset + unpadded_bytes_per_row]
                .copy_from_slice(&mapped[source_offset..source_offset + unpadded_bytes_per_row]);
        }
        drop(mapped);
        buffer.unmap();

        bytes
    }

    pub(crate) const fn bgra_pixel(color: crate::theme::ThemeColor) -> [u8; 4] {
        [color.b, color.g, color.r, color.a]
    }
}
