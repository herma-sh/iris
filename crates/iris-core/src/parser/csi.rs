use super::actions::{Action, ModeParams};

/// Parses a CSI final byte into one or more terminal actions.
#[must_use]
#[cfg(test)]
pub fn parse_csi(params: &[u16], private_marker: Option<u8>, final_byte: u8) -> Vec<Action> {
    let mut actions = Vec::new();
    parse_csi_into(params, private_marker, final_byte, &mut actions);
    actions
}

pub fn parse_csi_into(
    params: &[u16],
    private_marker: Option<u8>,
    final_byte: u8,
    actions: &mut Vec<Action>,
) {
    match final_byte {
        b'A' => actions.push(Action::CursorUp(param_or(params, 0, 1))),
        b'B' => actions.push(Action::CursorDown(param_or(params, 0, 1))),
        b'C' => actions.push(Action::CursorForward(param_or(params, 0, 1))),
        b'D' => actions.push(Action::CursorBack(param_or(params, 0, 1))),
        b'E' => actions.push(Action::CursorNextLine(param_or(params, 0, 1))),
        b'F' => actions.push(Action::CursorPreviousLine(param_or(params, 0, 1))),
        b'G' | b'`' => actions.push(Action::CursorColumn(param_or(params, 0, 1))),
        b'I' => actions.push(Action::ForwardTab(param_or(params, 0, 1))),
        b'H' | b'f' => actions.push(Action::CursorPosition {
            row: param_or(params, 0, 1),
            col: param_or(params, 1, 1),
        }),
        b'L' => actions.push(Action::InsertLines(param_or(params, 0, 1))),
        b'M' => actions.push(Action::DeleteLines(param_or(params, 0, 1))),
        b'P' => actions.push(Action::DeleteCharacters(param_or(params, 0, 1))),
        b'a' => actions.push(Action::CursorForward(param_or(params, 0, 1))),
        b'd' => actions.push(Action::VerticalPosition(param_or(params, 0, 1))),
        b'e' => actions.push(Action::CursorDown(param_or(params, 0, 1))),
        b'@' => actions.push(Action::InsertCharacters(param_or(params, 0, 1))),
        b'J' => actions.push(Action::EraseDisplay(param_or(params, 0, 0))),
        b'K' => actions.push(Action::EraseLine(param_or(params, 0, 0))),
        b'S' => actions.push(Action::ScrollUp(param_or(params, 0, 1))),
        b'T' => actions.push(Action::ScrollDown(param_or(params, 0, 1))),
        b'Z' => actions.push(Action::BackTab(param_or(params, 0, 1))),
        b'X' => actions.push(Action::EraseCharacters(param_or(params, 0, 1))),
        b'g' => actions.push(Action::ClearTabStop(param_or(params, 0, 0))),
        b'r' => actions.push(Action::SetScrollRegion {
            top: param_or(params, 0, 1),
            bottom: params.get(1).copied().unwrap_or(0),
        }),
        b'm' => actions.push(Action::SetGraphicsRendition(Action::parse_sgr(params))),
        b's' => actions.push(Action::SaveCursor),
        b'u' => actions.push(Action::RestoreCursor),
        b'h' => {
            let modes = normalized_modes(params, private_marker);
            if !modes.is_empty() {
                actions.push(Action::SetModes {
                    private: private_marker == Some(b'?'),
                    modes,
                });
            }
        }
        b'l' => {
            let modes = normalized_modes(params, private_marker);
            if !modes.is_empty() {
                actions.push(Action::ResetModes {
                    private: private_marker == Some(b'?'),
                    modes,
                });
            }
        }
        _ => {}
    }
}

fn param_or(params: &[u16], index: usize, default: u16) -> u16 {
    match params.get(index).copied() {
        Some(0) | None => default,
        Some(value) => value,
    }
}

fn normalized_modes(params: &[u16], private_marker: Option<u8>) -> ModeParams {
    if private_marker.is_some() && private_marker != Some(b'?') {
        return ModeParams::new();
    }

    if private_marker.is_none() && params.is_empty() {
        return ModeParams::new();
    }

    params.iter().copied().filter(|value| *value != 0).collect()
}

#[cfg(test)]
mod tests {
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
}
