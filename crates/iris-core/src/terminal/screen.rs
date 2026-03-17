use crate::cursor::Cursor;
use crate::error::Result;
use crate::grid::Grid;
use crate::modes::Mode;

use super::{AlternateScreenState, Terminal};

impl Terminal {
    pub(super) fn index(&mut self) {
        if self.grid.rows() == 0 {
            return;
        }

        let (top, bottom) = self.active_scroll_region();
        if self.cursor.position.row >= top && self.cursor.position.row <= bottom {
            if self.cursor.position.row == bottom {
                let _ = self.grid.scroll_up_range(top, bottom, 1);
            } else {
                self.cursor.position.row += 1;
            }
        } else if self.cursor.position.row + 1 >= self.grid.rows() {
            self.grid.scroll_up(1);
        } else {
            self.cursor.move_down(1, self.grid.rows());
        }
    }

    pub(super) fn reverse_index(&mut self) {
        if self.grid.rows() == 0 {
            return;
        }

        let (top, bottom) = self.active_scroll_region();
        if self.cursor.position.row >= top && self.cursor.position.row <= bottom {
            if self.cursor.position.row == top {
                let _ = self.grid.scroll_down_range(top, bottom, 1);
            } else {
                self.cursor.move_up(1);
            }
        } else if self.cursor.position.row == 0 {
            self.grid.scroll_down(1);
        } else {
            self.cursor.move_up(1);
        }
    }

    pub(super) fn scroll_up(&mut self, count: u16) -> Result<()> {
        let (top, bottom) = self.active_scroll_region();
        self.grid
            .scroll_up_range(top, bottom, usize::from(count.max(1)))
    }

    pub(super) fn scroll_down(&mut self, count: u16) -> Result<()> {
        let (top, bottom) = self.active_scroll_region();
        self.grid
            .scroll_down_range(top, bottom, usize::from(count.max(1)))
    }

    pub(super) fn set_scroll_region(&mut self, top: u16, bottom: u16) {
        self.scroll_region =
            normalize_scroll_region(self.grid.rows(), usize::from(top), usize::from(bottom));
        let home_row = self.scroll_region.map_or(
            0,
            |(region_top, _)| {
                if self.modes.origin {
                    region_top
                } else {
                    0
                }
            },
        );
        self.move_cursor(home_row, 0);
    }

    pub(super) fn apply_modes(&mut self, reset: bool, private: bool, params: &[u16]) -> Result<()> {
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

    pub(super) fn set_alternate_screen(&mut self, enabled: bool) -> Result<()> {
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

        self.alternate_screen_state = Some(AlternateScreenState {
            grid: std::mem::replace(&mut self.grid, alternate_grid),
            cursor: self.cursor.save(),
            scroll_region: self.scroll_region,
        });
        self.cursor = Cursor::new();
        self.scroll_region = None;
        self.saved_cursor = None;
        self.modes.alternate_screen = true;
        Ok(())
    }

    fn exit_alternate_screen(&mut self) {
        if !self.modes.alternate_screen {
            return;
        }

        if let Some(mut alternate_screen_state) = self.alternate_screen_state.take() {
            alternate_screen_state.grid.mark_all_damage();
            self.grid = alternate_screen_state.grid;
            self.scroll_region = alternate_screen_state.scroll_region;
            self.cursor.restore(alternate_screen_state.cursor);
            self.move_cursor(self.cursor.position.row, self.cursor.position.col);
        } else {
            self.cursor = Cursor::new();
            self.scroll_region = None;
        }

        self.saved_cursor = None;
        self.modes.alternate_screen = false;
    }

    pub(super) fn active_scroll_region(&self) -> (usize, usize) {
        self.scroll_region.unwrap_or_else(|| {
            let rows = self.grid.rows();
            if rows == 0 {
                (0, 0)
            } else {
                (0, rows - 1)
            }
        })
    }
}

pub(super) fn normalize_scroll_region(
    rows: usize,
    top: usize,
    bottom: usize,
) -> Option<(usize, usize)> {
    if rows == 0 {
        return None;
    }

    let normalized_top = top.max(1);
    let normalized_bottom = if bottom == 0 { rows } else { bottom };
    if normalized_top >= normalized_bottom || normalized_bottom > rows {
        return None;
    }

    let top_index = normalized_top - 1;
    let bottom_index = normalized_bottom - 1;
    if top_index == 0 && bottom_index + 1 == rows {
        None
    } else {
        Some((top_index, bottom_index))
    }
}
