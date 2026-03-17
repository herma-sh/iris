use super::{parse_control, Action, Parser, ParserState};

impl Parser {
    pub(super) fn parse_escape(&mut self, byte: u8) -> Vec<Action> {
        if let Some(action) = parse_control(byte) {
            return vec![action];
        }

        self.state = ParserState::Ground;
        match byte {
            0x1b => {
                self.state = ParserState::Escape;
                Vec::new()
            }
            b'(' => {
                self.state = ParserState::EscapeCharset(0);
                Vec::new()
            }
            b')' => {
                self.state = ParserState::EscapeCharset(1);
                Vec::new()
            }
            b'[' => {
                self.state = ParserState::CsiEntry;
                self.params.clear();
                self.current_param = None;
                self.private_marker = None;
                Vec::new()
            }
            b']' => {
                self.state = ParserState::OscString;
                self.osc_buffer.clear();
                Vec::new()
            }
            b'P' => {
                self.state = ParserState::DcsString;
                self.dcs_buffer.clear();
                Vec::new()
            }
            b'X' | b'^' | b'_' => {
                self.state = ParserState::IgnoreString;
                self.ignored_string_len = 0;
                Vec::new()
            }
            b'D' => vec![Action::Index],
            b'E' => vec![Action::NextLine],
            b'M' => vec![Action::ReverseIndex],
            b'7' => vec![Action::SaveCursor],
            b'8' => vec![Action::RestoreCursor],
            _ => Vec::new(),
        }
    }
}
