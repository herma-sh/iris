use self::movement::default_tab_stops;
use self::screen::normalize_scroll_region;
use crate::cell::{Cell, CellAttrs};
use crate::cursor::{Cursor, SavedCursor};
use crate::damage::{DamageRegion, ScrollDelta};
use crate::error::{validate_printable_ascii, Result};
use crate::grid::{Grid, GridSize};
use crate::modes::TerminalModes;
use crate::parser::Action;
use crate::scrollback::{Line, Scrollback, ScrollbackConfig, SearchConfig, SearchResult};
use crate::selection::{Selection, SelectionEngine, SelectionKind};

mod editing;
mod movement;
mod screen;

const BRACKETED_PASTE_START: &str = "\u{1b}[200~";
const BRACKETED_PASTE_END: &str = "\u{1b}[201~";

#[cfg(test)]
#[path = "../test/terminal/tests.rs"]
mod tests;
#[cfg(test)]
#[path = "../test/terminal/tests_ascii.rs"]
mod tests_ascii;
#[cfg(test)]
#[path = "../test/terminal/tests_erase.rs"]
mod tests_erase;

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
    /// Active selection state for copy operations.
    selection: SelectionEngine,
    scrollback: Scrollback,
    scrollback_view_offset: usize,
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
    scrollback_view_offset: usize,
}

impl Terminal {
    /// Creates a terminal with the provided visible dimensions.
    pub fn new(rows: usize, cols: usize) -> Result<Self> {
        Self::new_with_scrollback(rows, cols, ScrollbackConfig::default())
    }

    /// Creates a terminal with the provided visible dimensions and scrollback settings.
    pub fn new_with_scrollback(
        rows: usize,
        cols: usize,
        scrollback_config: ScrollbackConfig,
    ) -> Result<Self> {
        Ok(Self {
            grid: Grid::new(GridSize { rows, cols })?,
            cursor: Cursor::new(),
            modes: TerminalModes::new(),
            attrs: CellAttrs::default(),
            window_title: None,
            active_hyperlink: None,
            selection: SelectionEngine::new(),
            scrollback: Scrollback::new(scrollback_config),
            scrollback_view_offset: 0,
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

    /// Writes a contiguous ASCII run using single-width cells.
    pub fn write_ascii_run(&mut self, bytes: &[u8]) -> Result<()> {
        if bytes.is_empty() || self.grid.rows() == 0 || self.grid.cols() == 0 {
            return Ok(());
        }

        validate_printable_ascii(bytes)?;

        let cols = self.grid.cols();
        let mut remaining = bytes;

        while !remaining.is_empty() {
            let row = self
                .cursor
                .position
                .row
                .min(self.grid.rows().saturating_sub(1));
            let col = self.cursor.position.col.min(cols.saturating_sub(1));

            if self.modes.wrap {
                let available = cols.saturating_sub(col);
                let chunk_len = remaining.len().min(available);
                self.grid
                    .write_ascii_run(row, col, &remaining[..chunk_len], self.attrs)?;

                if col + chunk_len < cols {
                    self.cursor.position.col = col + chunk_len;
                    break;
                }

                remaining = &remaining[chunk_len..];
                if remaining.is_empty() {
                    self.cursor.position.col = 0;
                    self.line_feed()?;
                    break;
                }

                self.cursor.position.col = 0;
                self.line_feed()?;
                continue;
            }

            if col + remaining.len() < cols {
                self.grid.write_ascii_run(row, col, remaining, self.attrs)?;
                self.cursor.position.col = col + remaining.len();
                break;
            }

            let last_col = cols.saturating_sub(1);
            if col < last_col {
                let prefix_len = last_col - col;
                if prefix_len > 0 {
                    self.grid
                        .write_ascii_run(row, col, &remaining[..prefix_len], self.attrs)?;
                }
                self.grid.write_ascii_run(
                    row,
                    last_col,
                    &remaining[(remaining.len() - 1)..],
                    self.attrs,
                )?;
            } else {
                self.grid.write_ascii_run(
                    row,
                    col,
                    &remaining[(remaining.len() - 1)..],
                    self.attrs,
                )?;
            }

            self.cursor.position.col = last_col;
            break;
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

    /// Encodes paste payload bytes according to active bracketed paste mode.
    #[must_use]
    pub fn paste_bytes(&self, text: &str) -> Vec<u8> {
        if !self.modes.bracketed_paste {
            return text.as_bytes().to_vec();
        }

        let mut payload = Vec::with_capacity(
            BRACKETED_PASTE_START.len() + text.len() + BRACKETED_PASTE_END.len(),
        );
        payload.extend_from_slice(BRACKETED_PASTE_START.as_bytes());
        payload.extend_from_slice(text.as_bytes());
        payload.extend_from_slice(BRACKETED_PASTE_END.as_bytes());
        payload
    }

    /// Returns retained scrollback history.
    #[must_use]
    pub const fn scrollback(&self) -> &Scrollback {
        &self.scrollback
    }

    /// Returns the current viewport offset from the bottom in rows.
    #[must_use]
    pub const fn scrollback_view_offset(&self) -> usize {
        self.scrollback_view_offset
    }

    /// Returns scrollback matches for the provided query configuration.
    #[must_use]
    pub fn search_scrollback(&self, config: &SearchConfig) -> Vec<SearchResult> {
        self.scrollback.search_with_config(config)
    }

    /// Scrolls the viewport up by one row.
    pub fn scroll_line_up(&mut self) {
        self.scroll_lines_up(1);
    }

    /// Scrolls the viewport down by one row.
    pub fn scroll_line_down(&mut self) {
        self.scroll_lines_down(1);
    }

    /// Scrolls the viewport up by one page.
    pub fn scroll_page_up(&mut self) {
        self.scroll_lines_up(self.grid.rows().max(1));
    }

    /// Scrolls the viewport down by one page.
    pub fn scroll_page_down(&mut self) {
        self.scroll_lines_down(self.grid.rows().max(1));
    }

    /// Scrolls the viewport to the oldest retained scrollback line.
    pub fn scroll_to_top(&mut self) {
        self.scrollback_view_offset = self.scrollback.len();
    }

    /// Scrolls the viewport back to live output.
    pub fn scroll_to_bottom(&mut self) {
        self.scrollback_view_offset = 0;
    }

    /// Returns the current terminal selection, if any.
    #[must_use]
    pub const fn selection(&self) -> Option<&Selection> {
        self.selection.selection()
    }

    /// Returns `true` while an in-progress selection drag is active.
    #[must_use]
    pub fn is_selecting(&self) -> bool {
        self.selection.is_selecting()
    }

    /// Returns `true` when the terminal has a completed selection.
    #[must_use]
    pub fn has_selection(&self) -> bool {
        self.selection.has_selection()
    }

    /// Returns `true` when the provided visible grid position is selected.
    #[must_use]
    pub fn selection_contains(&self, row: usize, col: usize) -> bool {
        if row >= self.grid.rows() || col >= self.grid.cols() {
            return false;
        }

        self.selection.contains(row, col)
    }

    /// Returns selected visible column bounds for a row, if selected.
    #[must_use]
    pub fn selection_row_bounds(&self, row: usize) -> Option<(usize, usize)> {
        if row >= self.grid.rows() {
            return None;
        }

        self.selection.row_bounds(row, self.grid.cols())
    }

    /// Returns the inclusive selected visible row span when selected.
    #[must_use]
    pub fn selection_row_span(&self) -> Option<(usize, usize)> {
        let (start, end) = self.selection.row_span()?;
        let visible_start = 0usize;
        let visible_end = self.grid.rows().checked_sub(1)?;
        let clamped_start = start.max(visible_start);
        let clamped_end = end.min(visible_end);

        (clamped_start <= clamped_end).then_some((clamped_start, clamped_end))
    }

    /// Starts a selection anchored to the provided grid position.
    pub fn start_selection(&mut self, row: usize, col: usize, kind: SelectionKind) {
        let Some((row, col)) = self.clamp_selection_position(row, col) else {
            self.selection.cancel();
            return;
        };
        self.selection.start(row, col, kind);
    }

    /// Extends the current selection endpoint to the provided grid position.
    pub fn extend_selection(&mut self, row: usize, col: usize) {
        let Some((row, col)) = self.clamp_selection_position(row, col) else {
            self.selection.cancel();
            return;
        };
        self.selection.extend(row, col);
    }

    /// Completes the current selection.
    pub fn complete_selection(&mut self) {
        self.selection.complete();
    }

    /// Cancels and clears any existing selection.
    pub fn cancel_selection(&mut self) {
        self.selection.cancel();
    }

    /// Selects the word at the provided grid position.
    pub fn select_word(&mut self, row: usize, col: usize) {
        self.selection.select_word(&self.grid, row, col);
    }

    /// Selects the entire line at the provided row.
    pub fn select_line(&mut self, row: usize) {
        self.selection.select_line(&self.grid, row);
    }

    /// Returns selected text without copy-specific formatting adjustments.
    #[must_use]
    pub fn selected_text(&self) -> Option<String> {
        self.selection.selected_text(&self.grid)
    }

    /// Returns selected text formatted for clipboard copy behavior.
    #[must_use]
    pub fn copy_selection_text(&self) -> Option<String> {
        self.selection.copy_text(&self.grid)
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
        self.selection.cancel();
        self.clamp_scrollback_view_offset();
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

    /// Returns and clears the latest pending visible-grid scroll delta.
    pub fn take_scroll_delta(&mut self) -> Option<ScrollDelta> {
        self.grid.take_scroll_delta()
    }

    /// Restores previously drained visible damage regions to the terminal grid.
    pub fn restore_damage(&mut self, damage: &[DamageRegion]) {
        self.grid.restore_damage(damage);
    }

    /// Restores a previously drained visible-grid scroll delta.
    pub fn restore_scroll_delta(&mut self, scroll_delta: Option<ScrollDelta>) {
        self.grid.restore_scroll_delta(scroll_delta);
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
        self.scrollback.clear();
        self.scrollback_view_offset = 0;
        self.tab_stops = default_tab_stops(self.grid.cols());
        self.scroll_region = None;
        self.saved_cursor = None;
        self.selection.cancel();
        Ok(())
    }

    fn scroll_lines_up(&mut self, lines: usize) {
        if lines == 0 {
            return;
        }

        let max_offset = self.scrollback.len();
        self.scrollback_view_offset = self
            .scrollback_view_offset
            .saturating_add(lines)
            .min(max_offset);
    }

    fn scroll_lines_down(&mut self, lines: usize) {
        if lines == 0 {
            return;
        }

        self.scrollback_view_offset = self.scrollback_view_offset.saturating_sub(lines);
    }

    fn clamp_scrollback_view_offset(&mut self) {
        self.scrollback_view_offset = self.scrollback_view_offset.min(self.scrollback.len());
    }

    fn capture_scrollback_rows(&mut self, top: usize, bottom: usize, count: usize) {
        if self.modes.alternate_screen
            || count == 0
            || self.grid.rows() == 0
            || self.grid.cols() == 0
        {
            return;
        }
        if top != 0 || bottom.saturating_add(1) != self.grid.rows() {
            return;
        }

        let region_rows = bottom.saturating_sub(top).saturating_add(1);
        let capture_count = count.min(region_rows);
        if capture_count == 0 {
            return;
        }

        let mut captured = Vec::with_capacity(capture_count);
        for row in top..(top + capture_count) {
            if let Some(cells) = self.grid.row(row) {
                captured.push(Line::new(cells.to_vec(), false));
            }
        }

        if self.scrollback_view_offset > 0 {
            self.scrollback_view_offset =
                self.scrollback_view_offset.saturating_add(captured.len());
        }

        for line in captured {
            self.scrollback.push(line);
        }

        self.clamp_scrollback_view_offset();
    }

    fn clamp_selection_position(&self, row: usize, col: usize) -> Option<(usize, usize)> {
        if self.grid.rows() == 0 || self.grid.cols() == 0 {
            return None;
        }

        Some((
            row.min(self.grid.rows().saturating_sub(1)),
            col.min(self.grid.cols().saturating_sub(1)),
        ))
    }
}
