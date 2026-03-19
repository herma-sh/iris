use thiserror::Error;

/// The result type used by `iris-render-wgpu`.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors returned by the renderer bootstrap layer.
#[derive(Debug, Error)]
pub enum Error {
    /// No compatible GPU adapter could be found for the configured backend set.
    #[error("no suitable GPU adapter found for the configured backends")]
    NoAdapter,

    /// The GPU device request failed.
    #[error("GPU device creation failed: {reason}")]
    RequestDevice { reason: String },

    /// Surface creation failed.
    #[error("surface creation failed: {reason}")]
    CreateSurface { reason: String },

    /// An atlas requires non-zero dimensions.
    #[error("atlas size must be non-zero, got {width}x{height}")]
    InvalidAtlasSize { width: u32, height: u32 },

    /// Atlas allocations must fit within the atlas.
    #[error("atlas allocation must fit within the atlas, got {width}x{height}")]
    InvalidAtlasAllocation { width: u32, height: u32 },

    /// Atlas upload data must match the target region dimensions.
    #[error("atlas upload size mismatch: expected {expected} bytes, got {actual}")]
    InvalidAtlasUploadSize { expected: usize, actual: usize },

    /// The atlas has no remaining room for the requested allocation.
    #[error("atlas is full for allocation {width}x{height}")]
    AtlasFull { width: u32, height: u32 },

    /// A glyph cache entry must not be reused with different dimensions.
    #[error(
        "glyph cache entry {key} already exists with size {cached_width}x{cached_height}, cannot reuse it for {requested_width}x{requested_height}"
    )]
    GlyphCacheEntryMismatch {
        /// The caller-defined glyph cache key.
        key: u64,
        /// The cached glyph width.
        cached_width: u32,
        /// The cached glyph height.
        cached_height: u32,
        /// The requested glyph width.
        requested_width: u32,
        /// The requested glyph height.
        requested_height: u32,
    },

    /// Continuation cells must not be emitted as standalone text instances.
    #[error("continuation cells cannot be encoded as renderable text instances")]
    ContinuationCellNotRenderable,

    /// A texture surface requires non-zero dimensions.
    #[error("texture surface size must be non-zero, got {width}x{height}")]
    InvalidTextureSurfaceSize { width: u32, height: u32 },

    /// A texture surface must be usable as a render attachment.
    #[error("texture surface usage must include RENDER_ATTACHMENT")]
    InvalidTextureSurfaceUsage,

    /// A presentation surface requires non-zero dimensions.
    #[error("surface size must be non-zero, got {width}x{height}")]
    InvalidSurfaceSize { width: u32, height: u32 },

    /// The selected adapter cannot present to the target surface.
    #[error("surface is not supported by the selected adapter")]
    SurfaceUnsupportedByAdapter,

    /// The requested presentation mode is unsupported for the selected surface.
    #[error("requested surface present mode is unsupported: {present_mode:?}")]
    UnsupportedSurfacePresentMode {
        /// The unsupported presentation mode.
        present_mode: wgpu::PresentMode,
    },

    /// The requested alpha mode is unsupported for the selected surface.
    #[error("requested surface alpha mode is unsupported: {alpha_mode:?}")]
    UnsupportedSurfaceAlphaMode {
        /// The unsupported alpha mode.
        alpha_mode: wgpu::CompositeAlphaMode,
    },
}
