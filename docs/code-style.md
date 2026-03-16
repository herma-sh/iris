# Iris Code Style

Best practices for writing code that is minimal, fast, maintainable, and readable.

## Core Principles

### 1. Correctness First, Then Optimize

```rust
// DON'T: Optimize before it works
fn parse_byte(&mut self, b: u8) {
    // SIMD optimization for what?
    unsafe { ... }
}

// DO: Write clear code first
fn parse_byte(&mut self, b: u8) {
    match self.state {
        State::Ground => self.handle_ground(b),
        State::Escape => self.handle_escape(b),
        State::Csi => self.handle_csi(b),
    }
}
// Then benchmark. Then optimize only hot paths.
```

### 2. No Allocations in Hot Paths

```rust
// DON'T: Allocate in the parsing loop
fn write_cell(&mut self, cell: Cell) {
    self.cells.push(cell);  // Vec::push may reallocate
}

// DO: Pre-allocate, write by index
fn write_cell(&mut self, col: usize, row: usize, cell: Cell) {
    let idx = row * self.cols + col;
    self.cells[idx] = cell;
}
```

### 3. Explicit Over Clever

```rust
// DON'T: Clever one-liner
let valid = s.chars().all(|c| c.is_ascii_graphic() || c == ' ');

// DO: Explicit and readable
fn is_valid_terminal_char(c: char) -> bool {
    c.is_ascii_graphic() || c == ' '
}

let valid = s.chars().all(is_valid_terminal_char);
```

---

## Minimal Code

### Avoid Abstraction for Abstraction's Sake

```rust
// DON'T: Unnecessary trait
trait CellWriter {
    fn write(&mut self, cell: Cell);
}

// DO: Simple function
fn write_cell(grid: &mut Grid, col: usize, row: usize, cell: Cell) {
    grid.cells[row * grid.cols + col] = cell;
}
```

### No Dead Code

Every function, struct, and module must justify its existence. If it's not used, delete it.

```rust
// DON'T: "Might need this later"
fn eventually_useful_function() { ... }

// DO: Delete it. Add it back when needed.
```

### Prefer Functions Over Methods When State Isn't Needed

```rust
// DON'T: Method that doesn't use self
impl Grid {
    fn cell_index(cols: usize, row: usize, col: usize) -> usize {
        row * cols + col
    }
}

// DO: Free function
fn cell_index(cols: usize, row: usize, col: usize) -> usize {
    row * cols + col
}
```

---

## Fast Code

### Know Your Hot Paths

Label hot paths explicitly:

```rust
/// HOT PATH: Called for every byte from PTY.
/// Must not allocate.
#[inline(always)]
fn parse_byte(&mut self, b: u8) {
    // ...
}
```

### Use Inline Wisely

```rust
// DON'T: Inline everything
#[inline(always)]
fn format_debug_string(s: &str) -> String { ... }  // Cold path

// DO: Inline only hot paths
#[inline(always)]
fn cell_index(&self, row: usize, col: usize) -> usize {
    row * self.cols + col
}
```

### Prefer Stack Over Heap

```rust
// DON'T: Heap allocation
fn parse_params(data: &[u8]) -> Box<[i64]> {
    data.iter().map(...).collect()
}

// DO: Stack array with known maximum
fn parse_params(data: &[u8]) -> ArrayVec<i64, 16> {
    let mut params = ArrayVec::new();
    // ...
    params
}
```

### Avoid Bounds Checks Where Safe

```rust
// DON'T: Redundant bounds check
for row in 0..self.rows {
    for col in 0..self.cols {
        let idx = self.cell_index(row, col);  // Bounds check inside
    }
}

// DO: Use unsafe only with SAFETY comment, after profiling
for row in 0..self.rows {
    for col in 0..self.cols {
        let idx = row * self.cols + col;
        // SAFETY: idx is computed from valid row/col range
        let cell = unsafe { self.cells.get_unchecked(idx) };
    }
}
```

---

## Maintainable Code

### Single Responsibility

```rust
// DON'T: God struct
struct Terminal {
    grid: Grid,
    parser: Parser,
    renderer: Renderer,
    pty: Pty,
    config: Config,
    // ... 20 more fields
}

// DO: Separate concerns
struct Terminal {
    grid: Grid,
    cursor: Cursor,
    modes: Modes,
}

struct TerminalSession {
    terminal: Terminal,
    pty: PtyHandle,
}
```

### Functions Should Do One Thing

```rust
// DON'T: Does too much
fn handle_input(&mut self, input: &[u8]) {
    self.parse(input);
    self.render();
    self.update_title();
    self.handle_bell();
}

// DO: One responsibility
fn handle_input(&mut self, input: &[u8]) {
    self.parse(input);
}

fn tick(&mut self) {
    if self.needs_render() {
        self.render();
    }
}
```

### No Surprise Side Effects

```rust
// DON'T: Hidden mutation
fn get_cell(&self, row: usize, col: usize) -> &Cell {
    self.ensure_capacity(row, col);  // Unexpected mutation!
    &self.cells[row * self.cols + col]
}

// DO: Explicit mutation
fn get_cell(&self, row: usize, col: usize) -> Option<&Cell> {
    self.cells.get(row * self.cols + col)
}

fn grow_if_needed(&mut self, rows: usize) { ... }
```

---

## Comments

### Explain Why, Not What

```rust
// DON'T: Explains the obvious
// Increment row by 1
self.row += 1;

// DO: Explains the why
// Origin mode uses (1,1) as home, so we offset by 1
// to convert to (0,0)-based indexing
let row = if self.modes.origin { self.row + 1 } else { self.row };
```

### Document Public APIs

```rust
/// Writes a cell to the grid at the specified position.
///
/// # Arguments
/// * `col` - Column index (0-based)
/// * `row` - Row index (0-based)
///
/// # Panics
/// Panics if `col >= self.cols` or `row >= self.rows`.
///
/// # Example
/// ```
/// let mut grid = Grid::new(80, 24);
/// grid.write(0, 0, Cell::new('A'));
/// ```
pub fn write(&mut self, col: usize, row: usize, cell: Cell) {
    // ...
}
```

### Use TODO and FIXME Meaningfully

```rust
// DON'T: Vague TODO
// TODO: fix this
fn handle_resize(&mut self) { ... }

// DO: Specific, actionable
// TODO(#42): Handle resize during scrollback reflow.
// Currently loses scroll position for wrapped lines.
fn handle_resize(&mut self) { ... }
```

### SAFETY Comments for Unsafe

```rust
// DON'T: Unsafe without explanation
let cell = unsafe { self.cells.get_unchecked(idx) };

// DO: SAFETY comment
// SAFETY: idx is computed from row * self.cols + col where
// row < self.rows and col < self.cols are guaranteed by the caller.
let cell = unsafe { self.cells.get_unchecked(idx) };
```

---

## Readable Code

### Naming

```rust
// DON'T: Abbreviations
fn proc_bs(&mut self) { ... }  // What's BS?
let cp = self.cp;  // Code point? Cursor position?

// DO: Full words
fn process_backspace(&mut self) { ... }
let cursor_position = self.cursor_position;
```

### Consistent Naming Patterns

```rust
// Consistent verb patterns
fn parse_byte(&mut self, b: u8) { ... }    // parse_* for input
fn render_cell(&self, cell: &Cell) { ... }  // render_* for output
fn handle_escape(&mut self, b: u8) { ... }  // handle_* for events

// Consistent noun patterns
struct CellAttrs { ... }   // Attrs for attributes
struct TerminalModes { ... } // Modes for terminal modes
struct PtyConfig { ... }     // Config for configuration
```

### Avoid Deep Nesting

```rust
// DON'T: Deep nesting
fn handle_csi(&mut self, params: &[i64], final: u8) {
    if final == 'm' {
        if params.len() > 0 {
            if params[0] == 0 {
                // ...
            } else if params[0] == 1 {
                // ...
            }
        }
    }
}

// DO: Early returns and match
fn handle_csi(&mut self, params: &[i64], final: u8) {
    match final {
        b'm' => self.handle_sgr(params),
        b'H' => self.handle_cup(params),
        b'J' => self.handle_ed(params),
        _ => {}  // Unknown sequence, ignore
    }
}

fn handle_sgr(&mut self, params: &[i64]) {
    let mode = params.first().copied().unwrap_or(0);
    match mode {
        0 => self.attrs.clear(),
        1 => self.attrs.bold = true,
        // ...
    }
}
```

### Use Type System

```rust
// DON'T: Primitives everywhere
fn write(&mut self, row: usize, col: usize, style: u8) { ... }

// DO: Strong types
#[derive(Clone, Copy)]
pub struct Row(usize);
#[derive(Clone, Copy)]
pub struct Col(usize);

pub enum CursorStyle {
    Block,
    Underline,
    Bar,
}

fn write(&mut self, row: Row, col: Col, style: CursorStyle) { ... }
```

---

## Project Conventions

### Error Handling

```rust
// Use thiserror for error types
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("invalid escape sequence: {0:?}")]
    InvalidEscape(u8),
    
    #[error("unexpected end of input")]
    UnexpectedEof,
}

// Return Result for fallible operations
pub fn parse(&mut self, input: &[u8]) -> Result<(), ParseError> {
    // ...
}
```

### No Panics in Production

```rust
// DON'T: Panic on invalid input
pub fn new(cols: usize, rows: usize) -> Grid {
    assert!(cols > 0, "cols must be positive");
    assert!(rows > 0, "rows must be positive");
    // ...
}

// DO: Return Result for recoverable errors
pub fn new(cols: usize, rows: usize) -> Result<Grid, Error> {
    if cols == 0 {
        return Err(Error::InvalidDimensions("cols must be positive"));
    }
    // ...
}

// DO: Use Option for missing values
pub fn get_cell(&self, col: usize, row: usize) -> Option<&Cell> {
    self.cells.get(row * self.cols + col)
}
```

### Module Organization

```rust
// lib.rs - Re-export at crate root
mod cell;
mod grid;
mod terminal;
mod parser;

pub use cell::{Cell, CellAttrs, CellFlags};
pub use grid::{Grid, Scrollback};
pub use terminal::Terminal;
pub use parser::Parser;
```

### Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn grid_write_updates_cell() {
        let mut grid = Grid::new(80, 24);
        let cell = Cell::new('A');
        grid.write(0, 0, cell.clone());
        assert_eq!(grid.get_cell(0, 0), Some(&cell));
    }
    
    #[test]
    fn parser_handles_basic_chars() {
        let mut parser = Parser::new();
        parser.parse(b"Hello");
        // Verify grid state
    }
}
```

---

## Code Review Checklist

Before submitting code:

- [ ] Does it compile without warnings?
- [ ] Are all public items documented?
- [ ] Are there tests for new functionality?
- [ ] Is there a benchmark for hot paths?
- [ ] Are unsafe blocks documented with SAFETY?
- [ ] Are Result/Option used instead of panics?
- [ ] Is naming consistent with project patterns?
- [ ] Are comments explaining why, not what?
- [ ] Is the PR focused on one thing?
- [ ] Are there no allocations in hot paths?