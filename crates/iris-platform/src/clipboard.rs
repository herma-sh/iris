use crate::error::Result;

/// Clipboard access abstraction.
pub trait Clipboard {
    /// Reads the current clipboard text if available.
    fn get_text(&self) -> Result<Option<String>>;

    /// Replaces the clipboard contents.
    fn set_text(&mut self, text: &str) -> Result<()>;
}

/// Fallback clipboard implementation used until platform integration lands.
#[derive(Debug, Default)]
pub struct NoopClipboard {
    text: Option<String>,
}

impl Clipboard for NoopClipboard {
    /// Get the current clipboard text, if any.
    ///
    /// # Returns
    ///
    /// `Ok(Some(text))` with the stored clipboard text, `Ok(None)` if the clipboard is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut c = NoopClipboard::default();
    /// assert_eq!(c.get_text().unwrap(), None);
    /// c.set_text("hello").unwrap();
    /// assert_eq!(c.get_text().unwrap(), Some("hello".to_string()));
    /// ```
    fn get_text(&self) -> Result<Option<String>> {
        Ok(self.text.clone())
    }

    /// Sets the clipboard contents to `text`, or clears the clipboard if `text` is empty.
    ///
    /// On success, returns `Ok(())`.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut cb = NoopClipboard::default();
    /// cb.set_text("hello").unwrap();
    /// assert_eq!(cb.get_text().unwrap(), Some("hello".to_string()));
    ///
    /// cb.set_text("").unwrap();
    /// assert_eq!(cb.get_text().unwrap(), None);
    /// ```
    fn set_text(&mut self, text: &str) -> Result<()> {
        if text.is_empty() {
            self.text = None;
        } else {
            self.text = Some(text.to_string());
        }
        Ok(())
    }
}
