# Iris Testing Strategy

Comprehensive testing approach for quality assurance.

## Testing Philosophy

### Quality Gates

| Gate | Purpose | When |
|------|---------|------|
| Unit tests | Correctness of individual components | Every commit |
| Integration tests | Component interaction | Every PR |
| Performance tests | Latency/fps/memory | Every merge to main |
| Conformance tests | VT compatibility | Before release |
| Manual QA | UX and edge cases | Before release |

### Test Pyramid

```
          ╱╲
         ╱  ╲        Manual QA (exploratory)
        ╱────╲       Conformance tests (vttest)
       ╱      ╲     Integration tests (component interaction)
      ╱────────╲    Performance tests (benchmarks)
     ╱          ╲   Unit tests (fast, isolated, numerous)
    ╱────────────╲
```

---

## Unit Tests

### Location

```
crates/
  iris-core/
    src/
      grid.rs        # #[cfg(test)] mod tests { ... }
      parser.rs      # #[cfg(test)] mod tests { ... }
    tests/
      integration_grid.rs
  iris-platform/
    tests/
      pty_tests.rs
```

### What to Test

#### iris-core

| Module | Test Coverage |
|--------|---------------|
| `grid` | Cell write, scroll, damage tracking, reflow |
| `parser` | All escape sequences, edge cases, malformed input |
| `terminal` | State transitions, cursor movement, modes |
| `selection` | Simple/linear/block selection, boundary conditions |
| `search` | Forward/backward search, regex, case sensitivity |

#### iris-platform

| Module | Test Coverage |
|--------|---------------|
| `pty` | Spawn, read, write, resize, exit |
| `clipboard` | Get/set, encoding, platform-specific |

### Unit Test Examples

```rust
// grid.rs
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn write_cell_updates_damage() {
        let mut grid = Grid::new(80, 24);
        assert!(!grid.is_damaged(0, 0));
        
        grid.write(0, 0, Cell::new('A'));
        assert!(grid.is_damaged(0, 0));
    }
    
    #[test]
    fn scroll_moves_content_up() {
        let mut grid = Grid::new(80, 24);
        grid.write(0, 23, Cell::new('X'));
        grid.scroll_up(1);
        
        assert_eq!(grid.cell(0, 22).unwrap().char, 'X');
    }
    
    #[test]
    fn reflow_preserves_content() {
        let mut grid = Grid::new(80, 24);
        // Write a line that wraps
        grid.write_line(0, "a".repeat(100));
        grid.resize(40, 24);
        
        // Content should be preserved
        assert!(grid.line_contains(0, "a".repeat(40)));
    }
}
```

### Parser Tests

```rust
// parser.rs
#[cfg(test)]
mod tests {
    use super::*;
    
    struct TestHandler {
        cells: Vec<(usize, usize, char)>,
    }
    
    impl Handler for TestHandler {
        fn print(&mut self, c: char) {
            // Track printed characters
        }
    }
    
    #[test]
    fn parse_basic_text() {
        let mut parser = Parser::new(TestHandler::new());
        parser.parse(b"Hello");
        
        // Should print 5 characters
        assert_eq!(parser.handler().cells.len(), 5);
    }
    
    #[test]
    fn parse_csi_cursor_move() {
        let mut parser = Parser::new(TestHandler::new());
        parser.parse(b"\x1b[10;20H");
        
        // Cursor should be at (20, 10)
        assert_eq!(parser.cursor(), (20, 10));
    }
    
    #[test]
    fn parse_osc_title() {
        let mut parser = Parser::new(TestHandler::new());
        parser.parse(b"\x1b]2;My Title\x07");
        
        assert_eq!(parser.title(), Some("My Title"));
    }
    
    #[test]
    fn parse_malformed_escape() {
        let mut parser = Parser::new(TestHandler::new());
        // Should not panic on malformed input
        parser.parse(b"\x1b[999999999999999");
        
        assert!(parser.is_ok());
    }
}
```

---

## Integration Tests

### Location

```
tests/
  integration/
    grid_parser.rs        # Grid + Parser integration
    pty_grid.rs          # PTY → Parser → Grid
    render_grid.rs       # Renderer reading Grid
```

### Integration Test Examples

```rust
// tests/integration/grid_parser.rs
use iris_core::{Grid, Parser, Terminal};
use iris_platform::PtyBackend;

#[test]
fn echo_produces_correct_output() {
    let mut terminal = Terminal::new(80, 24);
    let input = b"Hello, World!";
    
    let mut parser = Parser::new(&mut terminal);
    parser.parse(input);
    
    // Check grid contains expected text
    let line = terminal.grid().line(0);
    assert!(line.starts_with("Hello, World!"));
}

#[test]
fn cursor_movement_updates_grid() {
    let mut terminal = Terminal::new(80, 24);
    let mut parser = Parser::new(&mut terminal);
    
    // Move cursor, write, move back
    parser.parse(b"\x1b[10;20HX\x1b[1;1H");
    
    assert_eq!(terminal.grid().cell(19, 9).unwrap().char, 'X');
    assert_eq!(terminal.cursor(), (0, 0));
}
```

---

## Performance Tests

### Location

```
benches/
  parser.rs
  grid.rs
  render.rs
```

### Benchmark Examples

```rust
// benches/parser.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use iris_core::Parser;

fn bench_parser(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser");
    
    // Benchmark parsing speed
    group.bench_function("parse_1mb_text", |b| {
        let data: Vec<u8> = (0..1_000_000)
            .map(|i| if i % 80 == 0 { b'\n' } else { b'X' })
            .collect();
        
        b.iter(|| {
            let mut terminal = Terminal::new(80, 24);
            let mut parser = Parser::new(&mut terminal);
            parser.parse(black_box(&data));
        });
    });
    
    // Benchmark escape sequence handling
    group.bench_function("parse_csi_sequences", |b| {
        let data = b"\x1b[31m\x1b[1m\x1b[4mTest\x1b[0m".repeat(10000);
        
        b.iter(|| {
            let mut terminal = Terminal::new(80, 24);
            let mut parser = Parser::new(&mut terminal);
            parser.parse(black_box(&data));
        });
    });
    
    group.finish();
}

criterion_group!(benches, bench_parser);
criterion_main!(benches);
```

### Performance Targets

| Benchmark | Target | Failure Threshold |
|-----------|--------|-------------------|
| Parse 1MB plain text | < 10ms | Alert at > 50ms |
| Parse 10K CSI sequences | < 5ms | Alert at > 20ms |
| Grid resize (100K cells) | < 1ms | Alert at > 10ms |
| Render 80x24 grid | < 5ms | Alert at > 16ms |
| First paint | < 20ms | Alert at > 100ms |

---

## Conformance Tests

### vttest

Run `vttest` and verify:

- [ ] Cursor movement
- [ ] Screen features
- [ ] Character sets
- [ ] Colors
- [ ] Scroll regions
- [ ] Status line
- [ ] Double width/height
- [ ] Soft fonts

### DEC Test Patterns

- [ ] VT100 test patterns
- [ ] VT220 test patterns
- [ ] VT420 test patterns

### Terminal Compatibility

Test against real applications:

| Application | Test Cases |
|-------------|------------|
| vim | Editing, scrolling, syntax highlighting |
| tmux | Panes, windows, copy mode |
| htop | Scrolling, colors, updates |
| git | Diff output, colors |
| docker | Container logs, interactive |
| cargo | Build output, colors |

---

## Manual QA Checklist

### Core Functionality

- [ ] Local shell: bash, zsh, fish, pwsh
- [ ] SSH to Linux server
- [ ] SSH to macOS
- [ ] Windows (if supported)
- [ ] Copy/paste: Ctrl+Shift+C/V, middle-click, right-click
- [ ] Mouse selection: character, line, block
- [ ] Scroll: mouse wheel, keyboard
- [ ] Resize: window, font size
- [ ] Unicode: emoji, CJK, RTL

### Edge Cases

- [ ] Very long lines (>1000 columns)
- [ ] Rapid output (cat large file)
- [ ] Null bytes in output
- [ ] Invalid UTF-8
- [ ] Control sequences at boundaries
- [ ] Resize during output
- [ ] Connection loss (SSH)
- [ ] Process exit codes

### Platform-Specific

#### Windows

- [ ] ConPTY integration
- [ ] PowerShell
- [ ] WSL
- [ ] Git Bash
- [ ] Clink completion
- [ ] High DPI screens
- [ ] Windows Clipboard

#### macOS

- [ ] Metal rendering
- [ ] Touch Bar
- [ ] System color scheme change
- [ ] Retina displays

#### Linux

- [ ] X11 clipboard (PRIMARY and CLIPBOARD)
- [ ] Wayland clipboard
- [ ] High DPI scaling
- [ ] GTK integration

---

## CI Pipeline

### Stages

```yaml
stages:
  - lint
  - test
  - bench
  - conformance
  - build

lint:
  - cargo fmt --check
  - cargo clippy -- -D warnings
  
test:
  - cargo test --lib
  - cargo test --doc
  - cargo test --all-features

bench:
  - cargo bench --no-run
  # Run benchmarks and check against thresholds

conformance:
  - ./scripts/run-vttest.sh
  - ./scripts/test-real-apps.sh

build:
  - cargo build --release
  - cargo build --target x86_64-pc-windows-msvc
  - cargo build --target x86_64-apple-darwin
  - cargo build --target x86_64-unknown-linux-gnu
```

### Coverage Requirements

| Crate | Minimum Coverage |
|-------|-----------------|
| iris-core | 90% |
| iris-platform | 80% |
| iris-render-wgpu | 70% |
| Overall | 80% |

---

## Test Utilities

### Fixtures

```rust
// tests/fixtures/mod.rs
pub fn create_test_grid() -> Grid {
    Grid::new(80, 24)
}

pub fn create_test_terminal() -> Terminal {
    Terminal::new(80, 24)
}

pub fn sample_ansi_sequences() -> &'static [u8] {
    include_bytes!("fixtures/sample.ansi")
}
```

### Mocks

```rust
// tests/mocks/mod.rs
pub struct MockPty {
    output: VecDeque<u8>,
    input: Vec<u8>,
}

impl PtyBackend for MockPty {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, PtyError> {
        let len = std::cmp::min(buf.len(), self.output.len());
        self.output.drain(..len).read_exact(&mut buf[..len])?;
        Ok(len)
    }
}
```

### Helpers

```rust
// tests/helpers/mod.rs
pub fn assert_grid_contains(grid: &Grid, text: &str) {
    let content = grid.to_string();
    assert!(
        content.contains(text),
        "Grid does not contain '{}'. Content: {}", 
        text, content
    );
}

pub fn wait_for_condition<F>(condition: F, timeout: Duration) -> bool
where
    F: Fn() -> bool,
{
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if condition() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    false
}
```