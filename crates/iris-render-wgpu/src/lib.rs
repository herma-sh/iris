//! GPU-backed renderer bootstrap for Iris.
//!
//! This crate currently establishes `wgpu` device initialization and
//! testable off-screen render targets. Text pipelines, glyph caching, and
//! on-screen surface integration land in follow-up changes.

pub mod atlas;
pub mod error;
pub mod pipeline;
pub mod renderer;
pub mod surface;
pub mod texture;

pub use atlas::{AtlasConfig, AtlasRegion, AtlasSize, GlyphAtlas};
pub use error::{Error, Result};
pub use pipeline::FullscreenPipeline;
pub use renderer::{Renderer, RendererConfig};
pub use surface::{RendererSurface, SurfaceConfig, SurfaceSize};
pub use texture::{TextureSurface, TextureSurfaceConfig, TextureSurfaceSize};
