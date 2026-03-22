
use super::parse_csi;
use crate::cell::Color;
use crate::parser::{Action, GraphicsRendition};

#[test]
fn csi_uses_default_parameters() {
    assert_eq!(parse_csi(&[], None, b'A'), vec![Action::CursorUp(1)]);
    assert_eq!(
        parse_csi(&[], None, b'@'),
        vec![Action::InsertCharacters(1)]
    );
    assert_eq!(parse_csi(&[], None, b'J'), vec![Action::EraseDisplay(0)]);
    assert_eq!(parse_csi(&[], None, b'L'), vec![Action::InsertLines(1)]);
    assert_eq!(parse_csi(&[], None, b'M'), vec![Action::DeleteLines(1)]);
    assert_eq!(
        parse_csi(&[], None, b'P'),
        vec![Action::DeleteCharacters(1)]
    );
    assert_eq!(parse_csi(&[], None, b'S'), vec![Action::ScrollUp(1)]);
    assert_eq!(parse_csi(&[], None, b'I'), vec![Action::ForwardTab(1)]);
    assert_eq!(parse_csi(&[], None, b'Z'), vec![Action::BackTab(1)]);
    assert_eq!(parse_csi(&[], None, b'`'), vec![Action::CursorColumn(1)]);
    assert_eq!(parse_csi(&[], None, b'a'), vec![Action::CursorForward(1)]);
    assert_eq!(parse_csi(&[], None, b'e'), vec![Action::CursorDown(1)]);
    assert_eq!(parse_csi(&[], None, b'g'), vec![Action::ClearTabStop(0)]);
    assert_eq!(
        parse_csi(&[], None, b'r'),
        vec![Action::SetScrollRegion { top: 1, bottom: 0 }]
    );
}

#[test]
fn csi_parses_sgr_extended_colors() {
    assert_eq!(
        parse_csi(&[38, 5, 200], None, b'm'),
        vec![Action::SetGraphicsRendition(
            vec![GraphicsRendition::Foreground(Color::Indexed(200),)].into()
        )]
    );
}

#[test]
fn csi_rejects_non_dec_private_markers_for_modes() {
    assert!(parse_csi(&[4], Some(b'>'), b'h').is_empty());
    assert!(parse_csi(&[25], Some(b'<'), b'l').is_empty());
}

#[test]
fn csi_parses_explicit_erase_modes_and_scroll_region_reset() {
    assert_eq!(parse_csi(&[1], None, b'J'), vec![Action::EraseDisplay(1)]);
    assert_eq!(parse_csi(&[2], None, b'J'), vec![Action::EraseDisplay(2)]);
    assert_eq!(parse_csi(&[3], None, b'J'), vec![Action::EraseDisplay(3)]);
    assert_eq!(parse_csi(&[1], None, b'K'), vec![Action::EraseLine(1)]);
    assert_eq!(parse_csi(&[2], None, b'K'), vec![Action::EraseLine(2)]);
    assert_eq!(
        parse_csi(&[], None, b'r'),
        vec![Action::SetScrollRegion { top: 1, bottom: 0 }]
    );
}
