use super::Terminal;
use crate::parser::Action;

#[test]
fn terminal_applies_all_erase_display_modes() {
    let mut terminal = Terminal::new(3, 4).unwrap();
    terminal.write_char('A').unwrap();
    terminal.next_line().unwrap();
    terminal.write_char('B').unwrap();
    terminal.next_line().unwrap();
    terminal.write_char('C').unwrap();

    terminal.move_cursor(1, 0);
    terminal.apply_action(Action::EraseDisplay(0)).unwrap();
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
        Some(' ')
    );

    terminal.write_char('B').unwrap();
    terminal.next_line().unwrap();
    terminal.write_char('C').unwrap();

    terminal.move_cursor(1, 0);
    terminal.apply_action(Action::EraseDisplay(1)).unwrap();
    assert_eq!(
        terminal.grid.cell(0, 0).map(|cell| cell.character),
        Some(' ')
    );
    assert_eq!(
        terminal.grid.cell(1, 0).map(|cell| cell.character),
        Some(' ')
    );
    assert_eq!(
        terminal.grid.cell(2, 0).map(|cell| cell.character),
        Some('C')
    );

    terminal.apply_action(Action::EraseDisplay(2)).unwrap();
    assert_eq!(
        terminal.grid.cell(2, 0).map(|cell| cell.character),
        Some(' ')
    );

    terminal.write_char('Z').unwrap();
    assert_eq!(
        terminal.grid.cell(1, 0).map(|cell| cell.character),
        Some('Z')
    );
    terminal.apply_action(Action::EraseDisplay(3)).unwrap();
    assert_eq!(
        terminal.grid.cell(1, 0).map(|cell| cell.character),
        Some(' ')
    );
}

#[test]
fn terminal_applies_all_erase_line_modes() {
    let mut terminal = Terminal::new(1, 6).unwrap();
    for character in ['A', 'B', 'C', 'D', 'E'] {
        terminal.write_char(character).unwrap();
    }

    terminal.move_cursor(0, 2);
    terminal.apply_action(Action::EraseLine(0)).unwrap();
    assert_eq!(
        terminal.grid.cell(0, 0).map(|cell| cell.character),
        Some('A')
    );
    assert_eq!(
        terminal.grid.cell(0, 2).map(|cell| cell.character),
        Some(' ')
    );

    terminal.write_char('C').unwrap();
    terminal.write_char('D').unwrap();
    terminal.write_char('E').unwrap();

    terminal.move_cursor(0, 2);
    terminal.apply_action(Action::EraseLine(1)).unwrap();
    assert_eq!(
        terminal.grid.cell(0, 0).map(|cell| cell.character),
        Some(' ')
    );
    assert_eq!(
        terminal.grid.cell(0, 3).map(|cell| cell.character),
        Some('D')
    );

    terminal.apply_action(Action::EraseLine(2)).unwrap();
    assert_eq!(
        terminal.grid.cell(0, 3).map(|cell| cell.character),
        Some(' ')
    );
    assert_eq!(
        terminal.grid.cell(0, 4).map(|cell| cell.character),
        Some(' ')
    );
}

#[test]
fn terminal_resets_scroll_region_to_full_screen() {
    let mut terminal = Terminal::new(4, 2).unwrap();
    terminal.write_char('A').unwrap();
    terminal.next_line().unwrap();
    terminal.write_char('B').unwrap();
    terminal.next_line().unwrap();
    terminal.write_char('C').unwrap();
    terminal.next_line().unwrap();
    terminal.write_char('D').unwrap();

    terminal
        .apply_action(Action::SetScrollRegion { top: 2, bottom: 4 })
        .unwrap();
    terminal
        .apply_action(Action::SetScrollRegion { top: 1, bottom: 0 })
        .unwrap();
    terminal.move_cursor(3, 0);
    terminal.apply_action(Action::Index).unwrap();

    assert_eq!(
        terminal.grid.cell(0, 0).map(|cell| cell.character),
        Some('B')
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
