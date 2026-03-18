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
- Added phase-1 tab-stop handling for `HT`, `ESC H`, `CSI Z`, and `CSI g`, including configurable stops and backward tab movement.
- Added phase-1 insert/delete editing support for `CSI @`, `CSI P`, `CSI L`, and `CSI M`, including character shifts within a row and line shifts within the active scrolling region.
- Added phase-1 ESC handling for `ESC Z`, `ESC c`, `ESC =`, and `ESC >`, including keypad-mode tracking and full terminal reset coverage across parser, terminal, and integration tests.

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
