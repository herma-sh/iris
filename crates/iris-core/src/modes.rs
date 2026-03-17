/// Phase-0 terminal modes required by the core terminal state.
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
    /// Whether the cursor is visible.
    pub cursor_visible: bool,
    /// Whether cursor blinking is enabled.
    pub cursor_blink: bool,
}

impl TerminalModes {
    /// Constructs the default terminal modes.
    ///
    /// Defaults: `origin = false`, `wrap = true`, `insert = false`, `newline = false`,
    /// `cursor_visible = true`, `cursor_blink = true`.
    ///
    /// # Examples
    ///
    /// ```
    /// let modes = TerminalModes::new();
    /// assert!(!modes.origin);
    /// assert!(modes.wrap);
    /// assert!(!modes.insert);
    /// assert!(!modes.newline);
    /// assert!(modes.cursor_visible);
    /// assert!(modes.cursor_blink);
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self {
            origin: false,
            wrap: true,
            insert: false,
            newline: false,
            cursor_visible: true,
            cursor_blink: true,
        }
    }

    /// Set a DEC terminal mode flag to enabled or disabled.
    ///
    /// # Parameters
    ///
    /// - `mode`: which mode to change.
    /// - `enabled`: `true` to enable the mode, `false` to disable it.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut modes = TerminalModes::new();
    /// modes.set_mode(Mode::Wrap, false);
    /// assert!(!modes.wrap);
    /// ```
    pub fn set_mode(&mut self, mode: Mode, enabled: bool) {
        match mode {
            Mode::Origin => self.origin = enabled,
            Mode::Wrap => self.wrap = enabled,
            Mode::Insert => self.insert = enabled,
            Mode::Newline => self.newline = enabled,
            Mode::CursorVisible => self.cursor_visible = enabled,
            Mode::CursorBlink => self.cursor_blink = enabled,
        }
    }
}

impl Default for TerminalModes {
    /// Creates a TerminalModes instance populated with the module's default mode values.
    ///
    /// # Examples
    ///
    /// ```
    /// let modes = TerminalModes::default();
    /// assert!(modes.wrap);
    /// assert!(modes.cursor_visible);
    /// assert!(modes.cursor_blink);
    /// ```
    fn default() -> Self {
        Self::new()
    }
}

/// Supported mode identifiers for phase 0.
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
    /// Cursor visible.
    CursorVisible,
    /// Cursor blinking.
    CursorBlink,
}

#[cfg(test)]
mod tests {
    use super::{Mode, TerminalModes};

    #[test]
    fn terminal_modes_default_to_wrap_and_visible_cursor() {
        let modes = TerminalModes::new();
        assert!(modes.wrap);
        assert!(modes.cursor_visible);
        assert!(modes.cursor_blink);
    }

    #[test]
    fn terminal_modes_can_toggle_flags() {
        let mut modes = TerminalModes::new();
        modes.set_mode(Mode::Wrap, false);
        assert!(!modes.wrap);
    }
}
