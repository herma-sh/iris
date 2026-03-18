# Iris Development Phases

## Executive Summary

| Metric | Value |
|--------|-------|
| Total Duration | ~19-29 weeks (5-7 months) |
| Team Assumption | 1-2 developers initially, scaling as project matures |
| Release Strategy | v0.1 after Phase 7 (Hermes integration), v1.0 after Phase 15 |

---

## Phase Overview

| Phase | Name | Duration | Priority | Dependencies |
|-------|------|----------|----------|--------------|
| 0 | Foundation | 1-2 weeks | P0 Critical | None |
| 1 | Core Parser | 2-3 weeks | P0 Critical | Phase 0 |
| 2 | wgpu Renderer | 2-3 weeks | P0 Critical | Phase 1 |
| 3 | Selection & Clipboard | 1 week | P0 Critical | Phase 2 |
| 4 | Scrollback & Search | 1-2 weeks | P0 High | Phase 3 |
| 5 | Platform Polish | 2-3 weeks | P0 High | Phase 4 |
| 6 | Standalone Binary | 1-2 weeks | P1 High | Phase 5 |
| 7 | Hermes Integration | 1-2 weeks | P0 Critical | Phase 5 |
| 8 | Shell Integration | 1 week | P1 Medium | Phase 7 |
| 9 | Advanced Rendering | 2-3 weeks | P2 Medium | Phase 7 |
| 10 | Performance Optimization | 1-2 weeks | P1 High | Phase 9 |
| 11 | Inspector & Debug | 1 week | P2 Low | Phase 10 |
| 12 | Bidirectional Text | 1-2 weeks | P2 Medium | Phase 10 |
| 13 | Deep Color | 1 week | P3 Low | Phase 9 |
| 14 | Predictive Echo | 1-2 weeks | P2 Medium | Phase 7 |
| 15 | Quake & Polish | 1-2 weeks | P2 Medium | Phase 14 |

---

## Phase 0: Foundation

**Duration**: 1-2 weeks  
**Priority**: P0 Critical  
**Goal**: Establish crate structure and core types with zero rendering

### Deliverables

- [ ] iris-core crate compiles with Grid, Cell, Terminal types
- [ ] iris-platform crate compiles with PtyBackend trait
- [ ] Windows ConPTY implementation spawns process
- [ ] Unix PTY implementation spawns process
- [ ] Parser framework handles basic control characters (LF, CR, TAB, BS)
- [ ] All unit tests pass
- [ ] CI pipeline runs on all platforms

### Tasks

#### iris-core

| Task | Description | Est. |
|------|-------------|------|
| Crate structure | Cargo.toml, lib.rs, error.rs | 0.5d |
| Cell types | Cell, CellAttrs, CellFlags, CellWidth | 1d |
| Grid implementation | Pre-allocated buffer, write methods | 1d |
| DamageTracker | Mark dirty regions, take_damage | 0.5d |
| Cursor types | Cursor, CursorStyle, CursorPosition | 0.5d |
| Terminal struct | Grid + cursor + modes | 1d |
| TerminalModes | Origin, wrap, insert, newline modes | 0.5d |
| Unit tests | Grid operations, cell write | 1d |

#### iris-platform

| Task | Description | Est. |
|------|-------------|------|
| Crate structure | Cargo.toml, lib.rs, error.rs | 0.5d |
| PtyBackend trait | Spawn, read, write, resize, is_alive | 0.5d |
| Clipboard trait | get_text, set_text | 0.25d |
| FontProvider trait | enumerate, fallback_for | 0.25d |
| ImeHandler trait | set_position, active | 0.25d |
| Windows ConPTY | CreatePseudoConsole, spawn | 2d |
| Unix PTY | openpty, fork, exec | 1d |
| macOS PTY | Same as Unix | 0.5d |

#### iris-render-wgpu (Skeleton)

| Task | Description | Est. |
|------|-------------|------|
| Crate structure | Cargo.toml, lib.rs | 0.25d |
| Renderer trait | Definition only, no implementation | 0.25d |

### Testing

```rust
// Unit tests required
#[test] fn grid_write_updates_damage() { ... }
#[test] fn grid_scroll_moves_content() { ... }
#[test] fn grid_resize_preserves_content() { ... }
#[test] fn pty_spawn_returns_handle() { ... }
#[test] fn pty_read_write_works() { ... }
```

### Completion Criteria

| Criterion | Verification |
|-----------|--------------|
| All crates compile | `cargo build --all` |
| All tests pass | `cargo test --all` |
| Windows PTY works | Run iris-core tests on Windows |
| Unix PTY works | Run iris-core tests on Linux/macOS |
| Code review | PR approved by reviewer |
| CI green | All CI checks pass |

---

## Phase 1: Core Parser

**Duration**: 2-3 weeks  
**Priority**: P0 Critical  
**Goal**: Full ANSI/VT parser, with VTtest deferred until a runnable terminal binary exists

### Deliverables

- [ ] Parser handles all standard CSI sequences
- [ ] Parser handles OSC sequences (title, clipboard, hyperlink)
- [ ] Parser handles DCS/APC/PM/SOS sequences
- [ ] Parser handles control characters (LF, CR, TAB, BS, BEL)
- [ ] Parser handles charset switching (G0-G3)
- [ ] Parser handles screen modes (normal, alternate)
- [ ] Grid updates correctly from parser output
- [ ] Cursor positioning works correctly
- [ ] `vttest` passes basic sequences once the standalone binary exists (Phase 6)
- [ ] Character attributes (bold, italic, underline, etc.) work
- [ ] 16-color, 256-color, and true color modes work

### Tasks

#### Parser State Machine

| Task | Description | Est. |
|------|-------------|------|
| Parser state machine | Ground, Escape, CsiEntry, etc. | 2d |
| ESC sequences | Single-char escapes | 1d |
| CSI sequences | Cursor movement, erase, scroll | 3d |
| CSI parameters | Parameter parsing, defaults | 1d |
| OSC sequences | Title, clipboard, hyperlink (OSC8) | 2d |
| DCS sequences | Device control strings | 1d |
| APC/PM/SOS | Application, privacy, string | 0.5d |
| Control characters | LF, CR, TAB, BS, BEL, etc. | 1d |

#### Grid Operations

| Task | Description | Est. |
|------|-------------|------|
| Cursor movement commands | CUP, CUU, CUD, CUF, CUB, etc. | 1d |
| Erase commands | ED, EL, ECH | 1d |
| Scroll regions | DECSTBM, SU, SD | 1d |
| Tab stops | HTS, TBC | 0.5d |
| Character attributes | SGR (bold, italic, underline, etc.) | 1d |
| Color modes | 16-color, 256-color, true color | 1d |
| Screen modes | Normal/alternate buffer | 1d |
| Charset handling | G0-G3, DEC Special | 1d |

### Testing

| Task | Description | Est. |
|------|-------------|------|
| Parser unit tests | Each sequence type | 2d |
| Integration tests | Parser + Grid | 1d |
| vttest suite | Deferred to Phase 6 standalone binary validation | 1d |

### Completion Criteria

| Criterion | Verification |
|-----------|--------------|
| vttest basic passes | Deferred to Phase 6 once `iris` can host an interactive VTtest session |
| Character attributes work | Manual test: `echo -e "\e[1mBold\e[0m"` |
| True color works | Manual test: true color gradient script |
| Cursor movement works | Manual test: `tput cup 10 20` |
| Alternate screen works | Manual test: vim then exit |
| All parser tests pass | `cargo test parser::` |
| Code review | PR approved |

---

## Phase 2: wgpu Renderer

**Duration**: 2-3 weeks  
**Priority**: P0 Critical  
**Goal**: GPU-accelerated rendering with 60fps scrolling

### Deliverables

- [ ] wgpu device and surface initialization
- [ ] Render pipeline for cell rendering
- [ ] Glyph rasterization and texture atlas
- [ ] Font fallback chain
- [ ] Cursor rendering (block, underline, bar)
- [ ] Smooth scrolling
- [ ] Basic theme support
- [ ] Ligature rendering

### Tasks

#### wgpu Setup

| Task | Description | Est. |
|------|-------------|------|
| Device/Queue creation | wgpu instance, adapter, device | 1d |
| Surface creation | Window surface via winit | 0.5d |
| Render pipeline | Vertex/fragment shaders, pipeline | 2d |
| Uniform buffer | Viewport, scroll offset | 0.5d |
| Swap chain | Present mode, vsync | 0.5d |

#### Glyph Cache

| Task | Description | Est. |
|------|-------------|------|
| Texture atlas | Allocate glyph textures | 1d |
| Rasterizer | rustybuzz or cosmic-text | 2d |
| Font fallback | System fonts + Noto | 1d |
| Wide characters | CJK, emoji width | 1d |
| Ligature shaping | HarfBuzz integration | 2d |

#### Cell Rendering

| Task | Description | Est. |
|------|-------------|------|
| Vertex buffer | Cell quad vertices | 1d |
| Attribute batching | Batch by attrs_id | 1d |
| Draw calls | Submit to GPU | 0.5d |
| Damage optimization | Only render damaged rows | 1d |

#### Cursor Rendering

| Task | Description | Est. |
|------|-------------|------|
| Cursor geometry | Block, underline, bar | 1d |
| Cursor blink | Animation timer | 0.5d |
| Cursor color | Override option | 0.25d |

#### Theme Support

| Task | Description | Est. |
|------|-------------|------|
| Color schemes | TOML theme files | 1d |
| Font configuration | Family, size, line height | 0.5d |

### Completion Criteria

| Criterion | Verification |
|-----------|--------------|
| Window displays text | Run standalone binary, see text |
| 60fps scrolling | Benchmark: scroll 10M lines at 60fps |
| Ligatures render | Visual test: Fira Code `->` |
| Cursor styles | Visual test: all three styles |
| Theme loads | Test: load custom theme TOML |
| Memory < 50MB | Benchmark: 10k scrollback < 50MB |

---

## Phase 3: Selection & Clipboard

**Duration**: 1 week  
**Priority**: P0 Critical  
**Goal**: Mouse selection and clipboard integration

### Deliverables

- [ ] Mouse selection (character, word, line)
- [ ] Block selection (Alt+drag)
- [ ] Selection rendering
- [ ] Clipboard integration (copy, paste)
- [ ] Bracketed paste mode
- [ ] Middle-click paste (Linux PRIMARY)

### Tasks

#### Selection Model

| Task | Description | Est. |
|------|-------------|------|
| Selection struct | Anchor, kind, active | 0.5d |
| Simple selection | Character-by-character | 0.5d |
| Word selection | Double-click | 0.5d |
| Line selection | Triple-click | 0.5d |
| Block selection | Alt+drag rectangular | 1d |
| Keyboard selection | Shift+arrows | 0.5d |

#### Clipboard

| Task | Description | Est. |
|------|-------------|------|
| Clipboard trait impl | Windows, macOS, Linux | 1d |
| Copy operation | Selection to clipboard | 0.5d |
| Paste operation | Clipboard to PTY | 0.5d |
| Bracketed paste | OSC 2004h/l | 0.5d |
| PRIMARY clipboard | Linux middle-click | 0.5d |

#### Selection Rendering

| Task | Description | Est. |
|------|-------------|------|
| Selection highlight | Render selection rectangle | 0.5d |
| Scroll during select | Auto-scroll when dragging | 0.5d |

### Completion Criteria

| Criterion | Verification |
|-----------|--------------|
| Simple select works | Click+drag, copy, paste |
| Word select works | Double-click selects word |
| Line select works | Triple-click selects line |
| Block select works | Alt+drag creates rectangle |
| Keyboard select works | Shift+arrows extends selection |
| Clipboard works | Copy in Iris, paste elsewhere |
| Middle-click works | Linux: select, middle-click pastes |

---

## Phase 4: Scrollback & Search

**Duration**: 1-2 weeks  
**Priority**: P0 High  
**Goal**: Unlimited scrollback with search

### Deliverables

- [ ] Ring buffer for scrollback
- [ ] Scroll position tracking
- [ ] Scroll commands (Page Up/Down, Home/End)
- [ ] Alternative screen buffer handling
- [ ] Forward search
- [ ] Backward search
- [ ] Regex search (optional)
- [ ] Search highlighting

### Tasks

#### Scrollback Buffer

| Task | Description | Est. |
|------|-------------|------|
| Ring buffer implementation | Fixed capacity, FIFO | 1d |
| Line wrapping | Store wrapped lines | 1d |
| Scroll position | Viewport into scrollback | 0.5d |
| Scroll commands | Page Up/Down, Home/End | 0.5d |
| Alternate buffer | Switch between normal/alt | 0.5d |
| Memory management | Limit scrollback, compress old | 1d |

#### Search

| Task | Description | Est. |
|------|-------------|------|
| Search struct | Query, matches, current | 0.5d |
| Forward search | Find next match | 0.5d |
| Backward search | Find previous match | 0.5d |
| Regex search | Pattern matching | 1d |
| Search highlighting | Highlight all matches | 0.5d |
| Search in scrollback | Search full history | 0.5d |

### Completion Criteria

| Criterion | Verification |
|-----------|--------------|
| Scrollback works | Generate 100k lines, scroll |
| Search finds matches | Search for pattern, find all |
| Search highlights | All matches visible |
| Memory bounded | 100k scrollback < 200MB |
| Alternate buffer works | vim then exit, scrollback preserved |

---

## Phase 5: Platform Polish

**Duration**: 2-3 weeks  
**Priority**: P0 High  
**Goal**: First-class Windows, macOS, Linux support

### Deliverables

- [ ] Complete Windows ConPTY handling
- [ ] Windows clipboard (Win32 API)
- [ ] Windows font enumeration (DirectWrite)
- [ ] Windows high DPI (Per-Monitor DPI)
- [ ] Unix PTY completion
- [ ] X11/Wayland clipboard
- [ ] Linux font enumeration (fontconfig)
- [ ] macOS PTY completion
- [ ] macOS clipboard (NSPasteboard)
- [ ] macOS font enumeration (Core Text)
- [ ] IME composition and positioning
- [ ] Keyboard event normalization

### Tasks

#### Windows

| Task | Description | Est. |
|------|-------------|------|
| ConPTY completion | Resize handling, flow control | 1d |
| Win32 clipboard | Get/Set clipboard text | 0.5d |
| DirectWrite fonts | Enumerate, fallback | 1d |
| High DPI | Per-Monitor DPI v2 | 1d |
| Windows key events | Virtual key mapping | 1d |

#### Unix

| Task | Description | Est. |
|------|-------------|------|
| PTY completion | Signal handling, flow control | 1d |
| X11 clipboard | PRIMARY and CLIPBOARD | 1d |
| Wayland clipboard | wl_clipboard | 1d |
| fontconfig | Font enumeration | 0.5d |

#### macOS

| Task | Description | Est. |
|------|-------------|------|
| PTY completion | Same as Unix | 0.5d |
| NSPasteboard | Clipboard integration | 0.5d |
| Core Text fonts | Font enumeration | 0.5d |
| macOS key events | KeyCode mapping | 1d |

#### IME

| Task | Description | Est. |
|------|-------------|------|
| IME composition | Receive composition events | 1d |
| IME positioning | Position near cursor | 1d |
| IME commit/cancel | Handle commit and cancel | 0.5d |

### Completion Criteria

| Criterion | Verification |
|-----------|--------------|
| Windows works | Run on Windows 10/11 |
| macOS works | Run on macOS 12+ |
| Linux works | Run on Ubuntu/Fedora |
| High DPI works | Move between monitors |
| IME works | Test Japanese/Chinese input |
| Clipboard works | Copy/paste on all platforms |

---

## Phase 6: Standalone Binary

**Duration**: 1-2 weeks  
**Priority**: P1 High  
**Goal**: Installable standalone terminal application

### Deliverables

- [ ] TOML configuration file
- [ ] CLI argument parsing
- [ ] Window management
- [ ] "Open with Iris" registration (Windows)
- [ ] .desktop file (Linux)
- [ ] App bundle (macOS)
- [ ] SSH connection support
- [ ] Serial connection support (basic)
- [ ] Session profiles

### Tasks

#### Configuration

| Task | Description | Est. |
|------|-------------|------|
| TOML schema | Define all config options | 0.5d |
| Config loading | Parse and validate | 0.5d |
| Config hot-reload | Watch file for changes | 0.5d |
| Font configuration | Family, size, fallbacks | 0.5d |
| Theme configuration | Color scheme loading | 0.5d |

#### CLI

| Task | Description | Est. |
|------|-------------|------|
| Argument parsing | clap or similar | 0.5d |
| -e command execution | Run command in terminal | 0.5d |
| --working-directory | Start in directory | 0.25d |
| --config path | Custom config file | 0.25d |
| --ssh connection | SSH to host | 0.5d |
| --serial connection | Serial port | 0.5d |

#### Window Management

| Task | Description | Est. |
|------|-------------|------|
| winit window | Create and manage | 0.5d |
| Window events | Resize, close, focus | 0.5d |
| Multiple windows | Create new windows | 0.5d |

#### Platform Integration

| Task | Description | Est. |
|------|-------------|------|
| Windows registry | "Open with Iris" | 0.5d |
| Linux .desktop | Desktop entry | 0.25d |
| macOS Info.plist | App configuration | 0.25d |
| macOS app bundle | Create .app | 0.5d |

#### SSH Support

| Task | Description | Est. |
|------|-------------|------|
| SSH client | Use system ssh or library | 1d |
| Connection profiles | Save SSH config | 0.5d |
| Password auth | Prompt for password | 0.25d |
| Key auth | Use SSH agent | 0.25d |

### Completion Criteria

| Criterion | Verification |
|-----------|--------------|
| Binary runs | `iris` opens window |
| Config loads | Custom config applies |
| SSH works | `iris --ssh user@host` connects |
| Windows integration | Right-click → Open with Iris |
| Linux integration | .desktop appears in menu |
| macOS integration | .app bundle runs |

---

## Phase 7: Hermes Integration

**Duration**: 1-2 weeks  
**Priority**: P0 Critical  
**Goal**: Iris embeds in Hermes as Tauri plugin

### Deliverables

- [ ] Tauri plugin for Iris
- [ ] Surface sharing via raw-window-handle
- [ ] Input event forwarding
- [ ] Resize event handling
- [ ] Terminal lifecycle management
- [ ] TypeScript contract package (@iris/contract)
- [ ] React component package (@iris/react)

### Tasks

#### Tauri Plugin

| Task | Description | Est. |
|------|-------------|------|
| Plugin structure | Tauri plugin API | 1d |
| Surface sharing | raw-window-handle | 1d |
| Input forwarding | Bridge keyboard/mouse | 1d |
| Resize handling | Bridge resize events | 0.5d |
| Lifecycle | Create/destroy terminals | 0.5d |

#### TypeScript Integration

| Task | Description | Est. |
|------|-------------|------|
| @iris/contract | TypeScript interfaces | 1d |
| Event types | Input/output events | 0.5d |
| Command types | Terminal commands | 0.5d |
| @iris/react | React component | 1d |
| TerminalSurface | Main component | 0.5d |

#### Multi-Instance

| Task | Description | Est. |
|------|-------------|------|
| Multiple terminals | Tab/pane support | 1d |
| Instance management | Create/destroy/focus | 0.5d |

### Completion Criteria

| Criterion | Verification |
|-----------|--------------|
| Iris renders in Hermes | Visual: Iris inside Hermes window |
| Input forwarding works | Type in Hermes, see in Iris |
| Multiple terminals work | Create tabs/panes |
| TypeScript API clean | @iris/contract published |
| React component works | @iris/react in Hermes |

---

## Phase 8: Shell Integration

**Duration**: 1 week  
**Priority**: P1 Medium  
**Goal**: OSC 133 semantic prompts

### Deliverables

- [ ] OSC 133 sequence handling
- [ ] Prompt detection
- [ ] Jump to prompt (previous/next)
- [ ] Copy command output
- [ ] Command finished notification
- [ ] Auto-inject shell integration (bash, zsh, fish, pwsh)

### Tasks

#### OSC 133 Handling

| Task | Description | Est. |
|------|-------------|------|
| Parse OSC 133 | A, B, C, D sequences | 0.5d |
| Mark prompts | Track prompt positions | 0.5d |
| Mark output | Track output regions | 0.5d |

#### Shell Integration

| Task | Description | Est. |
|------|-------------|------|
| Detect shell | Query $SHELL | 0.25d |
| Inject bash | Preexec/precmd hooks | 0.5d |
| Inject zsh | Preexec/precmd hooks | 0.5d |
| Inject fish | Fish integration | 0.5d |
| Inject pwsh | PowerShell integration | 0.5d |

#### Prompt Navigation

| Task | Description | Est. |
|------|-------------|------|
| Previous prompt | Jump to previous | 0.25d |
| Next prompt | Jump to next | 0.25d |
| Copy output | Copy last command output | 0.5d |

### Completion Criteria

| Criterion | Verification |
|-----------|--------------|
| Prompts detected | OSC 133 marks visible |
| Shell integration works | bash/zsh/fish auto-configured |
| Jump to prompt | Ctrl+Up/Down navigates |
| Copy output | Copy last command output |

---

## Phase 9: Advanced Rendering

**Duration**: 2-3 weeks  
**Priority**: P2 Medium  
**Goal**: Kitty graphics protocol and Sixel support

### Deliverables

- [ ] Kitty graphics protocol (static images)
- [ ] Sixel graphics (fallback)
- [ ] OSC 8 hyperlink rendering
- [ ] Deep color (>24-bit) pipeline
- [ ] Ligature rendering refinement

### Tasks

#### Kitty Graphics

| Task | Description | Est. |
|------|-------------|------|
| Parse Kitty APC | ESC_G...ESC\ | 1d |
| Image transmission | a=T,f=100, base64 | 2d |
| Image placement | Left/center/right, rows/cols | 1d |
| Image cache | Upload once, display many | 1d |
| Image deletion | d=... commands | 0.5d |

#### Sixel Graphics

| Task | Description | Est. |
|------|-------------|------|
| Parse Sixel | DCS P... | 2d |
| Color palette | 2-256 colors | 1d |
| Image rendering | Convert to texture | 1d |

#### OSC 8 Hyperlinks

| Task | Description | Est. |
|------|-------------|------|
| Parse OSC 8 | ESC ] 8 ; ... | 0.5d |
| Link rendering | Underline on hover | 0.5d |
| Link activation | Ctrl+Click to open | 0.5d |

#### Deep Color

| Task | Description | Est. |
|------|-------------|------|
| Color struct | u16 per channel | 0.5d |
| Color space | sRGB, Display-P3, Rec2020 | 1d |
| Rendering | GPU shader changes | 1d |

### Completion Criteria

| Criterion | Verification |
|-----------|--------------|
| Kitty images render | `kitty +kitten icat image.png` |
| Sixel images render | Sixel test patterns |
| Hyperlinks work | OSC 8 links clickable |
| Deep color works | >24-bit colors displayed |

---

## Phase 10: Performance Optimization

**Duration**: 1-2 weeks  
**Priority**: P1 High  
**Goal**: Sub-4ms input latency, optimized parsing

### Deliverables

- [ ] SIMD parser optimization
- [ ] Async reflow during resize
- [ ] Scrollback compression
- [ ] Character width caching
- [ ] Single-instance mode
- [ ] Benchmark suite
- [ ] Performance regression CI

### Tasks

#### Parser Optimization

| Task | Description | Est. |
|------|-------------|------|
| SIMD ESC scan | Find escape sequences fast | 2d |
| Parameter parsing | SIMD for digits/semicolons | 1d |
| Benchmark | Compare before/after | 0.5d |

#### Grid Optimization

| Task | Description | Est. |
|------|-------------|------|
| Attribute table | Deduplicate attrs | 1d |
| Width cache | Cache wcwidth results | 0.5d |
| Zero-copy PTY | Eliminate intermediate buffers | 1d |

#### Async Reflow

| Task | Description | Est. |
|------|-------------|------|
| Chunked reflow | Process in chunks | 1d |
| Viewport anchor | Keep scroll position | 0.5d |
| Progressive paint | Show partial results | 0.5d |

#### Single Instance

| Task | Description | Est. |
|------|-------------|------|
| Unix socket | IPC for new windows | 1d |
| Windows named pipe | IPC for new windows | 1d |

### Completion Criteria

| Criterion | Verification |
|-----------|--------------|
| Input latency < 4ms | Benchmark: keypress to screen |
| Parse throughput > 100MB/s | Benchmark: parse speed |
| Memory bounded | Benchmark: memory usage |
| Startup < 100ms | Benchmark: cold start |

---

## Phase 11: Inspector & Debug

**Duration**: 1 week  
**Priority**: P2 Low  
**Goal**: Built-in debugging tools

### Deliverables

- [ ] Cell inspector (show attributes under cursor)
- [ ] Color palette display
- [ ] OSC 133 overlay
- [ ] Protocol monitor (raw sequences)
- [ ] Performance metrics (FPS, latency, memory)
- [ ] Font metrics display

### Tasks

| Task | Description | Est. |
|------|-------------|------|
| Cell inspector | Hover shows char, attrs, colors | 1d |
| Color palette | Show 16/256/true color | 0.5d |
| OSC 133 overlay | Visual prompt boundaries | 0.5d |
| Protocol monitor | Log raw escape sequences | 1d |
| Performance metrics | FPS, latency, memory display | 0.5d |
| Font metrics | Show loaded fonts | 0.5d |

### Completion Criteria

| Criterion | Verification |
|-----------|--------------|
| Inspector opens | Keyboard shortcut activates |
| Cell info shown | Hover shows cell attributes |
| Metrics visible | FPS/latency displayed |

---

## Phase 12: Bidirectional Text

**Duration**: 1-2 weeks  
**Priority**: P2 Medium  
**Goal**: RTL language support

### Deliverables

- [ ] Unicode Bidi algorithm
- [ ] RTL text shaping (via harfbuzz)
- [ ] Correct cursor movement in RTL
- [ ] Arabic/Hebrew rendering

### Tasks

| Task | Description | Est. |
|------|-------------|------|
| Bidi algorithm | Unicode bidi rules | 2d |
| RTL shaping | harfbuzz integration | 1d |
| Cursor movement | Visual vs logical | 1d |
| Testing | Arabic, Hebrew text | 1d |

### Completion Criteria

| Criterion | Verification |
|-----------|--------------|
| Arabic renders correctly | Visual test |
| Hebrew renders correctly | Visual test |
| Cursor moves correctly | Bidirectional cursor test |

---

## Phase 13: Deep Color

**Duration**: 1 week  
**Priority**: P3 Low  
**Goal**: >24-bit color support

### Deliverables

- [ ] 10-12 bit per channel colors
- [ ] Color space support (Display-P3, Rec2020)
- [ ] HDR-aware themes

### Tasks

| Task | Description | Est. |
|------|-------------|------|
| Extended color struct | u16 per channel | 0.5d |
| Color space enum | sRGB, Display-P3, Rec2020 | 0.5d |
| GPU rendering | Update shaders | 1d |
| Theme support | Extended color themes | 0.5d |

### Completion Criteria

| Criterion | Verification |
|-----------|--------------|
| 10-bit colors render | Visual test on HDR display |
| Color spaces work | Display-P3 gamut visible |

---

## Phase 14: Predictive Echo

**Duration**: 1-2 weeks  
**Priority**: P2 Medium  
**Goal**: Sub-4ms perceived latency over SSH

### Deliverables

- [ ] Predictive character display
- [ ] Prediction correction on mismatch
- [ ] Configurable prediction (on/off)
- [ ] SSH-specific optimizations

### Tasks

| Task | Description | Est. |
|------|-------------|------|
| Prediction engine | Guess what server will echo | 2d |
| Local echo | Show predicted immediately | 1d |
| Correction | Fix when prediction wrong | 1d |
| Configuration | Enable/disable per connection | 0.5d |

### Completion Criteria

| Criterion | Verification |
|-----------|--------------|
| Typing feels instant | SSH over 100ms latency |
| Corrections seamless | Mispredictions corrected smoothly |

---

## Phase 15: Quake & Polish

**Duration**: 1-2 weeks  
**Priority**: P2 Medium  
**Goal**: Final UX polish and drop-down terminal

### Deliverables

- [ ] Quake mode (drop-down terminal)
- [ ] Global hotkey
- [ ] Final UI polish
- [ ] Documentation
- [ ] Release v1.0

### Tasks

#### Quake Mode

| Task | Description | Est. |
|------|-------------|------|
| Dropdown window | Animate from screen edge | 1d |
| Global hotkey | System-wide hotkey | 1d |
| Always on top | Float above other windows | 0.5d |

#### Polish

| Task | Description | Est. |
|------|-------------|------|
| UI review | Consistency, spacing | 1d |
| Accessibility | Keyboard navigation, screen reader | 1d |
| Documentation | README, config docs | 1d |
| Release prep | CHANGELOG, version bump | 0.5d |

### Completion Criteria

| Criterion | Verification |
|-----------|--------------|
| Quake mode works | Hotkey toggles terminal |
| All platforms work | Windows, macOS, Linux |
| v1.0 released | Tagged release published |

---

## Milestone Schedule

| Milestone | Phase | Target |
|-----------|-------|--------|
| Alpha | Phase 5 complete | Week 10-12 |
| Beta | Phase 7 complete | Week 13-16 |
| v0.1 | Hermes integration works | Week 14-16 |
| v1.0 | Phase 15 complete | Week 19-29 |

---

## Testing Gates

### Per-Phase

| Gate | Requirement |
|------|-------------|
| Unit tests pass | `cargo test --all` |
| Benchmarks within target | `cargo bench` |
| Code review | PR approved |
| Documentation updated | Docs reflect changes |

### Pre-Release

| Gate | Requirement |
|------|-------------|
| vttest passes | All basic tests |
| Real-world apps | vim, tmux, htop, cargo, git |
| Platform testing | Windows, macOS, Linux |
| Performance targets | Input < 4ms, 60fps scroll |
| Security audit | `cargo audit` clean |

---

## Risk Register

| Risk | Impact | Mitigation |
|------|--------|------------|
| wgpu complexity | High | Phase 0-1 proof of concept first |
| Windows ConPTY bugs | Medium | Early Windows testing, MS docs |
| Performance regression | Medium | CI benchmarks, regression limits |
| Cross-platform issues | Medium | Platform-specific CI jobs |
| Unicode edge cases | Low | Comprehensive test suite |
| Security vulnerabilities | High | `cargo audit`, secure defaults |
