//! GPU-backed renderer bootstrap for Iris.
//!
//! This crate currently establishes `wgpu` device initialization and
//! testable off-screen render targets. Text rasterization, grid batching, and
//! on-screen rendering land in follow-up changes.

pub mod atlas;
pub mod cell;
pub mod error;
pub mod glyph;
pub mod pipeline;
pub mod renderer;
pub mod surface;
pub mod texture;

pub use atlas::{AtlasConfig, AtlasRegion, AtlasSize, GlyphAtlas};
pub use cell::{cell_instances_as_bytes, CellColors, CellInstance, TextBuffers, TextUniforms};
pub use error::{Error, Result};
pub use glyph::{CachedGlyph, GlyphBitmap, GlyphCache, GlyphKey};
pub use pipeline::{FullscreenPipeline, TextPipeline};
pub use renderer::{Renderer, RendererConfig};
pub use surface::{RendererSurface, SurfaceConfig, SurfaceSize};
pub use texture::{TextureSurface, TextureSurfaceConfig, TextureSurfaceSize};

#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::{Mutex, MutexGuard, OnceLock};

    pub(crate) fn gpu_test_lock() -> MutexGuard<'static, ()> {
        static GPU_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

        GPU_TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
