use super::actions::Action;

/// Parses a CSI final byte into one or more terminal actions.
#[must_use]
pub fn parse_csi(params: &[u16], private_marker: Option<u8>, final_byte: u8) -> Vec<Action> {
    match final_byte {
        b'A' => vec![Action::CursorUp(param_or(params, 0, 1))],
        b'B' => vec![Action::CursorDown(param_or(params, 0, 1))],
        b'C' => vec![Action::CursorForward(param_or(params, 0, 1))],
        b'D' => vec![Action::CursorBack(param_or(params, 0, 1))],
        b'E' => vec![Action::CursorNextLine(param_or(params, 0, 1))],
        b'F' => vec![Action::CursorPreviousLine(param_or(params, 0, 1))],
        b'G' => vec![Action::CursorColumn(param_or(params, 0, 1))],
        b'H' | b'f' => vec![Action::CursorPosition {
            row: param_or(params, 0, 1),
            col: param_or(params, 1, 1),
        }],
        b'L' => vec![Action::InsertLines(param_or(params, 0, 1))],
        b'M' => vec![Action::DeleteLines(param_or(params, 0, 1))],
        b'P' => vec![Action::DeleteCharacters(param_or(params, 0, 1))],
        b'd' => vec![Action::VerticalPosition(param_or(params, 0, 1))],
        b'@' => vec![Action::InsertCharacters(param_or(params, 0, 1))],
        b'J' => vec![Action::EraseDisplay(param_or(params, 0, 0))],
        b'K' => vec![Action::EraseLine(param_or(params, 0, 0))],
        b'S' => vec![Action::ScrollUp(param_or(params, 0, 1))],
        b'T' => vec![Action::ScrollDown(param_or(params, 0, 1))],
        b'Z' => vec![Action::BackTab(param_or(params, 0, 1))],
        b'X' => vec![Action::EraseCharacters(param_or(params, 0, 1))],
        b'g' => vec![Action::ClearTabStop(param_or(params, 0, 0))],
        b'r' => vec![Action::SetScrollRegion {
            top: param_or(params, 0, 1),
            bottom: params.get(1).copied().unwrap_or(0),
        }],
        b'm' => vec![Action::SetGraphicsRendition(Action::parse_sgr(params))],
        b's' => vec![Action::SaveCursor],
        b'u' => vec![Action::RestoreCursor],
        b'h' => {
            let modes = normalized_modes(params, private_marker);
            if modes.is_empty() {
                Vec::new()
            } else {
                vec![Action::SetModes {
                    private: private_marker == Some(b'?'),
                    modes,
                }]
            }
        }
        b'l' => {
            let modes = normalized_modes(params, private_marker);
            if modes.is_empty() {
                Vec::new()
            } else {
                vec![Action::ResetModes {
                    private: private_marker == Some(b'?'),
                    modes,
                }]
            }
        }
        _ => Vec::new(),
    }
}

fn param_or(params: &[u16], index: usize, default: u16) -> u16 {
    match params.get(index).copied() {
        Some(0) | None => default,
        Some(value) => value,
    }
}

fn normalized_modes(params: &[u16], private_marker: Option<u8>) -> Vec<u16> {
    if private_marker.is_some() && private_marker != Some(b'?') {
        return Vec::new();
    }

    if private_marker.is_none() && params.is_empty() {
        return Vec::new();
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
        assert_eq!(parse_csi(&[], None, b'Z'), vec![Action::BackTab(1)]);
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
            vec![Action::SetGraphicsRendition(vec![
                GraphicsRendition::Foreground(Color::Indexed(200),)
            ])]
        );
    }

    #[test]
    fn csi_rejects_non_dec_private_markers_for_modes() {
        assert!(parse_csi(&[4], Some(b'>'), b'h').is_empty());
        assert!(parse_csi(&[25], Some(b'<'), b'l').is_empty());
    }
}
