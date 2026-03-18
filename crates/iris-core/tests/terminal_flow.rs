use iris_core::{DamageRegion, Parser, Terminal};

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
fn parser_and_terminal_update_damage_and_cursor_state() {
    let mut terminal = Terminal::new(2, 6).unwrap();
    let mut parser = Parser::new();

    parser.advance(&mut terminal, b"hi\r\nx").unwrap();

    assert_eq!(
        terminal.grid.cell(0, 0).map(|cell| cell.character),
        Some('h')
    );
    assert_eq!(
        terminal.grid.cell(0, 1).map(|cell| cell.character),
        Some('i')
    );
    assert_eq!(
        terminal.grid.cell(1, 0).map(|cell| cell.character),
        Some('x')
    );
    assert_eq!(terminal.cursor.position.row, 1);
    assert_eq!(terminal.cursor.position.col, 1);

    let damage = terminal.take_damage();
    assert!(damage.contains(&DamageRegion::new(0, 0, 0, 1)));
    assert!(damage.contains(&DamageRegion::new(1, 1, 0, 0)));
}

#[test]
fn save_and_restore_cursor_round_trip_across_writes() {
    let mut terminal = Terminal::new(3, 3).unwrap();

    terminal.write_char('A').unwrap();
    terminal.save_cursor();
    terminal.write_char('B').unwrap();
    terminal.restore_cursor();
    terminal.write_char('C').unwrap();

    assert_eq!(
        terminal.grid.cell(0, 0).map(|cell| cell.character),
        Some('A')
    );
    assert_eq!(
        terminal.grid.cell(0, 1).map(|cell| cell.character),
        Some('C')
    );
    assert_eq!(terminal.cursor.position.col, 2);
}

#[test]
fn parser_handles_csi_cursor_and_erase_sequences() {
    let mut terminal = Terminal::new(3, 6).unwrap();
    let mut parser = Parser::new();

    parser
        .advance(&mut terminal, b"abc\x1b[1;2H\x1b[KZ")
        .unwrap();

    assert_eq!(
        terminal.grid.cell(0, 0).map(|cell| cell.character),
        Some('a')
    );
    assert_eq!(
        terminal.grid.cell(0, 1).map(|cell| cell.character),
        Some('Z')
    );
    assert_eq!(
        terminal.grid.cell(0, 2).map(|cell| cell.character),
        Some(' ')
    );
}

#[test]
fn parser_applies_sgr_attributes_to_printed_cells() {
    let mut terminal = Terminal::new(2, 8).unwrap();
    let mut parser = Parser::new();

    parser
        .advance(&mut terminal, b"\x1b[1;38;2;4;5;6mA\x1b[0mB")
        .unwrap();

    let styled = terminal.grid.cell(0, 0).copied().unwrap();
    let plain = terminal.grid.cell(0, 1).copied().unwrap();

    assert_eq!(styled.character, 'A');
    assert_eq!(plain.character, 'B');
    assert_ne!(styled.attrs, plain.attrs);
}

#[test]
fn parser_handles_escape_index_sequences() {
    let mut terminal = Terminal::new(2, 4).unwrap();
    let mut parser = Parser::new();

    parser
        .advance(&mut terminal, b"A\x1bEB\x1b[1;1H\x1bM")
        .unwrap();

    assert_eq!(
        terminal.grid.cell(0, 0).map(|cell| cell.character),
        Some(' ')
    );
    assert_eq!(
        terminal.grid.cell(1, 0).map(|cell| cell.character),
        Some('A')
    );
}

#[test]
fn parser_writes_utf8_text_across_input_boundaries() {
    let mut terminal = Terminal::new(2, 8).unwrap();
    let mut parser = Parser::new();

    parser.advance(&mut terminal, &[0xe2, 0x82]).unwrap();
    parser
        .advance(&mut terminal, &[0xac, b' ', 0xe4, 0xb8])
        .unwrap();
    parser.advance(&mut terminal, &[0xad]).unwrap();

    assert_eq!(
        terminal.grid.cell(0, 0).map(|cell| cell.character),
        Some('€')
    );
    assert_eq!(
        terminal.grid.cell(0, 1).map(|cell| cell.character),
        Some(' ')
    );
    assert_eq!(
        terminal.grid.cell(0, 2).map(|cell| cell.character),
        Some('中')
    );
}

#[test]
fn parser_applies_osc_title_and_hyperlink_actions() {
    let mut terminal = Terminal::new(2, 8).unwrap();
    let mut parser = Parser::new();

    parser.advance(&mut terminal, b"\x1b]2;Iris\x07").unwrap();
    parser
        .advance(
            &mut terminal,
            b"\x1b]8;id=prompt-1;https://example.com\x1b\\",
        )
        .unwrap();

    assert_eq!(terminal.window_title.as_deref(), Some("Iris"));
    assert_eq!(
        terminal
            .active_hyperlink
            .as_ref()
            .map(|link| link.uri.as_str()),
        Some("https://example.com")
    );

    parser.advance(&mut terminal, b"\x1b]8;;\x1b\\").unwrap();
    assert_eq!(terminal.active_hyperlink, None);
}

#[test]
fn parser_handles_chunked_mixed_sequence_stream() {
    let mut terminal = Terminal::new(3, 12).unwrap();
    let mut parser = Parser::new();

    advance_chunks(
        &mut parser,
        &mut terminal,
        &[
            b"\x1b]2;Sta",
            b"tus\x07\x1b[1;",
            b"31mER",
            b"R\x1b[0m ",
            b"\x1b*0\x1bNq",
            b"\x1b[2;",
            b"5HOK",
        ],
    );

    assert_eq!(terminal.window_title.as_deref(), Some("Status"));
    assert_eq!(row_text(&terminal, 0), format!("ERR {}       ", '\u{2500}'));
    assert_eq!(row_text(&terminal, 1), "    OK      ");
    assert_eq!(terminal.cursor.position.row, 1);
    assert_eq!(terminal.cursor.position.col, 6);
}

#[test]
fn parser_recovers_after_dcs_and_ignored_string_sequences() {
    let mut terminal = Terminal::new(2, 8).unwrap();
    let mut parser = Parser::new();

    parser
        .advance(
            &mut terminal,
            b"A\x1bPqignored\x1b\\B\x1bXskip\x1b\\C\x1b^hide\x1b\\D\x1b_drop\x1b\\E",
        )
        .unwrap();

    assert_eq!(
        terminal.grid.cell(0, 0).map(|cell| cell.character),
        Some('A')
    );
    assert_eq!(
        terminal.grid.cell(0, 1).map(|cell| cell.character),
        Some('B')
    );
    assert_eq!(
        terminal.grid.cell(0, 2).map(|cell| cell.character),
        Some('C')
    );
    assert_eq!(
        terminal.grid.cell(0, 3).map(|cell| cell.character),
        Some('D')
    );
    assert_eq!(
        terminal.grid.cell(0, 4).map(|cell| cell.character),
        Some('E')
    );
}

#[test]
fn parser_switches_between_primary_and_alternate_screen() {
    let mut terminal = Terminal::new(2, 4).unwrap();
    let mut parser = Parser::new();

    parser.advance(&mut terminal, b"A").unwrap();
    parser
        .advance(&mut terminal, b"\x1b[?1049hB\x1b[?1049l")
        .unwrap();

    assert_eq!(
        terminal.grid.cell(0, 0).map(|cell| cell.character),
        Some('A')
    );
    assert!(!terminal.modes.alternate_screen);
    assert_eq!(terminal.cursor.position.row, 0);
    assert_eq!(terminal.cursor.position.col, 1);
}

#[test]
fn parser_escape_reset_restores_initial_terminal_state() {
    let mut terminal = Terminal::new(2, 4).unwrap();
    let mut parser = Parser::new();

    parser
        .advance(
            &mut terminal,
            b"A\x1b]2;Iris\x07\x1b]8;;https://example.com\x1b\\\x1b=\x1b[?1049hB\x1bZ\x1bcC",
        )
        .unwrap();

    assert_eq!(
        terminal.grid.cell(0, 0).map(|cell| cell.character),
        Some('C')
    );
    assert_eq!(
        terminal.grid.cell(0, 1).map(|cell| cell.character),
        Some(' ')
    );
    assert_eq!(terminal.cursor.position.row, 0);
    assert_eq!(terminal.cursor.position.col, 1);
    assert_eq!(terminal.window_title, None);
    assert_eq!(terminal.active_hyperlink, None);
    assert!(!terminal.modes.alternate_screen);
    assert!(!terminal.modes.keypad);
}

#[test]
fn parser_applies_scroll_region_and_scroll_commands() {
    let mut terminal = Terminal::new(4, 2).unwrap();
    let mut parser = Parser::new();

    parser.advance(&mut terminal, b"A\r\nB\r\nC\r\nD").unwrap();
    parser.advance(&mut terminal, b"\x1b[2;4r\x1b[S").unwrap();

    assert_eq!(
        terminal.grid.cell(0, 0).map(|cell| cell.character),
        Some('A')
    );
    assert_eq!(
        terminal.grid.cell(1, 0).map(|cell| cell.character),
        Some('C')
    );
    assert_eq!(
        terminal.grid.cell(2, 0).map(|cell| cell.character),
        Some('D')
    );
    assert_eq!(
        terminal.grid.cell(3, 0).map(|cell| cell.character),
        Some(' ')
    );
}

#[test]
fn parser_applies_tab_stop_sequences() {
    let mut terminal = Terminal::new(1, 16).unwrap();
    let mut parser = Parser::new();

    parser
        .advance(&mut terminal, b"ABCD\x1bH\r\tX\x1b[ZY")
        .unwrap();

    assert_eq!(
        terminal.grid.cell(0, 0).map(|cell| cell.character),
        Some('A')
    );
    assert_eq!(
        terminal.grid.cell(0, 3).map(|cell| cell.character),
        Some('D')
    );
    assert_eq!(
        terminal.grid.cell(0, 4).map(|cell| cell.character),
        Some('Y')
    );
    assert_eq!(terminal.cursor.position.col, 5);
}

#[test]
fn parser_applies_forward_tab_sequences() {
    let mut terminal = Terminal::new(1, 20).unwrap();
    let mut parser = Parser::new();

    parser.advance(&mut terminal, b"A\x1b[2IB").unwrap();

    assert_eq!(
        terminal.grid.cell(0, 0).map(|cell| cell.character),
        Some('A')
    );
    assert_eq!(
        terminal.grid.cell(0, 16).map(|cell| cell.character),
        Some('B')
    );
    assert_eq!(terminal.cursor.position.col, 17);
}

#[test]
fn parser_applies_common_csi_alias_sequences() {
    let mut terminal = Terminal::new(3, 8).unwrap();
    let mut parser = Parser::new();

    parser
        .advance(&mut terminal, b"A\x1b[2e\x1b[4`B\x1b[2aC")
        .unwrap();

    assert_eq!(
        terminal.grid.cell(0, 0).map(|cell| cell.character),
        Some('A')
    );
    assert_eq!(
        terminal.grid.cell(2, 3).map(|cell| cell.character),
        Some('B')
    );
    assert_eq!(
        terminal.grid.cell(2, 6).map(|cell| cell.character),
        Some('C')
    );
    assert_eq!(terminal.cursor.position.row, 2);
    assert_eq!(terminal.cursor.position.col, 7);
}

#[test]
fn parser_applies_repeat_previous_character_sequences() {
    let mut terminal = Terminal::new(2, 8).unwrap();
    let mut parser = Parser::new();

    parser.advance(&mut terminal, b"A\x1b[3bB").unwrap();

    assert_eq!(row_text(&terminal, 0), "AAAAB   ");
    assert_eq!(terminal.cursor.position.row, 0);
    assert_eq!(terminal.cursor.position.col, 5);
}

#[test]
fn parser_applies_g2_and_g3_single_shift_sequences() {
    let mut terminal = Terminal::new(1, 8).unwrap();
    let mut parser = Parser::new();

    parser
        .advance(&mut terminal, b"\x1b*0\x1b+A\x1bNq\x1bO#q")
        .unwrap();

    assert_eq!(
        terminal.grid.cell(0, 0).map(|cell| cell.character),
        Some('\u{2500}')
    );
    assert_eq!(
        terminal.grid.cell(0, 1).map(|cell| cell.character),
        Some('\u{00a3}')
    );
    assert_eq!(
        terminal.grid.cell(0, 2).map(|cell| cell.character),
        Some('q')
    );
}

#[test]
fn parser_applies_insert_and_delete_character_sequences() {
    let mut terminal = Terminal::new(4, 6).unwrap();
    let mut parser = Parser::new();

    parser
        .advance(&mut terminal, b"ABCD\x1b[1;2H\x1b[2@XY\x1b[2P")
        .unwrap();

    assert_eq!(
        terminal.grid.cell(0, 0).map(|cell| cell.character),
        Some('A')
    );
    assert_eq!(
        terminal.grid.cell(0, 1).map(|cell| cell.character),
        Some('X')
    );
    assert_eq!(
        terminal.grid.cell(0, 2).map(|cell| cell.character),
        Some('Y')
    );
    assert_eq!(
        terminal.grid.cell(0, 3).map(|cell| cell.character),
        Some('D')
    );
}

#[test]
fn parser_applies_insert_line_sequences_within_scroll_region() {
    let mut terminal = Terminal::new(4, 2).unwrap();
    let mut parser = Parser::new();

    parser.advance(&mut terminal, b"A\r\nB\r\nC\r\nD").unwrap();
    parser
        .advance(&mut terminal, b"\x1b[2;4r\x1b[2;1H\x1b[L")
        .unwrap();

    assert_eq!(
        terminal.grid.cell(0, 0).map(|cell| cell.character),
        Some('A')
    );
    assert_eq!(
        terminal.grid.cell(1, 0).map(|cell| cell.character),
        Some(' ')
    );
    assert_eq!(
        terminal.grid.cell(2, 0).map(|cell| cell.character),
        Some('B')
    );
}

#[test]
fn parser_applies_erase_modes_and_scroll_region_reset_sequences() {
    let mut terminal = Terminal::new(4, 4).unwrap();
    let mut parser = Parser::new();

    parser.advance(&mut terminal, b"A\r\nB\r\nC\r\nD").unwrap();
    parser
        .advance(
            &mut terminal,
            b"\x1b[2;4r\x1b[S\x1b[r\x1b[4;1H\x1bD\x1b[2;1H\x1b[0J\x1b[1;2H\x1b[1KZ",
        )
        .unwrap();

    assert_eq!(row_text(&terminal, 0), " Z  ");
    assert_eq!(row_text(&terminal, 1), "    ");
    assert_eq!(row_text(&terminal, 2), "    ");
    assert_eq!(row_text(&terminal, 3), "    ");
}

#[test]
fn parser_replays_combined_screen_update_stream() {
    let mut terminal = Terminal::new(3, 24).unwrap();
    let mut parser = Parser::new();

    advance_chunks(
        &mut parser,
        &mut terminal,
        &[
            b"\x1b[2J\x1b[HHEAD",
            b"\x1b7\x1b[2;1H\x1b[1;32mOK\x1b[0m",
            b"\x1b8\x1b[2I",
            b"\x1b*0\x1bNq",
        ],
    );

    assert_eq!(
        row_text(&terminal, 0),
        format!("HEAD            {}       ", '\u{2500}')
    );
    assert_eq!(row_text(&terminal, 1), "OK                      ");
    assert_eq!(terminal.cursor.position.row, 0);
    assert_eq!(terminal.cursor.position.col, 17);
}
