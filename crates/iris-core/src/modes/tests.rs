
use super::{Mode, TerminalModes};

#[test]
fn terminal_modes_default_to_wrap_and_visible_cursor() {
    let modes = TerminalModes::new();
    assert!(modes.wrap);
    assert!(!modes.keypad);
    assert!(modes.cursor_visible);
    assert!(modes.cursor_blink);
}

#[test]
fn terminal_modes_can_toggle_flags() {
    let mut modes = TerminalModes::new();
    modes.set_mode(Mode::Wrap, false);
    assert!(!modes.wrap);

    modes.set_mode(Mode::BracketedPaste, true);
    assert!(modes.bracketed_paste);

    modes.set_mode(Mode::Keypad, true);
    assert!(modes.keypad);
}

#[test]
fn mode_parsing_maps_known_parameters() {
    assert_eq!(Mode::from_ansi_param(4), Some(Mode::Insert));
    assert_eq!(Mode::from_dec_private_param(25), Some(Mode::CursorVisible));
    assert_eq!(Mode::from_dec_private_param(9999), None);
}
