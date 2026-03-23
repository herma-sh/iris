//! Platform abstractions for PTY, clipboard, fonts, and IME support.

pub mod clipboard;
pub mod error;
pub mod fonts;
pub mod ime;
pub mod platform;
pub mod pty;

pub use clipboard::{
    copy_selection_to_clipboard, encode_paste_input, paste_bytes_from_clipboard,
    paste_from_clipboard, Clipboard, ClipboardSelection, NoopClipboard, PlatformClipboard,
    BRACKETED_PASTE_END, BRACKETED_PASTE_START,
};
pub use error::{ClipboardError, Error, FontError, ImeError, PtyError, Result};
pub use fonts::{FontInfo, FontProvider, NoopFontProvider};
pub use ime::{ImeHandler, ImePosition, NoopImeHandler};
pub use pty::{PortablePtyBackend, PtyBackend, PtyConfig};
