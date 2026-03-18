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
fn replays_vim_like_alternate_screen_redraw() {
    let mut terminal = Terminal::new(6, 24).unwrap();
    let mut parser = Parser::new();

    advance_chunks(
        &mut parser,
        &mut terminal,
        &[
            b"\x1b[?1049h\x1b[?25l\x1b[2J",
            b"\x1b[Hline one\r\nline",
            b" two\r\n~\r\n~\r\n~",
            b"\x1b[6;1H\x1b[7m  2,1   All            \x1b[0m",
            b"\x1b[2;1H",
        ],
    );

    assert_eq!(row_text(&terminal, 0), "line one                ");
    assert_eq!(row_text(&terminal, 1), "line two                ");
    assert_eq!(row_text(&terminal, 2), "~                       ");
    assert_eq!(row_text(&terminal, 3), "~                       ");
    assert_eq!(row_text(&terminal, 4), "~                       ");
    assert_eq!(row_text(&terminal, 5), "  2,1   All             ");
    assert!(terminal.modes.alternate_screen);
    assert!(!terminal.cursor.visible);
    assert_eq!(terminal.cursor.position.row, 1);
    assert_eq!(terminal.cursor.position.col, 0);
}

#[test]
fn replays_tmux_like_status_redraw_on_main_screen() {
    let mut terminal = Terminal::new(4, 30).unwrap();
    let mut parser = Parser::new();

    advance_chunks(
        &mut parser,
        &mut terminal,
        &[
            b"\x1b]2;main:logs\x07shell$ ls",
            b"\r\nsrc  Cargo.toml",
            b"\x1b7\x1b[4;1H\x1b[44;37m",
            b"[main] 0:bash* 1:logs     ",
            b"\x1b[0m\x1b8",
        ],
    );

    assert_eq!(terminal.window_title.as_deref(), Some("main:logs"));
    assert_eq!(row_text(&terminal, 0), "shell$ ls                     ");
    assert_eq!(row_text(&terminal, 1), "src  Cargo.toml               ");
    assert_eq!(row_text(&terminal, 3), "[main] 0:bash* 1:logs         ");
    assert_eq!(terminal.cursor.position.row, 1);
    assert_eq!(terminal.cursor.position.col, 15);
}
