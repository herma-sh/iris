//! GPU-backed renderer bootstrap for Iris.
//!
//! This phase-2 crate currently establishes `wgpu` device initialization and
//! testable off-screen render targets. Text pipelines, glyph caching, and
//! on-screen surface integration land in follow-up changes.

pub mod error;
pub mod renderer;
pub mod texture;

pub use error::{Error, Result};
pub use renderer::{Renderer, RendererConfig};
pub use texture::{TextureSurface, TextureSurfaceConfig, TextureSurfaceSize};
