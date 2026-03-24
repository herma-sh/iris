use crate::selection::SelectionKind;
use crate::terminal::Terminal;

/// Mouse buttons used by terminal input handling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
}

/// Modifier state carried with mouse input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MouseModifiers {
    pub alt: bool,
    pub ctrl: bool,
    pub shift: bool,
}

/// Mouse events consumed by selection input handling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionInputEvent {
    /// Mouse button press in terminal cell coordinates.
    Press {
        row: usize,
        col: usize,
        button: MouseButton,
        modifiers: MouseModifiers,
        click_count: u8,
    },
    /// Mouse movement in terminal cell coordinates.
    Move {
        row: usize,
        col: usize,
        modifiers: MouseModifiers,
    },
    /// Mouse button release in terminal cell coordinates.
    Release {
        row: usize,
        col: usize,
        button: MouseButton,
        modifiers: MouseModifiers,
    },
}

/// Stateful mouse-selection input controller.
///
/// This bridges mouse interaction semantics into `Terminal` selection APIs
/// without depending on any UI/windowing framework.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SelectionInputState {
    drag_selection_kind: Option<SelectionKind>,
    primary_button_down: bool,
}

impl SelectionInputState {
    /// Creates a new selection input state.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            drag_selection_kind: None,
            primary_button_down: false,
        }
    }

    /// Applies a mouse selection event to terminal selection state.
    ///
    /// Returns `true` when the event was consumed by selection handling.
    pub fn handle_event(&mut self, terminal: &mut Terminal, event: SelectionInputEvent) -> bool {
        match event {
            SelectionInputEvent::Press {
                row,
                col,
                button,
                modifiers,
                click_count,
            } => self.handle_press(terminal, row, col, button, modifiers, click_count),
            SelectionInputEvent::Move { row, col, .. } => self.handle_move(terminal, row, col),
            SelectionInputEvent::Release {
                row, col, button, ..
            } => self.handle_release(terminal, row, col, button),
        }
    }

    fn handle_press(
        &mut self,
        terminal: &mut Terminal,
        row: usize,
        col: usize,
        button: MouseButton,
        modifiers: MouseModifiers,
        click_count: u8,
    ) -> bool {
        if button != MouseButton::Left {
            return false;
        }

        self.primary_button_down = false;
        self.drag_selection_kind = None;

        let normalized_click_count = click_count.max(1);
        if normalized_click_count >= 3 {
            terminal.select_line(row);
            return true;
        }
        if normalized_click_count == 2 {
            terminal.select_word(row, col);
            return true;
        }

        let selection_kind = if modifiers.alt {
            SelectionKind::Block
        } else {
            SelectionKind::Simple
        };
        terminal.start_selection(row, col, selection_kind);
        self.drag_selection_kind = Some(selection_kind);
        self.primary_button_down = true;
        true
    }

    fn handle_move(&mut self, terminal: &mut Terminal, row: usize, col: usize) -> bool {
        if !self.primary_button_down || self.drag_selection_kind.is_none() {
            return false;
        }

        terminal.extend_selection(row, col);
        true
    }

    fn handle_release(
        &mut self,
        terminal: &mut Terminal,
        row: usize,
        col: usize,
        button: MouseButton,
    ) -> bool {
        if button != MouseButton::Left || !self.primary_button_down {
            return false;
        }

        if self.drag_selection_kind.is_some() {
            terminal.extend_selection(row, col);
            terminal.complete_selection();
        }
        self.drag_selection_kind = None;
        self.primary_button_down = false;
        true
    }
}

#[cfg(test)]
#[path = "test/input/tests.rs"]
mod tests;
