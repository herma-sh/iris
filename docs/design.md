# Iris Design

Architecture, crate structure, and design decisions.

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Host Applications                        │
├────────────────────────────┬────────────────────────────────┤
│      Iris Standalone        │         Hermes (Tauri)          │
│      (winit window)         │    (webview + Tauri plugin)     │
├────────────────────────────┴────────────────────────────────┤
│                    Iris Render Layer                         │
│                    (wgpu + surface abstraction)              │
├──────────────────────────────────────────────────────────────┤
│                      Iris Core                               │
│           (Terminal state, parser, buffer, search)           │
├──────────────────────────────────────────────────────────────┤
│                    Iris Platform                             │
│              (PTY, clipboard, IME, fonts)                    │
└──────────────────────────────────────────────────────────────┘
```

## Data Flow

```
PTY bytes → Parser → Grid (write damaged cells)
                       ↓
                  Renderer (read damaged regions)
```

## Crate Structure

### iris-core

Terminal state model, parser, buffer. **No windowing or render dependencies.**

```
iris-core/
├── lib.rs
├── cell.rs          # Cell, CellAttrs, CellFlags
├── grid.rs          # Grid, Row, Scrollback
├── damage.rs       # DamageTracker, damage regions
├── cursor.rs        # Cursor position and state
├── terminal.rs      # Terminal model (grid + cursor + modes)
├── parser.rs        # ANSI/VT sequence parser
├── selection.rs    # Selection model (simple, linear, block)
├── search.rs        # Search model
├── theme.rs         # Theme definitions
└── events.rs        # Event types
```

**Dependencies:**
- `bitflags` - Cell flags
- `thiserror` - Error types
- `parking_lot` - Fast mutex for grid access
- `unicode-width` - Character width calculation

### iris-platform

Platform integration. PTY, clipboard, IME, fonts.

```
iris-platform/
├── lib.rs
├── pty.rs           # PtyBackend trait
├── clipboard.rs     # Clipboard trait
├── font.rs          # FontProvider trait
├── ime.rs           # ImeHandler trait
├── keyboard.rs      # Keyboard normalization
└── platform/
    ├── mod.rs
    ├── windows.rs   # ConPTY, DirectWrite, Win32
    ├── unix.rs      # POSIX pty, X11 clipboard
    └── macos.rs     # macOS pty, NSPasteboard
```

**Dependencies:**
- `iris-core` - Terminal types
- `nix` (Unix) - PTY
- `windows` crate (Windows) - ConPTY, Win32 APIs

### iris-render-wgpu

wgpu-based renderer. GPU acceleration.

```
iris-render-wgpu/
├── lib.rs
├── renderer.rs      # WgpuRenderer main struct
├── glyph.rs         # Glyph rasterization
├── atlas.rs         # Texture atlas for glyphs
├── pipeline.rs      # Render pipeline setup
└── shaders/
    └── cell.wgsl    # Cell rendering shader
```

**Dependencies:**
- `wgpu` - GPU abstraction
- `winit` - Window creation
- `iris-core` - Terminal types

### iris-standalone

Standalone binary. winit window, config, CLI.

```
iris-standalone/
├── main.rs
├── config.rs        # TOML config loading
├── window.rs        # winit window management
└── args.rs          # CLI argument parsing
```

### TypeScript Integration

```
packages/
  terminal-contract/   # TypeScript host API
  terminal-react/      # React host component
  terminal-theme/       # Theme definitions
```

## Design Principles

### For Speed

1. **Zero-copy parsing** - Parse PTY output directly into grid cells without intermediate buffers
2. **Damage tracking** - Only touch cells that changed, only render regions that changed
3. **GPU-first** - All rendering through wgpu batch calls, no CPU rasterization per frame
4. **Fixed-size buffers** - Pre-allocate grid and scrollback buffers, no per-keystroke allocations
5. **Frame pacing** - Target consistent 16.67ms frame budget, drop frames before lagging input
6. **Lock-free reads** - Grid uses Arc<GridInner> for concurrent render reads without mutex
7. **SIMD where it matters** - Parser can use SIMD for finding escape sequences in runs

### For Beauty

1. **Typography-first** - Choose fonts for readability, not novelty
2. **Restraint** - Minimal chrome, focus on content
3. **Precision** - Subpixel-accurate positioning, consistent spacing
4. **Responsiveness** - Immediate visual feedback, no visual lag
5. **Density without clutter** - Show more content, less interface chrome

### For Flicker Prevention

| Flicker Type | Root Cause | Iris Prevention |
|--------------|------------|-----------------|
| Screen clear flash | Buffer cleared before redraw | Never clear - overwrite only |
| Cursor blink artifacts | Cursor drawn in separate pass | Cursor in same GPU pass |
| Resize flash | Sync reflow blocks UI thread | Async reflow, progressive paint |
| Scroll tear | Buffer swap mid-frame | VSync + double buffer |
| Burst output flicker | Parser can't keep up | Throttle + frame budget |
| Selection flicker | Computed after scroll | Lock scroll position during selection |
| Font load flash | Fallback font shows first | Pre-warm font cache |
| DPI change flash | Content jumps on scale change | Scale content, not position |

### For Code Quality

1. **No premature optimization** - Correctness first, then measure, then optimize
2. **Clear data flow** - PTY → Parser → Grid → Render should be traceable in code
3. **Explicit allocation** - Every heap allocation documented; no hidden Vec pushes in hot paths
4. **Zero unsafe in iris-core** - All safety invariants enforced at compile time
5. **Minimal unsafe in iris-platform** - Required for FFI, clearly commented
6. **Single responsibility** - Each module does one thing
7. **No panics in production** - All Result types handled; unwrap() banned outside tests

## Thread Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Main Thread                             │
│  - Event loop (winit/wgpu)                                   │
│  - Input handling                                            │
│  - Render dispatch                                            │
│  - ~16ms budget per frame                                     │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                     PTY Thread                               │
│  - Read from PTY (blocking)                                   │
│  - Write to parser output queue                               │
│  - Throttle if queue is full                                  │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                    Parser Thread                             │
│  - Consume PTY output                                         │
│  - Parse ANSI sequences                                       │
│  - Write to grid (Arc<Mutex<Grid>> or lock-free)             │
│  - Yield to main thread at frame boundary                     │
└─────────────────────────────────────────────────────────────┘
```

### Lock Strategy

- **Grid reads (render)**: Lock-free via Arc<GridSnapshot>
- **Grid writes (parser)**: Mutex protected, but only for write path
- **Damage queue**: Lock-free SPSC (single producer, single consumer)
- **Input events**: Lock-free MPMC queue

## Memory Management

### Ring Buffer for Scrollback

```rust
struct Scrollback {
    lines: Vec<Line>,
    head: usize,      // Index of oldest line
    len: usize,       // Number of valid lines
    capacity: usize,  // Max lines before overwrite
}
```

### Pre-allocated Grid

```rust
struct Grid {
    cells: Box<[Cell]>,  // Pre-allocated, fixed size
    rows: usize,
    cols: usize,
}
```

### Cell Compression

Most cells share attributes (same FG, BG, bold, etc.). Store unique attributes once, reference by index.

```rust
struct Cell {
    char: char,
    width: CellWidth,
    attrs_id: u16,  // Index into attributes table
}
```

## Windows-Specific Design

These are design-time concerns, not afterthoughts:

### PTY

- Use ConPTY (CreatePseudoConsole) for Windows 10 1809+
- Handle resize timing (must happen before process start or via specific API)
- Some escape sequences behave differently on Windows

### Keyboard

- Distinguish physical vs virtual key codes
- Handle AltGr correctly (Ctrl+Alt equivalent on some layouts)
- Dead key composition
- IME composition window positioning

### Fonts

- Use DirectWrite for font enumeration and fallback
- Handle font linking (Windows font substitution chain)
- CJK font fallback chains are complex

### High DPI

- Per-monitor DPI awareness
- DPI change handling (move between monitors)
- Text scaling factor

## Input Latency Path

```
Key press → winit event → IME composition (if active) → 
Parser (for control sequences) → Terminal::input() → 
PTY write → Render same frame
```

Target: **< 4ms** from key press to screen update.

## References

- [VT100 Programmer Pocket Guide](https://vt100.net/docs/vt100-ug/)
- [ECMA-48](https://www.ecma-international.org/publications-and-standards/standards/ecma-48/)
- [XTerm Control Sequences](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html)