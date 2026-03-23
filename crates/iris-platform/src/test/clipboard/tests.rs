use crate::clipboard::{
    copy_selection_to_clipboard, paste_from_clipboard, Clipboard, ClipboardSelection, NoopClipboard,
};
use crate::error::{ClipboardError, Error, Result};

#[derive(Debug, Default)]
struct MockClipboard {
    clipboard: Option<String>,
    primary: Option<String>,
    writes: Vec<(ClipboardSelection, String)>,
}

impl Clipboard for MockClipboard {
    fn get_text(&self) -> Result<Option<String>> {
        Ok(self.clipboard.clone())
    }

    fn set_text(&mut self, text: &str) -> Result<()> {
        self.writes
            .push((ClipboardSelection::Clipboard, text.to_string()));
        self.clipboard = Some(text.to_string());
        Ok(())
    }

    fn clear(&mut self) -> Result<()> {
        self.clipboard = None;
        Ok(())
    }

    fn get_primary(&self) -> Result<Option<String>> {
        Ok(self.primary.clone())
    }

    fn set_primary(&mut self, text: &str) -> Result<()> {
        self.writes
            .push((ClipboardSelection::Primary, text.to_string()));
        self.primary = Some(text.to_string());
        Ok(())
    }

    fn clear_primary(&mut self) -> Result<()> {
        self.primary = None;
        Ok(())
    }
}

#[test]
fn noop_clipboard_round_trips_standard_buffer() {
    let mut clipboard = NoopClipboard::new();

    clipboard.set_text("hello").unwrap();
    assert_eq!(clipboard.get_text().unwrap().as_deref(), Some("hello"));

    clipboard.clear().unwrap();
    assert_eq!(clipboard.get_text().unwrap(), None);
}

#[test]
fn noop_clipboard_rejects_primary_without_support() {
    let clipboard = NoopClipboard::new();

    let result = clipboard.get_primary();
    assert!(matches!(
        result,
        Err(Error::Clipboard(
            ClipboardError::PrimarySelectionUnavailable
        ))
    ));
}

#[test]
fn noop_clipboard_round_trips_primary_when_enabled() {
    let mut clipboard = NoopClipboard::with_primary_selection();

    clipboard.set_primary("linux-primary").unwrap();
    assert_eq!(
        clipboard.get_primary().unwrap().as_deref(),
        Some("linux-primary")
    );

    clipboard.clear_primary().unwrap();
    assert_eq!(clipboard.get_primary().unwrap(), None);
}

#[test]
fn copy_selection_to_clipboard_skips_empty_inputs() {
    let mut clipboard = MockClipboard::default();

    assert!(
        !copy_selection_to_clipboard(&mut clipboard, None, ClipboardSelection::Clipboard,).unwrap()
    );
    assert!(
        !copy_selection_to_clipboard(&mut clipboard, Some(""), ClipboardSelection::Clipboard,)
            .unwrap()
    );
    assert!(clipboard.writes.is_empty());
}

#[test]
fn copy_selection_to_clipboard_writes_requested_target() {
    let mut clipboard = MockClipboard::default();

    assert!(copy_selection_to_clipboard(
        &mut clipboard,
        Some("selected"),
        ClipboardSelection::Primary
    )
    .unwrap());
    assert_eq!(clipboard.writes.len(), 1);
    assert_eq!(
        clipboard.writes[0],
        (ClipboardSelection::Primary, "selected".to_string()),
    );
}

#[test]
fn paste_from_clipboard_reads_requested_target() {
    let clipboard = MockClipboard {
        clipboard: Some("standard".to_string()),
        primary: Some("primary".to_string()),
        writes: Vec::new(),
    };

    assert_eq!(
        paste_from_clipboard(&clipboard, ClipboardSelection::Clipboard)
            .unwrap()
            .as_deref(),
        Some("standard"),
    );
    assert_eq!(
        paste_from_clipboard(&clipboard, ClipboardSelection::Primary)
            .unwrap()
            .as_deref(),
        Some("primary"),
    );
}
