//! Platform abstractions for PTY, clipboard, fonts, and IME support.

pub mod clipboard;
pub mod error;
pub mod fonts;
pub mod ime;
pub mod platform;
pub mod pty;

pub use clipboard::{Clipboard, NoopClipboard};
pub use error::{ClipboardError, Error, FontError, ImeError, PtyError, Result};
pub use fonts::{FontInfo, FontProvider, NoopFontProvider};
pub use ime::{ImeHandler, ImePosition, NoopImeHandler};
pub use pty::{PortablePtyBackend, PtyBackend, PtyConfig};
