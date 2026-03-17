use super::control::parse_control;
use super::csi::parse_csi;
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
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self { max_params: 16 }
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
            ParserState::CsiEntry => self.parse_csi_entry(byte),
            ParserState::CsiParam => self.parse_csi_param(byte),
        }
    }

    fn parse_ground(&mut self, byte: u8) -> Vec<Action> {
        if let Some(action) = parse_control(byte) {
            return vec![action];
        }

        match byte {
            0x1b => {
                self.state = ParserState::Escape;
                Vec::new()
            }
            0x20..=0x7e => vec![Action::Print(char::from(byte))],
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
                let params = self.params.clone();
                let private_marker = self.private_marker.take();
                self.reset();
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
}

#[cfg(test)]
mod tests {
    use super::{Parser, ParserState};
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
            vec![Action::ResetModes(vec![25])]
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
}
