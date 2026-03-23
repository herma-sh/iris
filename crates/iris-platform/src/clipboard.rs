use crate::error::{ClipboardError, Result};

/// Clipboard buffer target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipboardSelection {
    /// The standard system clipboard.
    Clipboard,
    /// The Linux/X11 primary selection buffer.
    Primary,
}

/// Bracketed paste start sequence.
pub const BRACKETED_PASTE_START: &str = "\u{1b}[200~";
/// Bracketed paste end sequence.
pub const BRACKETED_PASTE_END: &str = "\u{1b}[201~";

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

/// Reads text from the requested clipboard buffer for paste operations.
pub fn paste_from_clipboard(
    clipboard: &impl Clipboard,
    source: ClipboardSelection,
) -> Result<Option<String>> {
    clipboard.read(source)
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

/// Fallback clipboard implementation used until platform integration lands.
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

/// Concrete scaffold clipboard used by platform composition until native
/// backends are connected.
#[derive(Debug)]
pub struct PlatformClipboard {
    inner: NoopClipboard,
}

impl PlatformClipboard {
    /// Creates a platform scaffold clipboard.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for PlatformClipboard {
    fn default() -> Self {
        #[cfg(target_os = "linux")]
        let inner = NoopClipboard::with_primary_selection();

        #[cfg(not(target_os = "linux"))]
        let inner = NoopClipboard::new();

        Self { inner }
    }
}

impl Clipboard for PlatformClipboard {
    fn get_text(&self) -> Result<Option<String>> {
        self.inner.get_text()
    }

    fn set_text(&mut self, text: &str) -> Result<()> {
        self.inner.set_text(text)
    }

    fn clear(&mut self) -> Result<()> {
        self.inner.clear()
    }

    fn get_primary(&self) -> Result<Option<String>> {
        self.inner.get_primary()
    }

    fn set_primary(&mut self, text: &str) -> Result<()> {
        self.inner.set_primary(text)
    }

    fn clear_primary(&mut self) -> Result<()> {
        self.inner.clear_primary()
    }
}

#[cfg(test)]
#[path = "test/clipboard/tests.rs"]
mod tests;
