use super::control::parse_control;
use super::csi::parse_csi;
use super::dcs::parse_dcs;
use super::osc::parse_osc;
use super::Action;
use crate::error::Result;
use crate::terminal::Terminal;

mod charset;
mod csi;
mod escape;
mod strings;
mod utf8;

#[cfg(test)]
mod tests;

/// Stateful ANSI/VT parser states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParserState {
    /// Normal character processing.
    Ground,
    /// `ESC` has been seen.
    Escape,
    /// `ESC (` / `ESC )` / `ESC *` / `ESC +` charset designation is pending.
    EscapeCharset(usize),
    /// `OSC` string collection.
    OscString,
    /// `OSC` escape terminator after `ESC`.
    OscEscape,
    /// `DCS` string collection.
    DcsString,
    /// `DCS` escape terminator after `ESC`.
    DcsEscape,
    /// Ignored SOS/PM/APC string collection.
    IgnoreString,
    /// Ignored SOS/PM/APC escape terminator after `ESC`.
    IgnoreStringEscape,
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
    /// Maximum DCS payload size retained.
    pub max_dcs_bytes: usize,
    /// Maximum SOS/PM/APC payload size skipped before resetting.
    pub max_ignored_string_bytes: usize,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            max_params: 16,
            max_osc_bytes: 4096,
            max_dcs_bytes: 4096,
            max_ignored_string_bytes: 4096,
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
    charsets: [Charset; 4],
    active_charset: usize,
    single_shift_charset: Option<usize>,
    osc_buffer: Vec<u8>,
    dcs_buffer: Vec<u8>,
    ignored_string_len: usize,
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
            charsets: [Charset::Ascii; 4],
            active_charset: 0,
            single_shift_charset: None,
            osc_buffer: Vec::with_capacity(config.max_osc_bytes.min(256)),
            dcs_buffer: Vec::with_capacity(config.max_dcs_bytes.min(256)),
            ignored_string_len: 0,
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
        self.dcs_buffer.clear();
        self.ignored_string_len = 0;
        self.single_shift_charset = None;
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
            ParserState::EscapeCharset(slot) => self.parse_escape_charset(slot, byte),
            ParserState::OscString => self.parse_osc_string(byte),
            ParserState::OscEscape => self.parse_osc_escape(byte),
            ParserState::DcsString => self.parse_dcs_string(byte),
            ParserState::DcsEscape => self.parse_dcs_escape(byte),
            ParserState::IgnoreString => self.parse_ignored_string(byte),
            ParserState::IgnoreStringEscape => self.parse_ignored_string_escape(byte),
            ParserState::CsiEntry => self.parse_csi_entry(byte),
            ParserState::CsiParam => self.parse_csi_param(byte),
        }
    }

    fn parse_ground(&mut self, byte: u8) -> Vec<Action> {
        if self.utf8_expected > 0 {
            return self.parse_utf8_continuation(byte);
        }

        if self.handle_charset_shift(byte) {
            return Vec::new();
        }

        if let Some(action) = parse_control(byte) {
            return vec![action];
        }

        match byte {
            0x1b => {
                self.state = ParserState::Escape;
                Vec::new()
            }
            0x20..=0x7e => vec![Action::Print(self.translate_printable_byte(byte))],
            0x80..=0xff => self.parse_utf8_lead(byte),
            _ => Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Charset {
    Ascii,
    Uk,
    DecSpecial,
}
