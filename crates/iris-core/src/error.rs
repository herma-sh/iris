use thiserror::Error;

/// Result type used across `iris-core`.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors produced by core terminal primitives.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum Error {
    /// The requested position fell outside the visible grid.
    #[error("invalid position ({row}, {col}) for grid size {rows}x{cols}")]
    InvalidPosition {
        row: usize,
        col: usize,
        rows: usize,
        cols: usize,
    },

    /// A resize operation failed validation.
    #[error("resize failed: {reason}")]
    ResizeFailed { reason: String },
}
