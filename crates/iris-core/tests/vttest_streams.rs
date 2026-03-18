use iris_core::{Parser, Terminal};

fn advance_chunks(parser: &mut Parser, terminal: &mut Terminal, chunks: &[&[u8]]) {
    for chunk in chunks {
        parser.advance(terminal, chunk).unwrap();
    }
}

fn row_text(terminal: &Terminal, row: usize) -> String {
    (0..terminal.grid.cols())
        .map(|col| {
            terminal
                .grid
                .cell(row, col)
                .map(|cell| cell.character)
                .unwrap_or(' ')
        })
        .collect()
}

#[test]
fn replays_chunked_vttest_style_margin_redraw() {
    let mut terminal = Terminal::new(6, 20).unwrap();
    let mut parser = Parser::new();

    advance_chunks(
        &mut parser,
        &mut terminal,
        &[
            b"\x1b[2J\x1b[HTI",
            b"TLE\x1b7\x1b[2;5r",
            b"\x1b[?6h\x1b[1;1H\x1b[1;33mO",
            b"ne\x1b[0m\r\n\tTw",
            b"o\r\n\x1b[2K\x1b[1;32mTh",
            b"ree\x1b[0m\r\nFour\x1b8",
            b"\x1b*0\x1bNq",
        ],
    );

    assert_eq!(
        row_text(&terminal, 0),
        format!("TITLE{}              ", '\u{2500}')
    );
    assert_eq!(row_text(&terminal, 1), "One                 ");
    assert_eq!(row_text(&terminal, 2), "        Two         ");
    assert_eq!(row_text(&terminal, 3), "Three               ");
    assert_eq!(row_text(&terminal, 4), "Four                ");
    assert_eq!(terminal.cursor.position.row, 0);
    assert_eq!(terminal.cursor.position.col, 6);
    assert!(terminal.modes.origin);
}

#[test]
fn replays_chunked_status_stream_with_origin_mode_scroll_and_restore() {
    let mut terminal = Terminal::new(6, 18).unwrap();
    let mut parser = Parser::new();

    advance_chunks(
        &mut parser,
        &mut terminal,
        &[
            b"\x1b[2J\x1b[H\x1b[1;34mME",
            b"NU\x1b[0m\x1b[s\x1b[2;6r",
            b"\x1b[?6h\x1b[Htop\r\nm",
            b"id\r\nbot\r\nlast",
            b"\x1b[H\x1b[S\x1b[2K\x1b[1;36mne",
            b"w\x1b[0m\tcol\x1b[?6l\x1b[u",
            b"\x1b)0\x0eq\x0f",
        ],
    );

    assert_eq!(
        row_text(&terminal, 0),
        format!("MENU{}             ", '\u{2500}')
    );
    assert_eq!(row_text(&terminal, 1), "new     col       ");
    assert_eq!(row_text(&terminal, 2), "bot               ");
    assert_eq!(row_text(&terminal, 3), "last              ");
    assert_eq!(terminal.cursor.position.row, 0);
    assert_eq!(terminal.cursor.position.col, 5);
    assert!(!terminal.modes.origin);
}
