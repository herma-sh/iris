# Iris Implementation

Core components, code patterns, and technical details.

## Core Components

### 1. Grid and Buffer Model

The buffer is the foundation. All terminal state lives here.

#### Cell Structure

```rust
pub struct Cell {
    pub char: char,
    pub width: CellWidth,      // 1, 2, or 0 (for continuation)
    pub attrs_id: u16,         // Index into attributes table
}

pub enum CellWidth {
    Normal = 1,   // Single-width character
    Wide = 2,      // Double-width (CJK, wide emoji)
    Zero = 0,      // Continuation cell or zero-width
}

bitflags! {
    pub struct CellFlags: u16 {
        const BOLD          = 0b00000001;
        const ITALIC        = 0b00000010;
        const UNDERLINE     = 0b00000100;
        const STRIKETHROUGH = 0b00001000;
        const INVERSE       = 0b00010000;
        const DIM           = 0b00100000;
        const BLINK         = 0b01000000;
        const HIDDEN        = 0b100000000;
    }
}

pub struct CellAttrs {
    pub fg: Color,
    pub bg: Color,
    pub flags: CellFlags,
}
```

#### Grid Structure

```rust
pub struct Grid {
    cells: Box<[Cell]>,       // Pre-allocated, fixed size
    rows: usize,
    cols: usize,
    damage: DamageTracker,
    scrollback: Scrollback,
}

pub struct Scrollback {
    lines: Vec<Line>,
    head: usize,      // Index of oldest line
    len: usize,      // Number of valid lines
    capacity: usize, // Max lines before overwrite
}

pub struct Line {
    cells: Vec<Cell>,
    wrapped: bool,  // Is this a continuation from previous line?
}
```

#### Damage Tracking

Instead of redrawing everything, track which regions changed:

```rust
pub struct DamageTracker {
    damaged_rows: BitSet,       // Which rows have changes
    damaged_regions: Vec<Rect>,  // Bounded regions for GPU batching
}

impl Grid {
    pub fn write(&mut self, col: usize, row: usize, cell: Cell) {
        let idx = row * self.cols + col;
        self.cells[idx] = cell;
        self.damage.mark(row);
    }
    
    pub fn take_damage(&mut self) -> Vec<Rect> {
        self.damage.take_regions()
    }
}
```

### 2. Terminal State

```rust
pub struct Terminal {
    grid: Grid,
    cursor: Cursor,
    modes: TerminalModes,
    tabs: TabStops,
    title: Option<String>,
    bell: bool,
}

pub struct Cursor {
    pub col: usize,
    pub row: usize,
    pub visible: bool,
    pub style: CursorStyle,
    pub saved: Option<CursorPosition>,  // For save/restore
}

pub enum CursorStyle {
    Block,
    Underline,
    Bar,
    BlinkingBlock,
    BlinkingUnderline,
    BlinkingBar,
}

pub struct TerminalModes {
    pub origin: bool,           // Origin mode
    pub wrap: bool,             // Auto-wrap
    pub insert: bool,           // Insert mode
    pub newline: bool,          // Newline mode (LNM)
    pub cursor_keys: bool,      // Cursor key mode
    pub keypad: bool,           // Keypad mode
    pub screen_mode: ScreenMode,
}

pub enum ScreenMode {
    Normal,
    Alternate,  // Alternate screen buffer
}
```

### 3. Parser

ANSI/VT sequence parser. Handles output from PTY.

```rust
pub struct Parser {
    state: ParserState,
    params: Vec<i64>,
    intermediates: Vec<u8>,
    handler: Box<dyn Handler>,
}

pub trait Handler {
    fn print(&mut self, c: char);
    fn execute(&mut self, byte: u8);           // Control characters
    fn csi_dispatch(&mut self, params: &[i64], intermediates: &[u8], final: u8);
    fn osc_dispatch(&mut self, osc: &OscSequence);
    fn dcs_dispatch(&mut self, dcs: &DcsSequence);
    fn apc_dispatch(&mut self, apc: &ApcSequence);
    fn pm_dispatch(&mut self, pm: &PmSequence);
    fn sos_dispatch(&mut self, sos: &SosSequence);
}
```

**Parser handles:**
- ESC sequences (ESC [ ... sequences)
- CSI sequences (ESC [ params final)
- OSC sequences (ESC ] ... BEL/ST)
- DCS/APC/PM/SOS (device control, application program, privacy message, start of string)
- Control characters (LF, CR, TAB, BS, BEL, etc.)

### 4. Selection Model

```rust
pub struct Selection {
    pub kind: SelectionKind,
    pub start: SelectionAnchor,
    pub end: SelectionAnchor,
    pub active: bool,
}

pub enum SelectionKind {
    Simple,  // Character-wise
    Linear,   // Line-wise
    Block,    // Rectangular
}

pub struct SelectionAnchor {
    pub col: usize,
    pub row: usize,
}
```

### 5. Search Model

```rust
pub struct Search {
    pub query: String,
    pub case_sensitive: bool,
    pub regex: bool,
    pub matches: Vec<MatchPosition>,
    pub current_match: Option<usize>,
}

pub struct MatchPosition {
    pub start: Position,
    pub end: Position,
}
```

### 6. Platform Layer

```rust
pub trait PtyBackend {
    fn spawn(&mut self, config: PtyConfig) -> Result<PtyHandle, PtyError>;
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, PtyError>;
    fn write(&mut self, data: &[u8]) -> Result<(), PtyError>;
    fn resize(&mut self, cols: usize, rows: usize) -> Result<(), PtyError>;
    fn is_alive(&self) -> bool;
}

// Platform-specific implementations:
// - Windows: ConPTY via CreatePseudoConsole
// - Unix: POSIX pty via openpty/fork/exec

pub trait Clipboard {
    fn get_text(&self) -> Option<String>;
    fn set_text(&self, text: &str);
}

pub trait FontProvider {
    fn enumerate(&self) -> Vec<FontInfo>;
    fn fallback_for(&self, c: char) -> Option<FontInfo>;
}

pub trait ImeHandler {
    fn set_ime_position(&self, x: f32, y: f32);
    fn ime_active(&self) -> bool;
}
```

### 7. Renderer

```rust
pub struct WgpuRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface,
    
    // Resources
    glyph_cache: GlyphCache,
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    
    // State
    view: RenderView,
}

pub struct GlyphCache {
    atlas: Atlas,              // Texture atlas for glyphs
    font_system: FontSystem,   // rustybuzz or similar
    rasterizer: Rasterizer,
}

pub struct RenderView {
    grid: &Grid,               // Reference to terminal grid
    viewport: Viewport,        // Scroll position
    cursor: CursorInfo,        // For rendering
    selection: Option<Selection>,
}
```

#### Rendering Pipeline

1. Process damage regions from grid
2. Update glyph cache for any new characters
3. Build vertex buffer for visible cells
4. Batch by attribute (all bold together, etc.)
5. Submit to GPU
6. Render cursor overlay
7. Render selection overlay

## Zero-Allocation Hot Path

The parsing hot path (PTY output → parser → grid) must not allocate:

```rust
// DON'T: Allocate per character
fn parse_char(&mut self, c: char) {
    let cell = Cell { char: c, attrs: self.attrs.clone() };  // Allocates!
    self.grid.write(cell);
}

// DO: Write directly into pre-allocated buffer
fn parse_char(&mut self, c: char) {
    let idx = self.cursor.row * self.grid.cols + self.cursor.col;
    self.grid.cells[idx].char = c;
    self.grid.cells[idx].attrs_id = self.attrs_id;  // Just an index
}
```

## Font & Typography Strategy

### Font Requirements

1. **Primary**: User-configured monospace font
2. **Fallback 1**: System default monospace
3. **Fallback 2**: Noto Sans Mono (bundled, comprehensive Unicode)
4. **Emoji**: System emoji font or Noto Color Emoji
5. **CJK**: System CJK font or Noto Sans CJK

### Line Height

Never trust font metrics for line height. Use explicit line height (e.g., 1.2 × font size) and position glyphs precisely. This prevents:
- Inconsistent spacing between fonts
- Cut-off descenders/ascenders
- Alignment issues between cells

## File Structure

```
hermes/
├── crates/
│   ├── iris-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── grid.rs
│   │       ├── cell.rs
│   │       ├── terminal.rs
│   │       ├── cursor.rs
│   │       ├── parser.rs
│   │       ├── selection.rs
│   │       ├── search.rs
│   │       ├── theme.rs
│   │       └── damage.rs
│   ├── iris-platform/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── pty.rs
│   │       ├── clipboard.rs
│   │       ├── font.rs
│   │       ├── ime.rs
│   │       ├── keyboard.rs
│   │       └── platform/
│   │           ├── mod.rs
│   │           ├── windows.rs
│   │           ├── unix.rs
│   │           └── macos.rs
│   ├── iris-render-wgpu/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── renderer.rs
│   │       ├── glyph.rs
│   │       ├── atlas.rs
│   │       ├── pipeline.rs
│   │       └── shaders/
│   │           └── cell.wgsl
│   └── iris-standalone/
│       ├── Cargo.toml
│       └── src/
│           └── main.rs
├── apps/
│   └── iris/
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── config.rs
│           ├── window.rs
│           └── args.rs
└── packages/
    ├── terminal-contract/
    │   ├── package.json
    │   └── src/
    │       ├── index.ts
    │       ├── types.ts
    │       ├── events.ts
    │       └── commands.ts
    ├── terminal-react/
    │   ├── package.json
    │   └── src/
    │       ├── index.ts
    │       └── TerminalSurface.tsx
    └── terminal-theme/
        ├── package.json
        └── src/
            ├── index.ts
            └── themes/
                ├── dark.ts
                └── light.ts
```

## Dependencies

### Rust Crates

```toml
# iris-core
thiserror = "1.0"
bitflags = "2.4"
parking_lot = "0.12"  # Fast mutex for grid access
unicode-width = "0.1"
unicode-segmentation = "1.10"tracing = "0.1"

# iris-platform (Unix)
nix = { version = "0.27", features = ["term", "process", "signal"] }

# iris-platform (Windows)
windows = { version = "0.52", features = [
    "Win32_Foundation",
    "Win32_System_Console",
    "Win32_System_Threading",
    "Win32_Security",
]}

# iris-render-wgpu
wgpu = "0.19"
winit = "0.29"
```

### TypeScript

```json
{
  "dependencies": {
    "@tauri-apps/api": "^2.0.0"
  }
}
```