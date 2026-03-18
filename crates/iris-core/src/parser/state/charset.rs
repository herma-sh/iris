use super::{Action, Charset, Parser, ParserState};

impl Parser {
    pub(super) fn parse_escape_charset(&mut self, slot: usize, byte: u8) -> Vec<Action> {
        if let Some(actions) = self.parse_embedded_control(byte) {
            return actions;
        }

        self.state = ParserState::Ground;
        if let Some(charset) = Charset::from_designator(byte) {
            self.charsets[slot] = charset;
        }
        Vec::new()
    }

    pub(super) fn handle_charset_shift(&mut self, byte: u8) -> bool {
        match byte {
            0x0e => {
                self.active_charset = 1;
                true
            }
            0x0f => {
                self.active_charset = 0;
                true
            }
            _ => false,
        }
    }

    pub(super) fn translate_printable_byte(&mut self, byte: u8) -> char {
        let charset = self
            .single_shift_charset
            .take()
            .unwrap_or(self.active_charset);
        self.charsets[charset].translate(byte)
    }
}

impl Charset {
    fn from_designator(byte: u8) -> Option<Self> {
        match byte {
            b'0' => Some(Self::DecSpecial),
            b'A' => Some(Self::Uk),
            b'B' => Some(Self::Ascii),
            _ => None,
        }
    }

    fn translate(self, byte: u8) -> char {
        match self {
            Self::Ascii => char::from(byte),
            Self::Uk => {
                if byte == b'#' {
                    '\u{00a3}'
                } else {
                    char::from(byte)
                }
            }
            Self::DecSpecial => translate_dec_special(byte),
        }
    }
}

fn translate_dec_special(byte: u8) -> char {
    match byte {
        b'j' => '\u{2518}',
        b'k' => '\u{2510}',
        b'l' => '\u{250c}',
        b'm' => '\u{2514}',
        b'n' => '\u{253c}',
        b'q' => '\u{2500}',
        b't' => '\u{251c}',
        b'u' => '\u{2524}',
        b'v' => '\u{2534}',
        b'w' => '\u{252c}',
        b'x' => '\u{2502}',
        _ => char::from(byte),
    }
}
