# Iris Testing Strategy

Testing in Iris is layered: fast unit coverage for protocol behavior, integration coverage for parser-to-terminal flows, benchmarks for hot paths, and conformance checks once a runnable terminal binary exists.

## Quality Gates

| Gate | Purpose | When |
|------|---------|------|
| Unit tests | Component correctness | Every commit |
| Integration tests | Cross-module behavior | Every PR |
| Performance tests | Hot-path regressions | Parser/render work and before merge when performance changes |
| Conformance tests | VT compatibility | Once the standalone binary exists |
| Manual QA | UX and platform validation | Before release |

## Test Pyramid

```text
          /\
         /  \        Manual QA
        /----\       Conformance tests
       /      \      Integration tests
      /--------\     Performance tests
     /          \    Unit tests
    /------------\
```

## Current Phase 1 Approach

Phase 1 focuses on `iris-core`, so most verification lives in parser, terminal, and integration tests:

- Unit tests cover parser states, CSI/OSC/DCS handling, SGR decoding, terminal editing, movement, and screen state.
- Integration tests cover chunked streams, realistic redraw flows, nested OSC behavior, and captured terminal-like output.
- Benchmarks cover parser-to-terminal throughput through the shipped `Parser::advance` path.
- VTtest is intentionally deferred until Phase 6 because `iris-core` alone does not provide a runnable interactive terminal binary.

## Unit Test Guidance

Unit tests should validate one behavior at a time and prefer exact action/state assertions.

### Parser example

```rust
use iris_core::{Action, Parser};

#[test]
fn parse_basic_text() {
    let mut parser = Parser::new();
    let actions = parser.parse(b"Hello");

    assert_eq!(
        actions,
        vec![
            Action::Print('H'),
            Action::Print('e'),
            Action::Print('l'),
            Action::Print('l'),
            Action::Print('o'),
        ]
    );
}

#[test]
fn parse_csi_cursor_move() {
    let mut parser = Parser::new();
    let actions = parser.parse(b"\x1b[10;20H");

    assert_eq!(actions, vec![Action::CursorPosition { row: 10, col: 20 }]);
}
```

### Terminal example

```rust
use iris_core::Terminal;

#[test]
fn write_updates_grid() {
    let mut terminal = Terminal::new(24, 80).unwrap();
    terminal.write_char('A').unwrap();

    assert_eq!(terminal.grid.cell(0, 0).unwrap().character, 'A');
}
```

## Integration Test Guidance

Integration tests should exercise the real parser-to-terminal boundary with representative byte streams.

```rust
use iris_core::{Parser, Terminal};

#[test]
fn stream_updates_terminal_state() {
    let mut terminal = Terminal::new(24, 80).unwrap();
    let mut parser = Parser::new();

    parser
        .advance(&mut terminal, b"\x1b[10;20HX\x1b[1;1H")
        .unwrap();

    assert_eq!(terminal.grid.cell(9, 19).unwrap().character, 'X');
    assert_eq!(terminal.cursor.position.row, 0);
    assert_eq!(terminal.cursor.position.col, 0);
}
```

Prefer captured or realistic streams when possible:

- chunked redraws
- alternate-screen app flows
- scroll-margin interactions
- OSC state recovery
- mixed UTF-8 and control sequences

## Performance Tests

Hot-path changes require benchmark coverage or an explicit benchmark impact review.

### Current parser benchmark

```rust
use std::hint::black_box;

use iris_core::{Parser, Terminal};

fn run_fixture(data: &[u8]) {
    let mut parser = Parser::new();
    let mut terminal = Terminal::new(24, 80).unwrap();
    parser.advance(&mut terminal, black_box(data)).unwrap();
}
```

Current shipped harness:

- `crates/iris-core/benches/parser_throughput.rs`

Phase 1 target:

- plain text: `>= 100 MiB/s`
- CSI stream: `>= 10M seq/s`

## Conformance Tests

### VTtest

VTtest is a release-facing conformance gate, but Iris cannot run it honestly until there is a standalone terminal binary that can:

- open a terminal window
- host a PTY session
- feed PTY output into `iris-core`
- render the resulting terminal state
- send keyboard input back to the child process

Because that binary does not exist in Phase 1, VTtest is deferred to Phase 6. Until then, parser conformance is approximated with unit tests, integration tests, and captured real-world redraw streams.

### Real application coverage

Captured output and later manual validation should cover:

- `vim`
- `tmux`
- `htop`
- `cargo`
- `git`

## CI Verification

Minimum verification for substantial `iris-core` work:

```bash
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all
```

Add this when hot paths changed:

```bash
cargo bench -p iris-core --bench parser_throughput
```

Enable VT conformance automation only after the standalone binary exists.

## Test Design Rules

- Test parser recovery on malformed input.
- Test chunk boundaries explicitly.
- Test hostile or oversized escape-sequence inputs.
- Keep integration fixtures realistic enough to catch parser/terminal boundary bugs.
- When a behavior is deferred because the required binary or platform layer does not exist yet, document the deferral instead of implying coverage that is not real.
- Prefer tests that exercise concrete backends and real or captured data over synthetic mocks.
- Do not add mock-data tests for behavior that is expected to gain meaningful real-backend coverage in the near term; add or defer to the real-backend tests.
- If mock-based tests are temporarily required, document why real-backend coverage is not yet practical and replace them when backend coverage lands.
