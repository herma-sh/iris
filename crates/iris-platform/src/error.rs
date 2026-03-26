use thiserror::Error;

/// Result type used across `iris-platform`.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors produced by platform services.
#[derive(Debug, Error)]
pub enum Error {
    /// PTY-related failure.
    #[error(transparent)]
    Pty(#[from] PtyError),
    /// Clipboard-related failure.
    #[error(transparent)]
    Clipboard(#[from] ClipboardError),
    /// Font-related failure.
    #[error(transparent)]
    Font(#[from] FontError),
    /// IME-related failure.
    #[error(transparent)]
    Ime(#[from] ImeError),
}

/// PTY-specific failures.
#[derive(Debug, Error)]
pub enum PtyError {
    /// Failed to create the underlying PTY pair.
    #[error("failed to open PTY: {reason}")]
    OpenFailed { reason: String },
    /// Failed to spawn the configured command.
    #[error("failed to spawn '{command}': {reason}")]
    SpawnFailed { command: String, reason: String },
    /// Attempted to use the PTY before spawning.
    #[error("PTY is not active")]
    NotActive,
    /// Failed to read from the PTY.
    #[error("failed to read from PTY: {reason}")]
    ReadFailed { reason: String },
    /// Failed to write to the PTY.
    #[error("failed to write to PTY: {reason}")]
    WriteFailed { reason: String },
    /// Failed to resize the PTY.
    #[error("failed to resize PTY: {reason}")]
    ResizeFailed { reason: String },
    /// Failed to query the child process status.
    #[error("failed to query PTY child status: {reason}")]
    StatusFailed { reason: String },
}

/// Clipboard-specific failures.
#[derive(Debug, Error)]
pub enum ClipboardError {
    /// Clipboard backend initialization failed.
    #[error("clipboard initialization failed")]
    InitializationFailed,
    /// Clipboard reads are unavailable.
    #[error("clipboard read is not available")]
    ReadUnavailable,
    /// Clipboard writes are unavailable.
    #[error("clipboard write is not available")]
    WriteUnavailable,
    /// Linux/X11 primary selection is unavailable.
    #[error("primary selection clipboard is not available")]
    PrimarySelectionUnavailable,
}

/// Font-related failures.
#[derive(Debug, Error)]
pub enum FontError {
    /// Font enumeration is unavailable.
    #[error("font enumeration is not available")]
    EnumerateUnavailable,
}

/// IME-related failures.
#[derive(Debug, Error)]
pub enum ImeError {
    /// IME positioning is unavailable.
    #[error("IME positioning is not available")]
    PositionUnavailable,
}
