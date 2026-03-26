use iris_core::{MouseButton, MouseModifiers, SelectionInputEvent};

/// Raw mouse events used to drive selection input integration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionMouseEvent {
    /// Mouse button press in terminal cell coordinates.
    Press {
        row: usize,
        col: usize,
        button: MouseButton,
        modifiers: MouseModifiers,
        timestamp_ms: u64,
    },
    /// Mouse move in terminal cell coordinates.
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

/// Configuration for multi-click selection detection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectionMouseEventAdapterConfig {
    /// Maximum interval between matching clicks to count as a multi-click.
    pub multi_click_interval_ms: u64,
}

impl Default for SelectionMouseEventAdapterConfig {
    fn default() -> Self {
        Self {
            multi_click_interval_ms: 500,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LeftPressRecord {
    row: usize,
    col: usize,
    timestamp_ms: u64,
    click_count: u8,
}

/// Adapter that translates raw mouse events into `iris-core`
/// `SelectionInputEvent` values with click-count classification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionMouseEventAdapter {
    config: SelectionMouseEventAdapterConfig,
    last_left_press: Option<LeftPressRecord>,
}

impl SelectionMouseEventAdapter {
    /// Creates a new mouse-event adapter.
    #[must_use]
    pub const fn new(config: SelectionMouseEventAdapterConfig) -> Self {
        Self {
            config,
            last_left_press: None,
        }
    }

    /// Resets multi-click tracking state.
    pub fn reset(&mut self) {
        self.last_left_press = None;
    }

    /// Translates a raw mouse event into a selection input event.
    #[must_use]
    pub fn translate(&mut self, event: SelectionMouseEvent) -> SelectionInputEvent {
        match event {
            SelectionMouseEvent::Press {
                row,
                col,
                button,
                modifiers,
                timestamp_ms,
            } => {
                let click_count = if button == MouseButton::Left {
                    let click_count = self.next_left_click_count(row, col, timestamp_ms);
                    self.last_left_press = Some(LeftPressRecord {
                        row,
                        col,
                        timestamp_ms,
                        click_count,
                    });
                    click_count
                } else {
                    self.last_left_press = None;
                    1
                };

                SelectionInputEvent::Press {
                    row,
                    col,
                    button,
                    modifiers,
                    click_count,
                }
            }
            SelectionMouseEvent::Move {
                row,
                col,
                modifiers,
            } => SelectionInputEvent::Move {
                row,
                col,
                modifiers,
            },
            SelectionMouseEvent::Release {
                row,
                col,
                button,
                modifiers,
            } => SelectionInputEvent::Release {
                row,
                col,
                button,
                modifiers,
            },
        }
    }

    fn next_left_click_count(&self, row: usize, col: usize, timestamp_ms: u64) -> u8 {
        let Some(previous_press) = self.last_left_press else {
            return 1;
        };

        let within_interval = timestamp_ms.saturating_sub(previous_press.timestamp_ms)
            <= self.config.multi_click_interval_ms;
        let same_position = previous_press.row == row && previous_press.col == col;
        if within_interval && same_position {
            previous_press.click_count.saturating_add(1).min(3)
        } else {
            1
        }
    }
}

impl Default for SelectionMouseEventAdapter {
    fn default() -> Self {
        Self::new(SelectionMouseEventAdapterConfig::default())
    }
}

#[cfg(test)]
#[path = "test/selection_input/tests.rs"]
mod tests;
