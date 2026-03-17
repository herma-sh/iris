use super::control::parse_control;
use super::csi::parse_csi;
use super::osc::parse_osc;
use super::Action;
use crate::error::Result;
use crate::terminal::Terminal;

/// Stateful ANSI/VT parser states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParserState {
    /// Normal character processing.
    Ground,
    /// `ESC` has been seen.
    Escape,
    /// `OSC` string collection.
    OscString,
    /// `OSC` escape terminator after `ESC`.
    OscEscape,
    /// `CSI` entry after `ESC [`.
    CsiEntry,
    /// `CSI` parameter collection.
    CsiParam,
}

/// Parser bounds used to avoid unbounded sequence accumulation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParserConfig {
    /// Maximum number of CSI parameters retained.
    pub max_params: usize,
    /// Maximum OSC payload size retained.
    pub max_osc_bytes: usize,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            max_params: 16,
            max_osc_bytes: 4096,
        }
    }
}

/// Stateful ANSI/VT parser for terminal byte streams.
#[derive(Clone, Debug)]
pub struct Parser {
    state: ParserState,
    config: ParserConfig,
    params: Vec<u16>,
    current_param: Option<u16>,
    private_marker: Option<u8>,
    osc_buffer: Vec<u8>,
    utf8_buffer: [u8; 4],
    utf8_len: usize,
    utf8_expected: usize,
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser {
    /// Creates a parser in the ground state.
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(ParserConfig::default())
    }

    /// Creates a parser with custom bounded configuration.
    #[must_use]
    pub fn with_config(config: ParserConfig) -> Self {
        Self {
            state: ParserState::Ground,
            params: Vec::with_capacity(config.max_params.min(16)),
            current_param: None,
            private_marker: None,
            osc_buffer: Vec::with_capacity(config.max_osc_bytes.min(256)),
            utf8_buffer: [0; 4],
            utf8_len: 0,
            utf8_expected: 0,
            config,
        }
    }

    /// Returns the current parser state.
    #[must_use]
    pub const fn state(&self) -> ParserState {
        self.state
    }

    /// Resets parser state and buffered sequence data.
    pub fn reset(&mut self) {
        self.state = ParserState::Ground;
        self.params.clear();
        self.current_param = None;
        self.private_marker = None;
        self.osc_buffer.clear();
        self.reset_utf8();
    }

    /// Parses input bytes into terminal actions.
    #[must_use]
    pub fn parse(&mut self, input: &[u8]) -> Vec<Action> {
        let mut actions = Vec::new();
        for &byte in input {
            actions.extend(self.parse_byte(byte));
        }
        actions
    }

    /// Consumes input bytes and applies the resulting actions to the terminal.
    pub fn advance(&mut self, terminal: &mut Terminal, input: &[u8]) -> Result<()> {
        for action in self.parse(input) {
            terminal.apply_action(action)?;
        }

        Ok(())
    }

    fn parse_byte(&mut self, byte: u8) -> Vec<Action> {
        match self.state {
            ParserState::Ground => self.parse_ground(byte),
            ParserState::Escape => self.parse_escape(byte),
            ParserState::OscString => self.parse_osc_string(byte),
            ParserState::OscEscape => self.parse_osc_escape(byte),
            ParserState::CsiEntry => self.parse_csi_entry(byte),
            ParserState::CsiParam => self.parse_csi_param(byte),
        }
    }

    fn parse_ground(&mut self, byte: u8) -> Vec<Action> {
        if self.utf8_expected > 0 {
            return self.parse_utf8_continuation(byte);
        }

        if let Some(action) = parse_control(byte) {
            return vec![action];
        }

        match byte {
            0x1b => {
                self.state = ParserState::Escape;
                Vec::new()
            }
            0x20..=0x7e => vec![Action::Print(char::from(byte))],
            0x80..=0xff => self.parse_utf8_lead(byte),
            _ => Vec::new(),
        }
    }

    fn parse_escape(&mut self, byte: u8) -> Vec<Action> {
        if let Some(action) = parse_control(byte) {
            return vec![action];
        }

        self.state = ParserState::Ground;
        match byte {
            0x1b => {
                self.state = ParserState::Escape;
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
            b'D' => vec![Action::Index],
            b'E' => vec![Action::NextLine],
            b'M' => vec![Action::ReverseIndex],
            b'7' => vec![Action::SaveCursor],
            b'8' => vec![Action::RestoreCursor],
            _ => Vec::new(),
        }
    }

    fn parse_csi_entry(&mut self, byte: u8) -> Vec<Action> {
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

    fn parse_osc_string(&mut self, byte: u8) -> Vec<Action> {
        match byte {
            0x07 => self.finish_osc(),
            0x1b => {
                self.state = ParserState::OscEscape;
                Vec::new()
            }
            _ => {
                if self.osc_buffer.len() >= self.config.max_osc_bytes {
                    self.reset();
                    return Vec::new();
                }

                self.osc_buffer.push(byte);
                Vec::new()
            }
        }
    }

    fn parse_osc_escape(&mut self, byte: u8) -> Vec<Action> {
        if byte == b'\\' {
            return self.finish_osc();
        }

        if self.osc_buffer.len() >= self.config.max_osc_bytes {
            self.reset();
            return self.parse_ground(byte);
        }

        self.osc_buffer.push(0x1b);
        self.osc_buffer.push(byte);
        self.state = ParserState::OscString;
        Vec::new()
    }

    fn parse_csi_param(&mut self, byte: u8) -> Vec<Action> {
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

    fn push_param(&mut self, value: u16) {
        if self.params.len() < self.config.max_params {
            self.params.push(value);
        }
    }

    fn parse_utf8_lead(&mut self, byte: u8) -> Vec<Action> {
        let expected = match utf8_sequence_len(byte) {
            Some(expected) => expected,
            None => return vec![Action::Print(char::REPLACEMENT_CHARACTER)],
        };

        self.utf8_buffer[0] = byte;
        self.utf8_len = 1;
        self.utf8_expected = expected;

        if expected == 1 {
            return self.finish_utf8_sequence();
        }

        Vec::new()
    }

    fn parse_utf8_continuation(&mut self, byte: u8) -> Vec<Action> {
        if !is_utf8_continuation(byte) {
            self.reset_utf8();
            let mut actions = vec![Action::Print(char::REPLACEMENT_CHARACTER)];
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

    fn finish_utf8_sequence(&mut self) -> Vec<Action> {
        let utf8_len = self.utf8_len;
        let character = std::str::from_utf8(&self.utf8_buffer[..utf8_len])
            .ok()
            .and_then(|text| text.chars().next())
            .unwrap_or(char::REPLACEMENT_CHARACTER);
        self.reset_utf8();
        vec![Action::Print(character)]
    }

    fn reset_utf8(&mut self) {
        self.utf8_len = 0;
        self.utf8_expected = 0;
    }

    fn finish_osc(&mut self) -> Vec<Action> {
        let payload = std::mem::take(&mut self.osc_buffer);
        self.state = ParserState::Ground;
        self.current_param = None;
        self.private_marker = None;
        self.reset_utf8();
        parse_osc(&payload)
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

#[cfg(test)]
mod tests {
    use super::{Parser, ParserConfig, ParserState};
    use crate::cell::{CellFlags, Color};
    use crate::parser::{Action, GraphicsRendition};

    #[test]
    fn parser_starts_in_ground_state() {
        assert_eq!(Parser::new().state(), ParserState::Ground);
    }

    #[test]
    fn parser_parses_printable_characters_and_escape_transitions() {
        let mut parser = Parser::new();
        assert_eq!(parser.parse(b"A"), vec![Action::Print('A')]);
        assert!(parser.parse(b"\x1b").is_empty());
        assert_eq!(parser.state(), ParserState::Escape);
    }

    #[test]
    fn parser_collects_csi_parameters_and_defaults() {
        let mut parser = Parser::new();
        assert_eq!(
            parser.parse(b"\x1b[12;24H"),
            vec![Action::CursorPosition { row: 12, col: 24 }]
        );
        assert_eq!(parser.state(), ParserState::Ground);

        let mut parser = Parser::new();
        assert_eq!(parser.parse(b"\x1b[A"), vec![Action::CursorUp(1)]);
    }

    #[test]
    fn parser_handles_private_modes_and_sgr() {
        let mut parser = Parser::new();
        assert_eq!(
            parser.parse(b"\x1b[?25l"),
            vec![Action::ResetModes {
                private: true,
                modes: vec![25],
            }]
        );
        assert_eq!(
            parser.parse(b"\x1b[1;31;48;5;240m"),
            vec![Action::SetGraphicsRendition(vec![
                GraphicsRendition::Bold(true),
                GraphicsRendition::Foreground(Color::Ansi(1)),
                GraphicsRendition::Background(Color::Indexed(240)),
            ])]
        );
    }

    #[test]
    fn parser_handles_escape_index_sequences() {
        let mut parser = Parser::new();
        assert_eq!(parser.parse(b"\x1bD"), vec![Action::Index]);
        assert_eq!(parser.parse(b"\x1bE"), vec![Action::NextLine]);
        assert_eq!(parser.parse(b"\x1bM"), vec![Action::ReverseIndex]);
    }

    #[test]
    fn parser_decodes_utf8_printable_characters() {
        let mut parser = Parser::new();
        assert_eq!(
            parser.parse("é中".as_bytes()),
            vec![Action::Print('é'), Action::Print('中')]
        );
    }

    #[test]
    fn parser_preserves_utf8_state_across_chunks() {
        let mut parser = Parser::new();
        assert!(parser.parse(&[0xe2, 0x82]).is_empty());
        assert_eq!(parser.parse(&[0xac]), vec![Action::Print('€')]);
    }

    #[test]
    fn parser_recovers_from_malformed_utf8_sequences() {
        let mut parser = Parser::new();
        assert_eq!(
            parser.parse(&[0xe2, b'A']),
            vec![
                Action::Print(char::REPLACEMENT_CHARACTER),
                Action::Print('A'),
            ]
        );
    }

    #[test]
    fn parser_handles_malformed_sequences_gracefully() {
        let mut parser = Parser::new();
        assert!(parser.parse(b"\x1b[12$").is_empty());
        assert_eq!(parser.state(), ParserState::Ground);
        assert_eq!(parser.parse(b"B"), vec![Action::Print('B')]);
    }

    #[test]
    fn parser_applies_actions_to_terminal() {
        let mut parser = Parser::new();
        let mut terminal = crate::terminal::Terminal::new(2, 8).unwrap();

        parser
            .advance(&mut terminal, b"\x1b[1;31mA\x1b[0m")
            .unwrap();

        let cell = terminal.grid.cell(0, 0).copied().unwrap();
        assert_eq!(cell.character, 'A');
        assert_eq!(cell.attrs.fg, Color::Ansi(1));
        assert!(cell.attrs.flags.contains(CellFlags::BOLD));
        assert_eq!(terminal.attrs.fg, Color::Default);
        assert!(!terminal.attrs.flags.contains(CellFlags::BOLD));
    }

    #[test]
    fn parser_parses_osc_window_title_with_bel_terminator() {
        let mut parser = Parser::new();
        assert_eq!(
            parser.parse(b"\x1b]2;Iris\x07"),
            vec![Action::SetWindowTitle("Iris".to_string())]
        );
    }

    #[test]
    fn parser_parses_osc_hyperlink_with_st_terminator() {
        let mut parser = Parser::new();
        assert_eq!(
            parser.parse(b"\x1b]8;id=prompt-1;https://example.com\x1b\\"),
            vec![Action::SetHyperlink {
                id: Some("prompt-1".to_string()),
                uri: "https://example.com".to_string(),
            }]
        );
    }

    #[test]
    fn parser_limits_osc_payload_growth() {
        let mut parser = Parser::with_config(ParserConfig {
            max_params: 16,
            max_osc_bytes: 4,
        });

        assert!(parser.parse(b"\x1b]2;h").is_empty());
        assert_eq!(parser.parse(b"ello"), vec![Action::Print('l'), Action::Print('o')]);
        assert_eq!(parser.state(), ParserState::Ground);
    }
}
