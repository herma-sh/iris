use crate::cell::{Cell, CellAttrs, CellFlags};
use crate::cursor::{Cursor, SavedCursor};
use crate::damage::DamageRegion;
use crate::error::Result;
use crate::grid::{Grid, GridSize};
use crate::modes::{Mode, TerminalModes};
use crate::parser::{Action, GraphicsRendition};
use crate::utils::TAB_WIDTH;

/// The visible terminal state used by Iris core.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Terminal {
    /// Visible grid contents.
    pub grid: Grid,
    /// Cursor state.
    pub cursor: Cursor,
    /// Terminal modes.
    pub modes: TerminalModes,
    /// Active attributes used for printed text.
    pub attrs: CellAttrs,
    /// The last OSC window title observed by the parser.
    pub window_title: Option<String>,
    /// The active OSC 8 hyperlink metadata.
    pub active_hyperlink: Option<Hyperlink>,
    primary_grid: Option<Grid>,
    alternate_screen_cursor: Option<SavedCursor>,
    saved_cursor: Option<SavedCursor>,
}

/// Hyperlink metadata emitted by OSC 8 sequences.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hyperlink {
    /// Optional hyperlink identifier.
    pub id: Option<String>,
    /// Target URI for the hyperlink.
    pub uri: String,
}

impl Terminal {
    /// Creates a terminal with the provided visible dimensions.
    pub fn new(rows: usize, cols: usize) -> Result<Self> {
        Ok(Self {
            grid: Grid::new(GridSize { rows, cols })?,
            cursor: Cursor::new(),
            modes: TerminalModes::new(),
            attrs: CellAttrs::default(),
            window_title: None,
            active_hyperlink: None,
            primary_grid: None,
            alternate_screen_cursor: None,
            saved_cursor: None,
        })
    }

    /// Writes a printable character at the cursor and advances the cursor.
    pub fn write_char(&mut self, character: char) -> Result<()> {
        if self.grid.rows() == 0 || self.grid.cols() == 0 {
            return Ok(());
        }

        let row = self
            .cursor
            .position
            .row
            .min(self.grid.rows().saturating_sub(1));
        let col = self
            .cursor
            .position
            .col
            .min(self.grid.cols().saturating_sub(1));
        let cell = Cell::with_attrs(character, self.attrs);
        let width = cell.width.columns();
        self.grid.write(row, col, cell)?;

        if col + width < self.grid.cols() {
            self.cursor.position.col = col + width;
        } else if self.modes.wrap {
            self.cursor.position.col = 0;
            self.line_feed()?;
        }

        Ok(())
    }

    /// Applies a parser-emitted terminal action.
    pub fn apply_action(&mut self, action: Action) -> Result<()> {
        match action {
            Action::Print(character) => self.write_char(character)?,
            Action::Bell => {}
            Action::Backspace => self.backspace(),
            Action::Tab => self.tab(),
            Action::LineFeed | Action::VerticalTab | Action::FormFeed => self.line_feed()?,
            Action::CarriageReturn => self.carriage_return(),
            Action::Index => self.index(),
            Action::NextLine => self.next_line()?,
            Action::ReverseIndex => self.reverse_index(),
            Action::SaveCursor => self.save_cursor(),
            Action::RestoreCursor => self.restore_cursor(),
            Action::CursorUp(count) => self.cursor_up(count),
            Action::CursorDown(count) => self.cursor_down(count),
            Action::CursorForward(count) => self.cursor_forward(count),
            Action::CursorBack(count) => self.cursor_back(count),
            Action::CursorNextLine(count) => self.cursor_next_line(count),
            Action::CursorPreviousLine(count) => self.cursor_previous_line(count),
            Action::CursorColumn(column) => self.cursor_column(column),
            Action::CursorPosition { row, col } => self.cursor_position(row, col),
            Action::VerticalPosition(row) => self.vertical_position(row),
            Action::EraseDisplay(mode) => self.erase_display(mode)?,
            Action::EraseLine(mode) => self.erase_line_mode(mode)?,
            Action::EraseCharacters(count) => self.erase_characters(count)?,
            Action::SetGraphicsRendition(renditions) => self.apply_sgr(&renditions),
            Action::SetWindowTitle(title) => self.window_title = Some(title),
            Action::SetHyperlink { id, uri } => {
                self.active_hyperlink = if uri.is_empty() {
                    None
                } else {
                    Some(Hyperlink { id, uri })
                };
            }
            Action::SetModes { private, modes } => self.apply_modes(false, private, &modes)?,
            Action::ResetModes { private, modes } => self.apply_modes(true, private, &modes)?,
        }

        Ok(())
    }

    /// Executes a single control character.
    pub fn execute_control(&mut self, byte: u8) -> Result<()> {
        self.apply_action(match byte {
            0x07 => Action::Bell,
            0x08 => Action::Backspace,
            0x09 => Action::Tab,
            0x0a => Action::LineFeed,
            0x0b => Action::VerticalTab,
            0x0c => Action::FormFeed,
            0x0d => Action::CarriageReturn,
            _ => return Ok(()),
        })
    }

    /// Moves the cursor to an absolute position inside the visible grid.
    pub fn move_cursor(&mut self, row: usize, col: usize) {
        if self.grid.rows() == 0 || self.grid.cols() == 0 {
            self.cursor.move_to(0, 0);
            return;
        }

        self.cursor.move_to(
            row.min(self.grid.rows().saturating_sub(1)),
            col.min(self.grid.cols().saturating_sub(1)),
        );
    }

    /// Resizes the terminal grid and clamps the cursor to the new bounds.
    pub fn resize(&mut self, rows: usize, cols: usize) -> Result<()> {
        self.grid.resize(GridSize { rows, cols })?;
        self.move_cursor(self.cursor.position.row, self.cursor.position.col);
        Ok(())
    }

    /// Saves the current cursor state.
    pub fn save_cursor(&mut self) {
        self.saved_cursor = Some(self.cursor.save());
    }

    /// Restores the saved cursor state when present.
    pub fn restore_cursor(&mut self) {
        if let Some(saved_cursor) = self.saved_cursor {
            self.cursor.restore(saved_cursor);
            self.move_cursor(self.cursor.position.row, self.cursor.position.col);
        }
    }

    /// Returns and clears the current damage list.
    pub fn take_damage(&mut self) -> Vec<DamageRegion> {
        self.grid.take_damage()
    }

    fn backspace(&mut self) {
        self.cursor.move_left(1);
    }

    fn carriage_return(&mut self) {
        self.cursor.position.col = 0;
    }

    fn index(&mut self) {
        if self.grid.rows() == 0 {
            return;
        }

        if self.cursor.position.row + 1 >= self.grid.rows() {
            self.grid.scroll_up(1);
        } else {
            self.cursor.move_down(1, self.grid.rows());
        }
    }

    fn next_line(&mut self) -> Result<()> {
        self.carriage_return();
        self.line_feed()
    }

    fn reverse_index(&mut self) {
        if self.grid.rows() == 0 {
            return;
        }

        if self.cursor.position.row == 0 {
            self.grid.scroll_down(1);
        } else {
            self.cursor.move_up(1);
        }
    }

    fn cursor_up(&mut self, count: u16) {
        self.cursor.move_up(usize::from(count.max(1)));
    }

    fn cursor_down(&mut self, count: u16) {
        if self.grid.rows() == 0 {
            return;
        }

        self.cursor
            .move_down(usize::from(count.max(1)), self.grid.rows());
    }

    fn cursor_forward(&mut self, count: u16) {
        if self.grid.cols() == 0 {
            return;
        }

        self.cursor
            .move_right(usize::from(count.max(1)), self.grid.cols());
    }

    fn cursor_back(&mut self, count: u16) {
        self.cursor.move_left(usize::from(count.max(1)));
    }

    fn cursor_next_line(&mut self, count: u16) {
        self.cursor_down(count);
        self.carriage_return();
    }

    fn cursor_previous_line(&mut self, count: u16) {
        self.cursor_up(count);
        self.carriage_return();
    }

    fn cursor_column(&mut self, column: u16) {
        if self.grid.cols() == 0 {
            self.cursor.position.col = 0;
            return;
        }

        self.cursor.position.col =
            usize::from(column.saturating_sub(1)).min(self.grid.cols().saturating_sub(1));
    }

    fn vertical_position(&mut self, row: u16) {
        if self.grid.rows() == 0 {
            self.cursor.position.row = 0;
            return;
        }

        self.cursor.position.row =
            usize::from(row.saturating_sub(1)).min(self.grid.rows().saturating_sub(1));
    }

    fn cursor_position(&mut self, row: u16, col: u16) {
        self.move_cursor(
            usize::from(row.saturating_sub(1)),
            usize::from(col.saturating_sub(1)),
        );
    }

    fn erase_display(&mut self, mode: u16) -> Result<()> {
        if self.grid.rows() == 0 || self.grid.cols() == 0 {
            return Ok(());
        }

        let row = self
            .cursor
            .position
            .row
            .min(self.grid.rows().saturating_sub(1));
        let col = self
            .cursor
            .position
            .col
            .min(self.grid.cols().saturating_sub(1));

        match mode {
            0 => {
                self.clear_row_range(row, col, self.grid.cols().saturating_sub(1))?;
                for clear_row in (row + 1)..self.grid.rows() {
                    self.grid.clear_row(clear_row)?;
                }
            }
            1 => {
                for clear_row in 0..row {
                    self.grid.clear_row(clear_row)?;
                }
                self.clear_row_range(row, 0, col)?;
            }
            2 | 3 => self.grid.clear(),
            _ => {}
        }

        Ok(())
    }

    fn erase_line_mode(&mut self, mode: u16) -> Result<()> {
        if self.grid.rows() == 0 || self.grid.cols() == 0 {
            return Ok(());
        }

        let row = self
            .cursor
            .position
            .row
            .min(self.grid.rows().saturating_sub(1));
        let col = self
            .cursor
            .position
            .col
            .min(self.grid.cols().saturating_sub(1));

        match mode {
            0 => self.clear_row_range(row, col, self.grid.cols().saturating_sub(1))?,
            1 => self.clear_row_range(row, 0, col)?,
            2 => self.grid.clear_row(row)?,
            _ => {}
        }

        Ok(())
    }

    fn erase_characters(&mut self, count: u16) -> Result<()> {
        if self.grid.rows() == 0 || self.grid.cols() == 0 {
            return Ok(());
        }

        let row = self
            .cursor
            .position
            .row
            .min(self.grid.rows().saturating_sub(1));
        let start = self
            .cursor
            .position
            .col
            .min(self.grid.cols().saturating_sub(1));
        let end = start
            .saturating_add(usize::from(count.max(1)))
            .saturating_sub(1)
            .min(self.grid.cols().saturating_sub(1));
        self.clear_row_range(row, start, end)
    }

    fn clear_row_range(&mut self, row: usize, start_col: usize, end_col: usize) -> Result<()> {
        if start_col > end_col {
            return Ok(());
        }

        for col in start_col..=end_col {
            self.grid.write(row, col, Cell::default())?;
        }

        Ok(())
    }

    fn apply_sgr(&mut self, renditions: &[GraphicsRendition]) {
        for rendition in renditions {
            match *rendition {
                GraphicsRendition::Reset => self.attrs = CellAttrs::default(),
                GraphicsRendition::Bold(enabled) => {
                    self.attrs.flags.set(CellFlags::BOLD, enabled);
                }
                GraphicsRendition::Dim(enabled) => {
                    self.attrs.flags.set(CellFlags::DIM, enabled);
                }
                GraphicsRendition::Italic(enabled) => {
                    self.attrs.flags.set(CellFlags::ITALIC, enabled);
                }
                GraphicsRendition::Underline(enabled) => {
                    self.attrs.flags.set(CellFlags::UNDERLINE, enabled);
                }
                GraphicsRendition::Blink(enabled) => {
                    self.attrs.flags.set(CellFlags::BLINK, enabled);
                }
                GraphicsRendition::Inverse(enabled) => {
                    self.attrs.flags.set(CellFlags::INVERSE, enabled);
                }
                GraphicsRendition::Hidden(enabled) => {
                    self.attrs.flags.set(CellFlags::HIDDEN, enabled);
                }
                GraphicsRendition::Strikethrough(enabled) => {
                    self.attrs.flags.set(CellFlags::STRIKETHROUGH, enabled);
                }
                GraphicsRendition::Foreground(color) => self.attrs.fg = color,
                GraphicsRendition::Background(color) => self.attrs.bg = color,
            }
        }
    }

    fn apply_modes(&mut self, reset: bool, private: bool, params: &[u16]) -> Result<()> {
        for &param in params {
            let mode = if private {
                Mode::from_dec_private_param(param)
            } else {
                Mode::from_ansi_param(param)
            };

            if let Some(mode) = mode {
                let enabled = !reset;
                match mode {
                    Mode::AlternateScreen => self.set_alternate_screen(enabled)?,
                    _ => self.modes.set_mode(mode, enabled),
                }
            }
        }

        self.cursor.visible = self.modes.cursor_visible;
        self.cursor.blinking = self.modes.cursor_blink;
        Ok(())
    }

    fn line_feed(&mut self) -> Result<()> {
        if self.grid.rows() == 0 {
            return Ok(());
        }

        self.index();

        if self.modes.newline {
            self.carriage_return();
        }

        Ok(())
    }

    fn tab(&mut self) {
        let cols = self.grid.cols();
        if cols == 0 {
            return;
        }

        let current = self.cursor.position.col;
        let next_tab_stop = ((current / TAB_WIDTH) + 1) * TAB_WIDTH;
        self.cursor.position.col = next_tab_stop.min(cols.saturating_sub(1));
    }

    fn set_alternate_screen(&mut self, enabled: bool) -> Result<()> {
        if enabled {
            self.enter_alternate_screen()
        } else {
            self.exit_alternate_screen();
            Ok(())
        }
    }

    fn enter_alternate_screen(&mut self) -> Result<()> {
        if self.modes.alternate_screen {
            return Ok(());
        }

        let size = self.grid.size();
        let mut alternate_grid = Grid::new(size)?;
        alternate_grid.mark_all_damage();

        self.primary_grid = Some(std::mem::replace(&mut self.grid, alternate_grid));
        self.alternate_screen_cursor = Some(self.cursor.save());
        self.cursor = Cursor::new();
        self.saved_cursor = None;
        self.modes.alternate_screen = true;
        Ok(())
    }

    fn exit_alternate_screen(&mut self) {
        if !self.modes.alternate_screen {
            return;
        }

        if let Some(mut primary_grid) = self.primary_grid.take() {
            primary_grid.mark_all_damage();
            self.grid = primary_grid;
        }

        if let Some(saved_cursor) = self.alternate_screen_cursor.take() {
            self.cursor.restore(saved_cursor);
            self.move_cursor(self.cursor.position.row, self.cursor.position.col);
        } else {
            self.cursor = Cursor::new();
        }

        self.saved_cursor = None;
        self.modes.alternate_screen = false;
    }
}

#[cfg(test)]
mod tests {
    use super::Terminal;
    use crate::cell::{CellFlags, Color};
    use crate::parser::{Action, GraphicsRendition};

    #[test]
    fn terminal_write_advances_cursor() {
        let mut terminal = Terminal::new(3, 4).unwrap();
        terminal.write_char('A').unwrap();
        assert_eq!(
            terminal.grid.cell(0, 0).map(|cell| cell.character),
            Some('A')
        );
        assert_eq!(terminal.cursor.position.col, 1);
    }

    #[test]
    fn terminal_line_feed_scrolls_at_bottom() {
        let mut terminal = Terminal::new(2, 4).unwrap();
        terminal.move_cursor(1, 0);
        terminal.write_char('Z').unwrap();
        terminal.execute_control(0x0a).unwrap();
        terminal.write_char('Q').unwrap();

        assert_eq!(
            terminal.grid.cell(0, 0).map(|cell| cell.character),
            Some('Z')
        );
        assert_eq!(
            terminal.grid.cell(1, 1).map(|cell| cell.character),
            Some('Q')
        );
    }

    #[test]
    fn terminal_resize_clamps_cursor() {
        let mut terminal = Terminal::new(8, 8).unwrap();
        terminal.move_cursor(7, 7);
        terminal.resize(2, 2).unwrap();
        assert_eq!(terminal.cursor.position.row, 1);
        assert_eq!(terminal.cursor.position.col, 1);
    }

    #[test]
    fn terminal_restore_cursor_clamps_after_resize() {
        let mut terminal = Terminal::new(8, 8).unwrap();
        terminal.move_cursor(7, 7);
        terminal.save_cursor();
        terminal.resize(2, 2).unwrap();
        terminal.restore_cursor();
        assert_eq!(terminal.cursor.position.row, 1);
        assert_eq!(terminal.cursor.position.col, 1);
    }

    #[test]
    fn terminal_applies_cursor_and_erase_actions() {
        let mut terminal = Terminal::new(3, 5).unwrap();
        terminal.write_char('A').unwrap();
        terminal.write_char('B').unwrap();
        terminal.write_char('C').unwrap();

        terminal
            .apply_action(Action::CursorPosition { row: 1, col: 2 })
            .unwrap();
        terminal.apply_action(Action::EraseLine(0)).unwrap();

        assert_eq!(
            terminal.grid.cell(0, 0).map(|cell| cell.character),
            Some('A')
        );
        assert_eq!(
            terminal.grid.cell(0, 1).map(|cell| cell.character),
            Some(' ')
        );
        assert_eq!(
            terminal.grid.cell(0, 2).map(|cell| cell.character),
            Some(' ')
        );
    }

    #[test]
    fn terminal_applies_sgr_and_modes() {
        let mut terminal = Terminal::new(2, 4).unwrap();
        terminal
            .apply_action(Action::SetGraphicsRendition(vec![
                GraphicsRendition::Bold(true),
                GraphicsRendition::Foreground(Color::Indexed(33)),
            ]))
            .unwrap();
        terminal.write_char('X').unwrap();
        terminal
            .apply_action(Action::ResetModes {
                private: true,
                modes: vec![25],
            })
            .unwrap();

        let cell = terminal.grid.cell(0, 0).copied().unwrap();
        assert!(cell.attrs.flags.contains(CellFlags::BOLD));
        assert_eq!(cell.attrs.fg, Color::Indexed(33));
        assert!(!terminal.cursor.visible);
    }

    #[test]
    fn terminal_next_line_and_reverse_index_follow_escape_semantics() {
        let mut terminal = Terminal::new(2, 4).unwrap();
        terminal.write_char('A').unwrap();
        terminal.apply_action(Action::NextLine).unwrap();
        terminal.write_char('B').unwrap();

        assert_eq!(terminal.cursor.position.row, 1);
        assert_eq!(
            terminal.grid.cell(1, 0).map(|cell| cell.character),
            Some('B')
        );

        terminal.move_cursor(0, 0);
        terminal.apply_action(Action::ReverseIndex).unwrap();
        assert_eq!(
            terminal.grid.cell(1, 0).map(|cell| cell.character),
            Some('A')
        );
    }

    #[test]
    fn terminal_mode_application_respects_private_marker() {
        let mut terminal = Terminal::new(2, 4).unwrap();

        terminal
            .apply_action(Action::SetModes {
                private: false,
                modes: vec![4],
            })
            .unwrap();
        assert!(terminal.modes.insert);

        terminal
            .apply_action(Action::ResetModes {
                private: true,
                modes: vec![4],
            })
            .unwrap();
        assert!(terminal.modes.insert);
    }

    #[test]
    fn terminal_switches_between_primary_and_alternate_screen() {
        let mut terminal = Terminal::new(2, 4).unwrap();
        terminal.write_char('A').unwrap();
        terminal.move_cursor(1, 2);

        terminal
            .apply_action(Action::SetModes {
                private: true,
                modes: vec![1049],
            })
            .unwrap();
        terminal.write_char('B').unwrap();

        assert!(terminal.modes.alternate_screen);
        assert_eq!(
            terminal.grid.cell(0, 0).map(|cell| cell.character),
            Some('B')
        );

        terminal
            .apply_action(Action::ResetModes {
                private: true,
                modes: vec![1049],
            })
            .unwrap();

        assert!(!terminal.modes.alternate_screen);
        assert_eq!(
            terminal.grid.cell(0, 0).map(|cell| cell.character),
            Some('A')
        );
        assert_eq!(terminal.cursor.position.row, 1);
        assert_eq!(terminal.cursor.position.col, 2);
    }

    #[test]
    fn terminal_tracks_osc_metadata_actions() {
        let mut terminal = Terminal::new(2, 4).unwrap();

        terminal
            .apply_action(Action::SetWindowTitle("Iris".to_string()))
            .unwrap();
        terminal
            .apply_action(Action::SetHyperlink {
                id: Some("prompt-1".to_string()),
                uri: "https://example.com".to_string(),
            })
            .unwrap();

        assert_eq!(terminal.window_title.as_deref(), Some("Iris"));
        assert_eq!(
            terminal.active_hyperlink,
            Some(super::Hyperlink {
                id: Some("prompt-1".to_string()),
                uri: "https://example.com".to_string(),
            })
        );

        terminal
            .apply_action(Action::SetHyperlink {
                id: None,
                uri: String::new(),
            })
            .unwrap();
        assert_eq!(terminal.active_hyperlink, None);
    }
}
