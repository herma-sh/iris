# Iris Testing Guide

## Purpose

Use this file when planning or validating tests, benchmarks, conformance checks, and release gates.

Primary references:

- [../docs/testing-strategy.md](../docs/testing-strategy.md)
- [../docs/benchmarks.md](../docs/benchmarks.md)
- [../docs/review-checklist.md](../docs/review-checklist.md)
- [../docs/release-criteria.md](../docs/release-criteria.md)

## Default Verification

For meaningful changes, run:

```bash
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all
```

## When To Add More

- Add unit tests for new component behavior.
- Add integration tests when behavior crosses parser, grid, PTY, renderer, or host boundaries.
- Add or update benchmarks when a change affects parser throughput, render latency, resize, startup, scroll performance, or memory.
- Add manual verification notes for platform-specific behavior.

## Terminal-Specific Checks

- Parser and escape-sequence changes should consider malformed input and bounded resource behavior.
- Terminal compatibility changes should consider `vttest` and real applications such as `vim`, `tmux`, `htop`, `git`, and build tools.
- Scrollback, selection, and search changes should include large-history or boundary-condition coverage.

## Renderer Checks

- Confirm damage tracking behavior remains correct.
- Watch for GPU validation errors.
- Validate at least one typical viewport and one larger viewport.
- Verify cursor, selection, and text rendering behavior visually when renderer code changes.

## Platform Checks

- Windows: ConPTY, clipboard, DPI, PowerShell, IME-sensitive paths
- Linux: X11 or Wayland clipboard, font discovery, selection behavior
- macOS: clipboard, scaling, keyboard behavior, surface or backend differences

## Release-Oriented Checks

For high-risk or phase-completion work, consider:

```bash
cargo bench
cargo audit
```

And add:

- `vttest`
- real application validation
- platform matrix verification
- performance threshold comparison against documented targets
