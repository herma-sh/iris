# Iris Research

Lessons from other terminals, common problems, and solutions.

## Lessons from Other Terminals

### Alacritty

**What it does well:**
- "Do one thing well" - minimal features, maximum performance
- GPU-accelerated rendering via OpenGL ES 2.0
- Configuration via simple TOML
- Correctness, performance, appearance, simplicity, portability priorities

**Lessons for Iris:**
- Don't try to be everything - excel at terminal emulation first
- Speed comes from focused scope, not feature accumulation
- GPU rendering is table stakes for modern terminals

### WezTerm

**What it does well:**
- WebGPU/DX12/Vulkan/Metal rendering
- Built-in multiplexer (tabs, panes, SSH)
- Lua configuration for extensibility
- Ligatures, color emoji, font fallback

**Lessons for Iris:**
- wgpu is the right choice (Metal on macOS, Vulkan/DX12 elsewhere)
- Font fallback chains are critical for Unicode support
- Config flexibility via scripting is powerful

### Kitty

**What it does well:**
- OpenGL rendering with SIMD optimization
- Kitty Graphics Protocol for inline images
- Python scripting for configuration
- Vector CPU instructions for performance balance

**Lessons for Iris:**
- SIMD optimization for finding escape sequences in runs
- Graphics protocol can be added later (not MVP)
- Terminal can be fast AND feature-rich

### Ghostty (Mitchell Hashimoto)

**What it does well:**
- Native UI per platform (Swift/AppKit on macOS, GTK4 on Linux)
- `libghostty` core with 90%+ code sharing
- Platform-specific optimizations (ARM SIMD on Apple Silicon)
- Fast startup, scrolling, I/O throughput

**Lessons for Iris:**
- Separate core library from UI surface
- Use platform-native APIs for look/feel
- Platform-specific SIMD where it matters
- "Fast, feature-rich, and native" is achievable together

### Foot (Wayland-native)

**What it does well:**
- Client/server architecture (shared server process)
- Wayland-native, no XWayland
- CPU-side software rendering (faster for low-memory systems)
- Instant startup via shared server

**Lessons for Iris:**
- Consider shared process model for embedded mode
- CPU rendering can be faster than GPU for some workloads
- Native integration beats compatibility layers

### Contour

**What it does well:**
- Modular library separation (vtbackend, vtparser, vtrasterizer, vtpty)
- Multithreaded: main thread for UI, worker for PTY output
- 6-clipboard support, vi modes, inline images

**Lessons for Iris:**
- Separate parsing from rendering cleanly
- Input thread blocked by output processing is a real problem
- Unicode handling is fundamentally hard

## Performance Anti-Patterns

| Problem | Cause | Iris Solution |
|---------|--------|---------------|
| Slow scrolling | Redrawing entire buffer each frame | Damage tracking + GPU batching |
| Input latency | Copy-on-write delays, event queue bloat | Direct PTY read → parser → grid path |
| Memory bloat | Storing full history with no limits | Fixed-size scrollback ring buffer |
| Resize jank | Synchronous reflow blocking UI | Async reflow with progressive render |
| Dropped frames | CPU-bound rendering loop | wgpu GPU rendering, frame pacing |
| Heavy output lag | cat large file = freeze | Throttled output parsing, backpressure |

## Visual/UX Anti-Patterns

| Problem | Cause | Iris Solution |
|---------|--------|---------------|
| Blurry fonts | Incorrect DPI scaling | Per-monitor DPI, integer scaling |
| Jagged text | No subpixel antialiasing | Subpixel AA via wgpu |
| CJK gaps/overlap | Wrong cell width assumptions | Unicode width calculation (UAX11) |
| Emoji breaks | Emoji counted as double-width cells | Proper grapheme cluster handling |
| Cursor flicker | Redrawing cursor separately | Cursor in same render pass as cells |
| Selection artifacts | Selection computed after scroll | Track scroll position during selection |
| Color banding | 16-bit color in 2024 | True color (24-bit) from day one |
| Inconsistent line height | Fonts with bad metrics | Explicit line height, not font metrics |
| Wrong italics | No italic font fallback | Font cascade list with italic variants |

## Cross-Platform Anti-Patterns

| Problem | Cause | Iris Solution |
|---------|--------|---------------|
| Windows escape issues | Assuming Unix behavior only | Test on Windows first, ConPTY-aware |
| Clipboard encoding | Platform-specific clipboards | Abstracted clipboard trait per platform |
| IME composition | Not handling pre-edit text | IME-aware input handling |
| HiDPI confusion | Assuming 96 DPI forever | Scale-aware from architecture start |
| Font discovery | Hardcoded font paths | Platform font APIs per OS |

## Unicode & Character Width Challenges

### The Core Problem

Unicode characters don't have uniform width in terminals:
- **Single-width**: Most Latin, ASCII (1 cell)
- **Double-width**: Most CJK characters (2 cells)
- **Ambiguous-width**: Can be 1 or 2 depending on locale
- **Zero-width**: Combining marks, ZWJ sequences
- **Variable**: Emoji sequences with skin tone, gender modifiers

### Why Terminals Get It Wrong

1. **`wcwidth()` is often outdated** - Ships with OS, lags behind Unicode
2. **Applications disagree with terminals** - Shell calculates width differently
3. **Emoji sequences** - ZWJ sequences like "family" emoji can be 2+ characters wide
4. **Ambiguous characters** - East Asian vs non-Asian width disagreement

### Iris Approach

| Issue | Iris Solution |
|-------|---------------|
| Unicode version lag | Bundle own Unicode data, update regularly |
| wcwidth accuracy | Use `unicode-width` crate, extend for latest Unicode |
| Emoji sequences | Grapheme cluster detection; pre-computed emoji widths |
| Ambiguous width | Configurable; default to East Asian width for CJK users |
| Combining marks | Track state; don't advance cursor for zero-width |

## Resize & Reflow Challenges

### Why Resize Is Hard

When terminal resizes horizontally:
1. Every wrapped line must be re-wrapped to new width
2. Scrollback buffer (potentially millions of lines) must be reflowed
3. Cursor position must be recalculated
4. All while maintaining correct semantic meaning (what's a prompt vs output)

### Common Failure Modes

| Failure | Symptom | Iris Prevention |
|---------|---------|-----------------|
| Synchronous reflow | UI freezes during resize | Async reflow with progressive paint |
| Cut-off lines | Text truncated, whitespace gaps | Track wrap state, rewrap correctly |
| Cursor jump | Cursor ends up in wrong place | Recalculate cursor position during reflow |
| Scroll position lost | View jumps unexpectedly | Preserve scroll percentage, not row index |
| Memory spike | Full scrollback duped for reflow | In-place reflow, no copies |

### Iris Reflow Strategy

1. **Preserve semantic lines** - Track original line boundaries, not just wrapped lines
2. **Chunked reflow** - Process in batches, yield to UI between chunks
3. **Progressive paint** - Show partial results immediately, refine as reflow continues
4. **Cursor anchoring** - Keep cursor cell visible during reflow
5. **Scroll anchoring** - Keep viewport position relative to content, not absolute row

## Performance Benchmarks

| Metric | Ghostty Target | Alacritty | WezTerm | Kitty | Iris Target |
|--------|---------------|-----------|---------|-------|-------------|
| Startup time | < 50ms | ~100ms | ~150ms | ~200ms | < 30ms |
| Input latency | ~8ms | ~10ms | ~15ms | ~12ms | < 4ms |
| Scroll FPS | 60fps @ 1M lines | 60fps | 60fps | 60fps | 60fps @ 10M lines |
| Memory (10k scrollback) | ~20MB | ~30MB | ~50MB | ~40MB | < 25MB |
| Time to first prompt | < 20ms | ~50ms | ~80ms | ~60ms | < 15ms |

## Ghostty-Beating Strategies

### What Ghostty Does Well

- Zig's zero-cost abstractions and explicit memory layout
- Single-threaded render path (no lock contention)
- OpenGL/Metal native rendering
- Damage region rendering
- Optimized ANSI parser

### Iris Strategies to Exceed

| Area | Ghostty Approach | Iris Approach |
|------|------------------|---------------|
| Memory | Arena allocation | Pre-allocated ring buffers |
| Parser | State machine | State machine + SIMD for runs |
| Render | OpenGL/Metal | wgpu (Vulkan/DX12/Metal) |
| Threading | Single-thread | Lock-free Arc for grid reads |
| Allocation | Minimal | Zero in hot path |