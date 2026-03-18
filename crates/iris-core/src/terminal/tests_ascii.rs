use pretty_assertions::assert_eq;

use super::Terminal;
use crate::error::Error;

#[test]
fn terminal_write_ascii_run_rejects_non_printable_bytes_without_partial_write() {
    let mut terminal = Terminal::new(1, 4).unwrap();

    let error = terminal.write_ascii_run(b"AB\nC").unwrap_err();

    assert_eq!(error, Error::InvalidAsciiRun { byte: b'\n' });
    assert_eq!(terminal.cursor.position.row, 0);
    assert_eq!(terminal.cursor.position.col, 0);
    assert_eq!(
        terminal.grid.cell(0, 0).map(|cell| cell.character),
        Some(' ')
    );
    assert_eq!(
        terminal.grid.cell(0, 1).map(|cell| cell.character),
        Some(' ')
    );
    assert_eq!(
        terminal.grid.cell(0, 2).map(|cell| cell.character),
        Some(' ')
    );
    assert_eq!(
        terminal.grid.cell(0, 3).map(|cell| cell.character),
        Some(' ')
    );
    assert!(terminal.take_damage().is_empty());
}

#[test]
fn terminal_write_ascii_run_rejects_utf8_bytes_without_partial_write() {
    let mut terminal = Terminal::new(1, 4).unwrap();

    let error = terminal.write_ascii_run(b"A\xC3").unwrap_err();

    assert_eq!(error, Error::InvalidAsciiRun { byte: 0xC3 });
    assert_eq!(terminal.cursor.position.row, 0);
    assert_eq!(terminal.cursor.position.col, 0);
    assert_eq!(
        terminal.grid.cell(0, 0).map(|cell| cell.character),
        Some(' ')
    );
    assert_eq!(
        terminal.grid.cell(0, 1).map(|cell| cell.character),
        Some(' ')
    );
    assert!(terminal.take_damage().is_empty());
}
