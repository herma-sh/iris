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
