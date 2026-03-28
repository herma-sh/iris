use iris_core::cell::{Cell, CellAttrs, Color};
use iris_core::damage::{DamageRegion, ScrollDelta};
use iris_core::parser::Action;
use iris_core::selection::SelectionKind;
use iris_core::terminal::Terminal;
use iris_core::SearchConfig;

use super::{
    partial_scroll_copy_region, scroll_copy_region, TerminalRenderer, TerminalRendererConfig,
};
use crate::error::Error;
use crate::font::FontRasterizerConfig;
use crate::renderer::{Renderer, RendererConfig};
use crate::texture::{TextureSurfaceConfig, TextureSurfaceSize};
use crate::theme::{Theme, ThemeColor};
use crate::TextUniforms;

fn cell_region_has_non_background(
    pixels: &[u8],
    surface_size: TextureSurfaceSize,
    cell_origin: (usize, usize),
    cell_size: (usize, usize),
    background: [u8; 4],
) -> bool {
    let row_stride = surface_size.width as usize * background.len();
    let start_x = cell_origin.0 * cell_size.0;
    let start_y = cell_origin.1 * cell_size.1;

    for row in start_y..start_y + cell_size.1 {
        for col in start_x..start_x + cell_size.0 {
            let offset = row * row_stride + col * background.len();
            if pixels[offset..offset + background.len()] != background {
                return true;
            }
        }
    }

    false
}

fn cell_region_matches_background(
    pixels: &[u8],
    surface_size: TextureSurfaceSize,
    cell_origin: (usize, usize),
    cell_size: (usize, usize),
    background: [u8; 4],
) -> bool {
    let row_stride = surface_size.width as usize * background.len();
    let start_x = cell_origin.0 * cell_size.0;
    let start_y = cell_origin.1 * cell_size.1;

    for row in start_y..start_y + cell_size.1 {
        for col in start_x..start_x + cell_size.0 {
            let offset = row * row_stride + col * background.len();
            if pixels[offset..offset + background.len()] != background {
                return false;
            }
        }
    }

    true
}

fn pixel_at(pixels: &[u8], surface_size: TextureSurfaceSize, position: (usize, usize)) -> [u8; 4] {
    let row_stride = surface_size.width as usize * 4;
    let offset = position.1 * row_stride + position.0 * 4;
    [
        pixels[offset],
        pixels[offset + 1],
        pixels[offset + 2],
        pixels[offset + 3],
    ]
}

fn band_matches_background(
    pixels: &[u8],
    surface_size: TextureSurfaceSize,
    row_range: std::ops::Range<usize>,
    background: [u8; 4],
) -> bool {
    let row_stride = surface_size.width as usize * background.len();
    for row in row_range {
        for col in 0..surface_size.width as usize {
            let offset = row * row_stride + col * background.len();
            if pixels[offset..offset + background.len()] != background {
                return false;
            }
        }
    }

    true
}

#[test]
fn terminal_renderer_prepares_and_renders_terminal_state() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(32, 16).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let mut terminal_renderer = match TerminalRenderer::new(
        &renderer,
        surface.format(),
        TerminalRendererConfig {
            text: crate::text_renderer::TextRendererConfig {
                theme: Theme {
                    background: ThemeColor::rgb(0x00, 0x00, 0x00),
                    cursor: ThemeColor::rgb(0xff, 0x00, 0x00),
                    ..Theme::default()
                },
                uniforms: TextUniforms::new([32.0, 16.0], [16.0, 16.0], 0.0),
                ..Default::default()
            },
            ..Default::default()
        },
    ) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoUsableSystemFont) => return,
        Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
    };
    let mut terminal = Terminal::new(1, 2).expect("terminal should be created");
    terminal
        .write_char('A')
        .expect("terminal write should succeed");

    terminal_renderer
        .prepare_terminal(&renderer, &terminal)
        .expect("terminal frame should prepare");
    terminal_renderer.render_to_texture_surface(&renderer, &surface);

    let pixels = crate::test_support::read_texture_surface(&renderer, &surface);
    let background = crate::test_support::bgra_pixel(terminal_renderer.theme().background);
    assert_eq!(terminal_renderer.instance_count(), 1);
    assert_eq!(terminal_renderer.cursor_instance_count(), 1);
    assert!(
        pixels
            .chunks_exact(background.len())
            .any(|pixel| pixel != background),
        "prepared terminal state should render text and cursor pixels"
    );
}

#[test]
fn terminal_renderer_updates_selection_highlight_without_grid_or_cursor_changes() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(32, 16).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let mut terminal_renderer = match TerminalRenderer::new(
        &renderer,
        surface.format(),
        TerminalRendererConfig {
            text: crate::text_renderer::TextRendererConfig {
                theme: Theme {
                    foreground: ThemeColor::rgb(0xff, 0xff, 0xff),
                    background: ThemeColor::rgb(0x00, 0x00, 0x00),
                    ..Theme::default()
                },
                uniforms: TextUniforms::new([32.0, 16.0], [16.0, 16.0], 0.0),
                ..Default::default()
            },
            ..Default::default()
        },
    ) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoUsableSystemFont) => return,
        Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
    };
    let mut terminal = Terminal::new(1, 2).expect("terminal should be created");
    terminal
        .write_char('A')
        .expect("terminal write should succeed");
    terminal.cursor.visible = false;

    terminal_renderer
        .prepare_terminal(&renderer, &terminal)
        .expect("initial terminal frame should prepare");
    terminal_renderer.render_to_texture_surface(&renderer, &surface);
    let unselected_pixels = crate::test_support::read_texture_surface(&renderer, &surface);

    let _ = terminal.take_damage();
    let _ = terminal.take_scroll_delta();
    terminal.start_selection(0, 0, SelectionKind::Simple);
    terminal.extend_selection(0, 0);
    terminal.complete_selection();

    terminal_renderer
        .update_terminal(&renderer, &mut terminal)
        .expect("selection-only update should repaint highlighted cells");
    terminal_renderer.render_to_texture_surface(&renderer, &surface);
    let selected_pixels = crate::test_support::read_texture_surface(&renderer, &surface);

    terminal.cancel_selection();
    terminal_renderer
        .update_terminal(&renderer, &mut terminal)
        .expect("selection-clear update should repaint previously selected cells");
    terminal_renderer.render_to_texture_surface(&renderer, &surface);
    let cleared_pixels = crate::test_support::read_texture_surface(&renderer, &surface);

    let unselected_color = pixel_at(&unselected_pixels, surface.size(), (1, 1));
    let selected_color = pixel_at(&selected_pixels, surface.size(), (1, 1));
    let re_cleared_color = pixel_at(&cleared_pixels, surface.size(), (1, 1));
    let selected_background = crate::test_support::bgra_pixel(terminal_renderer.theme().foreground);
    let background = crate::test_support::bgra_pixel(terminal_renderer.theme().background);

    assert_eq!(
        unselected_color, background,
        "unselected terminal cells should render with the theme background color"
    );
    assert_eq!(
        selected_color, selected_background,
        "selection-only updates should swap selected cell background/foreground colors"
    );
    assert_eq!(
        re_cleared_color, background,
        "clearing selection should repaint selected cells back to the normal background"
    );
}

#[test]
fn terminal_renderer_updates_search_highlight_without_grid_or_cursor_changes() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(32, 16).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let mut terminal_renderer = match TerminalRenderer::new(
        &renderer,
        surface.format(),
        TerminalRendererConfig {
            text: crate::text_renderer::TextRendererConfig {
                theme: Theme {
                    foreground: ThemeColor::rgb(0xff, 0xff, 0xff),
                    background: ThemeColor::rgb(0x00, 0x00, 0x00),
                    ..Theme::default()
                },
                uniforms: TextUniforms::new([32.0, 16.0], [16.0, 16.0], 0.0),
                ..Default::default()
            },
            ..Default::default()
        },
    ) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoUsableSystemFont) => return,
        Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
    };
    let mut terminal = Terminal::new(1, 2).expect("terminal should be created");
    terminal
        .write_char('A')
        .expect("terminal write should succeed");
    terminal.cursor.visible = false;

    let search = SearchConfig {
        pattern: "A".to_string(),
        case_sensitive: true,
        use_regex: false,
        whole_word: false,
        wrap: true,
    };

    terminal_renderer
        .prepare_terminal_with_search(&renderer, &terminal, None)
        .expect("initial terminal frame should prepare");
    terminal_renderer.render_to_texture_surface(&renderer, &surface);
    let unhighlighted_pixels = crate::test_support::read_texture_surface(&renderer, &surface);

    let _ = terminal.take_damage();
    let _ = terminal.take_scroll_delta();
    terminal_renderer
        .update_terminal_with_search(&renderer, &mut terminal, Some(&search))
        .expect("search-only update should repaint highlighted cells");
    terminal_renderer.render_to_texture_surface(&renderer, &surface);
    let highlighted_pixels = crate::test_support::read_texture_surface(&renderer, &surface);

    terminal_renderer
        .update_terminal_with_search(&renderer, &mut terminal, None)
        .expect("search-clear update should repaint previously highlighted cells");
    terminal_renderer.render_to_texture_surface(&renderer, &surface);
    let cleared_pixels = crate::test_support::read_texture_surface(&renderer, &surface);

    let unhighlighted_color = pixel_at(&unhighlighted_pixels, surface.size(), (1, 1));
    let highlighted_color = pixel_at(&highlighted_pixels, surface.size(), (1, 1));
    let cleared_color = pixel_at(&cleared_pixels, surface.size(), (1, 1));
    let highlighted_background =
        crate::test_support::bgra_pixel(terminal_renderer.theme().foreground);
    let background = crate::test_support::bgra_pixel(terminal_renderer.theme().background);

    assert_eq!(
        unhighlighted_color, background,
        "unhighlighted cells should render with the theme background color"
    );
    assert_eq!(
        highlighted_color, highlighted_background,
        "search-only updates should apply highlight colors for visible matches"
    );
    assert_eq!(
        cleared_color, background,
        "clearing search highlighting should repaint cells back to the normal background"
    );
}

#[test]
fn terminal_renderer_skips_search_highlight_when_viewport_detached_from_live_grid() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(32, 16).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let mut terminal_renderer = match TerminalRenderer::new(
        &renderer,
        surface.format(),
        TerminalRendererConfig {
            text: crate::text_renderer::TextRendererConfig {
                theme: Theme {
                    foreground: ThemeColor::rgb(0xff, 0xff, 0xff),
                    background: ThemeColor::rgb(0x00, 0x00, 0x00),
                    ..Theme::default()
                },
                uniforms: TextUniforms::new([32.0, 16.0], [16.0, 16.0], 0.0),
                ..Default::default()
            },
            ..Default::default()
        },
    ) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoUsableSystemFont) => return,
        Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
    };
    let mut terminal = Terminal::new(1, 2).expect("terminal should be created");
    terminal
        .write_char('X')
        .expect("terminal write should succeed");
    terminal
        .execute_control(0x0d)
        .expect("carriage return should reset cursor column");
    terminal
        .execute_control(0x0a)
        .expect("line feed should move content into scrollback");
    terminal
        .execute_control(0x0d)
        .expect("carriage return should reset cursor column");
    terminal
        .write_char('Y')
        .expect("terminal write should succeed");
    terminal.cursor.visible = false;
    terminal.scroll_line_up();

    let search = SearchConfig {
        pattern: "X".to_string(),
        case_sensitive: true,
        use_regex: false,
        whole_word: false,
        wrap: true,
    };

    terminal_renderer
        .prepare_terminal_with_search(&renderer, &terminal, None)
        .expect("initial terminal frame should prepare");
    terminal_renderer.render_to_texture_surface(&renderer, &surface);
    let baseline_pixels = crate::test_support::read_texture_surface(&renderer, &surface);

    let _ = terminal.take_damage();
    let _ = terminal.take_scroll_delta();
    terminal_renderer
        .update_terminal_with_search(&renderer, &mut terminal, Some(&search))
        .expect("detached viewport update should still succeed");
    terminal_renderer.render_to_texture_surface(&renderer, &surface);
    let highlighted_pixels = crate::test_support::read_texture_surface(&renderer, &surface);

    let baseline_color = pixel_at(&baseline_pixels, surface.size(), (1, 1));
    let highlighted_color = pixel_at(&highlighted_pixels, surface.size(), (1, 1));
    let background = crate::test_support::bgra_pixel(terminal_renderer.theme().background);

    assert_eq!(
        terminal.scrollback_view_offset(),
        1,
        "test setup must keep the viewport detached from live grid rows"
    );
    assert_eq!(
        baseline_color, background,
        "baseline cell should render with normal background"
    );
    assert_eq!(
        highlighted_color, baseline_color,
        "search highlighting should be skipped when viewport rows are not sourced from live grid"
    );
}

#[test]
fn terminal_renderer_updates_cached_frame_for_cursor_only_changes() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(32, 16).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let mut terminal_renderer = match TerminalRenderer::new(
        &renderer,
        surface.format(),
        TerminalRendererConfig {
            text: crate::text_renderer::TextRendererConfig {
                theme: Theme {
                    background: ThemeColor::rgb(0xff, 0x00, 0x00),
                    cursor: ThemeColor::rgb(0xff, 0xff, 0xff),
                    ..Theme::default()
                },
                uniforms: TextUniforms::new([32.0, 16.0], [16.0, 16.0], 0.0),
                ..Default::default()
            },
            ..Default::default()
        },
    ) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoUsableSystemFont) => return,
        Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
    };
    let mut terminal = Terminal::new(1, 2).expect("terminal should be created");
    terminal
        .write_char('A')
        .expect("terminal write should succeed");

    terminal_renderer
        .prepare_terminal(&renderer, &terminal)
        .expect("initial terminal frame should prepare");
    terminal_renderer.render_to_texture_surface(&renderer, &surface);
    assert_eq!(terminal_renderer.instance_count(), 1);
    let initial_pixels = crate::test_support::read_texture_surface(&renderer, &surface);
    let background = crate::test_support::bgra_pixel(terminal_renderer.theme().background);
    assert!(
        cell_region_has_non_background(
            &initial_pixels,
            surface.size(),
            (1, 0),
            (16, 16),
            background,
        ),
        "the old cursor cell should be visible before the cursor moves"
    );

    terminal.cursor.move_to(0, 0);

    terminal_renderer
        .update_terminal(&renderer, &mut terminal)
        .expect("cursor-only update should refresh the cached frame");
    terminal_renderer.render_to_texture_surface(&renderer, &surface);

    let cached_pixels =
        crate::test_support::read_texture_surface(&renderer, &terminal_renderer.frame_surface);
    let pixels = crate::test_support::read_texture_surface(&renderer, &surface);
    assert!(
        terminal_renderer.full_redraw_damage.len() >= 2,
        "cursor movement should include old/new cursor repaint regions"
    );
    assert_eq!(terminal_renderer.instance_count(), 2);
    assert_eq!(terminal_renderer.cursor_instance_count(), 1);
    assert_eq!(
        pixel_at(
            &cached_pixels,
            terminal_renderer.frame_surface.size(),
            (24, 24)
        ),
        background,
        "cursor-only updates should clear the old cursor cell in the cached frame"
    );
    assert_eq!(
        pixel_at(&pixels, surface.size(), (24, 8)),
        background,
        "cursor-only updates should clear the old cursor cell back to the background"
    );
    assert!(
        cell_region_has_non_background(&pixels, surface.size(), (0, 0), (16, 16), background),
        "cursor-only updates should preserve visible text and the new cursor position"
    );
}

#[test]
fn terminal_renderer_skips_noop_cursor_redraw_when_nothing_changed() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(48, 16).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let mut terminal_renderer = match TerminalRenderer::new(
        &renderer,
        surface.format(),
        TerminalRendererConfig {
            text: crate::text_renderer::TextRendererConfig {
                uniforms: TextUniforms::new([48.0, 16.0], [16.0, 16.0], 0.0),
                ..Default::default()
            },
            ..Default::default()
        },
    ) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoUsableSystemFont) => return,
        Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
    };
    let mut terminal = Terminal::new(1, 3).expect("terminal should be created");
    terminal
        .write_ascii_run(b"ab")
        .expect("terminal write should succeed");

    terminal_renderer
        .prepare_terminal(&renderer, &terminal)
        .expect("initial terminal frame should prepare");
    let initial_instances = terminal_renderer.instance_count();
    assert_eq!(
        initial_instances, 2,
        "full prepare should encode visible non-blank cells"
    );

    // Drain pending terminal state so the next update is a true no-op.
    let _ = terminal.take_damage();
    let _ = terminal.take_scroll_delta();

    terminal_renderer
        .update_terminal(&renderer, &mut terminal)
        .expect("no-op update should succeed");

    assert_eq!(
            terminal_renderer.instance_count(),
            initial_instances,
            "no-op updates should keep previous prepared instances instead of forcing cursor-cell redraw"
        );
}

#[test]
fn terminal_renderer_repaints_cell_damage_without_forcing_cursor_damage() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(48, 16).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let mut terminal_renderer = match TerminalRenderer::new(
        &renderer,
        surface.format(),
        TerminalRendererConfig {
            text: crate::text_renderer::TextRendererConfig {
                uniforms: TextUniforms::new([48.0, 16.0], [16.0, 16.0], 0.0),
                ..Default::default()
            },
            ..Default::default()
        },
    ) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoUsableSystemFont) => return,
        Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
    };
    let mut terminal = Terminal::new(1, 3).expect("terminal should be created");
    terminal
        .write_ascii_run(b"ab")
        .expect("terminal write should succeed");
    terminal.cursor.move_to(0, 2);

    terminal_renderer
        .prepare_terminal(&renderer, &terminal)
        .expect("initial terminal frame should prepare");
    let _ = terminal.take_damage();
    let _ = terminal.take_scroll_delta();

    terminal
        .grid
        .write(0, 0, Cell::new('x'))
        .expect("cell write should mark damage");
    let cursor_before = terminal.cursor;

    terminal_renderer
        .update_terminal(&renderer, &mut terminal)
        .expect("cell-only update should succeed");
    terminal_renderer.render_to_texture_surface(&renderer, &surface);

    let pixels = crate::test_support::read_texture_surface(&renderer, &surface);
    let background = crate::test_support::bgra_pixel(terminal_renderer.theme().background);
    assert_eq!(terminal.cursor, cursor_before);
    assert_eq!(
            terminal_renderer.instance_count(),
            1,
            "cell-only updates should not force cursor damage when cursor and scroll state are unchanged"
        );
    assert!(
        cell_region_has_non_background(&pixels, surface.size(), (0, 0), (16, 16), background),
        "the changed text cell should be repainted"
    );
    assert!(
        cell_region_has_non_background(&pixels, surface.size(), (2, 0), (16, 16), background),
        "the cursor overlay should remain visible at the unchanged cursor position"
    );
}

#[test]
fn terminal_renderer_rebuilds_cursor_overlay_when_damage_overlaps_cursor_cell() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(48, 16).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let cursor_theme = Theme {
        background: ThemeColor::rgb(0x00, 0x00, 0x00),
        cursor: ThemeColor::rgb(0x00, 0xff, 0x00),
        ..Theme::default()
    };
    let mut terminal_renderer = match TerminalRenderer::new(
        &renderer,
        surface.format(),
        TerminalRendererConfig {
            text: crate::text_renderer::TextRendererConfig {
                theme: cursor_theme.clone(),
                uniforms: TextUniforms::new([48.0, 16.0], [16.0, 16.0], 0.0),
                ..Default::default()
            },
            ..Default::default()
        },
    ) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoUsableSystemFont) => return,
        Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
    };
    let mut terminal = Terminal::new(1, 3).expect("terminal should be created");
    terminal
        .grid
        .write(0, 0, Cell::new('a'))
        .expect("seed cell should be written");
    terminal.cursor.move_to(0, 0);

    terminal_renderer
        .prepare_terminal(&renderer, &terminal)
        .expect("initial frame should prepare");
    let _ = terminal.take_damage();
    let _ = terminal.take_scroll_delta();

    terminal
        .grid
        .write(
            0,
            0,
            Cell {
                character: 'A',
                width: iris_core::cell::CellWidth::Double,
                attrs: CellAttrs::default(),
            },
        )
        .expect("wide replacement cell should be written");

    terminal_renderer
        .update_terminal(&renderer, &mut terminal)
        .expect("overlapping cell damage should update cursor overlay");
    terminal_renderer.render_to_texture_surface(&renderer, &surface);

    let pixels = crate::test_support::read_texture_surface(&renderer, &surface);
    let cursor_color = crate::test_support::bgra_pixel(cursor_theme.cursor);
    assert_eq!(
        pixel_at(&pixels, surface.size(), (24, 8)),
        cursor_color,
        "cursor overlay should refresh when overlapping damage changes the cursor cell span"
    );
}

#[test]
fn terminal_renderer_scroll_only_updates_still_include_cursor_repaint() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(16, 48).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let mut terminal_renderer = match TerminalRenderer::new(
        &renderer,
        surface.format(),
        TerminalRendererConfig {
            text: crate::text_renderer::TextRendererConfig {
                uniforms: TextUniforms::new([16.0, 48.0], [16.0, 16.0], 0.0),
                ..Default::default()
            },
            ..Default::default()
        },
    ) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoUsableSystemFont) => return,
        Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
    };
    let mut terminal = Terminal::new(3, 1).expect("terminal should be created");

    terminal_renderer
        .prepare_terminal(&renderer, &terminal)
        .expect("initial frame should prepare");
    let _ = terminal.take_damage();
    let _ = terminal.take_scroll_delta();
    let cursor_before = terminal.cursor;

    terminal
        .apply_action(Action::ScrollUp(1))
        .expect("scroll up should succeed");
    // Exercise the scroll-delta path directly without additional damage.
    let _ = terminal.take_damage();

    terminal_renderer
        .update_terminal(&renderer, &mut terminal)
        .expect("scroll-only update should succeed");
    terminal_renderer.render_to_texture_surface(&renderer, &surface);

    let pixels = crate::test_support::read_texture_surface(&renderer, &surface);
    let background = crate::test_support::bgra_pixel(terminal_renderer.theme().background);
    assert_eq!(terminal.cursor, cursor_before);
    assert_eq!(
        terminal_renderer.full_redraw_damage.len(),
        1,
        "scroll-only updates with an unchanged cursor should deduplicate cursor damage regions"
    );
    assert_eq!(terminal_renderer.cursor_instance_count(), 1);
    assert!(
        cell_region_has_non_background(
            &pixels,
            surface.size(),
            (cursor_before.position.col, cursor_before.position.row),
            (16, 16),
            background,
        ),
        "scroll-only updates should keep cursor pixels visible after retained-frame shifting"
    );
}

#[test]
fn damage_overlaps_region_detects_intersections() {
    let damage = [DamageRegion::new(0, 0, 0, 2), DamageRegion::new(2, 3, 4, 5)];

    assert!(super::damage_overlaps_region(
        &damage,
        DamageRegion::new(0, 0, 2, 4)
    ));
    assert!(super::damage_overlaps_region(
        &damage,
        DamageRegion::new(3, 3, 4, 4)
    ));
    assert!(!super::damage_overlaps_region(
        &damage,
        DamageRegion::new(1, 1, 0, 1)
    ));
    assert!(!super::damage_overlaps_region(
        &damage,
        DamageRegion::new(4, 4, 4, 5)
    ));
}

#[test]
fn terminal_renderer_clears_removed_text_during_damage_updates() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(32, 16).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let mut terminal_renderer = match TerminalRenderer::new(
        &renderer,
        surface.format(),
        TerminalRendererConfig {
            text: crate::text_renderer::TextRendererConfig {
                theme: Theme {
                    background: ThemeColor::rgb(0xff, 0x00, 0x00),
                    ..Theme::default()
                },
                uniforms: TextUniforms::new([32.0, 16.0], [16.0, 16.0], 0.0),
                ..Default::default()
            },
            ..Default::default()
        },
    ) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoUsableSystemFont) => return,
        Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
    };
    let mut terminal = Terminal::new(1, 2).expect("terminal should be created");
    terminal
        .write_char('A')
        .expect("terminal write should succeed");

    terminal_renderer
        .prepare_terminal(&renderer, &terminal)
        .expect("initial terminal frame should prepare");
    terminal
        .grid
        .write(0, 0, iris_core::cell::Cell::default())
        .expect("cell clear should mark damage");
    terminal.cursor.visible = false;

    terminal_renderer
        .update_terminal(&renderer, &mut terminal)
        .expect("blank-cell damage should refresh the cached frame");
    terminal_renderer.render_to_texture_surface(&renderer, &surface);

    let pixels = crate::test_support::read_texture_surface(&renderer, &surface);
    let background = crate::test_support::bgra_pixel(terminal_renderer.theme().background);
    assert!(
        pixels
            .chunks_exact(background.len())
            .all(|pixel| pixel == background),
        "blank default cells should clear previously rendered glyphs back to the background"
    );
}

#[test]
fn terminal_renderer_updates_theme_and_uniform_state() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let mut terminal_renderer = match TerminalRenderer::new(
        &renderer,
        wgpu::TextureFormat::Bgra8UnormSrgb,
        TerminalRendererConfig::default(),
    ) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoUsableSystemFont) => return,
        Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
    };
    let theme = Theme {
        background: ThemeColor::rgb(0x12, 0x34, 0x56),
        ..Theme::default()
    };
    let uniforms = TextUniforms::new([800.0, 600.0], [10.0, 20.0], 4.0);

    terminal_renderer.set_theme(theme.clone());
    terminal_renderer.set_uniforms(&renderer, uniforms);

    assert_eq!(terminal_renderer.theme(), &theme);
    assert_eq!(terminal_renderer.uniforms(), uniforms);
    assert_eq!(terminal_renderer.font_size_px(), 14.0);
    assert_eq!(
        terminal_renderer.frame_surface_size(),
        TextureSurfaceSize::new(800, 1800).expect("surface dimensions are valid")
    );
}

#[test]
fn terminal_renderer_updates_font_size_and_rebuilds_cached_resources() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let mut terminal_renderer = match TerminalRenderer::new(
        &renderer,
        wgpu::TextureFormat::Bgra8UnormSrgb,
        TerminalRendererConfig::default(),
    ) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoUsableSystemFont) => return,
        Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
    };
    let mut terminal = Terminal::new(1, 1).expect("terminal should be created");
    terminal
        .write_char('A')
        .expect("terminal write should succeed");

    terminal_renderer
        .prepare_terminal(&renderer, &terminal)
        .expect("initial terminal frame should prepare");
    let _ = terminal.take_damage();
    let _ = terminal.take_scroll_delta();
    terminal_renderer
        .set_font_size_px(&renderer, 18.0)
        .expect("font size update should succeed");
    terminal_renderer
        .update_terminal(&renderer, &mut terminal)
        .expect("font size change should trigger a full cached-frame rebuild");

    assert_eq!(terminal_renderer.font_size_px(), 18.0);
}

#[test]
fn terminal_renderer_rejects_invalid_font_size_updates() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let mut terminal_renderer = match TerminalRenderer::new(
        &renderer,
        wgpu::TextureFormat::Bgra8UnormSrgb,
        TerminalRendererConfig::default(),
    ) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoUsableSystemFont) => return,
        Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
    };

    let result = terminal_renderer.set_font_size_px(&renderer, 0.0);

    assert!(matches!(result, Err(Error::InvalidFontSize { size: 0.0 })));
    assert_eq!(terminal_renderer.font_size_px(), 14.0);
}

#[test]
fn terminal_renderer_invalidates_cached_frame_when_cell_metrics_change() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(32, 16).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let mut terminal_renderer = match TerminalRenderer::new(
        &renderer,
        surface.format(),
        TerminalRendererConfig {
            text: crate::text_renderer::TextRendererConfig {
                theme: Theme {
                    background: ThemeColor::rgb(0x00, 0x00, 0x00),
                    cursor: ThemeColor::rgb(0xff, 0x00, 0x00),
                    ..Theme::default()
                },
                uniforms: TextUniforms::new([32.0, 16.0], [16.0, 16.0], 0.0),
                ..Default::default()
            },
            ..Default::default()
        },
    ) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoUsableSystemFont) => return,
        Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
    };
    let mut terminal = Terminal::new(1, 2).expect("terminal should be created");
    terminal
        .write_char('A')
        .expect("terminal write should succeed");

    terminal_renderer
        .prepare_terminal(&renderer, &terminal)
        .expect("initial terminal frame should prepare");
    terminal_renderer.render_to_texture_surface(&renderer, &surface);

    let background = crate::test_support::bgra_pixel(terminal_renderer.theme().background);
    let initial_pixels = crate::test_support::read_texture_surface(&renderer, &surface);
    assert_ne!(
        pixel_at(&initial_pixels, surface.size(), (24, 8)),
        background,
        "the initial cursor should occupy the second 16px cell"
    );

    terminal_renderer.set_uniforms(&renderer, TextUniforms::new([32.0, 16.0], [8.0, 16.0], 0.0));
    terminal_renderer
        .update_terminal(&renderer, &mut terminal)
        .expect("metric changes should force a full cached-frame rebuild");
    terminal_renderer.render_to_texture_surface(&renderer, &surface);

    let updated_pixels = crate::test_support::read_texture_surface(&renderer, &surface);
    assert_eq!(
        pixel_at(&updated_pixels, surface.size(), (24, 8)),
        background,
        "metric changes should clear stale pixels from the old cursor span"
    );
    assert_ne!(
        pixel_at(&updated_pixels, surface.size(), (12, 8)),
        background,
        "metric changes should redraw the cursor at the new cell width"
    );
}

#[test]
fn scroll_copy_region_returns_none_for_excessive_shift() {
    let uniforms = TextUniforms::new([32.0, 32.0], [16.0, 16.0], 0.0);
    let frame_size = TextureSurfaceSize::new(32, 96).expect("frame size is valid");
    let delta = ScrollDelta::new(0, 1, 100);

    assert!(scroll_copy_region(uniforms, frame_size, delta).is_none());
}

#[test]
fn scroll_copy_region_returns_none_for_zero_width_frame() {
    let uniforms = TextUniforms::new([32.0, 32.0], [16.0, 16.0], 0.0);
    let delta = ScrollDelta::new(0, 1, 1);

    assert!(scroll_copy_region(
        uniforms,
        TextureSurfaceSize {
            width: 0,
            height: 96,
        },
        delta,
    )
    .is_none());
}

#[test]
fn scroll_copy_region_returns_none_when_copy_exceeds_frame_height() {
    let uniforms = TextUniforms::new([32.0, 32.0], [16.0, 16.0], 0.0);
    let delta = ScrollDelta::new(0, 1, -1);

    assert!(scroll_copy_region(
        uniforms,
        TextureSurfaceSize::new(32, 70).expect("frame size is valid"),
        delta,
    )
    .is_none());
}

#[test]
fn scroll_copy_region_returns_none_for_invalid_cell_height() {
    let uniforms = TextUniforms::new([32.0, 32.0], [16.0, 0.0], 0.0);
    let delta = ScrollDelta::new(0, 1, 1);

    assert!(scroll_copy_region(
        uniforms,
        TextureSurfaceSize::new(32, 96).expect("frame size is valid"),
        delta,
    )
    .is_none());
}

#[test]
fn partial_scroll_copy_region_returns_none_for_invalid_cell_height() {
    let uniforms = TextUniforms::new([16.0, 48.0], [16.0, f32::NAN], 0.0);
    let frame_size = TextureSurfaceSize::new(16, 144).expect("frame size is valid");
    let delta = ScrollDelta::new(1, 2, 1);

    assert!(partial_scroll_copy_region(uniforms, frame_size, delta).is_none());
}

#[test]
fn partial_scroll_copy_region_returns_expected_shift_for_middle_band() {
    let uniforms = TextUniforms::new([16.0, 48.0], [16.0, 16.0], 0.0);
    let frame_size = TextureSurfaceSize::new(16, 144).expect("frame size is valid");
    let delta = ScrollDelta::new(1, 2, 1);

    let (region_top, region_bottom, source_y, destination_y, copy_height) =
        partial_scroll_copy_region(uniforms, frame_size, delta)
            .expect("partial scroll copy region should be generated");

    assert_eq!(region_top, 64);
    assert_eq!(region_bottom, 96);
    assert_eq!(source_y, 80);
    assert_eq!(destination_y, 64);
    assert_eq!(copy_height, 16);
}

#[test]
fn terminal_renderer_applies_scroll_offset_during_presentation() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(32, 16).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let mut terminal_renderer = match TerminalRenderer::new(
        &renderer,
        surface.format(),
        TerminalRendererConfig {
            text: crate::text_renderer::TextRendererConfig {
                theme: Theme {
                    background: ThemeColor::rgb(0xff, 0x00, 0x00),
                    ..Theme::default()
                },
                uniforms: TextUniforms::new([32.0, 16.0], [16.0, 16.0], 8.0),
                ..Default::default()
            },
            ..Default::default()
        },
    ) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoUsableSystemFont) => return,
        Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
    };
    let mut terminal = Terminal::new(1, 2).expect("terminal should be created");
    terminal
        .write_char('A')
        .expect("terminal write should succeed");
    terminal.cursor.visible = false;

    terminal_renderer
        .prepare_terminal(&renderer, &terminal)
        .expect("terminal frame should prepare");
    terminal_renderer.render_to_texture_surface(&renderer, &surface);

    let background = crate::test_support::bgra_pixel(terminal_renderer.theme().background);
    let cached_pixels =
        crate::test_support::read_texture_surface(&renderer, &terminal_renderer.frame_surface);
    let pixels = crate::test_support::read_texture_surface(&renderer, &surface);

    assert!(
        cell_region_has_non_background(
            &cached_pixels,
            terminal_renderer.frame_surface.size(),
            (0, 1),
            (16, 16),
            background,
        ),
        "the cached frame should still contain the unshifted glyph content"
    );
    assert!(
        cell_region_matches_background(
            &cached_pixels,
            terminal_renderer.frame_surface.size(),
            (0, 0),
            (16, 16),
            background,
        ),
        "the cached frame should reserve a blank top overscan row"
    );
    assert!(
        band_matches_background(&pixels, surface.size(), 0..8, background),
        "positive presentation scroll offset should reveal background at the top edge"
    );
    assert!(
        cell_region_has_non_background(&pixels, surface.size(), (0, 0), (16, 16), background),
        "the shifted presentation should still contain visible glyph pixels"
    );
}

#[test]
fn terminal_renderer_retains_offscreen_rows_for_smooth_scroll_presentation() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(16, 32).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let mut terminal_renderer = match TerminalRenderer::new(
        &renderer,
        surface.format(),
        TerminalRendererConfig {
            text: crate::text_renderer::TextRendererConfig {
                theme: Theme {
                    background: ThemeColor::rgb(0x00, 0x00, 0x00),
                    ..Theme::default()
                },
                uniforms: TextUniforms::new([16.0, 32.0], [16.0, 16.0], 0.0),
                ..Default::default()
            },
            ..Default::default()
        },
    ) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoUsableSystemFont) => return,
        Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
    };
    let mut terminal = Terminal::new(2, 1).expect("terminal should be created");
    terminal.cursor.visible = false;
    terminal
        .grid
        .write(
            0,
            0,
            iris_core::cell::Cell::with_attrs(
                ' ',
                iris_core::cell::CellAttrs {
                    bg: iris_core::cell::Color::Rgb {
                        r: 0xff,
                        g: 0x00,
                        b: 0x00,
                    },
                    ..Default::default()
                },
            ),
        )
        .expect("top styled cell should be written");
    terminal
        .grid
        .write(
            1,
            0,
            iris_core::cell::Cell::with_attrs(
                ' ',
                iris_core::cell::CellAttrs {
                    bg: iris_core::cell::Color::Rgb {
                        r: 0x00,
                        g: 0xff,
                        b: 0x00,
                    },
                    ..Default::default()
                },
            ),
        )
        .expect("bottom styled cell should be written");

    terminal_renderer
        .prepare_terminal(&renderer, &terminal)
        .expect("initial terminal frame should prepare");
    terminal
        .apply_action(iris_core::parser::Action::ScrollUp(1))
        .expect("scroll up should succeed");
    terminal
        .grid
        .write(
            1,
            0,
            iris_core::cell::Cell::with_attrs(
                ' ',
                iris_core::cell::CellAttrs {
                    bg: iris_core::cell::Color::Rgb {
                        r: 0x00,
                        g: 0x00,
                        b: 0xff,
                    },
                    ..Default::default()
                },
            ),
        )
        .expect("new bottom styled cell should be written");
    terminal_renderer.set_uniforms(
        &renderer,
        TextUniforms::new([16.0, 32.0], [16.0, 16.0], 16.0),
    );
    terminal_renderer
        .update_terminal(&renderer, &mut terminal)
        .expect("scroll update should preserve overscan rows");
    terminal_renderer.render_to_texture_surface(&renderer, &surface);

    let smooth_scroll_pixels = crate::test_support::read_texture_surface(&renderer, &surface);
    assert_eq!(
        pixel_at(&smooth_scroll_pixels, surface.size(), (8, 8)),
        [0x00, 0x00, 0xff, 0xff],
        "positive scroll offset should keep the old top row in the overscan band"
    );
    assert_eq!(
        pixel_at(&smooth_scroll_pixels, surface.size(), (8, 24)),
        [0x00, 0xff, 0x00, 0xff],
        "positive scroll offset should keep the old bottom row visible during the transition"
    );

    terminal_renderer.set_uniforms(
        &renderer,
        TextUniforms::new([16.0, 32.0], [16.0, 16.0], 0.0),
    );
    terminal_renderer.render_to_texture_surface(&renderer, &surface);

    let settled_pixels = crate::test_support::read_texture_surface(&renderer, &surface);
    assert_eq!(
        pixel_at(&settled_pixels, surface.size(), (8, 8)),
        [0x00, 0xff, 0x00, 0xff],
        "settling the scroll offset should reveal the new top row"
    );
    assert_eq!(
        pixel_at(&settled_pixels, surface.size(), (8, 24)),
        [0xff, 0x00, 0x00, 0xff],
        "settling the scroll offset should reveal the newly exposed bottom row"
    );
}

#[test]
fn terminal_renderer_retains_full_viewport_overscan_for_multi_line_scrolls() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(16, 48).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let mut terminal_renderer = match TerminalRenderer::new(
        &renderer,
        surface.format(),
        TerminalRendererConfig {
            text: crate::text_renderer::TextRendererConfig {
                theme: Theme {
                    background: ThemeColor::rgb(0x00, 0x00, 0x00),
                    ..Theme::default()
                },
                uniforms: TextUniforms::new([16.0, 48.0], [16.0, 16.0], 0.0),
                ..Default::default()
            },
            ..Default::default()
        },
    ) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoUsableSystemFont) => return,
        Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
    };
    let mut terminal = Terminal::new(3, 1).expect("terminal should be created");
    terminal.cursor.visible = false;
    for (row, color) in [
        (0, (0xff, 0x00, 0x00)),
        (1, (0x00, 0xff, 0x00)),
        (2, (0x00, 0x00, 0xff)),
    ] {
        terminal
            .grid
            .write(
                row,
                0,
                iris_core::cell::Cell::with_attrs(
                    ' ',
                    iris_core::cell::CellAttrs {
                        bg: iris_core::cell::Color::Rgb {
                            r: color.0,
                            g: color.1,
                            b: color.2,
                        },
                        ..Default::default()
                    },
                ),
            )
            .expect("styled cell should be written");
    }

    terminal_renderer
        .prepare_terminal(&renderer, &terminal)
        .expect("initial terminal frame should prepare");
    terminal
        .apply_action(iris_core::parser::Action::ScrollUp(2))
        .expect("scroll up should succeed");
    for (row, color) in [(1, (0xff, 0xff, 0x00)), (2, (0x00, 0xff, 0xff))] {
        terminal
            .grid
            .write(
                row,
                0,
                iris_core::cell::Cell::with_attrs(
                    ' ',
                    iris_core::cell::CellAttrs {
                        bg: iris_core::cell::Color::Rgb {
                            r: color.0,
                            g: color.1,
                            b: color.2,
                        },
                        ..Default::default()
                    },
                ),
            )
            .expect("new styled cell should be written");
    }

    terminal_renderer.set_uniforms(
        &renderer,
        TextUniforms::new([16.0, 48.0], [16.0, 16.0], 32.0),
    );
    terminal_renderer
        .update_terminal(&renderer, &mut terminal)
        .expect("multi-line scroll update should preserve overscan rows");
    terminal_renderer.render_to_texture_surface(&renderer, &surface);

    let transition_pixels = crate::test_support::read_texture_surface(&renderer, &surface);
    assert_eq!(
        pixel_at(&transition_pixels, surface.size(), (8, 8)),
        [0x00, 0x00, 0xff, 0xff]
    );
    assert_eq!(
        pixel_at(&transition_pixels, surface.size(), (8, 24)),
        [0x00, 0xff, 0x00, 0xff]
    );
    assert_eq!(
        pixel_at(&transition_pixels, surface.size(), (8, 40)),
        [0xff, 0x00, 0x00, 0xff]
    );

    terminal_renderer.set_uniforms(
        &renderer,
        TextUniforms::new([16.0, 48.0], [16.0, 16.0], 0.0),
    );
    terminal_renderer.render_to_texture_surface(&renderer, &surface);

    let settled_pixels = crate::test_support::read_texture_surface(&renderer, &surface);
    assert_eq!(
        pixel_at(&settled_pixels, surface.size(), (8, 8)),
        [0xff, 0x00, 0x00, 0xff]
    );
    assert_eq!(
        pixel_at(&settled_pixels, surface.size(), (8, 24)),
        [0x00, 0xff, 0xff, 0xff]
    );
    assert_eq!(
        pixel_at(&settled_pixels, surface.size(), (8, 40)),
        [0xff, 0xff, 0x00, 0xff]
    );
}

#[test]
fn terminal_renderer_renders_partial_scroll_regions_correctly() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(16, 48).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let mut terminal_renderer = match TerminalRenderer::new(
        &renderer,
        surface.format(),
        TerminalRendererConfig {
            text: crate::text_renderer::TextRendererConfig {
                theme: Theme {
                    background: ThemeColor::rgb(0x00, 0x00, 0x00),
                    ..Theme::default()
                },
                uniforms: TextUniforms::new([16.0, 48.0], [16.0, 16.0], 0.0),
                ..Default::default()
            },
            ..Default::default()
        },
    ) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoUsableSystemFont) => return,
        Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
    };
    let mut terminal = Terminal::new(3, 1).expect("terminal should be created");
    terminal.cursor.visible = false;
    terminal
        .grid
        .write(
            0,
            0,
            Cell::with_attrs(
                ' ',
                CellAttrs {
                    bg: Color::Rgb {
                        r: 0xff,
                        g: 0x00,
                        b: 0x00,
                    },
                    ..CellAttrs::default()
                },
            ),
        )
        .expect("top row should be written");
    terminal
        .grid
        .write(
            1,
            0,
            Cell::with_attrs(
                ' ',
                CellAttrs {
                    bg: Color::Rgb {
                        r: 0x00,
                        g: 0xff,
                        b: 0x00,
                    },
                    ..CellAttrs::default()
                },
            ),
        )
        .expect("middle row should be written");
    terminal
        .grid
        .write(
            2,
            0,
            Cell::with_attrs(
                ' ',
                CellAttrs {
                    bg: Color::Rgb {
                        r: 0x00,
                        g: 0x00,
                        b: 0xff,
                    },
                    ..CellAttrs::default()
                },
            ),
        )
        .expect("bottom row should be written");

    terminal_renderer
        .prepare_terminal(&renderer, &terminal)
        .expect("initial frame should prepare");
    terminal_renderer.render_to_texture_surface(&renderer, &surface);

    let red = crate::test_support::bgra_pixel(ThemeColor::rgb(0xff, 0x00, 0x00));
    let green = crate::test_support::bgra_pixel(ThemeColor::rgb(0x00, 0xff, 0x00));
    let blue = crate::test_support::bgra_pixel(ThemeColor::rgb(0x00, 0x00, 0xff));
    let background = crate::test_support::bgra_pixel(terminal_renderer.theme().background);

    let initial_pixels = crate::test_support::read_texture_surface(&renderer, &surface);
    assert_eq!(pixel_at(&initial_pixels, surface.size(), (8, 8)), red);
    assert_eq!(pixel_at(&initial_pixels, surface.size(), (8, 24)), green);
    assert_eq!(pixel_at(&initial_pixels, surface.size(), (8, 40)), blue);

    let _ = terminal.take_damage();
    let _ = terminal.take_scroll_delta();
    terminal
        .apply_action(Action::SetScrollRegion { top: 2, bottom: 3 })
        .expect("scroll region should be set");
    terminal
        .apply_action(Action::ScrollUp(1))
        .expect("scroll up should succeed");

    terminal_renderer
        .update_terminal(&renderer, &mut terminal)
        .expect("scroll-region update should render");
    terminal_renderer.render_to_texture_surface(&renderer, &surface);

    let updated_pixels = crate::test_support::read_texture_surface(&renderer, &surface);
    assert_eq!(
        pixel_at(&updated_pixels, surface.size(), (8, 8)),
        red,
        "rows outside the scroll region should remain unchanged"
    );
    assert_eq!(
        pixel_at(&updated_pixels, surface.size(), (8, 24)),
        blue,
        "scrolling should move the previous bottom row into the middle row"
    );
    assert_eq!(
        pixel_at(&updated_pixels, surface.size(), (8, 40)),
        background,
        "scrolling should clear the exposed bottom row to the theme background"
    );
}

#[test]
fn terminal_renderer_applies_partial_scroll_delta_without_damage_repaint() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(16, 48).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let mut terminal_renderer = match TerminalRenderer::new(
        &renderer,
        surface.format(),
        TerminalRendererConfig {
            text: crate::text_renderer::TextRendererConfig {
                theme: Theme {
                    background: ThemeColor::rgb(0x00, 0x00, 0x00),
                    ..Theme::default()
                },
                uniforms: TextUniforms::new([16.0, 48.0], [16.0, 16.0], 0.0),
                ..Default::default()
            },
            ..Default::default()
        },
    ) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoUsableSystemFont) => return,
        Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
    };
    let mut terminal = Terminal::new(3, 1).expect("terminal should be created");
    terminal.cursor.visible = false;
    for (row, color) in [
        (0, (0xff, 0x00, 0x00)),
        (1, (0x00, 0xff, 0x00)),
        (2, (0x00, 0x00, 0xff)),
    ] {
        terminal
            .grid
            .write(
                row,
                0,
                Cell::with_attrs(
                    ' ',
                    CellAttrs {
                        bg: Color::Rgb {
                            r: color.0,
                            g: color.1,
                            b: color.2,
                        },
                        ..CellAttrs::default()
                    },
                ),
            )
            .expect("styled row should be written");
    }

    terminal_renderer
        .prepare_terminal(&renderer, &terminal)
        .expect("initial frame should prepare");
    terminal_renderer.render_to_texture_surface(&renderer, &surface);

    let red = crate::test_support::bgra_pixel(ThemeColor::rgb(0xff, 0x00, 0x00));
    let blue = crate::test_support::bgra_pixel(ThemeColor::rgb(0x00, 0x00, 0xff));
    let background = crate::test_support::bgra_pixel(terminal_renderer.theme().background);

    let _ = terminal.take_damage();
    let _ = terminal.take_scroll_delta();
    terminal
        .apply_action(Action::SetScrollRegion { top: 2, bottom: 3 })
        .expect("scroll region should be set");
    terminal
        .apply_action(Action::ScrollUp(1))
        .expect("scroll up should succeed");

    // Force update_terminal to rely on scroll-delta shifting for this pass.
    let _ = terminal.take_damage();

    terminal_renderer
        .update_terminal(&renderer, &mut terminal)
        .expect("scroll-region delta should shift the retained frame");
    terminal_renderer.render_to_texture_surface(&renderer, &surface);

    let updated_pixels = crate::test_support::read_texture_surface(&renderer, &surface);
    assert_eq!(pixel_at(&updated_pixels, surface.size(), (8, 8)), red);
    assert_eq!(pixel_at(&updated_pixels, surface.size(), (8, 24)), blue);
    assert_eq!(
        pixel_at(&updated_pixels, surface.size(), (8, 40)),
        background
    );
}

#[test]
fn terminal_renderer_invalidates_cached_frame_on_theme_change() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(32, 16).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let mut terminal_renderer = match TerminalRenderer::new(
        &renderer,
        surface.format(),
        TerminalRendererConfig {
            text: crate::text_renderer::TextRendererConfig {
                theme: Theme {
                    background: ThemeColor::rgb(0x00, 0x00, 0x00),
                    ..Theme::default()
                },
                uniforms: TextUniforms::new([32.0, 16.0], [16.0, 16.0], 0.0),
                ..Default::default()
            },
            ..Default::default()
        },
    ) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoUsableSystemFont) => return,
        Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
    };
    let mut terminal = Terminal::new(1, 2).expect("terminal should be created");
    terminal
        .write_char('A')
        .expect("terminal write should succeed");
    terminal.cursor.visible = false;

    terminal_renderer
        .prepare_terminal(&renderer, &terminal)
        .expect("initial terminal frame should prepare");
    terminal_renderer.render_to_texture_surface(&renderer, &surface);
    let initial_pixels = crate::test_support::read_texture_surface(&renderer, &surface);
    let initial_background = crate::test_support::bgra_pixel(terminal_renderer.theme().background);
    assert!(
        cell_region_matches_background(
            &initial_pixels,
            surface.size(),
            (1, 0),
            (16, 16),
            initial_background,
        ),
        "the blank trailing cell should use the initial theme background"
    );

    terminal_renderer.set_theme(Theme {
        background: ThemeColor::rgb(0xff, 0x00, 0x00),
        ..Theme::default()
    });
    terminal_renderer
        .update_terminal(&renderer, &mut terminal)
        .expect("theme invalidation should force a redraw on the next update");
    terminal_renderer.render_to_texture_surface(&renderer, &surface);

    let updated_pixels = crate::test_support::read_texture_surface(&renderer, &surface);
    let updated_background = crate::test_support::bgra_pixel(terminal_renderer.theme().background);
    assert_ne!(initial_background, updated_background);
    assert!(
        cell_region_matches_background(
            &updated_pixels,
            surface.size(),
            (1, 0),
            (16, 16),
            updated_background,
        ),
        "theme changes should invalidate the cached frame and redraw blank cells"
    );
}

#[test]
fn terminal_renderer_marks_present_uniforms_dirty_only_when_inputs_change() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(32, 16).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let mut terminal_renderer = match TerminalRenderer::new(
        &renderer,
        surface.format(),
        TerminalRendererConfig {
            text: crate::text_renderer::TextRendererConfig {
                uniforms: TextUniforms::new([32.0, 16.0], [16.0, 16.0], 0.0),
                ..Default::default()
            },
            ..Default::default()
        },
    ) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoUsableSystemFont) => return,
        Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
    };

    assert!(
        !terminal_renderer.present_uniforms_dirty.get(),
        "constructor should flush the initial present uniforms"
    );

    terminal_renderer.render_to_texture_surface(&renderer, &surface);
    assert!(
        !terminal_renderer.present_uniforms_dirty.get(),
        "rendering without input changes should not dirty present uniforms"
    );

    terminal_renderer.set_theme(Theme {
        background: ThemeColor::rgb(0x10, 0x20, 0x30),
        ..Theme::default()
    });
    assert!(
        terminal_renderer.present_uniforms_dirty.get(),
        "theme changes should mark present uniforms dirty"
    );
    terminal_renderer.render_to_texture_surface(&renderer, &surface);
    assert!(
        !terminal_renderer.present_uniforms_dirty.get(),
        "rendering should flush dirty present uniforms"
    );

    terminal_renderer.set_uniforms(
        &renderer,
        TextUniforms::new([48.0, 16.0], [16.0, 16.0], 0.0),
    );
    assert!(
        terminal_renderer.present_uniforms_dirty.get(),
        "uniform changes should mark present uniforms dirty"
    );
    terminal_renderer.render_to_texture_surface(&renderer, &surface);
    assert!(
        !terminal_renderer.present_uniforms_dirty.get(),
        "rendering should clear the dirty flag after the GPU write"
    );
}

#[test]
fn terminal_renderer_handles_update_before_prepare() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(32, 16).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let mut terminal_renderer = match TerminalRenderer::new(
        &renderer,
        surface.format(),
        TerminalRendererConfig {
            text: crate::text_renderer::TextRendererConfig {
                uniforms: TextUniforms::new([32.0, 16.0], [16.0, 16.0], 0.0),
                ..Default::default()
            },
            ..Default::default()
        },
    ) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoUsableSystemFont) => return,
        Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
    };
    let mut terminal = Terminal::new(1, 2).expect("terminal should be created");
    terminal
        .write_char('A')
        .expect("terminal write should succeed");

    terminal_renderer
        .update_terminal(&renderer, &mut terminal)
        .expect("update should fall back to a full redraw before any prepare call");
    terminal_renderer.render_to_texture_surface(&renderer, &surface);

    let pixels = crate::test_support::read_texture_surface(&renderer, &surface);
    let background = crate::test_support::bgra_pixel(terminal_renderer.theme().background);
    assert_eq!(terminal_renderer.instance_count(), 1);
    assert!(
        cell_region_has_non_background(&pixels, surface.size(), (0, 0), (16, 16), background),
        "the first incremental update should produce a visible cached frame"
    );
}

#[test]
fn terminal_renderer_restores_terminal_damage_when_incremental_update_fails() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(32, 16).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let mut terminal_renderer = match TerminalRenderer::new(
        &renderer,
        surface.format(),
        TerminalRendererConfig {
            text: crate::text_renderer::TextRendererConfig {
                uniforms: TextUniforms::new([32.0, 16.0], [16.0, 16.0], 0.0),
                ..Default::default()
            },
            font_rasterizer: FontRasterizerConfig {
                font_size_px: 4096.0,
                ..Default::default()
            },
        },
    ) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoUsableSystemFont) => return,
        Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
    };
    let mut terminal = Terminal::new(1, 2).expect("terminal should be created");
    terminal.cursor.visible = false;

    terminal_renderer
        .prepare_terminal(&renderer, &terminal)
        .expect("blank terminal frame should prepare");
    terminal
        .write_char('A')
        .expect("terminal write should succeed");

    let result = terminal_renderer.update_terminal(&renderer, &mut terminal);

    assert!(matches!(
        result,
        Err(Error::GlyphRasterizationFailed { .. })
    ));
    assert_eq!(
        terminal.take_damage(),
        vec![iris_core::damage::DamageRegion::new(0, 0, 0, 0)],
        "failed incremental updates should restore terminal damage"
    );
}

#[test]
fn terminal_renderer_invalidates_cached_frame_when_scroll_shift_update_fails() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(16, 48).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let mut terminal_renderer = match TerminalRenderer::new(
        &renderer,
        surface.format(),
        TerminalRendererConfig {
            text: crate::text_renderer::TextRendererConfig {
                uniforms: TextUniforms::new([16.0, 48.0], [16.0, 16.0], 0.0),
                ..Default::default()
            },
            font_rasterizer: FontRasterizerConfig {
                font_size_px: 4096.0,
                ..Default::default()
            },
        },
    ) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoUsableSystemFont) => return,
        Err(error) => panic!("terminal renderer failed unexpectedly: {error}"),
    };
    let mut terminal = Terminal::new(3, 1).expect("terminal should be created");
    terminal.cursor.visible = false;

    terminal_renderer
        .prepare_terminal(&renderer, &terminal)
        .expect("blank terminal frame should prepare");

    let _ = terminal.take_damage();
    let _ = terminal.take_scroll_delta();
    terminal
        .apply_action(Action::SetScrollRegion { top: 2, bottom: 3 })
        .expect("scroll region should be set");
    terminal
        .apply_action(Action::ScrollUp(1))
        .expect("scroll up should succeed");
    terminal.move_cursor(1, 0);
    terminal
        .write_char('A')
        .expect("terminal write should succeed");

    let result = terminal_renderer.update_terminal(&renderer, &mut terminal);

    assert!(matches!(
        result,
        Err(Error::GlyphRasterizationFailed { .. })
    ));
    assert!(
        !terminal_renderer.frame_initialized,
        "incremental failures after scroll shifts should invalidate the cached frame"
    );
    assert_eq!(
        terminal.take_scroll_delta(),
        Some(ScrollDelta::new(1, 2, 1)),
        "failed incremental updates should restore consumed scroll deltas"
    );
}
