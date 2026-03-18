use super::{parse_csi, Action, Parser, ParserState};

impl Parser {
    pub(super) fn parse_csi_entry(&mut self, byte: u8, actions: &mut Vec<Action>) {
        if self.parse_embedded_control(byte, actions) {
            return;
        }

        match byte {
            0x1b => {
                self.state = ParserState::Escape;
            }
            b'?' | b'>' | b'<' | b'=' => {
                self.private_marker = Some(byte);
                self.state = ParserState::CsiParam;
            }
            b'0'..=b'9' => {
                self.current_param = Some((byte - b'0') as u16);
                self.state = ParserState::CsiParam;
            }
            b';' => {
                self.push_param(0);
                self.state = ParserState::CsiParam;
            }
            0x20..=0x2f => {
                self.intermediates.clear();
                self.intermediates.push(byte);
                self.state = ParserState::CsiIntermediate;
            }
            0x40..=0x7e => {
                self.state = ParserState::Ground;
                let private_marker = self.private_marker.take();
                if byte == b'b' && private_marker.is_none() {
                    self.repeat_last_printed_into(1, actions);
                } else {
                    actions.extend(parse_csi(&[], private_marker, byte));
                }
            }
            _ => {
                self.reset();
            }
        }
    }

    pub(super) fn parse_csi_param(&mut self, byte: u8, actions: &mut Vec<Action>) {
        if self.parse_embedded_control(byte, actions) {
            return;
        }

        match byte {
            0x1b => {
                self.state = ParserState::Escape;
            }
            b'0'..=b'9' => {
                let digit = (byte - b'0') as u16;
                let next = self
                    .current_param
                    .unwrap_or(0)
                    .saturating_mul(10)
                    .saturating_add(digit);
                self.current_param = Some(next);
            }
            b';' => {
                let current_param = self.current_param.take().unwrap_or(0);
                self.push_param(current_param);
            }
            0x20..=0x2f => {
                let current_param = self.current_param.take().unwrap_or(0);
                self.push_param(current_param);
                self.intermediates.clear();
                self.intermediates.push(byte);
                self.state = ParserState::CsiIntermediate;
            }
            0x40..=0x7e => {
                let current_param = self.current_param.take().unwrap_or(0);
                self.push_param(current_param);
                let private_marker = self.private_marker.take();
                self.state = ParserState::Ground;
                self.current_param = None;
                if byte == b'b' && private_marker.is_none() {
                    let count = self.params.first().copied().unwrap_or(1);
                    self.repeat_last_printed_into(count, actions);
                } else {
                    actions.extend(parse_csi(&self.params, private_marker, byte));
                }
                self.params.clear();
                self.intermediates.clear();
            }
            _ => {
                self.reset();
            }
        }
    }

    pub(super) fn parse_csi_intermediate(&mut self, byte: u8, actions: &mut Vec<Action>) {
        if self.parse_embedded_control(byte, actions) {
            return;
        }

        match byte {
            0x1b => {
                self.state = ParserState::Escape;
            }
            0x20..=0x2f => {
                if self.intermediates.len() < 4 {
                    self.intermediates.push(byte);
                }
            }
            0x40..=0x7e => {
                self.state = ParserState::Ground;
                self.intermediates.clear();
                self.params.clear();
                self.current_param = None;
                self.private_marker = None;
            }
            _ => {
                self.reset();
            }
        }
    }

    pub(super) fn push_param(&mut self, value: u16) {
        if self.params.len() < self.config.max_params {
            self.params.push(value);
        }
    }
}
