use super::{parse_control, parse_csi, Action, Parser, ParserState};

impl Parser {
    pub(super) fn parse_csi_entry(&mut self, byte: u8) -> Vec<Action> {
        if let Some(action) = parse_control(byte) {
            return vec![action];
        }

        match byte {
            0x1b => {
                self.state = ParserState::Escape;
                Vec::new()
            }
            b'?' | b'>' | b'<' | b'=' => {
                self.private_marker = Some(byte);
                self.state = ParserState::CsiParam;
                Vec::new()
            }
            b'0'..=b'9' => {
                self.current_param = Some((byte - b'0') as u16);
                self.state = ParserState::CsiParam;
                Vec::new()
            }
            b';' => {
                self.push_param(0);
                self.state = ParserState::CsiParam;
                Vec::new()
            }
            0x40..=0x7e => {
                self.state = ParserState::Ground;
                parse_csi(&[], self.private_marker.take(), byte)
            }
            _ => {
                self.reset();
                Vec::new()
            }
        }
    }

    pub(super) fn parse_csi_param(&mut self, byte: u8) -> Vec<Action> {
        if let Some(action) = parse_control(byte) {
            return vec![action];
        }

        match byte {
            0x1b => {
                self.state = ParserState::Escape;
                Vec::new()
            }
            b'0'..=b'9' => {
                let digit = (byte - b'0') as u16;
                let next = self
                    .current_param
                    .unwrap_or(0)
                    .saturating_mul(10)
                    .saturating_add(digit);
                self.current_param = Some(next);
                Vec::new()
            }
            b';' => {
                let current_param = self.current_param.take().unwrap_or(0);
                self.push_param(current_param);
                Vec::new()
            }
            0x40..=0x7e => {
                let current_param = self.current_param.take().unwrap_or(0);
                self.push_param(current_param);
                let params = std::mem::replace(
                    &mut self.params,
                    Vec::with_capacity(self.config.max_params.min(16)),
                );
                let private_marker = self.private_marker.take();
                self.state = ParserState::Ground;
                self.current_param = None;
                parse_csi(&params, private_marker, byte)
            }
            _ => {
                self.reset();
                Vec::new()
            }
        }
    }

    pub(super) fn push_param(&mut self, value: u16) {
        if self.params.len() < self.config.max_params {
            self.params.push(value);
        }
    }
}
