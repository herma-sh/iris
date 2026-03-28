//! Platform abstractions for PTY, clipboard, fonts, and IME support.

pub mod clipboard;
pub mod dpi;
pub mod error;
pub mod fonts;
pub mod ime;
pub mod keyboard;
pub mod platform;
pub mod pty;
pub mod selection_input;

pub use clipboard::{
    copy_selection_to_clipboard, copy_terminal_selection_to_clipboard, encode_paste_input,
    paste_bytes_from_clipboard, paste_bytes_from_source, paste_from_clipboard, paste_from_source,
    paste_terminal_bytes_from_clipboard, paste_terminal_bytes_from_source, Clipboard,
    ClipboardSelection, NativeClipboard, NoopClipboard, PasteSource, PlatformClipboard,
    SelectionClipboardController, BRACKETED_PASTE_END, BRACKETED_PASTE_START,
};
pub use dpi::{DpiScale, BASELINE_DPI};
pub use error::{ClipboardError, Error, FontError, ImeError, PtyError, Result};
pub use fonts::{FontInfo, FontProvider, NoopFontProvider, PlatformFontProvider};
pub use ime::{ImeComposition, ImeHandler, ImePosition, NoopImeHandler};
pub use keyboard::{
    encode_terminal_key_input, normalize_keyboard_event, KeyModifiers, KeyboardPlatform,
    NormalizedKey, NormalizedKeyboardEvent, PlatformKeyCode, PlatformKeyboardEvent,
};
pub use pty::{PortablePtyBackend, PtyBackend, PtyConfig};
pub use selection_input::{
    SelectionDirection, SelectionEventFlow, SelectionEventFlowConfig, SelectionEventFlowOutcome,
    SelectionKeyboardEvent, SelectionMouseEvent, SelectionMouseEventAdapter,
    SelectionMouseEventAdapterConfig, SelectionWindowGeometry, SelectionWindowMouseEvent,
    SelectionWindowMouseEventAdapter, SelectionWindowMouseEventAdapterConfig,
};
