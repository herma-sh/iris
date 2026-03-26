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

/// Mouse events expressed in window pixel coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SelectionWindowMouseEvent {
    /// Mouse button press in window pixel coordinates.
    Press {
        x_px: f32,
        y_px: f32,
        button: MouseButton,
        modifiers: MouseModifiers,
        timestamp_ms: u64,
    },
    /// Mouse move in window pixel coordinates.
    Move {
        x_px: f32,
        y_px: f32,
        modifiers: MouseModifiers,
    },
    /// Mouse button release in window pixel coordinates.
    Release {
        x_px: f32,
        y_px: f32,
        button: MouseButton,
        modifiers: MouseModifiers,
    },
}

/// Terminal cell geometry used to map window pixels into cell coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SelectionWindowGeometry {
    /// Left pixel origin of the terminal grid inside the window.
    pub origin_x_px: f32,
    /// Top pixel origin of the terminal grid inside the window.
    pub origin_y_px: f32,
    /// Cell width in pixels.
    pub cell_width_px: f32,
    /// Cell height in pixels.
    pub cell_height_px: f32,
    /// Number of visible terminal rows.
    pub rows: usize,
    /// Number of visible terminal columns.
    pub cols: usize,
}

impl SelectionWindowGeometry {
    fn is_valid(self) -> bool {
        self.origin_x_px.is_finite()
            && self.origin_y_px.is_finite()
            && self.cell_width_px.is_finite()
            && self.cell_width_px > 0.0
            && self.cell_height_px.is_finite()
            && self.cell_height_px > 0.0
            && self.rows > 0
            && self.cols > 0
    }
}

/// Configuration for window-to-cell selection event adaptation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectionWindowMouseEventAdapterConfig {
    /// Clamps out-of-bounds points to the nearest visible cell when true.
    pub clamp_to_visible_grid: bool,
}

impl Default for SelectionWindowMouseEventAdapterConfig {
    fn default() -> Self {
        Self {
            clamp_to_visible_grid: true,
        }
    }
}

/// Adapter that maps window pixel mouse events to terminal cell events.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectionWindowMouseEventAdapter {
    config: SelectionWindowMouseEventAdapterConfig,
}

impl SelectionWindowMouseEventAdapter {
    /// Creates a new window-mouse event adapter.
    #[must_use]
    pub const fn new(config: SelectionWindowMouseEventAdapterConfig) -> Self {
        Self { config }
    }

    /// Translates a window-space mouse event into a cell-space selection event.
    #[must_use]
    pub fn translate(
        &self,
        event: SelectionWindowMouseEvent,
        geometry: SelectionWindowGeometry,
    ) -> Option<SelectionMouseEvent> {
        match event {
            SelectionWindowMouseEvent::Press {
                x_px,
                y_px,
                button,
                modifiers,
                timestamp_ms,
            } => {
                let (row, col) = self.window_point_to_cell(x_px, y_px, geometry)?;
                Some(SelectionMouseEvent::Press {
                    row,
                    col,
                    button,
                    modifiers,
                    timestamp_ms,
                })
            }
            SelectionWindowMouseEvent::Move {
                x_px,
                y_px,
                modifiers,
            } => {
                let (row, col) = self.window_point_to_cell(x_px, y_px, geometry)?;
                Some(SelectionMouseEvent::Move {
                    row,
                    col,
                    modifiers,
                })
            }
            SelectionWindowMouseEvent::Release {
                x_px,
                y_px,
                button,
                modifiers,
            } => {
                let (row, col) = self.window_point_to_cell(x_px, y_px, geometry)?;
                Some(SelectionMouseEvent::Release {
                    row,
                    col,
                    button,
                    modifiers,
                })
            }
        }
    }

    fn window_point_to_cell(
        &self,
        x_px: f32,
        y_px: f32,
        geometry: SelectionWindowGeometry,
    ) -> Option<(usize, usize)> {
        if !geometry.is_valid() || !x_px.is_finite() || !y_px.is_finite() {
            return None;
        }

        let rel_x = x_px - geometry.origin_x_px;
        let rel_y = y_px - geometry.origin_y_px;
        let mut col = (rel_x / geometry.cell_width_px).floor() as isize;
        let mut row = (rel_y / geometry.cell_height_px).floor() as isize;
        let max_col = geometry.cols as isize - 1;
        let max_row = geometry.rows as isize - 1;

        if self.config.clamp_to_visible_grid {
            col = col.clamp(0, max_col);
            row = row.clamp(0, max_row);
        } else if col < 0 || row < 0 || col > max_col || row > max_row {
            return None;
        }

        Some((row as usize, col as usize))
    }
}

impl Default for SelectionWindowMouseEventAdapter {
    fn default() -> Self {
        Self::new(SelectionWindowMouseEventAdapterConfig::default())
    }
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
    /// Window-space adapter settings for pixel-to-cell translation.
    pub window_mouse: SelectionWindowMouseEventAdapterConfig,
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
            window_mouse: SelectionWindowMouseEventAdapterConfig::default(),
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
    window_mouse_adapter: SelectionWindowMouseEventAdapter,
    clipboard_controller: SelectionClipboardController,
    auto_copy_on_select: bool,
}

impl SelectionEventFlow {
    /// Creates a configured selection event flow.
    #[must_use]
    pub const fn new(config: SelectionEventFlowConfig) -> Self {
        Self {
            mouse_adapter: SelectionMouseEventAdapter::new(config.mouse),
            window_mouse_adapter: SelectionWindowMouseEventAdapter::new(config.window_mouse),
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

    /// Handles a window-space mouse event by translating it to cell
    /// coordinates and applying normal selection event flow.
    pub fn handle_window_mouse_event(
        &mut self,
        terminal: &mut Terminal,
        clipboard: &mut impl Clipboard,
        event: SelectionWindowMouseEvent,
        geometry: SelectionWindowGeometry,
    ) -> Result<SelectionEventFlowOutcome> {
        let Some(cell_event) = self.window_mouse_adapter.translate(event, geometry) else {
            return Ok(SelectionEventFlowOutcome::default());
        };

        self.handle_mouse_event(terminal, clipboard, cell_event)
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
