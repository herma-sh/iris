use crate::clipboard::{Clipboard, ClipboardSelection, PasteSource, SelectionClipboardController};
use crate::error::Result;
use iris_core::{MouseButton, MouseModifiers, SelectionInputEvent, Terminal};

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

/// Configuration for end-to-end selection event flow handling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectionEventFlowConfig {
    /// Multi-click adapter settings for raw mouse press classification.
    pub mouse: SelectionMouseEventAdapterConfig,
    /// Clipboard target for copy operations.
    pub copy_target: ClipboardSelection,
    /// Clipboard source strategy for paste operations.
    pub paste_source: PasteSource,
    /// Enables copy-on-selection behavior in `handle_mouse_event`.
    pub auto_copy_on_select: bool,
}

impl Default for SelectionEventFlowConfig {
    fn default() -> Self {
        Self {
            mouse: SelectionMouseEventAdapterConfig::default(),
            copy_target: ClipboardSelection::Clipboard,
            paste_source: PasteSource::PrimaryThenClipboard,
            auto_copy_on_select: false,
        }
    }
}

/// Outcome for a raw mouse event handled through `SelectionEventFlow`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SelectionEventFlowOutcome {
    /// Whether selection handling consumed the input event.
    pub consumed: bool,
    /// Whether selection text was copied to the configured clipboard target.
    pub copied: bool,
}

/// End-to-end selection event flow for platform integration.
///
/// This composes raw mouse adaptation, terminal selection updates, and
/// configured clipboard copy/paste behavior.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionEventFlow {
    mouse_adapter: SelectionMouseEventAdapter,
    clipboard_controller: SelectionClipboardController,
    auto_copy_on_select: bool,
}

impl SelectionEventFlow {
    /// Creates a configured selection event flow.
    #[must_use]
    pub const fn new(config: SelectionEventFlowConfig) -> Self {
        Self {
            mouse_adapter: SelectionMouseEventAdapter::new(config.mouse),
            clipboard_controller: SelectionClipboardController::new(
                config.copy_target,
                config.paste_source,
            ),
            auto_copy_on_select: config.auto_copy_on_select,
        }
    }

    /// Handles a raw mouse event against terminal selection state.
    ///
    /// When `auto_copy_on_select` is enabled, this also performs configured
    /// selection copy on left-button release or double/triple-click press
    /// events that complete selection immediately.
    pub fn handle_mouse_event(
        &mut self,
        terminal: &mut Terminal,
        clipboard: &mut impl Clipboard,
        event: SelectionMouseEvent,
    ) -> Result<SelectionEventFlowOutcome> {
        let selection_event = self.mouse_adapter.translate(event);
        let copy_after_event = self.should_copy_after_event(selection_event);
        let consumed = self
            .clipboard_controller
            .handle_selection_input_event(terminal, selection_event);
        let copied = if consumed && copy_after_event {
            self.clipboard_controller
                .copy_selection(terminal, clipboard)?
        } else {
            false
        };

        Ok(SelectionEventFlowOutcome { consumed, copied })
    }

    /// Copies the current terminal selection to the configured target.
    pub fn copy_selection(
        &self,
        terminal: &Terminal,
        clipboard: &mut impl Clipboard,
    ) -> Result<bool> {
        self.clipboard_controller
            .copy_selection(terminal, clipboard)
    }

    /// Reads and encodes terminal paste bytes from the configured source.
    pub fn paste_terminal_bytes(
        &self,
        terminal: &Terminal,
        clipboard: &impl Clipboard,
    ) -> Result<Option<Vec<u8>>> {
        self.clipboard_controller
            .paste_terminal_bytes(terminal, clipboard)
    }

    /// Returns the configured copy target.
    #[must_use]
    pub const fn copy_target(&self) -> ClipboardSelection {
        self.clipboard_controller.copy_target()
    }

    /// Returns the configured paste source.
    #[must_use]
    pub const fn paste_source(&self) -> PasteSource {
        self.clipboard_controller.paste_source()
    }

    /// Returns whether auto-copy-on-select is enabled.
    #[must_use]
    pub const fn auto_copy_on_select(&self) -> bool {
        self.auto_copy_on_select
    }

    /// Resets multi-click tracking state.
    pub fn reset_click_state(&mut self) {
        self.mouse_adapter.reset();
    }

    const fn should_copy_after_event(&self, event: SelectionInputEvent) -> bool {
        if !self.auto_copy_on_select {
            return false;
        }

        match event {
            SelectionInputEvent::Release {
                button: MouseButton::Left,
                ..
            } => true,
            SelectionInputEvent::Press {
                button: MouseButton::Left,
                click_count,
                ..
            } => click_count >= 2,
            _ => false,
        }
    }
}

impl Default for SelectionEventFlow {
    fn default() -> Self {
        Self::new(SelectionEventFlowConfig::default())
    }
}

#[cfg(test)]
#[path = "test/selection_input/tests.rs"]
mod tests;
