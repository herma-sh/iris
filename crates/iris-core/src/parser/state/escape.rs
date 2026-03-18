use super::{Action, Parser, ParserState};

impl Parser {
    pub(super) fn parse_escape(&mut self, byte: u8, actions: &mut Vec<Action>) {
        if self.parse_embedded_control(byte, actions) {
            return;
        }

        self.state = ParserState::Ground;
        match byte {
            0x1b => {
                self.state = ParserState::Escape;
            }
            b'(' => {
                self.state = ParserState::EscapeCharset(0);
            }
            b')' => {
                self.state = ParserState::EscapeCharset(1);
            }
            b'*' => {
                self.state = ParserState::EscapeCharset(2);
            }
            b'+' => {
                self.state = ParserState::EscapeCharset(3);
            }
            b'[' => {
                self.state = ParserState::CsiEntry;
                self.params.clear();
                self.intermediates.clear();
                self.current_param = None;
                self.private_marker = None;
            }
            b']' => {
                self.state = ParserState::OscString;
                self.osc_buffer.clear();
            }
            b'P' => {
                self.state = ParserState::DcsString;
                self.dcs_buffer.clear();
            }
            b'X' | b'^' | b'_' => {
                self.state = ParserState::IgnoreString;
                self.ignored_string_len = 0;
            }
            b'D' => actions.push(Action::Index),
            b'E' => actions.push(Action::NextLine),
            b'H' => actions.push(Action::SetTabStop),
            b'M' => actions.push(Action::ReverseIndex),
            b'N' => {
                self.single_shift_charset = Some(2);
            }
            b'O' => {
                self.single_shift_charset = Some(3);
            }
            b'Z' => actions.push(Action::DeviceAttributes),
            b'7' => actions.push(Action::SaveCursor),
            b'8' => actions.push(Action::RestoreCursor),
            b'=' => actions.push(Action::SetKeypadMode(true)),
            b'>' => actions.push(Action::SetKeypadMode(false)),
            b'c' => {
                self.reset_terminal_state();
                actions.push(Action::ResetTerminal);
            }
            _ => {}
        }
    }
}
