# Iris Performance Benchmarks

Measurable performance targets and the benchmark workflow used to track them.

## Performance Philosophy

Terminal UX is defined by latency first and throughput second:

- Typing must feel instant.
- Scrolling must stay smooth under sustained output.
- Resize and redraw work must not block the UI.
- Hot paths should avoid hidden heap churn.

## Current Benchmarks

### Parser Throughput

The repository currently ships one concrete benchmark harness:

- Path: `crates/iris-core/benches/parser_throughput.rs`
- Scope: parser-to-terminal throughput through `Parser::advance`
- Fixtures:
  - `plain_text_1mb`
  - `csi_stream_100k`

This harness intentionally measures the shipped path, not `Parser::parse` in isolation. That keeps the benchmark aligned with the real work performed by `iris-core` when PTY output is applied to terminal state.

```rust
use std::hint::black_box;

use iris_core::{Parser, Terminal};

fn run_fixture(data: &[u8]) {
    let mut parser = Parser::new();
    let mut terminal = Terminal::new(24, 80).unwrap();
    parser.advance(&mut terminal, black_box(data)).unwrap();
    black_box(terminal);
}
```

### Phase 1 Targets

| Fixture | Target | Failure threshold |
|---------|--------|-------------------|
| Plain text throughput | >= 100 MiB/s | < 50 MiB/s |
| CSI sequence throughput | >= 10M seq/s | < 5M seq/s |

### Latest Verified Results

Verified runs on 2026-03-18 ranged roughly:

| Fixture | Result |
|---------|--------|
| `plain_text_1mb` | `144-151 MiB/s` |
| `csi_stream_100k` | `11.1M-11.2M seq/s` |

These runs were taken after the parser action-buffer reuse work, CSI allocation reductions, ASCII ground-state fast path, and batched terminal/grid ASCII writes.

### Renderer Throughput (Phase 2)

The repository now ships a renderer benchmark harness focused on retained-frame preparation:

- Path: `crates/iris-render-wgpu/benches/renderer_throughput.rs`
- Scope:
  - full-frame prepare + present to an off-screen texture
  - retained full-grid scroll update + present
  - renderer-memory estimate for retained surfaces, atlas, and instance buffers
- Fixture:
  - `160x45` terminal grid at `9x18` cell metrics

Latest verified run on `2026-03-21`:

| Fixture | Result |
|---------|--------|
| `full_prepare_160x45` | `~0.29 ms/frame` |
| `retained_scroll_update_160x45` | `~3500 updates/s` |
| `estimated_renderer_memory` | `~13.34 MiB` |

## Planned Benchmarks

Additional benchmark areas are still planned for later phases:

- Grid operations
- Render latency and damage-only redraw cost at larger viewport tiers
- Startup time
- Memory usage under large scrollback (deferred until scrollback lands in Phase 4)
- Interactive latency under heavy output

Those benches should follow the same rule as the current parser harness: measure the shipped execution path rather than a simplified micro-benchmark that skips real state updates.

## Running Benchmarks

### Local Development

```bash
# Run the shipped parser throughput benchmark
cargo bench -p iris-core --bench parser_throughput

# Run the renderer throughput benchmark
cargo bench -p iris-render-wgpu --bench renderer_throughput

# Build all benches without running them
cargo bench --all --no-run

# Optional profiler-driven follow-up
cargo flamegraph --bench parser_throughput -p iris-core
```

### CI

```bash
# Phase 1 performance gate
cargo bench -p iris-core --bench parser_throughput
```

## Regression Policy

When benchmarked performance regresses:

1. Profile the changed path.
2. Identify the concrete hot spot.
3. Optimize the implementation with measurements, not guesses.
4. Rerun the benchmark and record the new baseline.
5. Update docs when the methodology or target changes.

## Benchmark Hygiene

- Benchmark hot paths after correctness is established.
- Prefer stable fixtures over ad hoc shell commands.
- Keep throughput results tied to exact commands.
- Avoid changing targets without documenting why.
- If a benchmark is deferred because the required binary or subsystem does not exist yet, say so explicitly.
