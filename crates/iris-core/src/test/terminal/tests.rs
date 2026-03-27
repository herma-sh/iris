use super::Terminal;
use crate::cell::{Cell, CellFlags, Color};
use crate::parser::{Action, GraphicsRendition};
use crate::scrollback::{Line, ScrollbackConfig};
use crate::selection::SelectionKind;

#[test]
fn terminal_write_advances_cursor() {
    let mut terminal = Terminal::new(3, 4).unwrap();
    terminal.write_char('A').unwrap();
    assert_eq!(
        terminal.grid.cell(0, 0).map(|cell| cell.character),
        Some('A')
    );
    assert_eq!(terminal.cursor.position.col, 1);
}

#[test]
fn terminal_line_feed_scrolls_at_bottom() {
    let mut terminal = Terminal::new(2, 4).unwrap();
    terminal.move_cursor(1, 0);
    terminal.write_char('Z').unwrap();
    terminal.execute_control(0x0a).unwrap();
    terminal.write_char('Q').unwrap();

    assert_eq!(
        terminal.grid.cell(0, 0).map(|cell| cell.character),
        Some('Z')
    );
    assert_eq!(
        terminal.grid.cell(1, 1).map(|cell| cell.character),
        Some('Q')
    );
}

#[test]
fn terminal_resize_clamps_cursor() {
    let mut terminal = Terminal::new(8, 8).unwrap();
    terminal.move_cursor(7, 7);
    terminal.resize(2, 2).unwrap();
    assert_eq!(terminal.cursor.position.row, 1);
    assert_eq!(terminal.cursor.position.col, 1);
}

#[test]
fn terminal_restore_cursor_clamps_after_resize() {
    let mut terminal = Terminal::new(8, 8).unwrap();
    terminal.move_cursor(7, 7);
    terminal.save_cursor();
    terminal.resize(2, 2).unwrap();
    terminal.restore_cursor();
    assert_eq!(terminal.cursor.position.row, 1);
    assert_eq!(terminal.cursor.position.col, 1);
}

#[test]
fn terminal_applies_cursor_and_erase_actions() {
    let mut terminal = Terminal::new(3, 5).unwrap();
    terminal.write_char('A').unwrap();
    terminal.write_char('B').unwrap();
    terminal.write_char('C').unwrap();

    terminal
        .apply_action(Action::CursorPosition { row: 1, col: 2 })
        .unwrap();
    terminal.apply_action(Action::EraseLine(0)).unwrap();

    assert_eq!(
        terminal.grid.cell(0, 0).map(|cell| cell.character),
        Some('A')
    );
    assert_eq!(
        terminal.grid.cell(0, 1).map(|cell| cell.character),
        Some(' ')
    );
    assert_eq!(
        terminal.grid.cell(0, 2).map(|cell| cell.character),
        Some(' ')
    );
}

#[test]
fn terminal_applies_sgr_and_modes() {
    let mut terminal = Terminal::new(2, 4).unwrap();
    terminal
        .apply_action(Action::SetGraphicsRendition(
            vec![
                GraphicsRendition::Bold(true),
                GraphicsRendition::Foreground(Color::Indexed(33)),
            ]
            .into(),
        ))
        .unwrap();
    terminal.write_char('X').unwrap();
    terminal
        .apply_action(Action::ResetModes {
            private: true,
            modes: vec![25].into(),
        })
        .unwrap();

    let cell = terminal.grid.cell(0, 0).copied().unwrap();
    assert!(cell.attrs.flags.contains(CellFlags::BOLD));
    assert_eq!(cell.attrs.fg, Color::Indexed(33));
    assert!(!terminal.cursor.visible);
}

#[test]
fn terminal_next_line_and_reverse_index_follow_escape_semantics() {
    let mut terminal = Terminal::new(2, 4).unwrap();
    terminal.write_char('A').unwrap();
    terminal.apply_action(Action::NextLine).unwrap();
    terminal.write_char('B').unwrap();

    assert_eq!(terminal.cursor.position.row, 1);
    assert_eq!(
        terminal.grid.cell(1, 0).map(|cell| cell.character),
        Some('B')
    );

    terminal.move_cursor(0, 0);
    terminal.apply_action(Action::ReverseIndex).unwrap();
    assert_eq!(
        terminal.grid.cell(1, 0).map(|cell| cell.character),
        Some('A')
    );
}

#[test]
fn terminal_mode_application_respects_private_marker() {
    let mut terminal = Terminal::new(2, 4).unwrap();

    terminal
        .apply_action(Action::SetModes {
            private: false,
            modes: vec![4].into(),
        })
        .unwrap();
    assert!(terminal.modes.insert);

    terminal
        .apply_action(Action::ResetModes {
            private: true,
            modes: vec![4].into(),
        })
        .unwrap();
    assert!(terminal.modes.insert);
}

#[test]
fn terminal_origin_mode_homes_cursor_on_set_and_reset() {
    let mut terminal = Terminal::new(5, 6).unwrap();
    terminal
        .apply_action(Action::SetScrollRegion { top: 2, bottom: 4 })
        .unwrap();
    terminal.move_cursor(4, 5);

    terminal
        .apply_action(Action::SetModes {
            private: true,
            modes: vec![6].into(),
        })
        .unwrap();

    assert!(terminal.modes.origin);
    assert_eq!(terminal.cursor.position.row, 1);
    assert_eq!(terminal.cursor.position.col, 0);

    terminal
        .apply_action(Action::ResetModes {
            private: true,
            modes: vec![6].into(),
        })
        .unwrap();

    assert!(!terminal.modes.origin);
    assert_eq!(terminal.cursor.position.row, 0);
    assert_eq!(terminal.cursor.position.col, 0);
}

#[test]
fn terminal_origin_mode_constrains_absolute_cursor_addressing() {
    let mut terminal = Terminal::new(5, 6).unwrap();
    terminal
        .apply_action(Action::SetScrollRegion { top: 2, bottom: 4 })
        .unwrap();
    terminal
        .apply_action(Action::SetModes {
            private: true,
            modes: vec![6].into(),
        })
        .unwrap();

    terminal
        .apply_action(Action::CursorPosition { row: 3, col: 7 })
        .unwrap();
    assert_eq!(terminal.cursor.position.row, 3);
    assert_eq!(terminal.cursor.position.col, 5);

    terminal.apply_action(Action::VerticalPosition(9)).unwrap();
    assert_eq!(terminal.cursor.position.row, 3);

    terminal.apply_action(Action::CursorUp(9)).unwrap();
    assert_eq!(terminal.cursor.position.row, 1);

    terminal.apply_action(Action::CursorDown(9)).unwrap();
    assert_eq!(terminal.cursor.position.row, 3);
}

#[test]
fn terminal_tracks_keypad_mode_actions() {
    let mut terminal = Terminal::new(2, 4).unwrap();

    terminal.apply_action(Action::SetKeypadMode(true)).unwrap();
    assert!(terminal.modes.keypad);

    terminal.apply_action(Action::SetKeypadMode(false)).unwrap();
    assert!(!terminal.modes.keypad);
}

#[test]
fn terminal_scrolls_within_active_region() {
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
    terminal.apply_action(Action::ScrollUp(1)).unwrap();

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
fn terminal_index_scrolls_inside_active_region() {
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
    terminal.move_cursor(3, 0);
    terminal.apply_action(Action::Index).unwrap();

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
fn terminal_switches_between_primary_and_alternate_screen() {
    let mut terminal = Terminal::new(2, 4).unwrap();
    terminal.write_char('A').unwrap();
    terminal.move_cursor(1, 2);
    terminal.scroll_region = Some((0, 1));

    terminal
        .apply_action(Action::SetModes {
            private: true,
            modes: vec![1049].into(),
        })
        .unwrap();
    terminal.write_char('B').unwrap();

    assert!(terminal.modes.alternate_screen);
    assert_eq!(
        terminal.grid.cell(0, 0).map(|cell| cell.character),
        Some('B')
    );

    terminal
        .apply_action(Action::ResetModes {
            private: true,
            modes: vec![1049].into(),
        })
        .unwrap();

    assert!(!terminal.modes.alternate_screen);
    assert_eq!(
        terminal.grid.cell(0, 0).map(|cell| cell.character),
        Some('A')
    );
    assert_eq!(terminal.scroll_region, Some((0, 1)));
    assert_eq!(terminal.cursor.position.row, 1);
    assert_eq!(terminal.cursor.position.col, 2);
}

#[test]
fn terminal_reset_restores_initial_state() {
    let mut terminal = Terminal::new(2, 8).unwrap();
    terminal.write_char('A').unwrap();
    terminal
        .apply_action(Action::SetWindowTitle("Iris".to_string()))
        .unwrap();
    terminal
        .apply_action(Action::SetHyperlink {
            id: Some("prompt-1".to_string()),
            uri: "https://example.com".to_string(),
        })
        .unwrap();
    terminal.apply_action(Action::SetKeypadMode(true)).unwrap();
    terminal
        .apply_action(Action::SetModes {
            private: true,
            modes: vec![1049].into(),
        })
        .unwrap();
    terminal.write_char('B').unwrap();

    terminal.apply_action(Action::ResetTerminal).unwrap();

    assert_eq!(
        terminal.grid.cell(0, 0).map(|cell| cell.character),
        Some(' ')
    );
    assert_eq!(terminal.cursor.position.row, 0);
    assert_eq!(terminal.cursor.position.col, 0);
    assert_eq!(terminal.window_title, None);
    assert_eq!(terminal.active_hyperlink, None);
    assert!(!terminal.modes.alternate_screen);
    assert!(!terminal.modes.keypad);
    assert!(terminal.cursor.visible);
    assert!(terminal.cursor.blinking);
}

#[test]
fn terminal_reset_clears_stale_alternate_screen_state() {
    let mut terminal = Terminal::new(2, 4).unwrap();
    terminal.alternate_screen_state = Some(super::AlternateScreenState {
        grid: crate::grid::Grid::new(crate::grid::GridSize { rows: 2, cols: 4 }).unwrap(),
        cursor: terminal.cursor.save(),
        scroll_region: Some((0, 1)),
        scrollback_view_offset: 0,
    });

    terminal.apply_action(Action::ResetTerminal).unwrap();

    assert_eq!(terminal.alternate_screen_state, None);
    assert!(!terminal.modes.alternate_screen);
}

#[test]
fn terminal_tracks_osc_metadata_actions() {
    let mut terminal = Terminal::new(2, 4).unwrap();

    terminal
        .apply_action(Action::SetWindowTitle("Iris".to_string()))
        .unwrap();
    terminal
        .apply_action(Action::SetHyperlink {
            id: Some("prompt-1".to_string()),
            uri: "https://example.com".to_string(),
        })
        .unwrap();

    assert_eq!(terminal.window_title.as_deref(), Some("Iris"));
    assert_eq!(
        terminal.active_hyperlink,
        Some(super::Hyperlink {
            id: Some("prompt-1".to_string()),
            uri: "https://example.com".to_string(),
        })
    );

    terminal
        .apply_action(Action::SetHyperlink {
            id: None,
            uri: String::new(),
        })
        .unwrap();
    assert_eq!(terminal.active_hyperlink, None);
}

#[test]
fn terminal_tab_uses_default_stops() {
    let mut terminal = Terminal::new(2, 20).unwrap();

    terminal.apply_action(Action::Tab).unwrap();
    assert_eq!(terminal.cursor.position.col, 8);

    terminal.apply_action(Action::Tab).unwrap();
    assert_eq!(terminal.cursor.position.col, 16);
}

#[test]
fn terminal_forward_tab_uses_counted_stops() {
    let mut terminal = Terminal::new(1, 20).unwrap();

    terminal.apply_action(Action::ForwardTab(2)).unwrap();
    assert_eq!(terminal.cursor.position.col, 16);
}

#[test]
fn terminal_custom_tab_stop_and_back_tab_round_trip() {
    let mut terminal = Terminal::new(1, 16).unwrap();
    terminal.move_cursor(0, 4);

    terminal.apply_action(Action::SetTabStop).unwrap();
    terminal.apply_action(Action::CarriageReturn).unwrap();
    terminal.apply_action(Action::Tab).unwrap();
    assert_eq!(terminal.cursor.position.col, 4);

    terminal.write_char('X').unwrap();
    terminal.apply_action(Action::BackTab(1)).unwrap();
    assert_eq!(terminal.cursor.position.col, 4);

    terminal.write_char('Y').unwrap();
    assert_eq!(
        terminal.grid.cell(0, 4).map(|cell| cell.character),
        Some('Y')
    );
}

#[test]
fn terminal_clears_current_and_all_tab_stops() {
    let mut terminal = Terminal::new(1, 16).unwrap();
    terminal.move_cursor(0, 4);

    terminal.apply_action(Action::SetTabStop).unwrap();
    terminal.apply_action(Action::ClearTabStop(0)).unwrap();
    terminal.apply_action(Action::CarriageReturn).unwrap();
    terminal.apply_action(Action::Tab).unwrap();
    assert_eq!(terminal.cursor.position.col, 8);

    terminal.apply_action(Action::ClearTabStop(3)).unwrap();
    terminal.apply_action(Action::CarriageReturn).unwrap();
    terminal.apply_action(Action::Tab).unwrap();
    assert_eq!(terminal.cursor.position.col, 15);
}

#[test]
fn terminal_inserts_and_deletes_characters() {
    let mut terminal = Terminal::new(1, 6).unwrap();
    terminal.write_char('A').unwrap();
    terminal.write_char('B').unwrap();
    terminal.write_char('C').unwrap();
    terminal.write_char('D').unwrap();

    terminal.move_cursor(0, 1);
    terminal.apply_action(Action::InsertCharacters(2)).unwrap();
    terminal.write_char('X').unwrap();
    terminal.write_char('Y').unwrap();

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
        Some('B')
    );

    terminal.move_cursor(0, 2);
    terminal.apply_action(Action::DeleteCharacters(2)).unwrap();

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
        Some('C')
    );
}

#[test]
fn terminal_inserts_and_deletes_lines_within_scroll_region() {
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
    terminal.move_cursor(1, 0);
    terminal.apply_action(Action::InsertLines(1)).unwrap();

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

    terminal.apply_action(Action::DeleteLines(1)).unwrap();
    assert_eq!(
        terminal.grid.cell(1, 0).map(|cell| cell.character),
        Some('B')
    );
    assert_eq!(
        terminal.grid.cell(2, 0).map(|cell| cell.character),
        Some('C')
    );
}

#[test]
fn terminal_insert_and_delete_lines_noop_on_zero_row_grids() {
    let mut terminal = Terminal::new(0, 4).unwrap();

    terminal.apply_action(Action::InsertLines(1)).unwrap();
    terminal.apply_action(Action::DeleteLines(1)).unwrap();

    assert_eq!(terminal.grid.rows(), 0);
}

#[test]
fn terminal_restore_damage_replays_drained_regions() {
    let mut terminal = Terminal::new(1, 2).unwrap();
    terminal.write_char('A').unwrap();

    let damage = terminal.take_damage();
    assert_eq!(damage, vec![crate::damage::DamageRegion::new(0, 0, 0, 0)]);
    assert!(terminal.take_damage().is_empty());

    terminal.restore_damage(&damage);

    assert_eq!(terminal.take_damage(), damage);
}

#[test]
fn terminal_restore_scroll_delta_replays_drained_scrolls() {
    let mut terminal = Terminal::new(2, 2).unwrap();
    terminal.move_cursor(1, 0);
    terminal.execute_control(0x0a).unwrap();

    let scroll_delta = terminal.take_scroll_delta();
    assert_eq!(scroll_delta, Some(crate::damage::ScrollDelta::new(0, 1, 1)));
    assert_eq!(terminal.take_scroll_delta(), None);

    terminal.restore_scroll_delta(scroll_delta);

    assert_eq!(
        terminal.take_scroll_delta(),
        Some(crate::damage::ScrollDelta::new(0, 1, 1))
    );
}

#[test]
fn terminal_paste_bytes_returns_raw_text_when_bracketed_mode_is_disabled() {
    let terminal = Terminal::new(2, 4).unwrap();
    let payload = terminal.paste_bytes("raw-data");

    assert_eq!(payload, b"raw-data");
}

#[test]
fn terminal_paste_bytes_wraps_text_when_bracketed_mode_is_enabled() {
    let mut terminal = Terminal::new(2, 4).unwrap();
    terminal
        .apply_action(Action::SetModes {
            private: true,
            modes: vec![2004].into(),
        })
        .unwrap();

    let payload = terminal.paste_bytes("wrapped");
    assert_eq!(payload, b"\x1b[200~wrapped\x1b[201~");
}

#[test]
fn terminal_paste_bytes_preserves_multibyte_utf8_with_and_without_bracketing() {
    let mut terminal = Terminal::new(2, 4).unwrap();
    let text = "€🙂";

    let expected_raw = vec![0xE2, 0x82, 0xAC, 0xF0, 0x9F, 0x99, 0x82];
    assert_eq!(terminal.paste_bytes(text), expected_raw);

    terminal
        .apply_action(Action::SetModes {
            private: true,
            modes: vec![2004].into(),
        })
        .unwrap();

    let expected_wrapped = vec![
        0x1B, 0x5B, 0x32, 0x30, 0x30, 0x7E, 0xE2, 0x82, 0xAC, 0xF0, 0x9F, 0x99, 0x82, 0x1B, 0x5B,
        0x32, 0x30, 0x31, 0x7E,
    ];
    assert_eq!(terminal.paste_bytes(text), expected_wrapped);
}

#[test]
fn terminal_selection_lifecycle_tracks_selected_and_copy_text() {
    let mut terminal = Terminal::new(2, 12).unwrap();
    terminal.write_ascii_run(b"hello world").unwrap();

    assert!(!terminal.is_selecting());
    assert!(!terminal.has_selection());
    assert_eq!(terminal.selected_text(), None);
    assert_eq!(terminal.copy_selection_text(), None);

    terminal.start_selection(0, 6, SelectionKind::Simple);
    assert!(terminal.is_selecting());
    assert!(!terminal.has_selection());

    terminal.extend_selection(0, 10);
    terminal.complete_selection();

    assert!(!terminal.is_selecting());
    assert!(terminal.has_selection());
    assert_eq!(terminal.selected_text().as_deref(), Some("world"));
    assert_eq!(terminal.copy_selection_text().as_deref(), Some("world"));

    terminal.cancel_selection();
    assert!(!terminal.has_selection());
    assert_eq!(terminal.selection(), None);
}

#[test]
fn terminal_word_and_line_selection_wrappers_extract_expected_text() {
    let mut terminal = Terminal::new(2, 8).unwrap();
    terminal.write_ascii_run(b"first").unwrap();
    terminal.apply_action(Action::NextLine).unwrap();
    terminal.write_ascii_run(b"second").unwrap();

    terminal.select_word(0, 1);
    assert_eq!(
        terminal.selection().map(|selection| selection.kind),
        Some(SelectionKind::Word)
    );
    assert_eq!(terminal.selected_text().as_deref(), Some("first"));

    terminal.select_line(1);
    assert_eq!(
        terminal.selection().map(|selection| selection.kind),
        Some(SelectionKind::Line)
    );
    assert_eq!(terminal.selected_text().as_deref(), Some("second  "));
    assert_eq!(
        terminal.copy_selection_text().as_deref(),
        Some("second  \n")
    );
}

#[test]
fn terminal_resize_and_reset_clear_selection_state() {
    let mut terminal = Terminal::new(2, 12).unwrap();
    terminal.write_ascii_run(b"hello world").unwrap();

    terminal.start_selection(0, 0, SelectionKind::Simple);
    terminal.extend_selection(0, 4);
    terminal.complete_selection();
    assert!(terminal.has_selection());

    terminal.resize(1, 6).unwrap();
    assert!(!terminal.has_selection());
    assert_eq!(terminal.selection(), None);

    terminal.start_selection(0, 0, SelectionKind::Simple);
    terminal.extend_selection(0, 4);
    terminal.complete_selection();
    assert!(terminal.has_selection());

    terminal.apply_action(Action::ResetTerminal).unwrap();
    assert!(!terminal.has_selection());
    assert_eq!(terminal.selection(), None);
}

#[test]
fn terminal_alternate_screen_transitions_clear_selection_state() {
    let mut terminal = Terminal::new(2, 12).unwrap();
    terminal.write_ascii_run(b"hello world").unwrap();

    terminal.start_selection(0, 0, SelectionKind::Simple);
    terminal.extend_selection(0, 4);
    terminal.complete_selection();
    assert!(terminal.has_selection());

    terminal
        .apply_action(Action::SetModes {
            private: true,
            modes: vec![1049].into(),
        })
        .unwrap();
    assert!(!terminal.has_selection());
    assert_eq!(terminal.selection(), None);

    terminal.start_selection(0, 0, SelectionKind::Simple);
    terminal.extend_selection(0, 2);
    terminal.complete_selection();
    assert!(terminal.has_selection());

    terminal
        .apply_action(Action::ResetModes {
            private: true,
            modes: vec![1049].into(),
        })
        .unwrap();
    assert!(!terminal.has_selection());
    assert_eq!(terminal.selection(), None);
}

#[test]
fn terminal_selection_queries_require_completed_selection() {
    let mut terminal = Terminal::new(1, 8).unwrap();
    terminal.write_ascii_run(b"selection").unwrap();

    terminal.start_selection(0, 1, SelectionKind::Simple);
    terminal.extend_selection(0, 3);

    assert!(!terminal.selection_contains(0, 2));
    assert!(!terminal.selection_contains(0, 8));
    assert_eq!(terminal.selection_row_bounds(0), None);
    assert_eq!(terminal.selection_row_span(), None);

    terminal.complete_selection();

    assert!(terminal.selection_contains(0, 2));
    assert_eq!(terminal.selection_row_bounds(0), Some((1, 3)));
    assert_eq!(terminal.selection_row_span(), Some((0, 0)));
}

#[test]
fn terminal_selection_queries_return_expected_linear_row_bounds() {
    let mut terminal = Terminal::new(3, 5).unwrap();

    terminal.start_selection(0, 3, SelectionKind::Simple);
    terminal.extend_selection(2, 1);
    terminal.complete_selection();

    assert_eq!(terminal.selection_row_span(), Some((0, 2)));
    assert_eq!(terminal.selection_row_bounds(0), Some((3, 4)));
    assert_eq!(terminal.selection_row_bounds(1), Some((0, 4)));
    assert_eq!(terminal.selection_row_bounds(2), Some((0, 1)));
    assert_eq!(terminal.selection_row_bounds(3), None);
}

#[test]
fn terminal_selection_queries_return_expected_block_bounds() {
    let mut terminal = Terminal::new(3, 5).unwrap();

    terminal.start_selection(0, 1, SelectionKind::Block);
    terminal.extend_selection(2, 3);
    terminal.complete_selection();

    assert_eq!(terminal.selection_row_span(), Some((0, 2)));
    assert_eq!(terminal.selection_row_bounds(0), Some((1, 3)));
    assert_eq!(terminal.selection_row_bounds(1), Some((1, 3)));
    assert_eq!(terminal.selection_row_bounds(2), Some((1, 3)));
    assert!(terminal.selection_contains(1, 2));
    assert!(!terminal.selection_contains(1, 4));
    assert!(!terminal.selection_contains(4, 1));
}

#[test]
fn terminal_line_feed_captures_scrolled_primary_row_in_scrollback() {
    let mut terminal = Terminal::new_with_scrollback(
        2,
        4,
        ScrollbackConfig {
            max_lines: 16,
            max_memory_bytes: None,
        },
    )
    .unwrap();

    terminal.write_ascii_run(b"AA").unwrap();
    terminal.apply_action(Action::NextLine).unwrap();
    terminal.write_ascii_run(b"BB").unwrap();
    terminal.apply_action(Action::NextLine).unwrap();

    assert_eq!(terminal.scrollback().len(), 1);
    assert_eq!(
        terminal.scrollback().newest(0).map(Line::text).as_deref(),
        Some("AA  ")
    );
}

#[test]
fn terminal_scroll_up_captures_full_screen_scrollback_lines() {
    let mut terminal = Terminal::new(3, 3).unwrap();
    for (row, ch) in ['A', 'B', 'C'].into_iter().enumerate() {
        terminal.grid.write(row, 0, Cell::new(ch)).unwrap();
    }

    terminal.apply_action(Action::ScrollUp(2)).unwrap();

    assert_eq!(terminal.scrollback().len(), 2);
    assert_eq!(
        terminal.scrollback().newest(0).map(Line::text).as_deref(),
        Some("B  ")
    );
    assert_eq!(
        terminal.scrollback().newest(1).map(Line::text).as_deref(),
        Some("A  ")
    );
}

#[test]
fn terminal_scroll_up_inside_custom_region_does_not_capture_scrollback() {
    let mut terminal = Terminal::new(4, 3).unwrap();
    for (row, ch) in ['A', 'B', 'C', 'D'].into_iter().enumerate() {
        terminal.grid.write(row, 0, Cell::new(ch)).unwrap();
    }

    terminal
        .apply_action(Action::SetScrollRegion { top: 2, bottom: 4 })
        .unwrap();
    terminal.apply_action(Action::ScrollUp(1)).unwrap();

    assert!(terminal.scrollback().is_empty());
}

#[test]
fn terminal_alternate_screen_scroll_does_not_mutate_primary_scrollback() {
    let mut terminal = Terminal::new(2, 4).unwrap();
    terminal.write_ascii_run(b"AA").unwrap();
    terminal.apply_action(Action::NextLine).unwrap();
    terminal.write_ascii_run(b"BB").unwrap();
    terminal.apply_action(Action::NextLine).unwrap();
    let scrollback_before_alt = terminal.scrollback().len();

    terminal
        .apply_action(Action::SetModes {
            private: true,
            modes: vec![1049].into(),
        })
        .unwrap();
    terminal.write_ascii_run(b"CC").unwrap();
    terminal.apply_action(Action::NextLine).unwrap();
    terminal.write_ascii_run(b"DD").unwrap();
    terminal.apply_action(Action::NextLine).unwrap();
    terminal
        .apply_action(Action::ResetModes {
            private: true,
            modes: vec![1049].into(),
        })
        .unwrap();

    assert_eq!(terminal.scrollback().len(), scrollback_before_alt);
}

#[test]
fn terminal_viewport_scroll_commands_adjust_offset_within_scrollback_bounds() {
    let mut terminal = Terminal::new(3, 4).unwrap();
    for index in 0..10 {
        terminal
            .scrollback
            .push(Line::from_text(&format!("line-{index}"), false));
    }

    assert_eq!(terminal.scrollback_view_offset(), 0);
    terminal.scroll_page_up();
    assert_eq!(terminal.scrollback_view_offset(), 3);
    terminal.scroll_line_up();
    assert_eq!(terminal.scrollback_view_offset(), 4);
    terminal.scroll_to_top();
    assert_eq!(terminal.scrollback_view_offset(), 10);
    terminal.scroll_page_down();
    assert_eq!(terminal.scrollback_view_offset(), 7);
    terminal.scroll_line_down();
    assert_eq!(terminal.scrollback_view_offset(), 6);
    terminal.scroll_to_bottom();
    assert_eq!(terminal.scrollback_view_offset(), 0);
}
