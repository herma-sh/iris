use thiserror::Error;

/// Result type used across `iris-core`.
pub type Result<T> = std::result::Result<T, Error>;

pub(crate) fn validate_printable_ascii(bytes: &[u8]) -> Result<()> {
    if let Some(&byte) = bytes.iter().find(|&&byte| !matches!(byte, 0x20..=0x7e)) {
        return Err(Error::InvalidAsciiRun { byte });
    }

    Ok(())
}

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

    /// A fast-path ASCII write received a byte outside printable ASCII.
    #[error("invalid ASCII run byte 0x{byte:02X}; expected printable ASCII")]
    InvalidAsciiRun { byte: u8 },

    /// A resize operation failed validation.
    #[error("resize failed: {reason}")]
    ResizeFailed { reason: String },
}
