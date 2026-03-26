use crate::selection_input::{
    SelectionMouseEvent, SelectionMouseEventAdapter, SelectionMouseEventAdapterConfig,
};
use iris_core::{MouseButton, MouseModifiers, SelectionInputEvent};

#[test]
fn mouse_adapter_maps_single_left_press_to_click_count_one() {
    let mut adapter = SelectionMouseEventAdapter::default();

    let translated = adapter.translate(SelectionMouseEvent::Press {
        row: 2,
        col: 4,
        button: MouseButton::Left,
        modifiers: MouseModifiers::default(),
        timestamp_ms: 1000,
    });

    assert_eq!(
        translated,
        SelectionInputEvent::Press {
            row: 2,
            col: 4,
            button: MouseButton::Left,
            modifiers: MouseModifiers::default(),
            click_count: 1,
        }
    );
}

#[test]
fn mouse_adapter_counts_double_and_triple_clicks_within_interval() {
    let mut adapter = SelectionMouseEventAdapter::new(SelectionMouseEventAdapterConfig {
        multi_click_interval_ms: 400,
    });

    let first = adapter.translate(SelectionMouseEvent::Press {
        row: 1,
        col: 1,
        button: MouseButton::Left,
        modifiers: MouseModifiers::default(),
        timestamp_ms: 2000,
    });
    let second = adapter.translate(SelectionMouseEvent::Press {
        row: 1,
        col: 1,
        button: MouseButton::Left,
        modifiers: MouseModifiers::default(),
        timestamp_ms: 2200,
    });
    let third = adapter.translate(SelectionMouseEvent::Press {
        row: 1,
        col: 1,
        button: MouseButton::Left,
        modifiers: MouseModifiers::default(),
        timestamp_ms: 2350,
    });
    let fourth = adapter.translate(SelectionMouseEvent::Press {
        row: 1,
        col: 1,
        button: MouseButton::Left,
        modifiers: MouseModifiers::default(),
        timestamp_ms: 2390,
    });

    assert_eq!(
        first,
        SelectionInputEvent::Press {
            row: 1,
            col: 1,
            button: MouseButton::Left,
            modifiers: MouseModifiers::default(),
            click_count: 1,
        }
    );
    assert_eq!(
        second,
        SelectionInputEvent::Press {
            row: 1,
            col: 1,
            button: MouseButton::Left,
            modifiers: MouseModifiers::default(),
            click_count: 2,
        }
    );
    assert_eq!(
        third,
        SelectionInputEvent::Press {
            row: 1,
            col: 1,
            button: MouseButton::Left,
            modifiers: MouseModifiers::default(),
            click_count: 3,
        }
    );
    assert_eq!(
        fourth,
        SelectionInputEvent::Press {
            row: 1,
            col: 1,
            button: MouseButton::Left,
            modifiers: MouseModifiers::default(),
            click_count: 3,
        }
    );
}

#[test]
fn mouse_adapter_resets_click_count_for_interval_and_position_changes() {
    let mut adapter = SelectionMouseEventAdapter::new(SelectionMouseEventAdapterConfig {
        multi_click_interval_ms: 250,
    });

    let _ = adapter.translate(SelectionMouseEvent::Press {
        row: 0,
        col: 0,
        button: MouseButton::Left,
        modifiers: MouseModifiers::default(),
        timestamp_ms: 100,
    });
    let after_interval = adapter.translate(SelectionMouseEvent::Press {
        row: 0,
        col: 0,
        button: MouseButton::Left,
        modifiers: MouseModifiers::default(),
        timestamp_ms: 500,
    });
    let new_position = adapter.translate(SelectionMouseEvent::Press {
        row: 0,
        col: 1,
        button: MouseButton::Left,
        modifiers: MouseModifiers::default(),
        timestamp_ms: 600,
    });

    assert_eq!(
        after_interval,
        SelectionInputEvent::Press {
            row: 0,
            col: 0,
            button: MouseButton::Left,
            modifiers: MouseModifiers::default(),
            click_count: 1,
        }
    );
    assert_eq!(
        new_position,
        SelectionInputEvent::Press {
            row: 0,
            col: 1,
            button: MouseButton::Left,
            modifiers: MouseModifiers::default(),
            click_count: 1,
        }
    );
}

#[test]
fn mouse_adapter_passes_move_and_release_events_through() {
    let mut adapter = SelectionMouseEventAdapter::default();
    let modifiers = MouseModifiers {
        alt: true,
        ctrl: false,
        shift: true,
    };

    let move_event = adapter.translate(SelectionMouseEvent::Move {
        row: 3,
        col: 5,
        modifiers,
    });
    let release_event = adapter.translate(SelectionMouseEvent::Release {
        row: 3,
        col: 5,
        button: MouseButton::Left,
        modifiers,
    });

    assert_eq!(
        move_event,
        SelectionInputEvent::Move {
            row: 3,
            col: 5,
            modifiers,
        }
    );
    assert_eq!(
        release_event,
        SelectionInputEvent::Release {
            row: 3,
            col: 5,
            button: MouseButton::Left,
            modifiers,
        }
    );
}

#[test]
fn mouse_adapter_resets_left_click_sequence_after_non_left_press() {
    let mut adapter = SelectionMouseEventAdapter::default();
    let _ = adapter.translate(SelectionMouseEvent::Press {
        row: 4,
        col: 4,
        button: MouseButton::Left,
        modifiers: MouseModifiers::default(),
        timestamp_ms: 10,
    });
    let _ = adapter.translate(SelectionMouseEvent::Press {
        row: 4,
        col: 4,
        button: MouseButton::Right,
        modifiers: MouseModifiers::default(),
        timestamp_ms: 20,
    });
    let next_left = adapter.translate(SelectionMouseEvent::Press {
        row: 4,
        col: 4,
        button: MouseButton::Left,
        modifiers: MouseModifiers::default(),
        timestamp_ms: 30,
    });

    assert_eq!(
        next_left,
        SelectionInputEvent::Press {
            row: 4,
            col: 4,
            button: MouseButton::Left,
            modifiers: MouseModifiers::default(),
            click_count: 1,
        }
    );
}
