use super::{Action, Parser};

impl Parser {
    pub(super) fn parse_utf8_lead(&mut self, byte: u8) -> Vec<Action> {
        let expected = match utf8_sequence_len(byte) {
            Some(expected) => expected,
            None => return vec![self.print_action(char::REPLACEMENT_CHARACTER)],
        };

        self.utf8_buffer[0] = byte;
        self.utf8_len = 1;
        self.utf8_expected = expected;

        if expected == 1 {
            return self.finish_utf8_sequence();
        }

        Vec::new()
    }

    pub(super) fn parse_utf8_continuation(&mut self, byte: u8) -> Vec<Action> {
        if !is_utf8_continuation(byte) {
            self.reset_utf8();
            let mut actions = vec![self.print_action(char::REPLACEMENT_CHARACTER)];
            actions.extend(self.parse_ground(byte));
            return actions;
        }

        self.utf8_buffer[self.utf8_len] = byte;
        self.utf8_len += 1;

        if self.utf8_len == self.utf8_expected {
            return self.finish_utf8_sequence();
        }

        Vec::new()
    }

    pub(super) fn finish_utf8_sequence(&mut self) -> Vec<Action> {
        let utf8_len = self.utf8_len;
        let character = std::str::from_utf8(&self.utf8_buffer[..utf8_len])
            .ok()
            .and_then(|text| text.chars().next())
            .unwrap_or(char::REPLACEMENT_CHARACTER);
        self.reset_utf8();
        vec![self.print_action(character)]
    }

    pub(super) fn reset_utf8(&mut self) {
        self.utf8_len = 0;
        self.utf8_expected = 0;
    }
}

fn utf8_sequence_len(byte: u8) -> Option<usize> {
    match byte {
        0x00..=0x7f => Some(1),
        0xc2..=0xdf => Some(2),
        0xe0..=0xef => Some(3),
        0xf0..=0xf4 => Some(4),
        _ => None,
    }
}

fn is_utf8_continuation(byte: u8) -> bool {
    matches!(byte, 0x80..=0xbf)
}
