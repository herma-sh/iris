use crate::error::{ClipboardError, Error, Result};
use iris_core::{SelectionInputEvent, SelectionInputState, Terminal};
use std::sync::Mutex;

#[cfg(target_os = "linux")]
use arboard::{ClearExtLinux, GetExtLinux, LinuxClipboardKind, SetExtLinux};

/// Clipboard buffer target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipboardSelection {
    /// The standard system clipboard.
    Clipboard,
    /// The Linux/X11 primary selection buffer.
    Primary,
}

/// Paste source selection strategy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PasteSource {
    /// Read only from the standard system clipboard.
    Clipboard,
    /// Read only from the Linux/X11 primary selection.
    Primary,
    /// Prefer Linux/X11 primary selection, then fall back to standard
    /// clipboard when primary is unavailable or empty.
    PrimaryThenClipboard,
}

/// Bracketed paste start sequence.
pub const BRACKETED_PASTE_START: &str = "\u{1b}[200~";
/// Bracketed paste end sequence.
pub const BRACKETED_PASTE_END: &str = "\u{1b}[201~";

/// Selection and clipboard flow controller.
///
/// This bridges input-driven selection updates (`SelectionInputEvent`) with
/// copy/paste clipboard operations against a terminal instance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionClipboardController {
    selection_input: SelectionInputState,
    copy_target: ClipboardSelection,
    paste_source: PasteSource,
}

impl SelectionClipboardController {
    /// Creates a controller with explicit copy target and paste source.
    #[must_use]
    pub const fn new(copy_target: ClipboardSelection, paste_source: PasteSource) -> Self {
        Self {
            selection_input: SelectionInputState::new(),
            copy_target,
            paste_source,
        }
    }

    /// Returns the configured copy target.
    #[must_use]
    pub const fn copy_target(&self) -> ClipboardSelection {
        self.copy_target
    }

    /// Returns the configured paste source.
    #[must_use]
    pub const fn paste_source(&self) -> PasteSource {
        self.paste_source
    }

    /// Applies a selection input event to terminal selection state.
    ///
    /// Returns `true` when the event was consumed by selection handling.
    pub fn handle_selection_input_event(
        &mut self,
        terminal: &mut Terminal,
        event: SelectionInputEvent,
    ) -> bool {
        self.selection_input.handle_event(terminal, event)
    }

    /// Copies the terminal selection to the configured clipboard target.
    pub fn copy_selection(
        &self,
        terminal: &Terminal,
        clipboard: &mut impl Clipboard,
    ) -> Result<bool> {
        copy_terminal_selection_to_clipboard(terminal, clipboard, self.copy_target)
    }

    /// Reads and encodes paste bytes from the configured paste source according
    /// to terminal bracketed-paste mode.
    pub fn paste_terminal_bytes(
        &self,
        terminal: &Terminal,
        clipboard: &impl Clipboard,
    ) -> Result<Option<Vec<u8>>> {
        paste_terminal_bytes_from_source(terminal, clipboard, self.paste_source)
    }
}

impl Default for SelectionClipboardController {
    fn default() -> Self {
        Self::new(
            ClipboardSelection::Clipboard,
            PasteSource::PrimaryThenClipboard,
        )
    }
}

/// Clipboard access abstraction.
pub trait Clipboard {
    /// Reads the current clipboard text if available.
    fn get_text(&self) -> Result<Option<String>>;

    /// Replaces the clipboard contents.
    fn set_text(&mut self, text: &str) -> Result<()>;

    /// Clears the clipboard contents.
    fn clear(&mut self) -> Result<()>;

    /// Reads the Linux/X11 primary selection text if available.
    fn get_primary(&self) -> Result<Option<String>> {
        Err(ClipboardError::PrimarySelectionUnavailable.into())
    }

    /// Replaces the Linux/X11 primary selection contents.
    fn set_primary(&mut self, _text: &str) -> Result<()> {
        Err(ClipboardError::PrimarySelectionUnavailable.into())
    }

    /// Clears the Linux/X11 primary selection contents.
    fn clear_primary(&mut self) -> Result<()> {
        Err(ClipboardError::PrimarySelectionUnavailable.into())
    }

    /// Reads text from the requested clipboard buffer.
    fn read(&self, selection: ClipboardSelection) -> Result<Option<String>> {
        match selection {
            ClipboardSelection::Clipboard => self.get_text(),
            ClipboardSelection::Primary => self.get_primary(),
        }
    }

    /// Writes text to the requested clipboard buffer.
    fn write(&mut self, selection: ClipboardSelection, text: &str) -> Result<()> {
        match selection {
            ClipboardSelection::Clipboard => self.set_text(text),
            ClipboardSelection::Primary => self.set_primary(text),
        }
    }

    /// Clears the requested clipboard buffer.
    fn clear_selection(&mut self, selection: ClipboardSelection) -> Result<()> {
        match selection {
            ClipboardSelection::Clipboard => self.clear(),
            ClipboardSelection::Primary => self.clear_primary(),
        }
    }
}

/// Copies selected text into the requested clipboard buffer.
///
/// Returns `Ok(true)` when text was copied and `Ok(false)` when the input
/// selection was `None` or empty.
pub fn copy_selection_to_clipboard(
    clipboard: &mut impl Clipboard,
    selected_text: Option<&str>,
    target: ClipboardSelection,
) -> Result<bool> {
    let Some(text) = selected_text.filter(|text| !text.is_empty()) else {
        return Ok(false);
    };

    clipboard.write(target, text)?;
    Ok(true)
}

/// Copies terminal selection text into the requested clipboard buffer.
///
/// This uses `Terminal::copy_selection_text` so line selections preserve
/// terminal copy semantics (including trailing newline behavior).
pub fn copy_terminal_selection_to_clipboard(
    terminal: &Terminal,
    clipboard: &mut impl Clipboard,
    target: ClipboardSelection,
) -> Result<bool> {
    let selected_text = terminal.copy_selection_text();
    copy_selection_to_clipboard(clipboard, selected_text.as_deref(), target)
}

/// Reads text from the requested clipboard buffer for paste operations.
pub fn paste_from_clipboard(
    clipboard: &impl Clipboard,
    source: ClipboardSelection,
) -> Result<Option<String>> {
    clipboard.read(source)
}

/// Reads text using the requested paste-source strategy.
pub fn paste_from_source(
    clipboard: &impl Clipboard,
    source: PasteSource,
) -> Result<Option<String>> {
    match source {
        PasteSource::Clipboard => paste_from_clipboard(clipboard, ClipboardSelection::Clipboard),
        PasteSource::Primary => paste_from_clipboard(clipboard, ClipboardSelection::Primary),
        PasteSource::PrimaryThenClipboard => {
            match paste_from_clipboard(clipboard, ClipboardSelection::Primary) {
                Ok(Some(text)) if !text.is_empty() => Ok(Some(text)),
                Ok(Some(_))
                | Ok(None)
                | Err(Error::Clipboard(ClipboardError::PrimarySelectionUnavailable)) => {
                    paste_from_clipboard(clipboard, ClipboardSelection::Clipboard)
                }
                Err(error) => Err(error),
            }
        }
    }
}

/// Encodes paste input bytes, optionally wrapping with bracketed paste markers.
#[must_use]
pub fn encode_paste_input(text: &str, bracketed_paste_mode: bool) -> Vec<u8> {
    if !bracketed_paste_mode {
        return text.as_bytes().to_vec();
    }

    let mut payload =
        Vec::with_capacity(BRACKETED_PASTE_START.len() + text.len() + BRACKETED_PASTE_END.len());
    payload.extend_from_slice(BRACKETED_PASTE_START.as_bytes());
    payload.extend_from_slice(text.as_bytes());
    payload.extend_from_slice(BRACKETED_PASTE_END.as_bytes());
    payload
}

/// Reads text from the requested clipboard source and returns PTY-ready paste
/// bytes with optional bracketed paste wrapping.
pub fn paste_bytes_from_clipboard(
    clipboard: &impl Clipboard,
    source: ClipboardSelection,
    bracketed_paste_mode: bool,
) -> Result<Option<Vec<u8>>> {
    let Some(text) = paste_from_clipboard(clipboard, source)? else {
        return Ok(None);
    };

    Ok(Some(encode_paste_input(&text, bracketed_paste_mode)))
}

/// Reads text from the requested clipboard source and returns paste bytes
/// encoded according to the terminal's active bracketed-paste mode.
pub fn paste_terminal_bytes_from_clipboard(
    terminal: &Terminal,
    clipboard: &impl Clipboard,
    source: ClipboardSelection,
) -> Result<Option<Vec<u8>>> {
    let Some(text) = paste_from_clipboard(clipboard, source)? else {
        return Ok(None);
    };

    Ok(Some(terminal.paste_bytes(&text)))
}

/// Reads text using the requested paste source strategy and returns PTY-ready
/// paste bytes with optional bracketed paste wrapping.
pub fn paste_bytes_from_source(
    clipboard: &impl Clipboard,
    source: PasteSource,
    bracketed_paste_mode: bool,
) -> Result<Option<Vec<u8>>> {
    let Some(text) = paste_from_source(clipboard, source)? else {
        return Ok(None);
    };

    Ok(Some(encode_paste_input(&text, bracketed_paste_mode)))
}

/// Reads text using the requested paste-source strategy and returns paste
/// bytes encoded according to the terminal's active bracketed-paste mode.
pub fn paste_terminal_bytes_from_source(
    terminal: &Terminal,
    clipboard: &impl Clipboard,
    source: PasteSource,
) -> Result<Option<Vec<u8>>> {
    let Some(text) = paste_from_source(clipboard, source)? else {
        return Ok(None);
    };

    Ok(Some(terminal.paste_bytes(&text)))
}

/// Native system clipboard implementation.
///
/// This uses `arboard` for cross-platform clipboard access and keeps one
/// clipboard context alive for Linux selection ownership behavior.
pub struct NativeClipboard {
    inner: Mutex<arboard::Clipboard>,
}

impl NativeClipboard {
    /// Creates a native clipboard backend.
    pub fn new() -> Result<Self> {
        let clipboard = arboard::Clipboard::new()
            .map_err(|_| Error::Clipboard(ClipboardError::ReadUnavailable))?;
        Ok(Self {
            inner: Mutex::new(clipboard),
        })
    }

    fn lock_for_read(&self) -> Result<std::sync::MutexGuard<'_, arboard::Clipboard>> {
        self.inner
            .lock()
            .map_err(|_| Error::Clipboard(ClipboardError::ReadUnavailable))
    }

    fn lock_for_write(&self) -> Result<std::sync::MutexGuard<'_, arboard::Clipboard>> {
        self.inner
            .lock()
            .map_err(|_| Error::Clipboard(ClipboardError::WriteUnavailable))
    }

    fn map_read_text(
        result: std::result::Result<String, arboard::Error>,
    ) -> Result<Option<String>> {
        match result {
            Ok(text) => Ok(Some(text)),
            Err(arboard::Error::ContentNotAvailable) => Ok(None),
            Err(_) => Err(ClipboardError::ReadUnavailable.into()),
        }
    }

    fn map_write_result(result: std::result::Result<(), arboard::Error>) -> Result<()> {
        result.map_err(|_| ClipboardError::WriteUnavailable.into())
    }

    #[cfg(target_os = "linux")]
    fn map_primary_read_text(
        result: std::result::Result<String, arboard::Error>,
    ) -> Result<Option<String>> {
        match result {
            Ok(text) => Ok(Some(text)),
            Err(arboard::Error::ContentNotAvailable) => Ok(None),
            Err(arboard::Error::ClipboardNotSupported) => {
                Err(ClipboardError::PrimarySelectionUnavailable.into())
            }
            Err(_) => Err(ClipboardError::ReadUnavailable.into()),
        }
    }

    #[cfg(target_os = "linux")]
    fn map_primary_write_result(result: std::result::Result<(), arboard::Error>) -> Result<()> {
        match result {
            Ok(()) => Ok(()),
            Err(arboard::Error::ClipboardNotSupported) => {
                Err(ClipboardError::PrimarySelectionUnavailable.into())
            }
            Err(_) => Err(ClipboardError::WriteUnavailable.into()),
        }
    }
}

impl std::fmt::Debug for NativeClipboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeClipboard { ... }")
    }
}

impl Clipboard for NativeClipboard {
    fn get_text(&self) -> Result<Option<String>> {
        let mut clipboard = self.lock_for_read()?;
        Self::map_read_text(clipboard.get_text())
    }

    fn set_text(&mut self, text: &str) -> Result<()> {
        let mut clipboard = self.lock_for_write()?;
        Self::map_write_result(clipboard.set_text(text.to_string()))
    }

    fn clear(&mut self) -> Result<()> {
        let mut clipboard = self.lock_for_write()?;
        Self::map_write_result(clipboard.clear())
    }

    fn get_primary(&self) -> Result<Option<String>> {
        #[cfg(target_os = "linux")]
        {
            let mut clipboard = self.lock_for_read()?;
            return Self::map_primary_read_text(
                clipboard
                    .get()
                    .clipboard(LinuxClipboardKind::Primary)
                    .text(),
            );
        }

        #[cfg(not(target_os = "linux"))]
        {
            Err(ClipboardError::PrimarySelectionUnavailable.into())
        }
    }

    fn set_primary(&mut self, text: &str) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            let mut clipboard = self.lock_for_write()?;
            return Self::map_primary_write_result(
                clipboard
                    .set()
                    .clipboard(LinuxClipboardKind::Primary)
                    .text(text.to_string()),
            );
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = text;
            Err(ClipboardError::PrimarySelectionUnavailable.into())
        }
    }

    fn clear_primary(&mut self) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            let mut clipboard = self.lock_for_write()?;
            return Self::map_primary_write_result(
                clipboard
                    .clear_with()
                    .clipboard(LinuxClipboardKind::Primary),
            );
        }

        #[cfg(not(target_os = "linux"))]
        {
            Err(ClipboardError::PrimarySelectionUnavailable.into())
        }
    }
}

/// Fallback clipboard implementation used when native clipboard
/// initialization is unavailable.
#[derive(Debug, Default)]
pub struct NoopClipboard {
    text: Option<String>,
    primary: Option<String>,
    primary_enabled: bool,
}

impl NoopClipboard {
    /// Creates a scaffold clipboard without PRIMARY selection support.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            text: None,
            primary: None,
            primary_enabled: false,
        }
    }

    /// Creates a scaffold clipboard with PRIMARY selection support.
    #[must_use]
    pub const fn with_primary_selection() -> Self {
        Self {
            text: None,
            primary: None,
            primary_enabled: true,
        }
    }

    fn validate_primary(&self) -> Result<()> {
        if self.primary_enabled {
            Ok(())
        } else {
            Err(ClipboardError::PrimarySelectionUnavailable.into())
        }
    }
}

impl Clipboard for NoopClipboard {
    fn get_text(&self) -> Result<Option<String>> {
        Ok(self.text.clone())
    }

    fn set_text(&mut self, text: &str) -> Result<()> {
        self.text = Some(text.to_string());
        Ok(())
    }

    fn clear(&mut self) -> Result<()> {
        self.text = None;
        Ok(())
    }

    fn get_primary(&self) -> Result<Option<String>> {
        self.validate_primary()?;
        Ok(self.primary.clone())
    }

    fn set_primary(&mut self, text: &str) -> Result<()> {
        self.validate_primary()?;
        self.primary = Some(text.to_string());
        Ok(())
    }

    fn clear_primary(&mut self) -> Result<()> {
        self.validate_primary()?;
        self.primary = None;
        Ok(())
    }
}

#[derive(Debug)]
enum PlatformClipboardBackend {
    Native(NativeClipboard),
    Noop(NoopClipboard),
}

/// Platform clipboard facade using native backend with deterministic fallback.
#[derive(Debug)]
pub struct PlatformClipboard {
    inner: PlatformClipboardBackend,
}

impl PlatformClipboard {
    /// Creates a platform clipboard backend.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn fallback_noop() -> NoopClipboard {
        #[cfg(target_os = "linux")]
        {
            NoopClipboard::with_primary_selection()
        }

        #[cfg(not(target_os = "linux"))]
        {
            NoopClipboard::new()
        }
    }

    fn from_native_or_fallback(native: Result<NativeClipboard>) -> Self {
        let inner = match native {
            Ok(native) => PlatformClipboardBackend::Native(native),
            Err(_) => PlatformClipboardBackend::Noop(Self::fallback_noop()),
        };

        Self { inner }
    }
}

impl Default for PlatformClipboard {
    fn default() -> Self {
        Self::from_native_or_fallback(NativeClipboard::new())
    }
}

impl Clipboard for PlatformClipboard {
    fn get_text(&self) -> Result<Option<String>> {
        match &self.inner {
            PlatformClipboardBackend::Native(native) => native.get_text(),
            PlatformClipboardBackend::Noop(noop) => noop.get_text(),
        }
    }

    fn set_text(&mut self, text: &str) -> Result<()> {
        match &mut self.inner {
            PlatformClipboardBackend::Native(native) => native.set_text(text),
            PlatformClipboardBackend::Noop(noop) => noop.set_text(text),
        }
    }

    fn clear(&mut self) -> Result<()> {
        match &mut self.inner {
            PlatformClipboardBackend::Native(native) => native.clear(),
            PlatformClipboardBackend::Noop(noop) => noop.clear(),
        }
    }

    fn get_primary(&self) -> Result<Option<String>> {
        match &self.inner {
            PlatformClipboardBackend::Native(native) => native.get_primary(),
            PlatformClipboardBackend::Noop(noop) => noop.get_primary(),
        }
    }

    fn set_primary(&mut self, text: &str) -> Result<()> {
        match &mut self.inner {
            PlatformClipboardBackend::Native(native) => native.set_primary(text),
            PlatformClipboardBackend::Noop(noop) => noop.set_primary(text),
        }
    }

    fn clear_primary(&mut self) -> Result<()> {
        match &mut self.inner {
            PlatformClipboardBackend::Native(native) => native.clear_primary(),
            PlatformClipboardBackend::Noop(noop) => noop.clear_primary(),
        }
    }
}

#[cfg(test)]
#[path = "test/clipboard/tests.rs"]
mod tests;
