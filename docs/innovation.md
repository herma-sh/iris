# Iris Innovation opportunities

Technological advances and underutilized features that could differentiate Iris.

## Overview

Most terminal emulators focus on the same baseline features: GPU rendering, true color, Unicode support. This document identifies emerging and underutilized technologies that could give Iris a unique competitive advantage.

---

## 1. Synchronized Output (DECSET 2026)

**Status**: Supported by Windows Terminal, iTerm2, Contour
**Underutilization**: Most terminals don't advertise or encourage using it

### What It Does

Synchronized output (DECSET 2026) allows applications to batch updates, preventing mid-frame tearing during complex redraws.

```ansi
CSI ? 2026 h  # Begin synchronized update mode
CSI ? 2026 l  # End synchronized update mode
```

When enabled, the terminal buffers all output without rendering. When disabled, the terminal atomically displays the entire batch.

### Why It Matters

- Eliminates flicker during complex TUI redraws
- Prevents visual tearing when applications redraw status bars, panes, etc.
- Critical for smooth animations in terminal UIs
- Zero application complexity increase (just wrap redraw in sequence)

### Iris Opportunity

**Implement as opt-in for embedded mode**: Hermes/imux can use synchronized output when composing complex interfaces. When embedding Iris in a multi-pane workspace, batch all terminal updates together.

**Advertise support**: Expose DECSET 2026 capability so applications can use it.

---

## 2. Predictive Local Echo

**Status**: Implemented by Mosh, VS Code Terminal (limited)
**Underutilization**: Most terminals don't offer this as a first-class feature

### What It Does

When typing over high-latency connections (SSH), the terminal predicts what the server will echo and displays it immediately, correcting if the prediction was wrong.

### Why It Matters

- typing over SSH feels instant even with 100+ ms latency
- Eliminates perceived lag in remote sessions
- Critical for professional workflows (SSH is core use case for Hermes)

### Iris Opportunity

**Built-in predictive echo for SSH sessions**:

```rust
// When PTY echo is expected (normal shell mode)
// Render predicted character immediately
// When actual echo arrives, compare and correct if needed

fn handle_keypress(&mut self, key: char) {
    if self.predictive_echo_enabled {
        self.render_predicted(key);  // Instant visual
        self.send_to_pty(key);        // Async send
    }
}

fn handle_pty_output(&mut self, data: &[u8]) {
    if self.predictive_echo_enabled {
        let predicted = self.take_predicted();
        if predicted != data {
            self.correct_prediction(predicted, data);
        }
    }
}
```

**This is a unique selling point**: Most terminals just accept latency. Iris can actively mitigate it.

---

## 3. OSC 8 Hyperlinks

**Status**: Widely supported (Alacritty, Kitty, WezTerm, iTerm2, Windows Terminal)
**Underutilization**: Many terminal apps don't generate them; many users don't know they exist

### What They Do

OSC 8 allows arbitrary text to become a clickable hyperlink, like HTML `<a>` tags:

```ansi
ESC ] 8 ; ; https://example.com ESC \ Link text ESC ] 8 ; ; ESC \
```

### Why It Matters

- `git log` can link commit hashes to GitHub/GitLab
- Compiler errors can link to source files
- Package managers can link to documentation
- Security: explicit links prevent URL spoofing

### Iris Opportunity

**First-class hyperlink support in embedded mode**:

- Render links with subtle underline styling
- Show full URL on hover (security)
- Ctrl+Click to open in default browser
- Configurable link patterns (e.g., link issue numbers to Jira)

---

## 4. Kitty Graphics Protocol

**Status**: Implemented by Kitty, WezTerm, Konsole
**Underutilization**: Most terminals don't implement it; application support is sparse

### What It Does

Kitty's graphics protocol allows embedding images, video, and complex graphics directly in the terminal:

```ansi
ESC _ G a=T,f=100; <base64 encoded PNG data> ESC \
```

Features:
- True color RGBA images
- Z-order (layering)
- Animation support
- Upload once, display many times (cache)
- Scaling, cropping, positioning

### Why It Matters

- `ls` with image previews
- `git diff` with rendered images
- Inline charts and graphs in CLI tools
- Potential for inline video

### Iris Opportunity

**Implement Kitty graphics protocol Phase 1 (static images)**:

This is a differentiator. Most terminals don't have it, and the ones that do (Kitty) are macOS/Linux-only. Iris could be the best Windows terminal with native image support.

**Use cases for Hermes**:
- Image previews in file browser
- Inline git diff images
- Chat/relay messages with embedded images

---

## 5. Semantic Prompt Markers (OSC 133)

**Status**: Supported by WezTerm, Ghostty, iTerm2
**Underutilization**: Most users don't enable shell integration

### What They Do

OSC 133 marks up the terminal output with semantic zones:

```ansi
OSC 133 ; A    # Mark start of input (prompt)
OSC 133 ; B    # Mark start of output
OSC 133 ; C    # Mark end of output
```

### Why It Matters

- Precise command detection for history
- Accurate copy of "last command output"
- Smart scrolling (jump between commands)
- Better workflow automation

### Iris Opportunity

**Auto-enable shell integration**:

1. Detect shell type on session start
2. Inject appropriate OSC 133 sequences via shell RC files
3. Use semantic zones for:
   - Copy last output (Cmd/Ctrl+Shift+C with context)
   - Navigate command history visually
   - Auto-scroll to last prompt

---

## 6. Desktop Notifications (OSC 9/777)

**Status**: Supported by many terminals
**Underutilization**: Rarely exposed as configurable features

### What They Do

Trigger desktop notifications from terminal:

```ansi
OSC 9 ; <message> ESC \          # Windows/iTerm2 style
OSC 777 ; notify ; <title> ; <body> ESC \  # Extended
```

### Why It Matters

- Long-running command completion
- Build failure alerts
- CI/CD pipeline notifications
- Unobtrusive status updates

### Iris Opportunity

**Configurable notification rules**:

```toml
# iris.toml
[notifications]
# Notify when command runs longer than threshold
long_running = { threshold = "30s", command_patterns = ["cargo build", "make", "npm install"] }

# Notify on exit codes
exit_code = { notify_on = [1, 2], exclude_patterns = ["test failures"] }

# Integration with Hermes
forward_to_hermes = true
```

---

## 7. Sixel Graphics

**Status**: Supported by xterm, iTerm2, mintty, WezTer, Contour, Windows Terminal
**Underutilization**: Considered "legacy" but useful

### What It Does

Sixel is a legacy bitmap graphics protocol from DEC terminals (1980s). Encodes images as 6-pixel-high strips.

### Why Consider It

- Broader compatibility than Kitty graphics
- Works over SSH without extra setup
- Decent image quality for many use cases
- Supported by many modern terminals

### Iris Position

**Implement Sixel as fallback, Kitty protocol as primary**:

- Detection: Query terminal capabilities (DA1/DA2)
- If Kitty protocol supported → use Kitty
- If Sixel supported → use Sixel
- Otherwise → text fallback

---

## 8. Deep Color Support

**Status**: No terminal supports >24-bit currently
**Opportunity**: First-to-market

### Color Depth Progression

| Depth | Bits Per Channel | Colors | Status |
|-------|------------------|--------|--------|
| 8-bit | 8 | 16.7M | Ancient (256 palette) |
| 24-bit | 8 | 16.7M | Standard now |
| 30-bit | 10 | 1B+ | HDR monitors |
| 36-bit | 12 | 68B+ | Professional displays |

### Why Deep Color Matters

Modern displays support:
- Display-P3 (wider gamut than sRGB)
- Rec.2020 (even wider)
- HDR10/Dolby Vision (higher brightness range)

Terminals currently clip these to sRGB. Iris can be first to support:
- 10-bit color channels
- Wider gamut rendering
- HDR-aware themes

### Implementation

```rust
struct Color {
    r: u16,  // 10-16 bits instead of 8
    g: u16,
    b: u16,
    a: u16,
    space: ColorSpace,  // sRGB, DisplayP3, Rec2020
}

enum ColorSpace {
    SRGB,
    DisplayP3,
    Rec2020,
}
```

This is forward-looking - most terminals don't think about this today.

---

## 9. Smart Tabs and Persistent History

**Status**: Implemented by Tabby, but often poorly
**Opportunity**: Do it right

### Smart Tabs

Most terminals create new tabs/connections without checking if one already exists.

| Behavior | Implementation |
|----------|---------------|
| Auto-detect existing | Hash connection params (host+user+port) |
| Focus existing tab | Instead of creating duplicate |
| Per-directory history | Match by working directory |
| Smart reconnect | Attach to existing tmux/screen session |
| Auto-naming | hostname:directory format |

### Persistent History

| Feature | Implementation |
|---------|---------------|
| Cross-session persistence | SQLite or JSON history file |
| Per-host separation | Separate history per SSH host |
| Per-directory context | Track command + directory |
| Fuzzy search | fzf-style history search |
| Exclusions | Regex patterns for sensitive commands |
| Sync (optional) | Export/import history |

---

## 10. First-Class UX Patterns

**Status**: Often overlooked, varies by terminal
**Opportunity**: Best-in-class speed and UX

### Multiple Input Methods

Every action should have multiple paths:

| Action | Mouse | Keyboard | Menu |
|--------|-------|----------|------|
| Copy | Select → release (auto-copy) | Ctrl+Shift+C | Right-click |
| Paste | Middle-click | Ctrl+Shift+V | Right-click |
| New tab | Click + button | Ctrl+Shift+T | File menu |
| Close tab | Click X | Ctrl+Shift+W | File menu |
| Split pane | Drag tab | Ctrl+Shift+E | View menu |

### Keyboard Flexibility

Support multiple keyboard conventions:

| Convention | Windows/Linux | macOS |
|------------|---------------|-------|
| Ctrl+C copy | Ctrl+C (no selection) | Cmd+C |
| Ctrl+Shift+C copy | Ctrl+Shift+C | Cmd+Opt+C |
| Both work | Yes | Yes |

Why? Users come from different backgrounds:
- iTerm2 users expect Cmd+C
- Windows Terminal users expect Ctrl+Shift+C
- bash users expect Ctrl+C to interrupt
- Make both work contextually

### Clipboard Harmony

| Clipboard | Purpose | Shortcut |
|-----------|---------|----------|
| PRIMARY | Selection, middle-click paste | Mouse only |
| CLIPBOARD | Standard copy/paste | Ctrl+Shift+C/V |

On Linux, both should work. On macOS/Windows, PRIMARY isn't used.

### Instant Feedback

| Action | Feedback |
|--------|----------|
| Key press | Character appears < 4ms |
| Copy | Brief highlight flash |
| Paste | Content appears immediately |
| Tab switch | Instant, no fade |
| Split resize | Live resize, no ghost |

### No-Modal Patterns

Avoid blocking operations:

| Instead of | Do this |
|-------------|----------|
| Modal dialog for paste warning | Inline warning, paste anyway |
| Blocking search | Asynchronous, results stream in |
| Modal config | Sidebar config, instant apply |

### Hover Information

| Hover Target | Show |
|---------------|------|
| URL | Full URL in status bar |
| OSC 8 link | Show destination, click to open |
| Cell | Unicode codepoint, width |
| Prompt | Duration, exit code |

---

## 11. Bidirectional Text Support

**Status**: Poor in most terminals
**Underutilization**: Critical for RTL languages, rarely handled correctly

### The Problem

Most terminals assume left-to-right text. Arabic, Hebrew, and other RTL languages:

- Need proper direction detection
- Need proper shaping (glyphs change based on position)
- Need proper cursor movement

### Iris Opportunity

**First-class RTL support**:

This is underserved. Correct bidi implementation could make Iris the go-to terminal for international users.

Implementation:
1. Detect text direction (using Unicode bidi algorithm)
2. Apply proper shaping (via harfbuzz or similar)
3. Handle cursor movement correctly (visual vs logical)

---

## 12. Structured Debug Output

**Status**: Novel concept, not implemented
**Underutilization**: New idea

### The Concept

Terminal output is unstructured text. What if applications could mark regions with semantic meaning?

```ansi
OSC 899 ; type=error ; file=main.rs:42 ; ESC \
error[E0277]: the trait bound `i32: Display` is not satisfied
OSC 899 ; type=error-end ; ESC \
```

### Why It Matters

- IDE-like navigation without IDE
- Semantic search in scrollback
- Better error highlighting
- Advanced copy (copy just errors, copy just filenames)

### Iris Opportunity

**Define and implement a semantic markup protocol**:

This could be Iris's signature innovation—a protocol that applications can optionally use to provide richer context for their output.

---

## Ghostty's Unique Innovations

What Ghostty does that others don't, and how Iris can expand on them.

### 1. libghostty Core Architecture

**What Ghostty does:** A C-ABI compatible shared library that handles all terminal logic, with platform-native GUIs consuming it.

**Why it matters:**
- 90%+ code sharing between platforms
- Clean separation of concerns
- Embeddable in other applications
- Testable in isolation

**Iris opportunity:**
- Same architecture: `iris-core` (C-ABI compatible) + platform GUIs
- Embedded mode: Hermes, VS Code extension, web via WASM
- Standalone mode: Native window using same core

### 2. Native Platform UI

**What Ghostty does:**
- macOS: Swift + AppKit + SwiftUI + Metal
- Linux: Zig + GTK4 + OpenGL
- No cross-platform UI framework

**Why it matters:**
- Native look and feel on each platform
- Access to platform-specific features (Quick Look, Force Touch on macOS)
- Users expect platform conventions for keyboard shortcuts
- Better integration with system accessibility

**Iris opportunity:**
- Windows: Rust + wgpu + WinUI or raw Win32
- macOS: Swift + AppKit + wgpu/Metal
- Linux: Rust + wgpu + GTK4
- Each platform gets its own native chrome around `iris-core`

### 3. Metal Renderer with Ligatures

**What Ghostty does:** Only terminal with Metal renderer that supports ligatures without falling back to CPU rendering.

**Why it matters:**
- Ligatures (Fira Code, JetBrains Mono) are popular
- Other Metal terminals (or lack thereof) don't support them
- GPU rendering ligatures is harder - Ghostty solved it

**Iris opportunity:**
- wgpu supports Metal/Vulkan/DX12
- Ligature support must be in GPU path from day one
- Don't fall back to CPU for any text rendering

### 4. OSC 133 Deep Integration

**What Ghostty does:**
- Most complete OSC 133 implementation
- Auto-injects shell integration for bash/zsh/fish
- Visual debug overlay for OSC 133 regions
- Click-to-move-cursor within prompt (Fish 4.1+, Nushell 0.111+)

**Why it matters:**
- Semantic understanding of terminal content
- Jump between prompts accurately
- Copy command output with a single click
- Resize handling that preserves prompt

**Iris opportunity:**
- Match Ghostty's OSC 133 completeness
- Add visual inspector (like Ghostty's debug overlay)
- Consider: Click-to-move-cursor within prompt

### 5. Terminal Inspector

**What Ghostty does:** Built-in debugging tool that shows:
- Cell attributes in real-time
- Unicode properties
- Active color palette
- Font metrics and grid dimensions
- Kitty graphics protocol output
- OSC 133 regions overlay
- Mouse position

**Why it matters:**
- Essential for debugging TUI apps
- Understanding why text renders incorrectly
- Performance bottleneck identification
- No other terminal has this built-in

**Iris opportunity:**
- Build inspector from day one
- Make it accessible via keyboard shortcut
- Include in embedded mode for Hermes developers

### 6. Dedicated I/O Thread

**What Ghostty does:** Separate thread for PTY I/O to prevent input jitter during heavy output.

**Why it matters:**
- `yes` shouldn't block typing
- `cat huge_file` shouldn't lag keystrokes
- Maintains responsiveness under load

**Iris opportunity:**
- Architecture this in from the start
- Main thread: events + render
- I/O thread: PTY read + parse
- Lock-free queue between them

### 7. Built-in Multiplexer

**What Ghostty does:** Tabs, splits, sessions without tmux.

**Why it matters:**
- Most users don't want to learn tmux
- NativeUX for what tmux does in terminal
- Session persistence across restarts

**Iris opportunity:**
- For standalone mode, consider built-in multiplexer
- For embedded mode, Hermes/imux handles this

### 8. Quick Terminal (macOS)

**What Ghostty does:** Dropdown terminal from menu bar on macOS.

**Why it matters:**
- Instant access without context switch
- Popular feature (similar to iTerm2, Kitty)
- Native macOS integration

**Iris opportunity:**
- Windows: System tray + hotkey dropdown
- Consider for standalone mode

### 9. Shader-Based Theming

**What Ghostty does:** GLSL shaders for visual effects.

**Why it matters:**
- Custom CRT effects, animations
-_gamma correction, color transforms
- Creative customization

**Iris opportunity:**
- wgpu supports shaders
- Consider user-shader support in embedded mode

### 10. Grapheme Clustering Done Right

**What Ghostty does:** Correct rendering of multi-codepoint emojis and RTL scripts.

**Why it matters:**
- Many terminals get emojis wrong
- RTL (Arabic, Hebrew) is often broken
- Ghostty explicitly tests and handles edge cases

**Iris opportunity:**
- Use `unicode-segmentation` for grapheme clusters
- Test against Ghostty's test cases
- Make RTL a first-class concern

### 11. Privacy-First Design

**What Ghostty does:** No telemetry, no accounts, no cloud features.

**Why it matters:**
- Terminals handle sensitive data (passwords, keys)
- Users trust local-only software
- No network calls = no network latency

**Iris opportunity:**
- Emphasize local-first in positioning
- All features work offline
- If adding sync, make it optional and self-hosted

### 12. Zero-Dependency VT Parser

**What Ghostty does:** `libghostty-vt` has zero dependencies, compilable to WebAssembly.

**Why it matters:**
- Easier to embed in constrained environments
- Web-based terminal emulators can use it
- Smaller attack surface

**Iris opportunity:**
- Keep `iris-core` dependency-minimal
- Consider WASM compilation target
-嵌入式 applications benefit from minimal deps

---

## What Ghostty Doesn't Have (Iris Opportunities)

### Windows Support

Ghostty is macOS + Linux only. Iris can be the fastest terminal on Windows with:
- Native WinUI chrome
- DirectWrite for fonts
- DX12 via wgpu
- ConPTY integration

### Embedded Mode

Ghostty is standalone only. Iris can be:
- Embedded in Hermes
- Embedded in VS Code/Neovim via extension
- Web-based via iris-core WASM

### Structured Output Protocol

Ghostty uses OSC 133 for prompt detection. Iris could define:
- Semantic regions for errors/warnings
- Structured command metadata
- Rich copy (copy command + output as JSON)

---

## Advanced Performance Techniques

Niche findings and low-level optimizations that can give Iris a performance edge.

### SIMD Parser Optimization

**Status**: Experimental, not widely implemented
**Opportunity**: High

Traditional ANSI parsers process characters one-by-one with a state machine. SIMD can accelerate specific parts:

**ESC character scanning:**
```rust
// SIMD can scan 16-32 bytes at once looking for ESC (0x1b)
fn find_esc_simd(data: &[u8]) -> Option<usize> {
    use std::arch::x86_64::*;
    // Process 32 bytes at a time looking for 0x1b
    // When found, fall back to scalar parser for sequence handling
}
```

**Parameter parsing in CSI:**
- SIMD to find semicolons in `ESC[38;2;R;G;Bm`
- SIMD to convert digit sequences to integers
- Character classification (digit vs semicolon vs final byte)

**Implementation strategy:**
1. Start with scalar state machine (correctness)
2. Profile raw text throughput (MB/s)
3. Add SIMD ESC scanning for "run" detection
4. Add SIMD for parameter parsing hot paths

### Character Width Caching

**Status**: Common, but often suboptimal
**Opportunity**: Medium-High

`wcwidth()` is called for every character to determine display width (1, 2, or 0). For wide Unicode (CJK, emoji), this is expensive.

**Naive approach:**
```rust
fn char_width(c: char) -> usize {
    wcwidth(c)  // Lookup every time
}
```

**Optimized approach:**
```rust
struct WidthCache {
    // LRU cache for recently seen characters
    cache: LruCache<char, usize>,
    
    // Inline fast path for ASCII
    fn width(&mut self, c: char) -> usize {
        if c <= '\x7f' {
            return 1;  // ASCII fast path, no lookup
        }
        *self.cache.get_or_insert(c, || wcwidth(c))
    }
}
```

**Even better - inline tables:**
```rust
// Pre-computed width tables for most common ranges
// CJK ranges, emoji ranges handled with simple bounds check
fn estimate_width(c: char) -> usize {
    match c as u32 {
        0x0000..=0x007F => 1,                              // ASCII
        0x1100..=0x115F => 2,                              // Hangul Jamo (wide)
        0x2329..=0x232A => 2,                              // Angle brackets
        0x2E80..=0x9FFF => 2,                              // CJK ranges
        0x1F300..=0x1F9FF => 2,                            // Emoji
        _ => wcwidth_fallback(c),                          // Full lookup
    }
}
```

### Compression for Scrollback

**Status**: Implemented by WindTerm (claims 20-90% reduction)
**Opportunity**: High for memory-constrained scenarios

Scrollback doesn't need random access - it's append-only with FIFO eviction. Compression can dramatically reduce memory:

```rust
struct CompressedScrollback {
    // Each line is compressed individually
    lines: Vec<CompressedLine>,
    // Recent lines are uncompressed for fast access
    recent: Vec<Line>,
    recent_count: usize,  // How many lines to keep uncompressed
}

struct CompressedLine {
    data: Vec<u8>,  // LZ4 or Zstd compressed
    decompressed_size: usize,
}
```

**Trade-offs:**
- CPU cost: Decompression on scroll
- Memory benefit: 3-10x compression ratio typical
- Optimal for: Large scrollback limits (100k+ lines)

**When to use:**
- User configures scrollback >50k lines
- Memory pressure detected
- User enables "memory saver" mode

### Damage Tracking at Cell Level

**Status**: Implemented by all GPU terminals, but optimization varies
**Opportunity**: Medium

Track which cells changed, but optimize how you track:

**Row-level tracking:**
```rust
struct DamageTracker {
    // BitSet is more cache-efficient than Vec<bool>
    damaged_rows: BitSet,
}

// Only process rows that changed
fn render(&self) {
    for row in self.damage.damaged_rows() {
        self.render_row(row);
    }
}
```

**Region tracking (more granular):**
```rust
struct DamageTracker {
    // Track intersecting dirty rectangles
    dirty_rects: Vec<Rect>,
    
    fn add_damage(&mut self, rect: Rect) {
        // Merge with existing rects if overlapping
        // Reduces total number of GPU draw calls
    }
}
```

**GPU-specific optimization:**
- Damage tracking guides which cells to include in vertex buffer
- Smaller vertex buffer = faster GPU submission
- Critical at 60fps with large terminals (200+ columns)

### Async Reflow

**Status**: Rarely implemented (most block UI during resize)
**Opportunity**: High UX differentiator

When terminal resizes, all wrapped lines must be reflowed. Naive implementation:

```rust
// DON'T: Block on reflow
fn resize(&mut self, new_cols: usize) {
    self.grid.reflow_all(new_cols);  // Blocks UI
}
```

**Async approach:**
```rust
fn resize(&mut self, new_cols: usize) {
    // Start reflow in background
    self.reflow_state = Some(ReflowState {
        new_cols,
        current_row: 0,
        total_rows: self.grid.rows,
        viewport_anchor: self.viewport.anchor(),
    });
}

fn tick(&mut self) {
    // Reflow in chunks, yield to UI
    if let Some(state) = &mut self.reflow_state {
        for _ in 0..CHUNK_SIZE {
            if !state.advance(&mut self.grid) {
                break;
            }
        }
    }
}
```

**UI during reflow:**
- Keep viewport anchored
- Show partial results immediately
- Progressive refinement

### Semantic Wrap Preservation

**Status**: Novel concept
**Opportunity**: High UX differentiator

Most terminals treat reflow as purely visual - they don't track "semantic lines". This causes:

- Prompts to be cut off mid-line
- Error messages to wrap strangely
- Git diffs to become unreadable

**Semantic approach:**
```rust
struct Line {
    cells: Vec<Cell>,
    wrapped: bool,           // Continuation from previous?
    semantic_boundary: bool, // Start of semantic unit (prompt, error, etc.)
}

// During reflow, preserve semantic boundaries
fn reflow(&mut self, new_cols: usize) {
    for line in &mut self.lines {
        if line.semantic_boundary {
            // Try to keep semantic unit together
            // Add spacing if needed
        }
    }
}
```

**Detection heuristics:**
- Shell prompt detection (OSC 133 markers)
- Detect common patterns (`Error:`, `>>`, diff markers)
- User annotations (mark region as "keep together")

### Pre-allocated Attribute Table

**Status**: Common optimization
**Opportunity**: Medium

Most cells share the same attributes (same FG, BG, bold, etc.). Store unique attributes once:

```rust
struct Grid {
    cells: Box<[Cell]>,
    attrs_table: Vec<CellAttrs>,  // Unique attribute combinations
}

struct Cell {
    char: char,
    width: CellWidth,
    attrs_id: u16,  // Index into attrs_table
}
```

**Memory savings:**
- `CellAttrs` is ~16-24 bytes (FG, BG, flags)
- `attrs_id` is 2 bytes
- Most grids have <100 unique attribute combinations
- Savings: 14-22 bytes per cell at scale

**Implementation details:**
```rust
impl Grid {
    fn get_or_insert_attrs(&mut self, attrs: CellAttrs) -> u16 {
        if let Some(id) = self.attrs_index.get(&attrs) {
            return *id;
        }
        let id = self.attrs_table.len() as u16;
        self.attrs_table.push(attrs);
        self.attrs_index.insert(attrs, id);
        id
    }
}
```

### Zero-Copy PTY Read

**Status**: Common in high-performance terminals
**Opportunity**: Medium-High

Avoid copying PTY output to intermediate buffers:

```rust
// DON'T: Multiple copies
fn read_pty(&mut self) {
    let mut buf = [0u8; 4096];
    let n = self.pty.read(&mut buf)?;      // Copy 1
    let owned = buf[..n].to_vec();          // Copy 2
    self.parser.parse(&owned);              // Pass reference
}

// DO: Parse directly from read buffer
fn read_pty(&mut self) {
    let buf = self.buffer.get_mut();        // Pre-allocated
    let n = self.pty.read(buf)?;
    self.parser.parse(&buf[..n]);           // Zero copy
}
```

**Implementation detail:**
- Pre-allocate a single buffer
- Reuse it across all PTY reads
- Parser only holds references during parse

### Single-Instance Mode

**Status**: Implemented by Kitty, Alacritty
**Opportunity**: Medium for startup time

When a new terminal window is requested:
- Don't spawn new process
- Send request to existing process
- Existing process creates new window

```rust
// First instance
fn main() {
    let socket = UdsSocket::bind("/tmp/iris.sock")?;
    loop {
        let conn = socket.accept()?;
        handle_new_window(conn);
    }
}

// Subsequent invocations
fn main() {
    if let Ok(conn) = UdsSocket::connect("/tmp/iris.sock") {
        conn.send_new_window_request()?;
        return;  // Exit immediately
    }
    // No existing instance, become the daemon
    become_daemon();
}
```

**Benefits:**
- Near-instant window creation (no process spawn)
- Shared font cache (memory savings)
- Shared connection state