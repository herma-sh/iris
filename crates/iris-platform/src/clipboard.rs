use crate::error::Result;

/// Clipboard access abstraction.
pub trait Clipboard {
    /// Reads the current clipboard text if available.
    fn get_text(&self) -> Result<Option<String>>;

    /// Replaces the clipboard contents.
    fn set_text(&mut self, text: &str) -> Result<()>;

    /// Clears the clipboard contents.
    fn clear(&mut self) -> Result<()>;
}

/// Fallback clipboard implementation used until platform integration lands.
#[derive(Debug, Default)]
pub struct NoopClipboard {
    text: Option<String>,
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
}
