use crate::input::{MouseButton, MouseModifiers, SelectionInputEvent, SelectionInputState};
use crate::selection::SelectionKind;
use crate::Terminal;

#[test]
fn selection_input_drag_creates_simple_selection() {
    let mut terminal = Terminal::new(1, 9).expect("terminal should be created");
    terminal
        .write_ascii_run(b"abcdefgh")
        .expect("terminal write should succeed");

    let mut input = SelectionInputState::new();
    assert!(input.handle_event(
        &mut terminal,
        SelectionInputEvent::Press {
            row: 0,
            col: 1,
            button: MouseButton::Left,
            modifiers: MouseModifiers::default(),
            click_count: 1,
        },
    ));
    assert!(input.handle_event(
        &mut terminal,
        SelectionInputEvent::Move {
            row: 0,
            col: 3,
            modifiers: MouseModifiers::default(),
        },
    ));
    assert!(input.handle_event(
        &mut terminal,
        SelectionInputEvent::Release {
            row: 0,
            col: 3,
            button: MouseButton::Left,
            modifiers: MouseModifiers::default(),
        },
    ));

    assert_eq!(
        terminal.selection().map(|selection| selection.kind),
        Some(SelectionKind::Simple)
    );
    assert!(terminal.selection_contains(0, 2));
    assert_eq!(terminal.copy_selection_text().as_deref(), Some("bcd"));
}

#[test]
fn selection_input_double_click_selects_word() {
    let mut terminal = Terminal::new(1, 12).expect("terminal should be created");
    terminal
        .write_ascii_run(b"hello world")
        .expect("terminal write should succeed");

    let mut input = SelectionInputState::new();
    assert!(input.handle_event(
        &mut terminal,
        SelectionInputEvent::Press {
            row: 0,
            col: 6,
            button: MouseButton::Left,
            modifiers: MouseModifiers::default(),
            click_count: 2,
        },
    ));

    assert_eq!(
        terminal.selection().map(|selection| selection.kind),
        Some(SelectionKind::Word)
    );
    assert_eq!(terminal.copy_selection_text().as_deref(), Some("world"));
}

#[test]
fn selection_input_triple_click_selects_line() {
    let mut terminal = Terminal::new(1, 12).expect("terminal should be created");
    terminal
        .write_ascii_run(b"hello world")
        .expect("terminal write should succeed");

    let mut input = SelectionInputState::new();
    assert!(input.handle_event(
        &mut terminal,
        SelectionInputEvent::Press {
            row: 0,
            col: 4,
            button: MouseButton::Left,
            modifiers: MouseModifiers::default(),
            click_count: 3,
        },
    ));

    assert_eq!(
        terminal.selection().map(|selection| selection.kind),
        Some(SelectionKind::Line)
    );
    assert_eq!(
        terminal.copy_selection_text().as_deref(),
        Some("hello world \n")
    );
}

#[test]
fn selection_input_alt_drag_creates_block_selection() {
    let mut terminal = Terminal::new(3, 5).expect("terminal should be created");
    terminal.move_cursor(0, 0);
    terminal
        .write_ascii_run(b"abcd")
        .expect("row write should succeed");
    terminal.move_cursor(1, 0);
    terminal
        .write_ascii_run(b"efgh")
        .expect("row write should succeed");
    terminal.move_cursor(2, 0);
    terminal
        .write_ascii_run(b"ijkl")
        .expect("row write should succeed");

    let mut input = SelectionInputState::new();
    assert!(input.handle_event(
        &mut terminal,
        SelectionInputEvent::Press {
            row: 0,
            col: 1,
            button: MouseButton::Left,
            modifiers: MouseModifiers {
                alt: true,
                ..MouseModifiers::default()
            },
            click_count: 1,
        },
    ));
    assert!(input.handle_event(
        &mut terminal,
        SelectionInputEvent::Move {
            row: 2,
            col: 2,
            modifiers: MouseModifiers {
                alt: true,
                ..MouseModifiers::default()
            },
        },
    ));
    assert!(input.handle_event(
        &mut terminal,
        SelectionInputEvent::Release {
            row: 2,
            col: 2,
            button: MouseButton::Left,
            modifiers: MouseModifiers {
                alt: true,
                ..MouseModifiers::default()
            },
        },
    ));

    assert_eq!(
        terminal.selection().map(|selection| selection.kind),
        Some(SelectionKind::Block)
    );
    assert_eq!(terminal.selection_row_bounds(1), Some((1, 2)));
    assert_eq!(
        terminal.copy_selection_text().as_deref(),
        Some("bc\nfg\njk")
    );
}

#[test]
fn selection_input_ignores_move_without_active_drag() {
    let mut terminal = Terminal::new(1, 4).expect("terminal should be created");
    let mut input = SelectionInputState::new();

    assert!(!input.handle_event(
        &mut terminal,
        SelectionInputEvent::Move {
            row: 0,
            col: 1,
            modifiers: MouseModifiers::default(),
        },
    ));
    assert!(!terminal.has_selection());
}

#[test]
fn selection_input_ignores_non_left_button_press() {
    let mut terminal = Terminal::new(1, 4).expect("terminal should be created");
    let mut input = SelectionInputState::new();

    assert!(!input.handle_event(
        &mut terminal,
        SelectionInputEvent::Press {
            row: 0,
            col: 1,
            button: MouseButton::Right,
            modifiers: MouseModifiers::default(),
            click_count: 1,
        },
    ));
    assert!(!terminal.has_selection());
}
