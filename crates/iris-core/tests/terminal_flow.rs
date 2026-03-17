use iris_core::{DamageRegion, Parser, Terminal};

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
