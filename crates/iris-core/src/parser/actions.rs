use crate::cell::Color;

/// Terminal operations emitted by the parser.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// Print a visible character.
    Print(char),
    /// Ring the terminal bell.
    Bell,
    /// Move the cursor one cell left.
    Backspace,
    /// Advance to the next tab stop.
    Tab,
    /// Move to the next line.
    LineFeed,
    /// Move to the next line and reset to column zero.
    VerticalTab,
    /// Move to the next line and reset to column zero.
    FormFeed,
    /// Return the cursor to column zero.
    CarriageReturn,
    /// Move down by one row, scrolling if needed.
    Index,
    /// Move down by one row and return to column zero.
    NextLine,
    /// Move up by one row, scrolling down if needed.
    ReverseIndex,
    /// Save the current cursor position.
    SaveCursor,
    /// Restore the saved cursor position.
    RestoreCursor,
    /// Move the cursor up by `count` rows.
    CursorUp(u16),
    /// Move the cursor down by `count` rows.
    CursorDown(u16),
    /// Move the cursor forward by `count` columns.
    CursorForward(u16),
    /// Move the cursor backward by `count` columns.
    CursorBack(u16),
    /// Move the cursor down by `count` rows and return to column zero.
    CursorNextLine(u16),
    /// Move the cursor up by `count` rows and return to column zero.
    CursorPreviousLine(u16),
    /// Move the cursor to a one-based column.
    CursorColumn(u16),
    /// Move the cursor to a one-based row and column.
    CursorPosition { row: u16, col: u16 },
    /// Move the cursor to a one-based row.
    VerticalPosition(u16),
    /// Erase visible content in the display.
    EraseDisplay(u16),
    /// Erase visible content in the current row.
    EraseLine(u16),
    /// Erase characters starting at the current cursor.
    EraseCharacters(u16),
    /// Apply SGR attributes.
    SetGraphicsRendition(Vec<GraphicsRendition>),
    /// Enable ANSI or DEC terminal modes.
    SetModes(Vec<u16>),
    /// Disable ANSI or DEC terminal modes.
    ResetModes(Vec<u16>),
}

/// A single SGR attribute change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphicsRendition {
    /// Reset all attributes to defaults.
    Reset,
    /// Toggle bold text.
    Bold(bool),
    /// Toggle faint text.
    Dim(bool),
    /// Toggle italic text.
    Italic(bool),
    /// Toggle underline.
    Underline(bool),
    /// Toggle blink.
    Blink(bool),
    /// Toggle inverse video.
    Inverse(bool),
    /// Toggle hidden text.
    Hidden(bool),
    /// Toggle strikethrough.
    Strikethrough(bool),
    /// Set foreground color.
    Foreground(Color),
    /// Set background color.
    Background(Color),
}

impl Action {
    /// Parses SGR parameters into a compact attribute update list.
    #[must_use]
    pub fn parse_sgr(params: &[u16]) -> Vec<GraphicsRendition> {
        if params.is_empty() {
            return vec![GraphicsRendition::Reset];
        }

        let mut renditions = Vec::new();
        let mut index = 0;

        while index < params.len() {
            match params[index] {
                0 => renditions.push(GraphicsRendition::Reset),
                1 => renditions.push(GraphicsRendition::Bold(true)),
                2 => renditions.push(GraphicsRendition::Dim(true)),
                3 => renditions.push(GraphicsRendition::Italic(true)),
                4 => renditions.push(GraphicsRendition::Underline(true)),
                5 => renditions.push(GraphicsRendition::Blink(true)),
                7 => renditions.push(GraphicsRendition::Inverse(true)),
                8 => renditions.push(GraphicsRendition::Hidden(true)),
                9 => renditions.push(GraphicsRendition::Strikethrough(true)),
                22 => {
                    renditions.push(GraphicsRendition::Bold(false));
                    renditions.push(GraphicsRendition::Dim(false));
                }
                23 => renditions.push(GraphicsRendition::Italic(false)),
                24 => renditions.push(GraphicsRendition::Underline(false)),
                25 => renditions.push(GraphicsRendition::Blink(false)),
                27 => renditions.push(GraphicsRendition::Inverse(false)),
                28 => renditions.push(GraphicsRendition::Hidden(false)),
                29 => renditions.push(GraphicsRendition::Strikethrough(false)),
                30..=37 => {
                    renditions.push(GraphicsRendition::Foreground(Color::Ansi(
                        (params[index] - 30) as u8,
                    )));
                }
                39 => renditions.push(GraphicsRendition::Foreground(Color::Default)),
                40..=47 => {
                    renditions.push(GraphicsRendition::Background(Color::Ansi(
                        (params[index] - 40) as u8,
                    )));
                }
                49 => renditions.push(GraphicsRendition::Background(Color::Default)),
                38 => {
                    if let Some((color, consumed)) = parse_extended_color(&params[(index + 1)..]) {
                        renditions.push(GraphicsRendition::Foreground(color));
                        index += consumed;
                    }
                }
                48 => {
                    if let Some((color, consumed)) = parse_extended_color(&params[(index + 1)..]) {
                        renditions.push(GraphicsRendition::Background(color));
                        index += consumed;
                    }
                }
                _ => {}
            }

            index += 1;
        }

        if renditions.is_empty() {
            renditions.push(GraphicsRendition::Reset);
        }

        renditions
    }
}

fn parse_extended_color(params: &[u16]) -> Option<(Color, usize)> {
    match params {
        [5, value, ..] => Some((Color::Indexed((*value).min(u8::MAX as u16) as u8), 2)),
        [2, red, green, blue, ..] => Some((
            Color::Rgb {
                r: (*red).min(u8::MAX as u16) as u8,
                g: (*green).min(u8::MAX as u16) as u8,
                b: (*blue).min(u8::MAX as u16) as u8,
            },
            4,
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{Action, GraphicsRendition};
    use crate::cell::Color;

    #[test]
    fn sgr_defaults_to_reset_when_empty() {
        assert_eq!(Action::parse_sgr(&[]), vec![GraphicsRendition::Reset]);
    }

    #[test]
    fn sgr_parses_truecolor_and_reset_codes() {
        assert_eq!(
            Action::parse_sgr(&[1, 38, 2, 1, 2, 3, 49, 22]),
            vec![
                GraphicsRendition::Bold(true),
                GraphicsRendition::Foreground(Color::Rgb { r: 1, g: 2, b: 3 }),
                GraphicsRendition::Background(Color::Default),
                GraphicsRendition::Bold(false),
                GraphicsRendition::Dim(false),
            ]
        );
    }
}
