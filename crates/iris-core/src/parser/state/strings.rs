use super::{parse_dcs, parse_osc, Action, Parser, ParserState};

impl Parser {
    pub(super) fn parse_osc_string(&mut self, byte: u8) -> Vec<Action> {
        if byte != 0x07 && byte != 0x1b {
            if let Some(actions) = self.parse_embedded_control(byte) {
                return actions;
            }
        }

        match byte {
            0x07 => self.finish_osc(),
            0x1b => {
                self.state = ParserState::OscEscape;
                Vec::new()
            }
            _ => {
                if self.osc_buffer.len() >= self.config.max_osc_bytes {
                    self.reset();
                    return self.parse_ground(byte);
                }

                self.osc_buffer.push(byte);
                Vec::new()
            }
        }
    }

    pub(super) fn parse_osc_escape(&mut self, byte: u8) -> Vec<Action> {
        if byte == b'\\' {
            return self.finish_osc();
        }

        if self.osc_buffer.len().saturating_add(2) > self.config.max_osc_bytes {
            // Drop the truncated OSC payload, including the pending ESC, and
            // resume parsing from the current byte in ground state.
            self.reset();
            return self.parse_ground(byte);
        }

        self.osc_buffer.push(0x1b);
        self.osc_buffer.push(byte);
        self.state = ParserState::OscString;
        Vec::new()
    }

    pub(super) fn parse_dcs_string(&mut self, byte: u8) -> Vec<Action> {
        if byte != 0x1b {
            if let Some(actions) = self.parse_embedded_control(byte) {
                return actions;
            }
        }

        match byte {
            0x1b => {
                self.state = ParserState::DcsEscape;
                Vec::new()
            }
            _ => {
                if self.dcs_buffer.len() >= self.config.max_dcs_bytes {
                    self.reset();
                    return self.parse_ground(byte);
                }

                self.dcs_buffer.push(byte);
                Vec::new()
            }
        }
    }

    pub(super) fn parse_dcs_escape(&mut self, byte: u8) -> Vec<Action> {
        if byte == b'\\' {
            return self.finish_dcs();
        }

        if self.dcs_buffer.len().saturating_add(2) > self.config.max_dcs_bytes {
            // Drop the truncated DCS payload, including the pending ESC, and
            // resume parsing from the current byte in ground state.
            self.reset();
            return self.parse_ground(byte);
        }

        self.dcs_buffer.push(0x1b);
        self.dcs_buffer.push(byte);
        self.state = ParserState::DcsString;
        Vec::new()
    }

    pub(super) fn parse_ignored_string(&mut self, byte: u8) -> Vec<Action> {
        if byte != 0x1b {
            if let Some(actions) = self.parse_embedded_control(byte) {
                return actions;
            }
        }

        match byte {
            0x1b => {
                self.state = ParserState::IgnoreStringEscape;
                Vec::new()
            }
            _ => {
                if self.ignored_string_len >= self.config.max_ignored_string_bytes {
                    self.reset();
                    return self.parse_ground(byte);
                }

                self.ignored_string_len += 1;
                Vec::new()
            }
        }
    }

    pub(super) fn parse_ignored_string_escape(&mut self, byte: u8) -> Vec<Action> {
        if byte == b'\\' {
            self.finish_ignored_string();
            return Vec::new();
        }

        if self.ignored_string_len.saturating_add(2) > self.config.max_ignored_string_bytes {
            // Drop the truncated ignored string, including the pending ESC, and
            // resume parsing from the current byte in ground state.
            self.reset();
            return self.parse_ground(byte);
        }

        self.ignored_string_len += 2;
        self.state = ParserState::IgnoreString;
        Vec::new()
    }

    pub(super) fn finish_osc(&mut self) -> Vec<Action> {
        let payload = std::mem::take(&mut self.osc_buffer);
        self.state = ParserState::Ground;
        self.current_param = None;
        self.private_marker = None;
        self.reset_utf8();
        parse_osc(&payload)
    }

    pub(super) fn finish_dcs(&mut self) -> Vec<Action> {
        let payload = std::mem::take(&mut self.dcs_buffer);
        self.state = ParserState::Ground;
        self.current_param = None;
        self.private_marker = None;
        self.reset_utf8();
        parse_dcs(&payload)
    }

    pub(super) fn finish_ignored_string(&mut self) {
        self.state = ParserState::Ground;
        self.current_param = None;
        self.private_marker = None;
        self.ignored_string_len = 0;
        self.reset_utf8();
    }
}
