/// Terminal modes required by the core terminal state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerminalModes {
    /// Whether origin mode is active.
    pub origin: bool,
    /// Whether wrapping is enabled at the end of a row.
    pub wrap: bool,
    /// Whether insert mode is active.
    pub insert: bool,
    /// Whether line feed also performs carriage return.
    pub newline: bool,
    /// Whether keypad application mode is active.
    pub keypad: bool,
    /// Whether the cursor is visible.
    pub cursor_visible: bool,
    /// Whether cursor blinking is enabled.
    pub cursor_blink: bool,
    /// Whether the alternate screen buffer is active.
    pub alternate_screen: bool,
    /// Whether bracketed paste mode is active.
    pub bracketed_paste: bool,
    /// Whether focus events should be reported.
    pub focus_event: bool,
    /// Whether synchronized output mode is active.
    pub synchronized_output: bool,
}

impl TerminalModes {
    /// Creates the default mode set.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            origin: false,
            wrap: true,
            insert: false,
            newline: false,
            keypad: false,
            cursor_visible: true,
            cursor_blink: true,
            alternate_screen: false,
            bracketed_paste: false,
            focus_event: false,
            synchronized_output: false,
        }
    }

    /// Applies a DEC mode flag.
    pub fn set_mode(&mut self, mode: Mode, enabled: bool) {
        match mode {
            Mode::Origin => self.origin = enabled,
            Mode::Wrap => self.wrap = enabled,
            Mode::Insert => self.insert = enabled,
            Mode::Newline => self.newline = enabled,
            Mode::Keypad => self.keypad = enabled,
            Mode::CursorVisible => self.cursor_visible = enabled,
            Mode::CursorBlink => self.cursor_blink = enabled,
            Mode::AlternateScreen => self.alternate_screen = enabled,
            Mode::BracketedPaste => self.bracketed_paste = enabled,
            Mode::FocusEvent => self.focus_event = enabled,
            Mode::SynchronizedOutput => self.synchronized_output = enabled,
        }
    }
}

impl Default for TerminalModes {
    fn default() -> Self {
        Self::new()
    }
}

/// Supported ANSI and DEC mode identifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Origin mode.
    Origin,
    /// Line wrap mode.
    Wrap,
    /// Insert mode.
    Insert,
    /// Newline mode.
    Newline,
    /// Keypad application mode.
    Keypad,
    /// Cursor visible.
    CursorVisible,
    /// Cursor blinking.
    CursorBlink,
    /// Alternate screen buffer.
    AlternateScreen,
    /// Bracketed paste mode.
    BracketedPaste,
    /// Focus event reporting.
    FocusEvent,
    /// Synchronized output mode.
    SynchronizedOutput,
}

impl Mode {
    /// Maps an ANSI mode parameter to a known mode.
    #[must_use]
    pub const fn from_ansi_param(param: u16) -> Option<Self> {
        // Keypad application mode is toggled by ESC = / ESC >, not CSI mode parameters.
        match param {
            4 => Some(Self::Insert),
            20 => Some(Self::Newline),
            _ => None,
        }
    }

    /// Maps a DEC private mode parameter to a known mode.
    #[must_use]
    pub const fn from_dec_private_param(param: u16) -> Option<Self> {
        match param {
            6 => Some(Self::Origin),
            7 => Some(Self::Wrap),
            12 => Some(Self::CursorBlink),
            25 => Some(Self::CursorVisible),
            1004 => Some(Self::FocusEvent),
            1049 => Some(Self::AlternateScreen),
            2004 => Some(Self::BracketedPaste),
            2026 => Some(Self::SynchronizedOutput),
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "test/modes/tests.rs"]
mod tests;
