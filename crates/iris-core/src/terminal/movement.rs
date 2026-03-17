use crate::utils::TAB_WIDTH;

use super::Terminal;

impl Terminal {
    pub(super) fn backspace(&mut self) {
        self.cursor.move_left(1);
    }

    pub(super) fn carriage_return(&mut self) {
        self.cursor.position.col = 0;
    }

    pub(super) fn next_line(&mut self) -> crate::error::Result<()> {
        self.carriage_return();
        self.line_feed()
    }

    pub(super) fn cursor_up(&mut self, count: u16) {
        self.cursor.move_up(usize::from(count.max(1)));
    }

    pub(super) fn cursor_down(&mut self, count: u16) {
        if self.grid.rows() == 0 {
            return;
        }

        self.cursor
            .move_down(usize::from(count.max(1)), self.grid.rows());
    }

    pub(super) fn cursor_forward(&mut self, count: u16) {
        if self.grid.cols() == 0 {
            return;
        }

        self.cursor
            .move_right(usize::from(count.max(1)), self.grid.cols());
    }

    pub(super) fn cursor_back(&mut self, count: u16) {
        self.cursor.move_left(usize::from(count.max(1)));
    }

    pub(super) fn cursor_next_line(&mut self, count: u16) {
        self.cursor_down(count);
        self.carriage_return();
    }

    pub(super) fn cursor_previous_line(&mut self, count: u16) {
        self.cursor_up(count);
        self.carriage_return();
    }

    pub(super) fn cursor_column(&mut self, column: u16) {
        if self.grid.cols() == 0 {
            self.cursor.position.col = 0;
            return;
        }

        self.cursor.position.col =
            usize::from(column.saturating_sub(1)).min(self.grid.cols().saturating_sub(1));
    }

    pub(super) fn vertical_position(&mut self, row: u16) {
        if self.grid.rows() == 0 {
            self.cursor.position.row = 0;
            return;
        }

        self.cursor.position.row =
            usize::from(row.saturating_sub(1)).min(self.grid.rows().saturating_sub(1));
    }

    pub(super) fn cursor_position(&mut self, row: u16, col: u16) {
        self.move_cursor(
            usize::from(row.saturating_sub(1)),
            usize::from(col.saturating_sub(1)),
        );
    }

    pub(super) fn line_feed(&mut self) -> crate::error::Result<()> {
        if self.grid.rows() == 0 {
            return Ok(());
        }

        self.index();

        if self.modes.newline {
            self.carriage_return();
        }

        Ok(())
    }

    pub(super) fn tab(&mut self) {
        let cols = self.grid.cols();
        if cols == 0 {
            return;
        }

        let current = self.cursor.position.col;
        let next_tab_stop = self
            .tab_stops
            .iter()
            .copied()
            .find(|stop| *stop > current)
            .unwrap_or(cols.saturating_sub(1));
        self.cursor.position.col = next_tab_stop.min(cols.saturating_sub(1));
    }

    pub(super) fn back_tab(&mut self, count: u16) {
        let current = self.cursor.position.col;
        let mut cursor_col = current;
        for _ in 0..count.max(1) {
            cursor_col = self
                .tab_stops
                .iter()
                .rev()
                .copied()
                .find(|stop| *stop < cursor_col)
                .unwrap_or(0);
        }
        self.cursor.position.col = cursor_col;
    }

    pub(super) fn set_tab_stop(&mut self) {
        let col = self
            .cursor
            .position
            .col
            .min(self.grid.cols().saturating_sub(1));
        if !self.tab_stops.contains(&col) {
            self.tab_stops.push(col);
            self.tab_stops.sort_unstable();
        }
    }

    pub(super) fn clear_tab_stops(&mut self, mode: u16) {
        match mode {
            0 => {
                let col = self.cursor.position.col;
                self.tab_stops.retain(|stop| *stop != col);
            }
            3 => self.tab_stops.clear(),
            _ => {}
        }
    }

    pub(super) fn resize_tab_stops(&mut self, cols: usize) {
        self.tab_stops.retain(|stop| *stop < cols);
        if self.tab_stops.is_empty() && cols > 0 {
            self.tab_stops = default_tab_stops(cols);
        }
    }
}

pub(super) fn default_tab_stops(cols: usize) -> Vec<usize> {
    (TAB_WIDTH..cols).step_by(TAB_WIDTH).collect()
}
