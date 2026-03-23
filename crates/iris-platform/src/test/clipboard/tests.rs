use crate::clipboard::{
    copy_selection_to_clipboard, copy_terminal_selection_to_clipboard, encode_paste_input,
    paste_bytes_from_clipboard, paste_bytes_from_source, paste_from_clipboard, paste_from_source,
    paste_terminal_bytes_from_clipboard, paste_terminal_bytes_from_source, Clipboard,
    ClipboardSelection, NoopClipboard, PasteSource, BRACKETED_PASTE_END, BRACKETED_PASTE_START,
};
use crate::error::{ClipboardError, Error, Result};
use iris_core::{Action, Terminal};

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
fn copy_terminal_selection_to_clipboard_returns_false_without_selection() {
    let terminal = Terminal::new(1, 8).unwrap();
    let mut clipboard = NoopClipboard::new();

    assert!(!copy_terminal_selection_to_clipboard(
        &terminal,
        &mut clipboard,
        ClipboardSelection::Clipboard,
    )
    .unwrap());
    assert_eq!(clipboard.get_text().unwrap(), None);
}

#[test]
fn copy_terminal_selection_to_clipboard_uses_terminal_copy_selection_text() {
    let mut terminal = Terminal::new(2, 8).unwrap();
    terminal.write_ascii_run(b"linecopy").unwrap();
    terminal.select_line(0);

    let mut clipboard = NoopClipboard::new();
    assert!(copy_terminal_selection_to_clipboard(
        &terminal,
        &mut clipboard,
        ClipboardSelection::Clipboard,
    )
    .unwrap());
    assert_eq!(clipboard.get_text().unwrap().as_deref(), Some("linecopy\n"));
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

#[test]
fn encode_paste_input_returns_raw_text_when_bracketed_mode_is_disabled() {
    let payload = encode_paste_input("line1\nline2", false);
    assert_eq!(payload, b"line1\nline2");
}

#[test]
fn encode_paste_input_wraps_text_when_bracketed_mode_is_enabled() {
    let payload = encode_paste_input("paste", true);

    let expected = format!("{BRACKETED_PASTE_START}paste{BRACKETED_PASTE_END}");
    assert_eq!(payload, expected.into_bytes());
}

#[test]
fn paste_bytes_from_clipboard_returns_none_when_source_is_empty() {
    let clipboard = NoopClipboard::new();
    assert_eq!(
        paste_bytes_from_clipboard(&clipboard, ClipboardSelection::Clipboard, true).unwrap(),
        None
    );
}

#[test]
fn paste_bytes_from_clipboard_reads_primary_and_wraps_when_enabled() {
    let mut clipboard = NoopClipboard::with_primary_selection();
    clipboard.set_primary("primary-data").unwrap();

    let payload = paste_bytes_from_clipboard(&clipboard, ClipboardSelection::Primary, true)
        .unwrap()
        .expect("primary clipboard should produce a payload");

    let expected = format!("{BRACKETED_PASTE_START}primary-data{BRACKETED_PASTE_END}");
    assert_eq!(payload, expected.into_bytes());
}

#[test]
fn paste_bytes_from_clipboard_returns_raw_bytes_when_bracketed_mode_is_disabled() {
    let mut clipboard = NoopClipboard::new();
    clipboard.set_text("raw-data").unwrap();

    let payload = paste_bytes_from_clipboard(&clipboard, ClipboardSelection::Clipboard, false)
        .unwrap()
        .expect("clipboard should produce a payload");

    assert_eq!(payload, b"raw-data");
}

#[test]
fn paste_from_source_primary_then_clipboard_uses_primary_when_available() {
    let mut clipboard = NoopClipboard::with_primary_selection();
    clipboard.set_text("clipboard").unwrap();
    clipboard.set_primary("primary").unwrap();

    assert_eq!(
        paste_from_source(&clipboard, PasteSource::PrimaryThenClipboard)
            .unwrap()
            .as_deref(),
        Some("primary"),
    );
}

#[test]
fn paste_from_source_primary_then_clipboard_falls_back_when_primary_unavailable() {
    let mut clipboard = NoopClipboard::new();
    clipboard.set_text("clipboard-fallback").unwrap();

    assert_eq!(
        paste_from_source(&clipboard, PasteSource::PrimaryThenClipboard)
            .unwrap()
            .as_deref(),
        Some("clipboard-fallback"),
    );
}

#[test]
fn paste_from_source_primary_then_clipboard_falls_back_when_primary_is_empty() {
    let mut clipboard = NoopClipboard::with_primary_selection();
    clipboard.set_text("clipboard-fallback").unwrap();
    clipboard.set_primary("").unwrap();

    assert_eq!(
        paste_from_source(&clipboard, PasteSource::PrimaryThenClipboard)
            .unwrap()
            .as_deref(),
        Some("clipboard-fallback"),
    );
}

#[test]
fn paste_bytes_from_source_primary_then_clipboard_wraps_fallback_payload() {
    let mut clipboard = NoopClipboard::new();
    clipboard.set_text("clipboard-fallback").unwrap();

    let payload = paste_bytes_from_source(&clipboard, PasteSource::PrimaryThenClipboard, true)
        .unwrap()
        .expect("fallback clipboard should produce a payload");

    let expected = format!("{BRACKETED_PASTE_START}clipboard-fallback{BRACKETED_PASTE_END}");
    assert_eq!(payload, expected.into_bytes());
}

#[test]
fn paste_terminal_bytes_from_clipboard_returns_none_when_source_is_empty() {
    let terminal = Terminal::new(2, 4).unwrap();
    let clipboard = NoopClipboard::new();

    assert_eq!(
        paste_terminal_bytes_from_clipboard(&terminal, &clipboard, ClipboardSelection::Clipboard)
            .unwrap(),
        None
    );
}

#[test]
fn paste_terminal_bytes_from_source_respects_terminal_bracketed_paste_mode() {
    let mut terminal = Terminal::new(2, 4).unwrap();
    let mut clipboard = NoopClipboard::new();
    clipboard.set_text("paste").unwrap();

    let raw = paste_terminal_bytes_from_source(&terminal, &clipboard, PasteSource::Clipboard)
        .unwrap()
        .expect("clipboard should produce a payload");
    assert_eq!(raw, b"paste");

    terminal
        .apply_action(Action::SetModes {
            private: true,
            modes: vec![2004].into(),
        })
        .unwrap();

    let wrapped = paste_terminal_bytes_from_source(&terminal, &clipboard, PasteSource::Clipboard)
        .unwrap()
        .expect("clipboard should produce a payload");
    let expected = format!("{BRACKETED_PASTE_START}paste{BRACKETED_PASTE_END}");
    assert_eq!(wrapped, expected.into_bytes());
}

#[test]
fn paste_terminal_bytes_from_source_primary_then_clipboard_falls_back_when_primary_is_empty() {
    let mut terminal = Terminal::new(2, 4).unwrap();
    terminal
        .apply_action(Action::SetModes {
            private: true,
            modes: vec![2004].into(),
        })
        .unwrap();

    let mut clipboard = NoopClipboard::with_primary_selection();
    clipboard.set_primary("").unwrap();
    clipboard.set_text("clipboard-fallback").unwrap();

    let payload =
        paste_terminal_bytes_from_source(&terminal, &clipboard, PasteSource::PrimaryThenClipboard)
            .unwrap()
            .expect("fallback clipboard should produce a payload");

    let expected = format!("{BRACKETED_PASTE_START}clipboard-fallback{BRACKETED_PASTE_END}");
    assert_eq!(payload, expected.into_bytes());
}
