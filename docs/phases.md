# Iris Development Phases

Development phases and timeline for Iris terminal emulator.

## Phase Overview

| Phase | Name | Duration | Focus |
|-------|------|----------|-------|
| 0 | Foundation | 1-2 weeks | Crate structure, basic types |
| 1 | Core Parser | 2-3 weeks | ANSI/VT parsing |
| 2 | wgpu Renderer | 2-3 weeks | GPU rendering |
| 3 | Selection/Clipboard | 1 week | Mouse interaction |
| 4 | Scrollback/Search | 1-2 weeks | History navigation |
| 5 | Platform Polish | 2-3 weeks | Windows, IME, fonts |
| 6 | Standalone Binary | 1-2 weeks | Config, CLI |
| 7 | Tauri Embedding | 1-2 weeks | Hermes integration |

---

## Phase 0: Foundation

**Duration**: 1-2 weeks

### Goals

- Establish crate structure
- Basic Grid and Cell types
- Minimal Terminal struct
- Parser skeleton
- Windows ConPTY integration skeleton

### Deliverables

- [ ] `iris-core/` with Grid, Cell, Terminal types
- [ ] `iris-platform/` with PtyBackend trait
- [ ] Windows ConPTY implementation (basic)
- [ ] Parser framework with basic control character handling

### Tasks

#### iris-core

1. Create crate structure
2. Implement `Cell`, `CellAttrs`, `CellFlags`, `CellWidth`
3. Implement `Grid` with pre-allocated buffer
4. Implement `DamageTracker`
5. Implement `Cursor` and `CursorStyle`
6. Create `Terminal` struct (grid + cursor + modes)
7. Add basic `TerminalModes` struct
8. Write unit tests for grid operations

#### iris-platform

1. Create crate structure
2. Define `PtyBackend` trait
3. Define `Clipboard` trait
4. Define `FontProvider` trait
5. Define `ImeHandler` trait
6. Create platform module structure
7. Implement Windows ConPTY backend (basic spawn/read/write)
8. Implement Unix PTY backend (basic spawn/read/write)

#### iris-render-wgpu

1. Create crate structure
2. Define `Renderer` trait
3. Define `GlyphCache` struct (empty implementation)

### No Rendering Yet

This phase is purely data structures and IO. No GPU rendering.

---

## Phase 1: Core Parser

**Duration**: 2-3 weeks

### Goals

- Full ANSI/VT parser
- CSI sequence handling
- OSC sequence handling (title, clipboard, hyperlink)
- Terminal mode switching
- Cursor movement and positioning

### Deliverables

- [ ] Parser passes vttest basic sequences
- [ ] Grid updates correctly from parser output
- [ ] Cursor positioning works

### Tasks

#### Parser Implementation

1. Implement state machine for parsing
2. Handle ESC sequences
3. Handle CSI sequences (cursor movement, erase, scroll)
4. Handle OSC sequences (title, clipboard, hyperlink)
5. Handle DCS/APC/PM/SOS (device control, etc.)
6. Handle control characters (LF, CR, TAB, BS, BEL)
7. Implement charset handling (G0-G3)
8. Implement screen modes (normal, alternate)

#### Grid Operations

1. Implement cursor movement commands
2. Implement erase commands (line, screen)
3. Implement scroll regions
4. Implement tab stops
5. Implement character attributes (bold, italic, etc.)
6. Implement 16-color and256-color modes
7. Implement true color (24-bit)

### Testing

- Run vttest suite
- Test individual CSI sequences
- Test control sequences
- Benchmark parser throughput

---

## Phase 2: wgpu Renderer

**Duration**: 2-3 weeks

### Goals

- wgpu init and window surface
- Glyph rasterization and cache
- Cell batch rendering
- Cursor rendering
- Basic scrolling

### Deliverables

- [ ] Render terminal content to window
- [ ] Visible text matches terminal state
- [ ] Smooth scrolling

### Tasks

#### wgpu Setup

1. Initialize wgpu device and queue
2. Create window surface (via winit)
3. Create render pipeline
4. Create uniform buffer for viewport

#### Glyph Cache

1. Implement texture atlas
2. Implement glyph rasterization (using rustybuzz or similar)
3. Implement font fallback chain
4. Cache frequently used glyphs
5. Handle wide characters (CJK, emoji)

#### Cell Rendering

1. Create vertex buffer for cells
2. Batch cells by attribute (bold, italic, etc.)
3. Submit draw calls
4. Handle viewport scrolling

#### Cursor Rendering

1. Create cursor geometry
2. Animate cursor blink (if enabled)
3. Handle cursor styles (block, underline, bar)

### Testing

- Visual output matches expected content
- Scroll performance benchmarks
- Memory usage with large fonts

---

## Phase 3: Selection and Clipboard

**Duration**: 1 week

### Goals

- Mouse selection (simple, linear, block)
- Copy to clipboard
- Paste from clipboard
- Selection rendering

### Deliverables

- [ ] Select and copy text
- [ ] Paste into terminal
- [ ] Visual selection feedback

### Tasks

#### Selection Model

1. Implement `Selection` struct
2. Implement selection kinds (simple, linear, block)
3. Implement selection anchors
4. Handle mouse drag for selection
5. Handle keyboard modifiers (Shift+Arrow)

#### Clipboard

1. Integrate with `Clipboard` trait
2. Implement copy operation
3. Implement paste operation
4. Handle bracketed paste mode

#### Selection Rendering

1. Render selection highlight
2. Handle scroll during selection
3. Handle selection across wrapped lines

---

## Phase 4: Scrollback and Search

**Duration**: 1-2 weeks

### Goals

- Scrollback buffer
- Scroll position tracking
- Search forward/backward
- Search highlighting

### Deliverables

- [ ] Navigate history
- [ ] Find text in scrollback
- [ ] Jump to matches

### Tasks

#### Scrollback Buffer

1. Implement ring buffer for scrollback
2. Implement scroll position tracking
3. Implement scroll commands (Page Up/Down, etc.)
4. Handle alternate screen buffer switching

#### Search

1. Implement `Search` struct
2. Implement forward search
3. Implement backward search
4. Implement regex search (optional)
5. Implement search highlighting
6. Handle search in scrollback

---

## Phase 5: Platform Polish

**Duration**: 2-3 weeks

### Goals

- Full Windows PTY handling
- Keyboard input normalization
- IME support
- Font fallback
- High DPI

### Deliverables

- [ ] Runs correctly on Windows with all input modes
- [ ] Fonts render with proper fallback
- [ ] Respects DPI settings

### Tasks

#### Windows

1. Complete ConPTY implementation
2. Handle Windows-specific escape sequences
3. Implement clipboard via Win32 API
4. Implement font enumeration via DirectWrite
5. Handle high DPI (Per-Monitor DPI)

#### Unix

1. Complete PTY implementation
2. Handle X11/Wayland clipboard
3. Implement font enumeration via fontconfig

#### macOS

1. Complete PTY implementation
2. Handle NSPasteboard
3. Implement font enumeration via Core Text

#### IME

1. Handle IME composition
2. Position IME window near cursor
3. Handle commit and cancel

---

## Phase 6: Standalone Binary

**Duration**: 1-2 weeks

### Goals

- Config file (TOML)
- CLI argument parsing
- Window management
- "Open with Iris" registration

### Deliverables

- [ ] `iris` binary runs standalone
- [ ] Configuration file works
- [ ] Right-clickfolder → Open with Iris

### Tasks

#### Configuration

1. Define TOML config schema
2. Implement config loading
3. Implement config hot-reload (optional)
4. Handle font configuration
5. Handle theme configuration

#### CLI

1. Implement argument parsing (clap or similar)
2. Handle `-e` command execution
3. Handle `--working-directory`
4. Handle `--config` path

#### Platform Integration

1. Windows: Register as "Open with" handler
2. macOS: Create app bundle
3. Linux: Create .desktop file

---

## Phase 7: Tauri Embedding

**Duration**: 1-2 weeks

### Goals

- Tauri plugin for Iris
- Surface sharing via raw-window-handle
- Event forwarding (input, resize)
- Lifecycle management

### Deliverables

- [ ] Iris renders inside Hermes webview
- [ ] Input forwarding works
- [ ] Multiple terminal tabs

### Tasks

#### Tauri Plugin

1. Create Tauri plugin structure
2. Expose Iris surface via raw-window-handle
3. Bridge input events
4. Bridge resize events
5. Handle lifecycle (create, destroy terminals)

#### TypeScript Integration

1. Create `@hermes/terminal-contract` package
2. Define TypeScript interfaces
3. Create `@hermes/terminal-react` component
4. Handle multiple terminal instances

---

## Testing Strategy

### Unit Tests

- Grid operations (write, scroll, damage tracking)
- Parser sequences (each CSI, OSC)
- Selection edge cases
- Search correctness

### Integration Tests

- PTY spawn and read/write
- Parser + Grid integration
- Renderer output comparison (screenshot diffs)

### Performance Tests

- Bench parser throughput (MB/s)
- Bench render latency (ms/frame)
- Memory under load

### Conformance Tests

- vttest
- DEC VT test patterns
- Kitty graphics protocol (if implemented)

---

## Risk Mitigation

###parser Bugs

- **Risk**: Incorrect handling of escape sequences
- **Mitigation**: Extensive vttest coverage, compare against Alacritty/WezTerm behavior

### Performance Issues

- **Risk**: Not meeting latency/fps targets
- **Mitigation**: Benchmarks in CI, performance regression tests

### Cross-Platform Issues

- **Risk**: Windows/macOS/Linux behavior differences
- **Mitigation**: Test on all platforms, platform-specific CI jobs

### Unicode Edge Cases

- **Risk**: Incorrect width for CJK/emoji
- **Mitigation**: Comprehensive Unicode test suite, update Unicode data regularly

---

## Success Criteria

Iris v1 is complete when:

1. ✅ Runs standalone on Windows, Linux, macOS
2. ✅ Renders at 60fps with smooth scrolling
3. ✅ Passes vttest basic sequences
4. ✅ Handles real-world workloads (tmux, htop, vim)
5. ✅ Input latency < 16ms
6. ✅ Memory usage < 50MB at 10k scrollback
7. ✅ Embeds into Hermes via Tauri
8. ✅ "Open with Iris" works on Windows