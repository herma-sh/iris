use crate::clipboard::{
    Clipboard, ClipboardSelection, NoopClipboard, PasteSource, BRACKETED_PASTE_END,
    BRACKETED_PASTE_START,
};
use crate::selection_input::{
    SelectionEventFlow, SelectionEventFlowConfig, SelectionMouseEvent, SelectionMouseEventAdapter,
    SelectionMouseEventAdapterConfig, SelectionWindowGeometry, SelectionWindowMouseEvent,
    SelectionWindowMouseEventAdapter, SelectionWindowMouseEventAdapterConfig,
};
use iris_core::{Action, MouseButton, MouseModifiers, SelectionInputEvent, Terminal};

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

#[test]
fn window_mouse_adapter_translates_window_pixels_into_cell_coordinates() {
    let adapter = SelectionWindowMouseEventAdapter::default();
    let event = SelectionWindowMouseEvent::Press {
        x_px: 18.0,
        y_px: 37.0,
        button: MouseButton::Left,
        modifiers: MouseModifiers::default(),
        timestamp_ms: 1000,
    };
    let geometry = SelectionWindowGeometry {
        origin_x_px: 10.0,
        origin_y_px: 20.0,
        cell_width_px: 8.0,
        cell_height_px: 16.0,
        rows: 3,
        cols: 4,
    };

    let translated = adapter.translate(event, geometry);
    assert_eq!(
        translated,
        Some(SelectionMouseEvent::Press {
            row: 1,
            col: 1,
            button: MouseButton::Left,
            modifiers: MouseModifiers::default(),
            timestamp_ms: 1000,
        })
    );
}

#[test]
fn window_mouse_adapter_returns_none_for_out_of_bounds_when_clamp_is_disabled() {
    let adapter = SelectionWindowMouseEventAdapter::new(SelectionWindowMouseEventAdapterConfig {
        clamp_to_visible_grid: false,
    });
    let event = SelectionWindowMouseEvent::Move {
        x_px: -5.0,
        y_px: 12.0,
        modifiers: MouseModifiers::default(),
    };
    let geometry = SelectionWindowGeometry {
        origin_x_px: 0.0,
        origin_y_px: 0.0,
        cell_width_px: 8.0,
        cell_height_px: 16.0,
        rows: 2,
        cols: 2,
    };

    assert_eq!(adapter.translate(event, geometry), None);
}

#[test]
fn window_mouse_adapter_clamps_out_of_bounds_points_when_enabled() {
    let adapter = SelectionWindowMouseEventAdapter::new(SelectionWindowMouseEventAdapterConfig {
        clamp_to_visible_grid: true,
    });
    let event = SelectionWindowMouseEvent::Release {
        x_px: -10.0,
        y_px: 99.0,
        button: MouseButton::Left,
        modifiers: MouseModifiers::default(),
    };
    let geometry = SelectionWindowGeometry {
        origin_x_px: 0.0,
        origin_y_px: 0.0,
        cell_width_px: 10.0,
        cell_height_px: 10.0,
        rows: 3,
        cols: 4,
    };

    let translated = adapter.translate(event, geometry);
    assert_eq!(
        translated,
        Some(SelectionMouseEvent::Release {
            row: 2,
            col: 0,
            button: MouseButton::Left,
            modifiers: MouseModifiers::default(),
        })
    );
}

#[test]
fn window_mouse_adapter_rejects_invalid_geometry() {
    let adapter = SelectionWindowMouseEventAdapter::default();
    let event = SelectionWindowMouseEvent::Move {
        x_px: 8.0,
        y_px: 8.0,
        modifiers: MouseModifiers::default(),
    };
    let geometry = SelectionWindowGeometry {
        origin_x_px: 0.0,
        origin_y_px: 0.0,
        cell_width_px: 0.0,
        cell_height_px: 10.0,
        rows: 2,
        cols: 2,
    };

    assert_eq!(adapter.translate(event, geometry), None);
}

#[test]
fn selection_event_flow_auto_copies_drag_selection_on_left_release() {
    let mut terminal = Terminal::new(1, 10).unwrap();
    terminal.write_ascii_run(b"abcdefghi").unwrap();
    let mut clipboard = NoopClipboard::new();
    let mut flow = SelectionEventFlow::new(SelectionEventFlowConfig {
        auto_copy_on_select: true,
        ..SelectionEventFlowConfig::default()
    });

    let press = flow
        .handle_mouse_event(
            &mut terminal,
            &mut clipboard,
            SelectionMouseEvent::Press {
                row: 0,
                col: 1,
                button: MouseButton::Left,
                modifiers: MouseModifiers::default(),
                timestamp_ms: 1000,
            },
        )
        .unwrap();
    let move_event = flow
        .handle_mouse_event(
            &mut terminal,
            &mut clipboard,
            SelectionMouseEvent::Move {
                row: 0,
                col: 3,
                modifiers: MouseModifiers::default(),
            },
        )
        .unwrap();
    let release = flow
        .handle_mouse_event(
            &mut terminal,
            &mut clipboard,
            SelectionMouseEvent::Release {
                row: 0,
                col: 3,
                button: MouseButton::Left,
                modifiers: MouseModifiers::default(),
            },
        )
        .unwrap();

    assert!(press.consumed);
    assert!(!press.copied);
    assert!(move_event.consumed);
    assert!(!move_event.copied);
    assert!(release.consumed);
    assert!(release.copied);
    assert_eq!(clipboard.get_text().unwrap().as_deref(), Some("bcd"));
}

#[test]
fn selection_event_flow_auto_copies_double_click_word_selection() {
    let mut terminal = Terminal::new(1, 8).unwrap();
    terminal.write_ascii_run(b"world").unwrap();
    let mut clipboard = NoopClipboard::new();
    let mut flow = SelectionEventFlow::new(SelectionEventFlowConfig {
        auto_copy_on_select: true,
        ..SelectionEventFlowConfig::default()
    });

    let first = flow
        .handle_mouse_event(
            &mut terminal,
            &mut clipboard,
            SelectionMouseEvent::Press {
                row: 0,
                col: 1,
                button: MouseButton::Left,
                modifiers: MouseModifiers::default(),
                timestamp_ms: 2000,
            },
        )
        .unwrap();
    let second = flow
        .handle_mouse_event(
            &mut terminal,
            &mut clipboard,
            SelectionMouseEvent::Press {
                row: 0,
                col: 1,
                button: MouseButton::Left,
                modifiers: MouseModifiers::default(),
                timestamp_ms: 2200,
            },
        )
        .unwrap();

    assert!(first.consumed);
    assert!(!first.copied);
    assert!(second.consumed);
    assert!(second.copied);
    assert_eq!(clipboard.get_text().unwrap().as_deref(), Some("world"));
}

#[test]
fn selection_event_flow_does_not_auto_copy_when_disabled() {
    let mut terminal = Terminal::new(1, 8).unwrap();
    terminal.write_ascii_run(b"abcdefg").unwrap();
    let mut clipboard = NoopClipboard::new();
    let mut flow = SelectionEventFlow::default();

    let _ = flow
        .handle_mouse_event(
            &mut terminal,
            &mut clipboard,
            SelectionMouseEvent::Press {
                row: 0,
                col: 1,
                button: MouseButton::Left,
                modifiers: MouseModifiers::default(),
                timestamp_ms: 10,
            },
        )
        .unwrap();
    let _ = flow
        .handle_mouse_event(
            &mut terminal,
            &mut clipboard,
            SelectionMouseEvent::Move {
                row: 0,
                col: 3,
                modifiers: MouseModifiers::default(),
            },
        )
        .unwrap();
    let release = flow
        .handle_mouse_event(
            &mut terminal,
            &mut clipboard,
            SelectionMouseEvent::Release {
                row: 0,
                col: 3,
                button: MouseButton::Left,
                modifiers: MouseModifiers::default(),
            },
        )
        .unwrap();

    assert!(release.consumed);
    assert!(!release.copied);
    assert_eq!(clipboard.get_text().unwrap(), None);
}

#[test]
fn selection_event_flow_delegates_paste_to_configured_source() {
    let mut terminal = Terminal::new(1, 8).unwrap();
    terminal
        .apply_action(Action::SetModes {
            private: true,
            modes: vec![2004].into(),
        })
        .unwrap();

    let mut clipboard = NoopClipboard::with_primary_selection();
    clipboard.set_primary("").unwrap();
    clipboard.set_text("fallback").unwrap();
    let flow = SelectionEventFlow::new(SelectionEventFlowConfig {
        copy_target: ClipboardSelection::Clipboard,
        paste_source: PasteSource::PrimaryThenClipboard,
        auto_copy_on_select: false,
        ..SelectionEventFlowConfig::default()
    });

    let payload = flow
        .paste_terminal_bytes(&terminal, &clipboard)
        .unwrap()
        .expect("paste source should produce bytes");
    let expected = format!("{BRACKETED_PASTE_START}fallback{BRACKETED_PASTE_END}");
    assert_eq!(payload, expected.into_bytes());
}

#[test]
fn selection_event_flow_handles_window_mouse_events_end_to_end() {
    let mut terminal = Terminal::new(1, 10).unwrap();
    terminal.write_ascii_run(b"abcdefghi").unwrap();
    let mut clipboard = NoopClipboard::new();
    let mut flow = SelectionEventFlow::new(SelectionEventFlowConfig {
        auto_copy_on_select: true,
        ..SelectionEventFlowConfig::default()
    });
    let geometry = SelectionWindowGeometry {
        origin_x_px: 0.0,
        origin_y_px: 0.0,
        cell_width_px: 10.0,
        cell_height_px: 20.0,
        rows: 1,
        cols: 10,
    };

    let press = flow
        .handle_window_mouse_event(
            &mut terminal,
            &mut clipboard,
            SelectionWindowMouseEvent::Press {
                x_px: 15.0,
                y_px: 10.0,
                button: MouseButton::Left,
                modifiers: MouseModifiers::default(),
                timestamp_ms: 1000,
            },
            geometry,
        )
        .unwrap();
    let move_event = flow
        .handle_window_mouse_event(
            &mut terminal,
            &mut clipboard,
            SelectionWindowMouseEvent::Move {
                x_px: 35.0,
                y_px: 10.0,
                modifiers: MouseModifiers::default(),
            },
            geometry,
        )
        .unwrap();
    let release = flow
        .handle_window_mouse_event(
            &mut terminal,
            &mut clipboard,
            SelectionWindowMouseEvent::Release {
                x_px: 35.0,
                y_px: 10.0,
                button: MouseButton::Left,
                modifiers: MouseModifiers::default(),
            },
            geometry,
        )
        .unwrap();

    assert!(press.consumed);
    assert!(!press.copied);
    assert!(move_event.consumed);
    assert!(!move_event.copied);
    assert!(release.consumed);
    assert!(release.copied);
    assert_eq!(clipboard.get_text().unwrap().as_deref(), Some("bcd"));
}

#[test]
fn selection_event_flow_ignores_window_events_that_do_not_map_to_cells() {
    let mut terminal = Terminal::new(1, 10).unwrap();
    let mut clipboard = NoopClipboard::new();
    let mut flow = SelectionEventFlow::new(SelectionEventFlowConfig {
        window_mouse: SelectionWindowMouseEventAdapterConfig {
            clamp_to_visible_grid: false,
        },
        auto_copy_on_select: true,
        ..SelectionEventFlowConfig::default()
    });
    let geometry = SelectionWindowGeometry {
        origin_x_px: 0.0,
        origin_y_px: 0.0,
        cell_width_px: 10.0,
        cell_height_px: 20.0,
        rows: 1,
        cols: 10,
    };

    let outcome = flow
        .handle_window_mouse_event(
            &mut terminal,
            &mut clipboard,
            SelectionWindowMouseEvent::Press {
                x_px: -1.0,
                y_px: 10.0,
                button: MouseButton::Left,
                modifiers: MouseModifiers::default(),
                timestamp_ms: 100,
            },
            geometry,
        )
        .unwrap();

    assert!(!outcome.consumed);
    assert!(!outcome.copied);
}
