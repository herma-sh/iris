# Iris Performance Benchmarks

Measurable performance targets and measurement methodology.

## Performance Philosophy

### Latency Over Throughput

Terminal UX is defined by latency, not throughput:
- Typing must feel instant (< 4ms key-to-screen)
- Scrolling must be smooth (60fps sustained)
- Resize must not block (> 50ms is noticeable)

### Honey Metrics

Borrowed from Ghostty/others - metrics that matter to users:

| Metric | User Impact | Target |
|--------|-------------|--------|
| Time to interactive | First prompt visible | < 100ms |
| Input latency | Typing feels snappy | < 4ms |
| Scroll FPS | Smooth scrolling | 60 fps |
| Resize lag | Window drag feels responsive | < 50ms |
| Memory (10k lines) | Doesn't slow down system | < 50MB |

---

## Benchmark Suite

### 1. Parser Throughput

**What**: How fast we parse PTY output to grid updates.

**Test**: Parse various input sizes and measure throughput.

```rust
// benches/parser_throughput.rs

fn bench_parse_plain_text(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_throughput");
    
    // 1MB of plain text
    let data: Vec<u8> = (b'a'..=b'z').cycle().take(1_000_000).collect();
    group.throughput(Throughput::Bytes(data.len() as u64));
    group.bench_function("plain_1mb", |b| {
        b.iter(|| {
            let mut terminal = Terminal::new(80, 24);
            let mut parser = Parser::new(&mut terminal);
            parser.parse(black_box(&data))
        });
    });
    
    // 100K CSI sequences
    let data = b"\x1b[31mX\x1b[0m".repeat(25_000);
    group.throughput(Throughput::Bytes(data.len() as u64));
    group.bench_function("csi_100k", |b| {
        b.iter(|| {
            let mut terminal = Terminal::new(80, 24);
            let mut parser = Parser::new(&mut terminal);
            parser.parse(black_box(&data))
        });
    });
}
```

**Targets:**

| Test | Target | Failure |
|------|--------|---------|
| Plain text throughput | > 100 MB/s | < 50 MB/s |
| CSI sequence throughput | > 10M seq/s | < 5M seq/s |

### 2. Grid Operations

**What**: How fast grid operations complete.

```rust
// benches/grid_operations.rs

fn bench_grid_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("grid_operations");
    
    // Write single cell
    group.bench_function("write_single_cell", |b| {
        let mut grid = Grid::new(80, 24);
        b.iter(|| {
            grid.write(black_box(10), black_box(5), Cell::new('X'))
        });
    });
    
    // Write full line
    group.bench_function("write_full_line", |b| {
        let mut grid = Grid::new(80, 24);
        let line: Vec<Cell> = (0..80).map(|_| Cell::new('X')).collect();
        b.iter(|| {
            grid.write_line(black_box(0), black_box(&line))
        });
    });
    
    // Scroll up
    group.bench_function("scroll_up", |b| {
        let mut grid = Grid::new(80, 24);
        b.iter(|| grid.scroll_up(black_box(1)));
    });
    
    // Resize
    group.bench_function("resize", |b| {
        let mut grid = Grid::new(80, 24);
        b.iter(|| grid.resize(black_box(120), black_box(40)));
    });
}
```

**Targets:**

| Test | Target | Failure |
|------|--------|---------|
| Write single cell | < 1µs | > 10µs |
| Write full line | < 10µs | > 100µs |
| Scroll up | < 10µs | > 100µs |
| Resize (100K cells) | < 1ms | > 10ms |

### 3. Input Latency

**What**: Time from keystroke to screen update.

```rust
// benches/input_latency.rs

fn bench_input_to_screen(c: &mut Criterion) {
    // This requires mocking the full pipeline:
    // keypress -> parser -> grid -> render
    
    let mut group = c.benchmark_group("input_latency");
    
    group.bench_function("keypress_to_grid", |b| {
        let mut terminal = Terminal::new(80, 24);
        let mut parser = Parser::new(&mut terminal);
        b.iter(|| {
            parser.parse(black_box(b"A"))
        });
    });
}
```

**Targets:**

| Test | Target | Failure |
|------|--------|---------|
| Keypress to grid | < 1ms | > 10ms |
| Grid to render (mock) | < 50µs | > 1ms |

### 4. Rendering

**What**: How fast we render the grid to screen.

```rust
// benches/render.rs

fn bench_render_grid(c: &mut Criterion) {
    let mut group = c.benchmark_group("render");
    
    // Render 80x24 grid (typical)
    group.bench_function("render_80x24", |b| {
        let grid = create_filled_grid(80, 24);
        let mut renderer = MockRenderer::new();
        b.iter(|| renderer.render(black_box(&grid)));
    });
    
    // Render 200x60 grid (large)
    group.bench_function("render_200x60", |b| {
        let grid = create_filled_grid(200, 60);
        let mut renderer = MockRenderer::new();
        b.iter(|| renderer.render(black_box(&grid)));
    });
}
```

**Targets:**

| Test | Target | Failure |
|------|--------|---------|
| Render 80x24 | < 1ms | > 5ms |
| Render 200x60 | < 5ms | > 16ms |

### 5. Memory Usage

**What**: Memory consumption for various states.

```rust
// benches/memory.rs

fn bench_memory_usage(c: &mut Criterion) {
    // Memory benchmarks use separate process with memory profiling
    // These are integration tests, not Criterion benchmarks
    
    // Test cases:
    // - Empty terminal: < 1MB
    // - 10K scrollback: < 50MB
    // - 100K scrollback: < 200MB
    // - Dense grid (all cells filled): < 5MB
}
```

**Targets:**

| State | Target | Failure |
|-------|--------|---------|
| Empty terminal | < 1MB | > 10MB |
| 10K scrollback | < 50MB | > 100MB |
| 100K scrollback | < 200MB | > 500MB |
| Dense grid (80x24) | < 200KB | > 2MB |

---

## Framework Benchmarks

### Startup Time

**What**: Time from process start to first paint.

```bash
# Measure startup time
hyperfine --warmup 3 'iris --command "echo test; exit"'
```

**Targets:**

| Platform | Target | Failure |
|----------|--------|---------|
| Windows | < 100ms | > 500ms |
| macOS | < 100ms | > 500ms |
| Linux | < 100ms | > 500ms |

### Scrolling Under Load

**What**: FPS while scrolling through large output.

```bash
# Generate large output and scroll
# Measure FPS with tool like PresentMon (Windows) or mesa demos (Linux)
cat /dev/urandom | base64 | head -n 1000000
# Then scroll with mouse/keyboard
```

**Targets:**

| Scenario | Target | Failure |
|----------|--------|---------|
| Scroll 1M lines | 60 fps | < 30 fps |
| Scroll during cat | 60 fps | < 30 fps |

### Input During Heavy Output

**What**: Input responsiveness while processing output.

```bash
# Run heavy output and type
yes &
# Type in terminal - should feel responsive
```

**Targets:**

| Test | Target | Failure |
|------|--------|---------|
| Typing during `yes` | < 10ms | > 50ms |
| Typing during `cat large_file` | < 10ms | > 50ms |

---

## Regression Detection

### CI Integration

```yaml
# .github/workflows/bench.yml
bench:
  script:
    - cargo bench --save-baseline main
    - cargo bench --load-baseline main --compare
  artifacts:
    - target/criterion/
```

### Regression Thresholds

| Metric | Regression Threshold |
|--------|---------------------|
| Throughput | > 10% slower |
| Latency | > 20% slower |
| Memory | > 20% larger |
| FPS | > 5 fps drop |

### Alerts

When regression detected:
1. CI fails
2. GitHub issue created
3. PR blocked until fixed

---

## Benchmark Running

### Local Development

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench -- parser

# Run with flamegraph
cargo bench --parser -- --profile-time=5

# Compare against baseline
cargo bench --save-baseline before-change
# make changes
cargo bench --load-baseline before-change
```

### CI Benchmarks

```bash
# Run in CI
cargo bench --save-baseline ci

# Compare PR against main
cargo bench --load-baseline ci
```

---

## Performance Monitoring

### Metrics to Track Over Time

| Metric | Collection | Storage |
|--------|------------|---------|
| Parser throughput | Benchmark | JSON in git |
| Render latency | Benchmark | JSON in git |
| Startup time | CI job | GitHub Actions artifacts |
| Memory usage | CI job | GitHub Actions artifacts |
| Binary size | CI job | GitHub Actions artifacts |

### Dashboard

Track trends at `<benchmark-dashboard-url>`:
- Parser throughput over time
- Input latency over time
- Memory usage over time
- Binary size over time

---

## Optimization Checklist

When a benchmark fails:

1. **Profile**: Run with profiling to find hotspot
2. **Identify**: Find slow function
3. **Optimize**: Apply optimization
4. **Verify**: Re-run benchmark
5. **Document**: Add comment explaining optimization

### Common Optimizations

| Hotspot | Optimization |
|---------|--------------|
| Parser | SIMD for ESC detection |
| Grid write | Inline hot path |
| Grid resize | Async reflow |
| Rendering | Reduce vertex buffer size |
| Memory | Attribute deduplication |