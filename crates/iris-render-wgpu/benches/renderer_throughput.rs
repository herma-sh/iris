use std::hint::black_box;
use std::time::{Duration, Instant};

use iris_core::parser::Action;
use iris_core::terminal::Terminal;
use iris_render_wgpu::atlas::{AtlasConfig, AtlasSize};
use iris_render_wgpu::cell::{CellInstance, TextUniforms};
use iris_render_wgpu::error::Error;
use iris_render_wgpu::renderer::{Renderer, RendererConfig};
use iris_render_wgpu::terminal_renderer::{TerminalRenderer, TerminalRendererConfig};
use iris_render_wgpu::texture::{TextureSurfaceConfig, TextureSurfaceSize};

const WARMUP_RUNS: usize = 5;
const MIN_BENCH_TIME: Duration = Duration::from_millis(750);
const TARGET_FRAME_TIME_MS: f64 = 16.0;
const TARGET_SCROLL_FPS: f64 = 60.0;
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
    let mut full_terminal_renderer = TerminalRenderer::new(
        &renderer,
        wgpu::TextureFormat::Bgra8UnormSrgb,
        config.clone(),
    )
    .expect("terminal renderer should initialize");
    let full_target = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(
                config.text.uniforms.resolution[0] as u32,
                config.text.uniforms.resolution[1] as u32,
            )
            .expect("benchmark render target dimensions should be valid"),
        ))
        .expect("full benchmark render target should initialize");

    let full_prepare = run_benchmark(|_| {
        full_terminal_renderer
            .prepare_terminal(&renderer, &full_terminal)
            .expect("full terminal prepare should succeed");
        full_terminal_renderer.render_to_texture_surface(&renderer, &full_target);
        black_box(&full_terminal_renderer);
    });
    let full_frame_ms = per_iteration_ms(&full_prepare);

    let mut scroll_terminal = seeded_terminal(GRID_ROWS, GRID_COLS);
    let mut scroll_terminal_renderer = TerminalRenderer::new(
        &renderer,
        wgpu::TextureFormat::Bgra8UnormSrgb,
        config.clone(),
    )
    .expect("terminal renderer should initialize");
    let scroll_target = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(
                config.text.uniforms.resolution[0] as u32,
                config.text.uniforms.resolution[1] as u32,
            )
            .expect("benchmark render target dimensions should be valid"),
        ))
        .expect("scroll benchmark render target should initialize");
    scroll_terminal_renderer
        .prepare_terminal(&renderer, &scroll_terminal)
        .expect("initial retained frame prepare should succeed");

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
        black_box(&scroll_terminal_renderer);
    });
    let scroll_fps = iterations_per_second(&scroll_update);

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
        "estimated_renderer_memory: {:.2} MiB (surfaces + atlas + instance buffers)",
        estimated_memory_mb
    );
    println!(
        "targets: full_prepare <= {:.2} ms/frame, retained_scroll >= {:.0} updates/s, memory <= {:.0} MiB",
        TARGET_FRAME_TIME_MS, TARGET_SCROLL_FPS, TARGET_MEMORY_MB
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
    let frame_surface_bytes = width * height * 4;
    let scroll_surface_bytes = frame_surface_bytes;
    let atlas_bytes =
        u64::from(config.text.atlas.size.width) * u64::from(config.text.atlas.size.height);
    let instance_bytes =
        config.text.initial_instance_capacity as u64 * std::mem::size_of::<CellInstance>() as u64;
    let total_bytes = frame_surface_bytes + scroll_surface_bytes + atlas_bytes + instance_bytes;

    total_bytes as f64 / (1024.0 * 1024.0)
}
