use std::hint::black_box;
use std::mem::size_of;
use std::time::{Duration, Instant};

use iris_core::parser::Action;
use iris_core::terminal::Terminal;
use iris_render_wgpu::atlas::{AtlasConfig, AtlasSize};
use iris_render_wgpu::cell::{CellInstance, TextUniforms};
use iris_render_wgpu::cursor::CursorInstance;
use iris_render_wgpu::error::Error;
use iris_render_wgpu::renderer::{Renderer, RendererConfig};
use iris_render_wgpu::terminal_renderer::{TerminalRenderer, TerminalRendererConfig};
use iris_render_wgpu::texture::{TextureSurfaceConfig, TextureSurfaceSize};

const WARMUP_RUNS: usize = 5;
const MIN_BENCH_TIME: Duration = Duration::from_millis(750);
const TARGET_FRAME_TIME_MS: f64 = 16.0;
const TARGET_SCROLL_FPS: f64 = 60.0;
const TARGET_MIXED_UPDATES_PER_SECOND: f64 = 60.0;
const TARGET_NOOP_UPDATES_PER_SECOND: f64 = 60.0;
const TARGET_MEMORY_MB: f64 = 50.0;

const GRID_ROWS: usize = 45;
const GRID_COLS: usize = 160;
const CELL_WIDTH_PX: f32 = 9.0;
const CELL_HEIGHT_PX: f32 = 18.0;

struct BenchResult {
    iterations: u64,
    elapsed: Duration,
}

fn main() {
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(Error::NoAdapter) => {
            println!("renderer_throughput");
            println!("===================");
            println!("skipped: no compatible GPU adapter was available");
            return;
        }
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };

    let mut config = TerminalRendererConfig::default();
    config.text.uniforms = TextUniforms::new(
        [
            GRID_COLS as f32 * CELL_WIDTH_PX,
            GRID_ROWS as f32 * CELL_HEIGHT_PX,
        ],
        [CELL_WIDTH_PX, CELL_HEIGHT_PX],
        0.0,
    );
    config.text.initial_instance_capacity = GRID_ROWS * GRID_COLS;
    config.text.atlas = AtlasConfig::new(
        AtlasSize::new(2048, 2048).expect("benchmark atlas dimensions should be valid"),
    );
    config.font_rasterizer.font_size_px = 14.0;

    let full_terminal = seeded_terminal(GRID_ROWS, GRID_COLS);
    let Some(mut full_terminal_renderer) =
        create_terminal_renderer(&renderer, "full-frame benchmark", config.clone())
    else {
        return;
    };
    let full_target = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(
                config.text.uniforms.resolution[0] as u32,
                config.text.uniforms.resolution[1] as u32,
            )
            .expect("benchmark render-target dimensions should be valid"),
        ))
        .expect("full benchmark render-target should initialize");

    let full_prepare = run_benchmark(|_| {
        full_terminal_renderer
            .prepare_terminal(&renderer, &full_terminal)
            .expect("full terminal prepare should succeed");
        full_terminal_renderer.render_to_texture_surface(&renderer, &full_target);
        wait_for_gpu(&renderer);
        black_box(&full_terminal_renderer);
    });
    let full_frame_ms = per_iteration_ms(&full_prepare);

    let mut scroll_terminal = seeded_terminal(GRID_ROWS, GRID_COLS);
    let Some(mut scroll_terminal_renderer) =
        create_terminal_renderer(&renderer, "retained-scroll benchmark", config.clone())
    else {
        return;
    };
    let scroll_target = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(
                config.text.uniforms.resolution[0] as u32,
                config.text.uniforms.resolution[1] as u32,
            )
            .expect("benchmark render-target dimensions should be valid"),
        ))
        .expect("scroll benchmark render-target should initialize");
    scroll_terminal_renderer
        .prepare_terminal(&renderer, &scroll_terminal)
        .expect("initial retained-frame prepare should succeed");

    let mut line_buffer = vec![b'a'; GRID_COLS];
    let scroll_update = run_benchmark(|iteration| {
        scroll_terminal
            .apply_action(Action::ScrollUp(1))
            .expect("scroll action should succeed");
        populate_ascii_line(&mut line_buffer, iteration as usize);
        scroll_terminal.move_cursor(GRID_ROWS - 1, 0);
        scroll_terminal
            .write_ascii_run(&line_buffer)
            .expect("line fill should succeed");
        scroll_terminal_renderer
            .update_terminal(&renderer, &mut scroll_terminal)
            .expect("retained scroll update should succeed");
        scroll_terminal_renderer.render_to_texture_surface(&renderer, &scroll_target);
        wait_for_gpu(&renderer);
        black_box(&scroll_terminal_renderer);
    });
    let scroll_fps = iterations_per_second(&scroll_update);

    let mut noop_terminal = seeded_terminal(GRID_ROWS, GRID_COLS);
    let Some(mut noop_terminal_renderer) =
        create_terminal_renderer(&renderer, "noop-retained benchmark", config.clone())
    else {
        return;
    };
    let noop_target = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(
                config.text.uniforms.resolution[0] as u32,
                config.text.uniforms.resolution[1] as u32,
            )
            .expect("benchmark render-target dimensions should be valid"),
        ))
        .expect("noop benchmark render-target should initialize");
    noop_terminal_renderer
        .prepare_terminal(&renderer, &noop_terminal)
        .expect("initial noop-frame prepare should succeed");
    let _ = noop_terminal.take_damage();
    let _ = noop_terminal.take_scroll_delta();
    let noop_update = run_benchmark(|_| {
        noop_terminal_renderer
            .update_terminal(&renderer, &mut noop_terminal)
            .expect("noop retained update should succeed");
        noop_terminal_renderer.render_to_texture_surface(&renderer, &noop_target);
        wait_for_gpu(&renderer);
        black_box(&noop_terminal_renderer);
    });
    let noop_updates_per_second = iterations_per_second(&noop_update);

    let mut mixed_terminal = seeded_terminal(GRID_ROWS, GRID_COLS);
    let Some(mut mixed_terminal_renderer) =
        create_terminal_renderer(&renderer, "mixed-update benchmark", config.clone())
    else {
        return;
    };
    let mixed_target = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(
                config.text.uniforms.resolution[0] as u32,
                config.text.uniforms.resolution[1] as u32,
            )
            .expect("benchmark render-target dimensions should be valid"),
        ))
        .expect("mixed benchmark render-target should initialize");
    mixed_terminal_renderer
        .prepare_terminal(&renderer, &mixed_terminal)
        .expect("initial mixed-frame prepare should succeed");

    let mut mixed_line_buffer = vec![b'a'; GRID_COLS];
    let mixed_update = run_benchmark(|iteration| {
        match iteration % 4 {
            0 => {
                // no-op update to measure retained no-change overhead.
            }
            1 => {
                let next_col = (iteration as usize) % GRID_COLS;
                mixed_terminal.cursor.move_to(GRID_ROWS - 1, next_col);
            }
            2 => {
                let row = (iteration as usize) % GRID_ROWS;
                let col = (iteration as usize) % GRID_COLS.saturating_sub(1).max(1);
                mixed_terminal.move_cursor(row, col);
                mixed_terminal
                    .write_char((b'a' + (iteration % 26) as u8) as char)
                    .expect("cell write should succeed");
            }
            _ => {
                mixed_terminal
                    .apply_action(Action::ScrollUp(1))
                    .expect("scroll action should succeed");
                populate_ascii_line(&mut mixed_line_buffer, iteration as usize);
                mixed_terminal.move_cursor(GRID_ROWS - 1, 0);
                mixed_terminal
                    .write_ascii_run(&mixed_line_buffer)
                    .expect("line fill should succeed");
            }
        }

        mixed_terminal_renderer
            .update_terminal(&renderer, &mut mixed_terminal)
            .expect("mixed retained update should succeed");
        mixed_terminal_renderer.render_to_texture_surface(&renderer, &mixed_target);
        wait_for_gpu(&renderer);
        black_box(&mixed_terminal_renderer);
    });
    let mixed_updates_per_second = iterations_per_second(&mixed_update);

    let estimated_memory_mb = estimate_renderer_memory_mb(&config);

    println!("renderer_throughput");
    println!("===================");
    println!(
        "full_prepare_{}x{}: {:.2} ms/frame over {} iterations ({:.3}s)",
        GRID_COLS,
        GRID_ROWS,
        full_frame_ms,
        full_prepare.iterations,
        full_prepare.elapsed.as_secs_f64()
    );
    println!(
        "retained_scroll_update_{}x{}: {:.2} updates/s over {} iterations ({:.3}s)",
        GRID_COLS,
        GRID_ROWS,
        scroll_fps,
        scroll_update.iterations,
        scroll_update.elapsed.as_secs_f64()
    );
    println!(
        "retained_noop_update_{}x{}: {:.2} updates/s over {} iterations ({:.3}s)",
        GRID_COLS,
        GRID_ROWS,
        noop_updates_per_second,
        noop_update.iterations,
        noop_update.elapsed.as_secs_f64()
    );
    println!(
        "retained_mixed_update_{}x{}: {:.2} updates/s over {} iterations ({:.3}s)",
        GRID_COLS,
        GRID_ROWS,
        mixed_updates_per_second,
        mixed_update.iterations,
        mixed_update.elapsed.as_secs_f64()
    );
    println!(
        "estimated_renderer_memory: {:.2} MiB (approximate; excludes driver/backend allocations)",
        estimated_memory_mb
    );
    println!(
        "targets: full_prepare <= {:.2} ms/frame, retained_scroll >= {:.0} updates/s, retained_noop >= {:.0} updates/s, retained_mixed >= {:.0} updates/s, memory <= {:.0} MiB",
        TARGET_FRAME_TIME_MS,
        TARGET_SCROLL_FPS,
        TARGET_NOOP_UPDATES_PER_SECOND,
        TARGET_MIXED_UPDATES_PER_SECOND,
        TARGET_MEMORY_MB
    );
}

fn run_benchmark<F>(mut runner: F) -> BenchResult
where
    F: FnMut(u64),
{
    for warmup in 0..WARMUP_RUNS {
        runner(warmup as u64);
    }

    let start = Instant::now();
    let mut iterations = 0_u64;
    while start.elapsed() < MIN_BENCH_TIME {
        runner(iterations);
        iterations += 1;
    }

    BenchResult {
        iterations: iterations.max(1),
        elapsed: start.elapsed(),
    }
}

fn create_terminal_renderer(
    renderer: &Renderer,
    context: &str,
    config: TerminalRendererConfig,
) -> Option<TerminalRenderer> {
    match TerminalRenderer::new(renderer, wgpu::TextureFormat::Bgra8UnormSrgb, config) {
        Ok(terminal_renderer) => Some(terminal_renderer),
        Err(Error::NoUsableSystemFont) => {
            println!("renderer_throughput");
            println!("===================");
            println!("skipped: no usable system font was available");
            None
        }
        Err(error) => panic!("terminal renderer should initialize for {context}: {error}"),
    }
}

fn wait_for_gpu(renderer: &Renderer) {
    // Include GPU completion in each sample so results represent completed-frame
    // throughput/latency rather than CPU-side submission-only throughput.
    renderer.device().poll(wgpu::Maintain::Wait);
}

fn seeded_terminal(rows: usize, cols: usize) -> Terminal {
    let mut terminal = Terminal::new(rows, cols).expect("benchmark terminal should initialize");
    let mut line = vec![b'a'; cols];
    for row in 0..rows {
        populate_ascii_line(&mut line, row);
        terminal.move_cursor(row, 0);
        terminal
            .write_ascii_run(&line)
            .expect("seed line write should succeed");
    }

    terminal
}

fn populate_ascii_line(line: &mut [u8], seed: usize) {
    for (index, byte) in line.iter_mut().enumerate() {
        *byte = b'a' + ((seed + index) % 26) as u8;
    }
}

fn per_iteration_ms(result: &BenchResult) -> f64 {
    (result.elapsed.as_secs_f64() * 1000.0) / result.iterations as f64
}

fn iterations_per_second(result: &BenchResult) -> f64 {
    result.iterations as f64 / result.elapsed.as_secs_f64()
}

fn estimate_renderer_memory_mb(config: &TerminalRendererConfig) -> f64 {
    let width = config.text.uniforms.resolution[0].max(1.0).round() as u64;
    let height = config.text.uniforms.resolution[1].max(1.0).round() as u64;

    // This estimate intentionally models major retained-renderer allocations
    // visible from config/input sizing. Backend allocator and driver-side
    // bookkeeping are intentionally excluded.
    let frame_surface_bytes = width * height * 4;
    let scroll_surface_bytes = frame_surface_bytes;
    let atlas_bytes =
        u64::from(config.text.atlas.size.width) * u64::from(config.text.atlas.size.height);
    let instance_bytes =
        config.text.initial_instance_capacity as u64 * size_of::<CellInstance>() as u64;
    let uniform_buffer_count = 2u64;
    let uniform_bytes = uniform_buffer_count * size_of::<TextUniforms>() as u64;
    let cursor_instance_bytes = size_of::<CursorInstance>() as u64;

    let estimated_glyph_entries = (config.text.initial_instance_capacity as u64).min(4_096);
    let glyph_cache_entry_bytes = 48u64;
    let glyph_cache_bytes = estimated_glyph_entries * glyph_cache_entry_bytes;

    let fallback_family_bytes = config
        .font_rasterizer
        .fallback_families
        .iter()
        .map(|family| family.len() as u64)
        .sum::<u64>();
    let primary_family_bytes = config
        .font_rasterizer
        .primary_family
        .as_ref()
        .map_or(0, |family| family.len() as u64);
    let rasterizer_state_bytes = fallback_family_bytes + primary_family_bytes + 128;

    let total_bytes = frame_surface_bytes
        + scroll_surface_bytes
        + atlas_bytes
        + instance_bytes
        + uniform_bytes
        + cursor_instance_bytes
        + glyph_cache_bytes
        + rasterizer_state_bytes;

    total_bytes as f64 / (1024.0 * 1024.0)
}
