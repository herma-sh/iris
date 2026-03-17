use super::Action;

/// Returns the parser action for a C0 control character, if supported.
#[must_use]
pub fn parse_control(byte: u8) -> Option<Action> {
    match byte {
        0x07 => Some(Action::Bell),
        0x08 => Some(Action::Backspace),
        0x09 => Some(Action::Tab),
        0x0a => Some(Action::LineFeed),
        0x0b => Some(Action::VerticalTab),
        0x0c => Some(Action::FormFeed),
        0x0d => Some(Action::CarriageReturn),
        _ => None,
    }
}
