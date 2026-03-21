# Changelog

All notable changes to Iris should be recorded in this file.

This project uses a phase-based versioning scheme:

| Phase | Version |
|-------|---------|
| 0 | `0.0.1` |
| 1 | `0.1.0` |
| 2 | `0.2.0` |
| 3 | `0.3.0` |
| 4 | `0.4.0` |
| 5 | `0.5.0` |
| 6 | `0.6.0` |
| 7 | `0.7.0` |
| 8 | `0.8.0` |
| 9 | `0.9.0` |
| 10 | `0.10.0` |
| 11 | `0.11.0` |
| 12 | `0.12.0` |
| 13 | `0.13.0` |
| 14 | `0.14.0` |
| 15 | `0.15.0` |

## Unreleased

Target release: `0.2.0`

### 2026-03-21

#### Added

- Added a stateful `TextRenderer` in `iris-render-wgpu` that owns the glyph atlas, glyph cache, text buffers, text pipeline, theme, and viewport uniforms needed to render `iris-core` grid content through the existing renderer bootstrap.
- Added owned `RasterizedGlyph` payloads plus renderer-side glyph-miss orchestration so damaged cells can request rasterization through an injected callback, populate the atlas/cache once, and then upload reusable text instances for drawing.
- Added renderer coverage for themed empty clears, cache reuse across repeated damage updates, and wide-cell glyph population when damage begins on a continuation column.
- Added a system-font-backed `FontRasterizer` in `iris-render-wgpu` using `fontdb` and `fontdue`, including best-effort primary-family selection, monospace defaults, fallback scanning, and a `TextRenderer` convenience path that prepares grid text directly from system fonts.
- Added a cursor overlay path in `iris-render-wgpu` with dedicated cursor instances, GPU buffers, WGSL shader, and render pipeline support for block, underline, and bar cursor styles layered over the text pass.
- Added a higher-level `TerminalRenderer` in `iris-render-wgpu` that owns the stateful text renderer plus system font rasterizer and prepares full visible frames directly from `iris-core` terminal state.
- Added a textured presentation pipeline in `iris-render-wgpu` so cached frame textures can be drawn into off-screen or presentation targets through a dedicated fullscreen sample pass.
- Added TOML-backed renderer theme loading in `iris-render-wgpu` (`Theme::from_toml_str` and `Theme::from_toml_file`) with strict field/type validation and support for top-level or `[colors]` theme tables.
- Added a Phase 2 renderer benchmark harness at `crates/iris-render-wgpu/benches/renderer_throughput.rs` covering full-frame preparation, retained scroll updates, and retained renderer memory estimates.
- Added explicit renderer font-rasterization regression coverage for best-effort CJK and emoji glyph discovery when fallback system fonts are available.

#### Changed

- Extended the text pipeline so callers can clear using an explicit color instead of a hardcoded black, allowing the new text-render path to respect the active theme background.
- Exported the new text-render integration types from `iris-render-wgpu` and reused normalized damage spans during glyph population so cache misses follow the same wide-cell handling as instance encoding.
- Corrected text-instance eligibility so blank cells with non-default attributes are rendered through a transparent glyph path instead of being skipped, preserving styled background cells during damage-driven draws.
- Hardened system font parsing with explicit font-data size bounds, rasterized glyph dimension caps before atlas allocation, and cached fallback-face lookups so repeated glyph misses do not rescan the full system font database.
- Reset prepared text-instance state at the start of each `TextRenderer::prepare_grid` call so failed prepares cannot leave stale instance counts active for later draws, and expanded renderer regression coverage for atlas exhaustion, empty damage, missing-font mapping, and continuation-origin rendering.
- Hardened font rasterizer initialization so `NaN` font sizes are rejected with the same `InvalidFontSize` error path as other non-positive inputs.
- Integrated the new cursor overlay into `TextRenderer` so prepared cursor state now renders alongside the text pass and correctly normalizes continuation-column cursors back to wide-cell lead positions.
- Hardened cursor-span normalization so defensive right-edge and orphan-continuation states fall back to single-cell overlays, and documented the single-instance cursor draw invariant in the cursor pipeline.
- Updated the terminal-facing renderer integration to retain a cached frame texture, apply incremental damage updates for changed text and old/new cursor regions, and present the cached output through a dedicated fullscreen sample pass.
- Reused normalized damage buffers across retained text prepares to avoid per-frame hot-path allocations, tuned retained damage scratch capacity for the common terminal-update case, and expanded terminal-renderer regression coverage for cursor clearing, theme invalidation, and update-before-prepare behavior.
- Moved retained-frame scroll offsets into the presentation pass so cached terminal content now renders at stable zero-offset coordinates, while presentation can shift and background-fill the visible viewport without forcing a cache redraw when only the scroll offset changes.
- Invalidated cached terminal frames when cell metrics change and restored drained terminal damage after failed incremental updates so renderer errors cannot leave stale pixels or dropped dirty regions behind.
- Added retained smooth-scroll coverage for full-grid scrolls by tracking core scroll deltas, preserving full-viewport overscan bands in the cached terminal frame, and shifting the retained frame before damage redraw so presentation can animate from previous rows into the new visible state without background gaps.
- Hardened scroll-delta restoration with debug-only overwrite assertions, consolidated signed scroll-line conversion, and expanded coverage for scroll merge/restore edge cases plus retained scroll-copy guard paths.
- Added symmetric downward and lower-bound scroll-merge regression coverage in `iris-core`, and avoided redundant present-uniform GPU writes by dirty-tracking terminal presentation state in `iris-render-wgpu`.
- Updated Phase 2 documentation and benchmark guidance to reflect completed TOML theme loading, CJK/emoji rasterization coverage, and the new renderer throughput benchmark command/results.
- Added terminal-renderer font-size updates that rebuild renderer-owned glyph state on size changes, plus regression coverage for successful size updates and invalid-size rejection.
- Added renderer integration coverage for partial scroll-region updates so non-full-grid scroll operations are now explicitly validated in the terminal renderer path.
- Extended retained-frame scroll shifting to handle partial scroll regions (not just full-grid deltas), preserving rows outside the active scroll window while shifting the affected band in-place.
- Hardened incremental renderer error recovery so failed updates now invalidate cached retained frames after scroll-shift mutation, preventing stale shifted textures from being reused on retry.
- Wired the renderer throughput benchmark as an executable crate benchmark target (`cargo bench -p iris-render-wgpu --bench renderer_throughput`) and refreshed the documented local baseline measurements.
- Refactored renderer benchmark setup to reuse a shared terminal-renderer initialization helper with consistent no-font skip handling and contextual panic messages.

### 2026-03-20

#### Added

- Added a renderer theme bootstrap in `iris-render-wgpu` with default terminal colors, ANSI and indexed color resolution, and cell-attribute mapping into render-ready foreground and background RGBA values.
- Added reusable text-instance encoding helpers in `iris-render-wgpu` that walk `iris-core` grid damage regions, resolve cell colors through the renderer theme, and collect cached-glyph-backed `CellInstance` values for later buffer uploads.

#### Changed

- Hardened renderer theme color resolution so low-index indexed colors respect custom theme palettes, dimmed colors retain minimum visibility for dark values, and boundary coverage now exercises ANSI wrapping plus 256-color cube and grayscale edges.
- Normalized overlapping renderer damage regions before text-instance encoding, added aggregate debug logging for cache-miss glyph skips, and expanded encoder coverage for empty, zero-sized, and out-of-bounds damage inputs.
- Normalized continuation-only damage spans so wide-cell lead glyphs are still encoded when a damage region begins on the trailing continuation column.

### 2026-03-19

#### Added

- Added a bootstrap atlas-backed text render pipeline and WGSL shader in `iris-render-wgpu`, including uniform bind-group creation and smoke coverage for off-screen text draw submission.

#### Changed

- Expanded text-pipeline coverage with GPU readback assertions for populated and zero-instance off-screen draws so the tests verify rendered output instead of only checking submission succeeds.

## 0.2.0 (In Progress)

Work window: `2026-03-19` to `2026-03-20`

### Added

- Began the renderer bootstrap in `iris-render-wgpu` with concrete `wgpu` instance/adapter/device initialization, validated off-screen texture render targets, and smoke coverage for clear-pass submission.
- Added renderer surface creation and configuration types in `iris-render-wgpu`, including validated surface sizing, capability-based format selection, and resize support for window-backed presentation targets.
- Added a bootstrap fullscreen render pipeline and WGSL shader in `iris-render-wgpu` so off-screen draw submission can be exercised before cell, glyph, and atlas rendering are implemented.
- Added a row-packed glyph atlas in `iris-render-wgpu` with validated atlas sizing, allocation, upload checks, and a renderer helper for atlas creation.
- Added a CPU-side glyph cache in `iris-render-wgpu` with typed cache keys, atlas-backed glyph entries, idempotent cache insertion, and a renderer helper for caching uploaded glyph masks.
- Added GPU-ready text uniforms and per-cell instance encoding in `iris-render-wgpu`, including atlas UV generation, style-flag packing, continuation-cell rejection, and raw instance-byte conversion for later buffer uploads.
- Added resizable text uniform and instance buffer helpers in `iris-render-wgpu`, including `CellInstance` vertex-layout metadata and renderer helpers for uploading text uniforms and instance data.
- Added a renderer theme bootstrap in `iris-render-wgpu` with default terminal colors, ANSI and indexed color resolution, and cell-attribute mapping into render-ready foreground and background RGBA values.

### Changed

- Hardened glyph-cache insertion to validate atlas upload sizing before allocation so failed uploads do not leak atlas space, and expanded glyph-cache edge-case coverage for invalid upload sizes, zero-sized bitmaps, and full-atlas behavior.
- Hardened glyph-atlas allocation bounds checks with checked arithmetic and expanded atlas allocator edge-case coverage for row-height tracking, zero-sized allocations, and exact-fill behavior.
- Expanded renderer surface coverage with direct tests for surface-state resize behavior and stored surface configuration metadata.
- Hardened renderer texture-surface creation so configs that omit `RENDER_ATTACHMENT` are rejected before allocating invalid render targets.
- Replaced the renderer trait stub with a concrete renderer bootstrap API so follow-up PRs can add real surfaces, pipelines, glyph caches, and damage-driven cell rendering without reworking crate boundaries again.

## 0.1.0 - 2026-03-18

Work window: `2026-03-17` to `2026-03-18`

### Added

- Began the parser implementation in `iris-core` with a modular parser state machine, CSI parsing, SGR decoding, and parser-driven terminal action application.
- Extended the parser foundation with UTF-8 printable character decoding across chunk boundaries and malformed-sequence recovery.
- Added the first bounded OSC parser support in `iris-core` for window-title and OSC 8 hyperlink sequences terminated by BEL or ST.
- Added bounded handling for DCS, SOS, PM, and APC string states so unsupported payloads terminate cleanly and resume normal parsing without unbounded growth.
- Added scroll-region handling for `CSI r`, `CSI S`, and `CSI T`, and made `Index`/`ReverseIndex` respect the active scrolling margins.
- Added G0/G1 character-set designation and `SI`/`SO` shifting in the parser, including DEC Special Graphics and UK ASCII translations for printable bytes.
- Completed G2/G3 character-set designation and `SS2`/`SS3` single-shift handling so one-shot charset selection now covers all four VT charset slots.
- Added `CSI I` forward-tabulation support so counted tab movement now covers both forward and backward CSI tab controls.
- Added support for common CSI cursor aliases ``CSI ` ``, `CSI a`, and `CSI e`, mapping them onto the existing absolute-column, forward, and downward cursor motions.
- Added `CSI b` repeat-previous-character support with parser-state tracking so repeated graphic output works across normal printable and UTF-8 decoded characters.
- Added tab-stop handling for `HT`, `ESC H`, `CSI Z`, and `CSI g`, including configurable stops and backward tab movement.
- Added insert/delete editing support for `CSI @`, `CSI P`, `CSI L`, and `CSI M`, including character shifts within a row and line shifts within the active scrolling region.
- Added ESC handling for `ESC Z`, `ESC c`, `ESC =`, and `ESC >`, including keypad-mode tracking and full terminal reset coverage across parser, terminal, and integration tests.
- Added chunked vttest-style redraw coverage with scroll margins, origin mode, save/restore cursor, SGR, tabs, charset shifts, and scroll operations in a dedicated `iris-core` integration test file.
- Added parser, terminal, and integration coverage for explicit `CSI J`/`CSI K` erase modes and for `CSI r` scroll-region reset semantics.
- Added parser recovery and control-handling coverage for embedded C0 controls plus `CAN`/`SUB` cancellation across CSI, escape, charset-designation, and string states.
- Added comprehensive SGR coverage for supported style toggles, standard/default ANSI colors, bright colors, and extended-color clamping.
- Added parser and integration coverage for nested-like OSC streams so malformed in-string `ESC ]` introducers stay literal until BEL/ST termination and subsequent real OSC updates still resynchronize cleanly.
- Added app-style integration coverage for realistic `vim`-like alternate-screen redraws and `tmux`-like status-line redraws on the main screen.
- Added explicit CSI intermediate handling so unsupported intermediate-byte sequences are consumed and ignored cleanly instead of being treated as malformed input.
- Added a `cargo bench` parser throughput harness in `crates/iris-core/benches/parser_throughput.rs` so plain-text MiB/s and CSI sequence throughput can be measured directly against the documented targets.
- Rewrote the root `README.md` to describe the current implemented capabilities, the immediate renderer work, current test coverage, standard verification commands, and the `main` / `dev` / `feature/*` branch workflow without the old pre-start roadmap framing.

### Changed

- Split the `iris-core` grid implementation into focused submodules so storage, write normalization, scrolling/editing operations, resize behavior, and tests stay below the structural warning threshold for oversized files.
- Corrected parser string-state cleanup so finishing DCS leaves ignored-string tracking untouched and finishing ignored strings no longer clears unrelated OSC or DCS buffers.
- Adjusted OSC overflow recovery to reset parser state while reprocessing the current byte in ground state instead of dropping it.
- Split the parser state machine into focused submodules so escape handling, string-state handling, UTF-8 decoding, and state tests are easier to maintain.
- Split the terminal state implementation into focused modules so movement, editing, screen-state handling, and tests stay below the structural warning threshold for oversized files.
- DEC private mode `1049` now switches between the primary and alternate screen buffers in `iris-core`, restoring the saved primary cursor when returning to the main screen.
- Hardened `ESC c` handling so parser-side terminal interpretation resets restore default charset slots and active charset instead of only clearing transient single-shift state.
- Expanded integration coverage with chunked mixed-sequence streams and combined screen-update flows closer to real terminal redraw behavior.
- Updated `docs/phases/01.md` to mark completed parser, integration, benchmark, and documentation milestones while explicitly deferring VTtest until the standalone terminal binary exists.
- Hardened full terminal reset so it always clears cached alternate-screen state even if the active mode flag is already false, and documented that keypad mode is controlled by `ESC =` / `ESC >` rather than CSI mode parameters.
- Corrected DEC origin-mode handling so enabling or resetting `CSI ? 6 h/l` homes the cursor appropriately and absolute cursor addressing clamps within the active scroll region while origin mode is active.
- Updated parser string and sequence handling so embedded controls continue to execute without corrupting buffered OSC/DCS payloads, while `CAN` and `SUB` now cancel the active sequence cleanly.
- Reduced parser and CSI hot-path allocation churn by reusing parser buffers, appending into shared action output, pushing completed CSI actions directly into that buffer, and storing common SGR and mode payloads inline with `smallvec`.
- Optimized the shipped parser-to-terminal path by extending the ASCII ground-state fast path to `Parser::advance` and adding batched ASCII terminal/grid writes with range-based damage marking for contiguous single-width output.
- Hardened the grid and terminal ASCII fast paths so `write_ascii_run` now rejects control bytes and raw UTF-8 bytes instead of treating arbitrary input bytes as printable single-width characters.
- Strengthened the terminal erase-mode regression tests so `ED 3` and `EL 2` assertions now verify cells that would expose partial or no-op erase implementations.
- Hardened the public `Grid::write_ascii_run` bounds arithmetic with checked addition so oversized ASCII-run lengths fail safely instead of relying on unchecked `usize` math.
- Added explicit grid tests for invalid scroll-range arguments so the `top > bottom` and `bottom >= rows` error paths are now covered for both range-scroll APIs.
- Parser throughput now clears the documented targets with `cargo bench -p iris-core --bench parser_throughput`, with verified 2026-03-18 runs ranging roughly from `144 MiB/s` to `151 MiB/s` on the plain-text fixture and from `11.1M` to `11.2M seq/s` on the CSI fixture.
- Cleaned up the benchmark, testing, and documentation index docs so `Parser::advance` is the documented parser-to-terminal harness and stale VTtest claims are removed.
- Corrected the docs index success criteria so the `docs/README.md` input latency target now matches the `< 4ms` value used in the performance targets table.
- Corrected the acceptance-criteria table to use `MiB/s` instead of `MB/s` for the parser throughput target so it matches the benchmark docs.
- Corrected stale `docs/testing-strategy.md` code examples to use the current `Terminal`/`Grid` API, including `terminal.grid`, `Cell.character`, and row/column ordering in grid assertions.

## 0.0.1 - 2026-03-17

### Added

- Root [README.md](./README.md) for Iris with project overview, architecture, targets, roadmap, and documentation links.
- Initial [CHANGELOG.md](./CHANGELOG.md) with phase-based version mapping.
- Agent rule updates requiring changelog maintenance on every meaningful project update.
- Cargo workspace for phase 0 with `iris-core`, `iris-platform`, and `iris-render-wgpu`.
- Phase-0 `iris-core` implementation covering cells, grid storage, damage tracking, cursor state, terminal modes, terminal state, and a basic control-character parser.
- Phase-0 `iris-platform` abstractions for PTY, clipboard, fonts, and IME, plus a native-backed cross-platform PTY implementation and integration tests.
- Phase-0 `iris-render-wgpu` skeleton with a renderer trait and opaque render surface boundary.
- Cross-platform GitHub Actions CI for formatting, clippy, and test execution on Windows, Linux, and macOS.
- Root [AGENTS.md](./AGENTS.md) as the repository entrypoint for coding agents.
- Focused agent guidance files for Rust, `wgpu`, and testing in [./.agents/rust.md](./.agents/rust.md), [./.agents/wgpu.md](./.agents/wgpu.md), and [./.agents/testing.md](./.agents/testing.md).
- Local [docs/rust-best-practices.md](./docs/rust-best-practices.md) embedding Iris-specific Rust guidance derived from Canonical Rust best practices.
- Local [docs/rust-api-guidelines.md](./docs/rust-api-guidelines.md) embedding Iris-specific Rust API design guidance derived from the official Rust API Guidelines.

### Changed

- Hardened phase-0 core invariants around grid sizing, wide-cell normalization, damage tracking, and cursor restore clamping based on review follow-up.
- Updated CI to pin third-party action revisions by commit SHA and to keep an explicit workspace build step before tests.
- Updated the platform layer to `portable-pty` `0.9.0`, preserved empty clipboard writes, and made the PTY integration tests stream output with timeout-based assertions for more reliable Unix CI coverage.
- Agent documentation structure now follows a lightweight root entrypoint plus modular `.agents/` references.
- Release documentation now explicitly allows unsigned development and preview builds, with signing deferred to later distribution-quality releases.
- Rust agent and rule documents now explicitly incorporate the local Rust best-practices standard.
- Rust agent and rule documents now explicitly incorporate the local Rust API guideline standard.
- Local Rust standards now explicitly defer to Iris-specific API and code-style docs when tradeoffs around minimalism, public fields, generics, or trait surface area arise.
