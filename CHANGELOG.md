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

Target release: `0.1.0`

### Added

- Began the phase-1 ANSI/VT parser implementation in `iris-core` with a modular parser state machine, CSI parsing, SGR decoding, and parser-driven terminal action application.
- Extended the phase-1 parser foundation with UTF-8 printable character decoding across chunk boundaries and malformed-sequence recovery.
- Added the first bounded OSC parser support in `iris-core` for window-title and OSC 8 hyperlink sequences terminated by BEL or ST.
- Added bounded phase-1 handling for DCS, SOS, PM, and APC string states so unsupported payloads terminate cleanly and resume normal parsing without unbounded growth.

### Changed

- Split the `iris-core` grid implementation into focused submodules so storage, write normalization, scrolling/editing operations, resize behavior, and tests stay below the structural warning threshold for oversized files.
- Corrected parser string-state cleanup so finishing DCS leaves ignored-string tracking untouched and finishing ignored strings no longer clears unrelated OSC or DCS buffers.
- Adjusted OSC overflow recovery to reset parser state while reprocessing the current byte in ground state instead of dropping it.
- Split the parser state machine into focused submodules so escape handling, string-state handling, UTF-8 decoding, and state tests are easier to maintain.
- Split the terminal state implementation into focused modules so movement, editing, screen-state handling, and tests stay below the structural warning threshold for oversized files.
- DEC private mode `1049` now switches between the primary and alternate screen buffers in `iris-core`, restoring the saved primary cursor when returning to the main screen.
- Added phase-1 scroll-region handling for `CSI r`, `CSI S`, and `CSI T`, and made `Index`/`ReverseIndex` respect the active scrolling margins.
- Added phase-1 G0/G1 character-set designation and `SI`/`SO` shifting in the parser, including DEC Special Graphics and UK ASCII translations for printable bytes.
- Completed phase-1 G2/G3 character-set designation and `SS2`/`SS3` single-shift handling so one-shot charset selection now covers all four VT charset slots.
- Added phase-1 `CSI I` forward-tabulation support so counted tab movement now covers both forward and backward CSI tab controls.
- Added phase-1 support for common CSI cursor aliases ``CSI ` ``, `CSI a`, and `CSI e`, mapping them onto the existing absolute-column, forward, and downward cursor motions.
- Added phase-1 `CSI b` repeat-previous-character support with parser-state tracking so repeated graphic output works across normal printable and UTF-8 decoded characters.
- Hardened phase-1 `ESC c` handling so parser-side terminal interpretation resets restore default charset slots and active charset instead of only clearing transient single-shift state.
- Expanded phase-1 integration coverage with chunked mixed-sequence streams and combined screen-update flows closer to real terminal redraw behavior.
- Updated the phase-1 checklist in `docs/phases/01.md` to mark the parser and integration milestones that are now complete, so progress tracking stays aligned with merged work.
- Updated the phase-1 implementation-order section in `docs/phases/01.md` to mark completed parser, charset, unit-test, and integration-test milestones while leaving VTtest, benchmarking, and final cleanup open.
- Added phase-1 tab-stop handling for `HT`, `ESC H`, `CSI Z`, and `CSI g`, including configurable stops and backward tab movement.
- Added phase-1 insert/delete editing support for `CSI @`, `CSI P`, `CSI L`, and `CSI M`, including character shifts within a row and line shifts within the active scrolling region.
- Added phase-1 ESC handling for `ESC Z`, `ESC c`, `ESC =`, and `ESC >`, including keypad-mode tracking and full terminal reset coverage across parser, terminal, and integration tests.
- Hardened full terminal reset so it always clears cached alternate-screen state even if the active mode flag is already false, and documented that keypad mode is controlled by `ESC =` / `ESC >` rather than CSI mode parameters.
- Added chunked vttest-style redraw coverage with scroll margins, origin mode, save/restore cursor, SGR, tabs, charset shifts, and scroll operations in a dedicated `iris-core` integration test file.
- Corrected DEC origin-mode handling so enabling or resetting `CSI ? 6 h/l` homes the cursor appropriately and absolute cursor addressing clamps within the active scroll region while origin mode is active.
- Added phase-1 parser, terminal, and integration coverage for explicit `CSI J`/`CSI K` erase modes and for `CSI r` scroll-region reset semantics.
- Updated the phase-1 checklist in `docs/phases/01.md` to mark erase-mode and scroll-region reset coverage complete.
- Added phase-1 parser recovery and control-handling coverage for embedded C0 controls plus `CAN`/`SUB` cancellation across CSI, escape, charset-designation, and string states.
- Updated parser string and sequence handling so embedded controls continue to execute without corrupting buffered OSC/DCS payloads, while `CAN` and `SUB` now cancel the active sequence cleanly.
- Added comprehensive phase-1 SGR coverage for supported style toggles, standard/default ANSI colors, bright colors, and extended-color clamping.
- Updated the phase-1 checklist in `docs/phases/01.md` to mark full supported SGR attribute-code coverage complete.
- Added phase-1 parser and integration coverage for nested-like OSC streams so malformed in-string `ESC ]` introducers stay literal until BEL/ST termination and subsequent real OSC updates still resynchronize cleanly.
- Updated the phase-1 checklist in `docs/phases/01.md` to mark nested OSC coverage complete.
- Added app-style phase-1 integration coverage for realistic `vim`-like alternate-screen redraws and `tmux`-like status-line redraws on the main screen.
- Updated the phase-1 checklist in `docs/phases/01.md` to mark real-terminal-output coverage complete.
- Added explicit phase-1 CSI intermediate handling so unsupported intermediate-byte sequences are consumed and ignored cleanly instead of being treated as malformed input.
- Updated the phase-1 checklist in `docs/phases/01.md` to mark CSI intermediate coverage complete.
- Added a phase-1 `cargo bench` parser throughput harness in `crates/iris-core/benches/parser_throughput.rs` so plain-text MiB/s and CSI sequence throughput can be measured directly against the documented targets.
- Reduced parser hot-path allocation churn by reusing parser buffers and appending actions into shared output vectors instead of allocating a fresh `Vec<Action>` per byte.
- Reduced CSI-path allocation churn further by pushing completed CSI actions directly into the shared parser output buffer and storing common SGR and mode payloads inline with `smallvec`.
- Improved the phase-1 parser throughput baseline from `21.20` to roughly `59-64 MiB/s` on the plain-text fixture and from `3.07M` to roughly `11.7M-12.2M seq/s` on the CSI fixture, while leaving the documented performance target open because plain-text parsing is still below target.
- Optimized `Parser::advance` with the same ASCII ground-state fast path used by `parse`, so parser-to-terminal throughput now avoids per-byte slow-path dispatch for contiguous printable text and common C0 controls.
- Aligned the parser throughput harness with the shipped parser-to-terminal path by benchmarking `Parser::advance` against a real `Terminal`, with current runs at roughly `75-76 MiB/s` for plain text and `10.9M-12.2M seq/s` for CSI-heavy streams.
- Added batched ASCII terminal/grid write paths plus range-based damage marking so contiguous single-width output no longer pays per-cell damage updates or Unicode width calculation in the hot path.
- Phase-1 parser throughput now clears the documented targets with `cargo bench -p iris-core --bench parser_throughput`, reaching roughly `176-177 MiB/s` on the plain-text fixture and `11.0M-11.1M seq/s` on the CSI fixture.

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
