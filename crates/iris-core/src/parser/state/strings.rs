use super::{parse_dcs, parse_osc, Action, Parser, ParserState};

impl Parser {
    pub(super) fn parse_osc_string(&mut self, byte: u8, actions: &mut Vec<Action>) {
        if byte != 0x07 && byte != 0x1b && self.parse_embedded_control(byte, actions) {
            return;
        }

        match byte {
            0x07 => self.finish_osc(actions),
            0x1b => {
                self.state = ParserState::OscEscape;
            }
            _ => {
                if self.osc_buffer.len() >= self.config.max_osc_bytes {
                    self.reset();
                    self.parse_ground(byte, actions);
                    return;
                }

                self.osc_buffer.push(byte);
            }
        }
    }

    pub(super) fn parse_osc_escape(&mut self, byte: u8, actions: &mut Vec<Action>) {
        if byte == b'\\' {
            self.finish_osc(actions);
            return;
        }

        if self.osc_buffer.len().saturating_add(2) > self.config.max_osc_bytes {
            // Drop the truncated OSC payload, including the pending ESC, and
            // resume parsing from the current byte in ground state.
            self.reset();
            self.parse_ground(byte, actions);
            return;
        }

        self.osc_buffer.push(0x1b);
        self.osc_buffer.push(byte);
        self.state = ParserState::OscString;
    }

    pub(super) fn parse_dcs_string(&mut self, byte: u8, actions: &mut Vec<Action>) {
        if byte != 0x1b && self.parse_embedded_control(byte, actions) {
            return;
        }

        match byte {
            0x1b => {
                self.state = ParserState::DcsEscape;
            }
            _ => {
                if self.dcs_buffer.len() >= self.config.max_dcs_bytes {
                    self.reset();
                    self.parse_ground(byte, actions);
                    return;
                }

                self.dcs_buffer.push(byte);
            }
        }
    }

    pub(super) fn parse_dcs_escape(&mut self, byte: u8, actions: &mut Vec<Action>) {
        if byte == b'\\' {
            self.finish_dcs(actions);
            return;
        }

        if self.dcs_buffer.len().saturating_add(2) > self.config.max_dcs_bytes {
            // Drop the truncated DCS payload, including the pending ESC, and
            // resume parsing from the current byte in ground state.
            self.reset();
            self.parse_ground(byte, actions);
            return;
        }

        self.dcs_buffer.push(0x1b);
        self.dcs_buffer.push(byte);
        self.state = ParserState::DcsString;
    }

    pub(super) fn parse_ignored_string(&mut self, byte: u8, actions: &mut Vec<Action>) {
        if byte != 0x1b && self.parse_embedded_control(byte, actions) {
            return;
        }

        match byte {
            0x1b => {
                self.state = ParserState::IgnoreStringEscape;
            }
            _ => {
                if self.ignored_string_len >= self.config.max_ignored_string_bytes {
                    self.reset();
                    self.parse_ground(byte, actions);
                    return;
                }

                self.ignored_string_len += 1;
            }
        }
    }

    pub(super) fn parse_ignored_string_escape(&mut self, byte: u8, actions: &mut Vec<Action>) {
        if byte == b'\\' {
            self.finish_ignored_string();
            return;
        }

        if self.ignored_string_len.saturating_add(2) > self.config.max_ignored_string_bytes {
            // Drop the truncated ignored string, including the pending ESC, and
            // resume parsing from the current byte in ground state.
            self.reset();
            self.parse_ground(byte, actions);
            return;
        }

        self.ignored_string_len += 2;
        self.state = ParserState::IgnoreString;
    }

    pub(super) fn finish_osc(&mut self, actions: &mut Vec<Action>) {
        self.state = ParserState::Ground;
        self.current_param = None;
        self.private_marker = None;
        self.reset_utf8();
        actions.extend(parse_osc(&self.osc_buffer));
        self.osc_buffer.clear();
    }

    pub(super) fn finish_dcs(&mut self, actions: &mut Vec<Action>) {
        self.state = ParserState::Ground;
        self.current_param = None;
        self.private_marker = None;
        self.reset_utf8();
        actions.extend(parse_dcs(&self.dcs_buffer));
        self.dcs_buffer.clear();
    }

    pub(super) fn finish_ignored_string(&mut self) {
        self.state = ParserState::Ground;
        self.current_param = None;
        self.private_marker = None;
        self.ignored_string_len = 0;
        self.reset_utf8();
    }
}
