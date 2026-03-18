use self::movement::default_tab_stops;
use self::screen::normalize_scroll_region;
use crate::cell::{Cell, CellAttrs};
use crate::cursor::{Cursor, SavedCursor};
use crate::damage::DamageRegion;
use crate::error::Result;
use crate::grid::{Grid, GridSize};
use crate::modes::TerminalModes;
use crate::parser::Action;

mod editing;
mod movement;
mod screen;

#[cfg(test)]
mod tests;

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
    tab_stops: Vec<usize>,
    alternate_screen_state: Option<AlternateScreenState>,
    scroll_region: Option<(usize, usize)>,
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct AlternateScreenState {
    grid: Grid,
    cursor: SavedCursor,
    scroll_region: Option<(usize, usize)>,
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
            tab_stops: default_tab_stops(cols),
            alternate_screen_state: None,
            scroll_region: None,
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
            Action::ForwardTab(count) => self.forward_tab(count),
            Action::LineFeed | Action::VerticalTab | Action::FormFeed => self.line_feed()?,
            Action::CarriageReturn => self.carriage_return(),
            Action::Index => self.index(),
            Action::NextLine => self.next_line()?,
            Action::ReverseIndex => self.reverse_index(),
            Action::ScrollUp(count) => self.scroll_up(count)?,
            Action::ScrollDown(count) => self.scroll_down(count)?,
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
            Action::InsertCharacters(count) => self.insert_characters(count)?,
            Action::DeleteCharacters(count) => self.delete_characters(count)?,
            Action::InsertLines(count) => self.insert_lines(count)?,
            Action::DeleteLines(count) => self.delete_lines(count)?,
            Action::EraseDisplay(mode) => self.erase_display(mode)?,
            Action::EraseLine(mode) => self.erase_line_mode(mode)?,
            Action::EraseCharacters(count) => self.erase_characters(count)?,
            Action::SetScrollRegion { top, bottom } => self.set_scroll_region(top, bottom),
            Action::SetGraphicsRendition(renditions) => self.apply_sgr(&renditions),
            Action::SetWindowTitle(title) => self.window_title = Some(title),
            Action::SetHyperlink { id, uri } => {
                self.active_hyperlink = if uri.is_empty() {
                    None
                } else {
                    Some(Hyperlink { id, uri })
                };
            }
            Action::DeviceAttributes => {}
            Action::ResetTerminal => self.reset_state()?,
            Action::SetKeypadMode(enabled) => self.modes.keypad = enabled,
            Action::SetModes { private, modes } => self.apply_modes(false, private, &modes)?,
            Action::ResetModes { private, modes } => self.apply_modes(true, private, &modes)?,
            Action::SetTabStop => self.set_tab_stop(),
            Action::ClearTabStop(mode) => self.clear_tab_stops(mode),
            Action::BackTab(count) => self.back_tab(count),
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
        self.resize_tab_stops(cols);
        self.scroll_region = self
            .scroll_region
            .and_then(|(top, bottom)| normalize_scroll_region(rows, top + 1, bottom + 1));
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

    fn reset_state(&mut self) -> Result<()> {
        if self.modes.alternate_screen {
            self.exit_alternate_screen();
        }

        self.alternate_screen_state = None;
        self.grid.clear();
        self.cursor = Cursor::new();
        self.modes = TerminalModes::new();
        self.attrs = CellAttrs::default();
        self.window_title = None;
        self.active_hyperlink = None;
        self.tab_stops = default_tab_stops(self.grid.cols());
        self.scroll_region = None;
        self.saved_cursor = None;
        Ok(())
    }
}
