# Iris API Design

Interface contracts and boundaries between crates.

## Design Principles

### Crate Boundaries

Each crate has a single responsibility and clear boundaries:

```
┌─────────────────────────────────────────────────────────────┐
│                    iris-standalone                           │
│                   (Binary entry point)                      │
├─────────────────────────────────────────────────────────────┤
│                    iris-render-wgpu                         │
│                    (GPU rendering)                           │
├─────────────────────────────────────────────────────────────┤
│                      iris-core                               │
│            (Terminal state, parser, buffer)                  │
│                  NO WINDOWING DEPENDENCIES                   │
├─────────────────────────────────────────────────────────────┤
│                    iris-platform                             │
│              (PTY, clipboard, IME, fonts)                    │
│                  PLATFORM-SPECIFIC CODE                       │
└─────────────────────────────────────────────────────────────┘
```

### Dependency Rules

1. **iris-core has NO external dependencies on windowing/rendering**
2. **iris-platform depends on iris-core, implements traits**
3. **iris-render-wgpu depends on iris-core for state, NOT iris-platform**
4. **iris-standalone ties everything together**

---

## Core Traits

### PtyBackend

Platform abstraction for PTY operations.

```rust
pub trait PtyBackend {
    type Handle;
    
    fn spawn(&mut self, config: PtyConfig) -> Result<Self::Handle, PtyError>;
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, PtyError>;
    fn write(&mut self, data: &[u8]) -> Result<(), PtyError>;
    fn resize(&mut self, cols: usize, rows: usize) -> Result<(), PtyError>;
    fn is_alive(&self) -> bool;
    fn wait(&mut self) -> Result<Option<i32>, PtyError>;
}

pub struct PtyConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub cwd: Option<PathBuf>,
    pub cols: usize,
    pub rows: usize,
}
```

**Implementations:**
- `WindowsConPty` - Windows ConPTY via CreatePseudoConsole
- `UnixPty` - POSIX pty via openpty/fork/exec

### Clipboard

Platform abstraction for clipboard operations.

```rust
pub trait Clipboard {
    fn get_text(&self) -> Result<Option<String>, ClipboardError>;
    fn set_text(&self, text: &str) -> Result<(), ClipboardError>;
    fn get_primary(&self) -> Result<Option<String>, ClipboardError>;  // Linux X11 PRIMARY
    fn set_primary(&self, text: &str) -> Result<(), ClipboardError>;
}
```

**Implementations:**
- `WindowsClipboard` - Win32 clipboard API
- `MacosClipboard` - NSPasteboard
- `LinuxClipboard` - X11/Wayland clipboard

### FontProvider

Platform abstraction for font discovery and glyph rasterization.

```rust
pub trait FontProvider {
    fn enumerate(&self) -> Vec<FontInfo>;
    fn fallback_for(&self, c: char, style: FontStyle) -> Option<FontInfo>;
    fn rasterize(&self, font: &FontInfo, c: char, size: f32) -> Result<Glyph, FontError>;
}

pub struct FontInfo {
    pub family: String,
    pub path: PathBuf,
    pub style: FontStyle,
    pub weight: FontWeight,
}

pub struct Glyph {
    pub width: f32,
    pub height: f32,
    pub bearing_x: f32,
    pub bearing_y: f32,
    pub advance: f32,
    pub data: Vec<u8>,  // RGBA bitmap
}
```

### ImeHandler

Platform abstraction for IME input.

```rust
pub trait ImeHandler {
    fn set_ime_position(&self, x: f32, y: f32);
    fn ime_active(&self) -> bool;
    fn set_ime_enabled(&mut self, enabled: bool);
}
```

---

## Core Types

### Grid and Cell

```rust
pub struct Grid {
    cells: Box<[Cell]>,
    rows: usize,
    cols: usize,
    damage: DamageTracker,
    scrollback: Scrollback,
}

pub struct Cell {
    pub char: char,
    pub width: CellWidth,
    pub attrs_id: u16,
}

pub enum CellWidth {
    Normal = 1,
    Wide = 2,
    Zero = 0,
}

bitflags! {
    pub struct CellFlags: u16 {
        const BOLD = 0b00000001;
        const ITALIC = 0b00000010;
        const UNDERLINE = 0b00000100;
        const STRIKETHROUGH = 0b00001000;
        const INVERSE = 0b00010000;
        const DIM = 0b00100000;
        const BLINK = 0b01000000;
        const HIDDEN = 0b10000000;
    }
}

pub struct CellAttrs {
    pub fg: Color,
    pub bg: Color,
    pub flags: CellFlags,
}

pub enum Color {
    Default,
    Indexed(u8),
    TrueColor(Rgb),
}
```

### Terminal State

```rust
pub struct Terminal {
    grid: Grid,
    alternate_grid: Option<Grid>,
    cursor: Cursor,
    modes: Modes,
    tabs: TabStops,
    title: Option<String>,
    working_directory: Option<PathBuf>,
}

pub struct Cursor {
    pub col: usize,
    pub row: usize,
    pub visible: bool,
    pub style: CursorStyle,
    pub saved_col: usize,
    pub saved_row: usize,
    pub saved_attrs: CellAttrs,
}

pub enum CursorStyle {
    Block,
    Underline,
    Bar,
    BlinkingBlock,
    BlinkingUnderline,
    BlinkingBar,
}

pub struct Modes {
    pub origin: bool,
    pub wrap: bool,
    pub insert: bool,
    pub newline: bool,
    pub cursor_keys: bool,
    pub keypad: bool,
    pub bracketed_paste: bool,
    pub focus_tracking: bool,
    pub synchronized_output: bool,
}
```

### Parser

```rust
pub struct Parser {
    state: ParserState,
    params: ArrayVec<i64, 16>,
    intermediates: ArrayVec<u8, 4>,
    handler: Box<dyn Handler>,
}

pub trait Handler {
    fn print(&mut self, c: char);
    fn execute(&mut self, byte: u8);
    fn csi_dispatch(&mut self, params: &[i64], intermediates: &[u8], final: u8);
    fn osc_dispatch(&mut self, osc: &OscSequence);
    fn dcs_dispatch(&mut self, dcs: &DcsSequence);
    fn apc_dispatch(&mut self, apc: &ApcSequence);
    fn pm_dispatch(&mut self, pm: &PmSequence);
    fn sos_dispatch(&mut self, sos: &SosSequence);
}

pub enum ParserState {
    Ground,
    Escape,
    CsiEntry,
    CsiParam,
    CsiIntermediate,
    OscString,
    DcsEntry,
    DcsParam,
    DcsIntermediate,
    DcsPassthrough,
    StringParam,
}
```

### Selection

```rust
pub struct Selection {
    pub kind: SelectionKind,
    pub anchor: SelectionAnchor,
    pub cursor: SelectionAnchor,
    pub active: bool,
}

pub enum SelectionKind {
    Simple,  // Character-by-character
    Linear,  // Line-by-line
    Block,   // Rectangular
}

pub struct SelectionAnchor {
    pub col: usize,
    pub row: usize,
}
```

### Damage Tracking

```rust
pub struct DamageTracker {
    damaged_rows: BitSet,
    damaged_regions: Vec<Rect>,
}

impl DamageTracker {
    pub fn mark(&mut self, row: usize);
    pub fn mark_range(&mut self, start_row: usize, end_row: usize);
    pub fn take_regions(&mut self) -> Vec<Rect>;
    pub fn is_damaged(&self, row: usize) -> bool;
}
```

---

## Events

### Terminal Events

Events emitted by the terminal for the renderer/integrators.

```rust
pub enum TerminalEvent {
    // Content changes
    ContentDamaged { regions: Vec<Rect> },
    ScrollbackChanged { lines_added: usize },
    
    // Cursor changes
    CursorMoved { col: usize, row: usize },
    CursorStyleChanged { style: CursorStyle },
    CursorVisibilityChanged { visible: bool },
    
    // Terminal state
    TitleChanged { title: Option<String> },
    WorkingDirectoryChanged { path: Option<PathBuf> },
    Bell,
    
    // Selection
    SelectionChanged { selection: Option<Selection> },
    
    // Mode changes
    ModeChanged { mode: Mode, enabled: bool },
    
    // Terminal lifecycle
    Exited { exit_code: Option<i32> },
}
```

### Input Events

Events handled by the terminal.

```rust
pub enum InputEvent {
    KeyPress { key: Key, modifiers: Modifiers },
   KeyPressRelease { key: Key, modifiers: Modifiers },
    MousePress { button: MouseButton, col: usize, row: usize, modifiers: Modifiers },
    MouseRelease { button: MouseButton, col: usize, row: usize, modifiers: Modifiers },
    MouseMove { col: usize, row: usize, modifiers: Modifiers },
    Scroll { delta: i32, col: usize, row: usize, modifiers: Modifiers },
    Resize { cols: usize, rows: usize },
    FocusGained,
    FocusLost,
    Paste { text: String },
}
```

---

## Error Handling

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("PTY error: {0}")]
    Pty(#[from] PtyError),
    
    #[error("Grid error: {0}")]
    Grid(#[from] GridError),
    
    #[error("Parser error: {0}")]
    Parser(#[from] ParserError),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum PtyError {
    #[error("Failed to spawn process: {0}")]
    SpawnFailed(String),
    
    #[error("PTY read failed: {0}")]
    ReadFailed(String),
    
    #[error("PTY write failed: {0}")]
    WriteFailed(String),
    
    #[error("PTY resize failed: {0}")]
    ResizeFailed(String),
}

#[derive(Debug, thiserror::Error)]
pub enum GridError {
    #[error("Invalid cell position: ({col}, {row})")]
    InvalidPosition { col: usize, row: usize },
    
    #[error("Grid resize failed")]
    ResizeFailed,
}
```

### Result Types

```rust
pub type Result<T> = std::result::Result<T, Error>;
pub type PtyResult<T> = std::result::Result<T, PtyError>;
pub type GridResult<T> = std::result::Result<T, GridError>;
```

---

## Versioning

### API Stability

- **Public API**: Types and traits marked `pub` in `lib.rs` are stable
- **Internal API**: Types marked `pub(crate)` may change between versions
- **Experimental API**: Types in `experimental` module may change

### Breaking Changes

Breaking changes require major version bump and migration guide.

### Deprecation

- Deprecate with `#[deprecated]` attribute
- Document replacement in doc comment
- Keep deprecated API for at least one major version

---

## Testing Contract

Each crate must provide:

```rust
// Unit tests in src/ module files
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_grid_write() { /* ... */ }
}

// Integration tests in tests/
// tests/integration_grid.rs

// Benchmarks in benches/
// benches/parser.rs
```

See `testing-strategy.md` for full testing requirements.