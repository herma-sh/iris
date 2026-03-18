# Iris

Iris is a Rust-first, cross-platform terminal platform built for speed, correctness, and clean embedding.

It is designed to work in two modes:

- standalone terminal application
- embeddable terminal surface for host applications

Windows, Linux, and macOS are all first-class targets, with Windows treated as a design-time priority.

## Goals

- fast input-to-screen latency
- smooth scrolling under large scrollback
- reliable ANSI and VT behavior
- clean typography and rendering
- strong platform integration for PTY, clipboard, IME, fonts, and DPI
- a clear API boundary between terminal core, platform services, renderer, and host integration

## Architecture

Iris is organized around a small set of focused layers:

- `iris-core`: terminal state, parser, buffer, scrollback, selection, search, themes, and events
- `iris-platform`: PTY, clipboard, keyboard normalization, IME, and font integration
- `iris-render-wgpu`: GPU rendering via `wgpu`
- `iris-standalone`: standalone terminal entry point
- `@iris/contract` and `@iris/react`: host-facing contracts and a thin React adapter

Core rule: terminal behavior lives in Rust, not in the UI layer.

## Design Principles

- Correctness before optimization
- Measured performance over vague smoothness claims
- No unnecessary allocations in hot paths
- Strict crate boundaries
- Minimal, readable code
- Original, restrained terminal UX

## Performance Targets

| Metric | Target |
|--------|--------|
| Input latency | `< 4ms` |
| Startup time | `< 30ms` aspirational, `< 100ms` release target |
| Scroll performance | `60fps` under heavy scrollback |
| Memory at 10k scrollback | `< 50MB` release target |

## Planned Feature Areas

- ANSI, VT, XTerm, and modern escape-sequence support
- GPU text rendering with damage tracking
- selection, clipboard, and search
- shell integration with prompt markers
- hyperlink support
- inline graphics support
- platform-native PTY, IME, font, and clipboard behavior
- standalone terminal workflows including configuration and session features

## Roadmap

Development is phased. The current plan runs from:

- Phase 0: foundation
- Phase 1: core parser
- Phase 2: wgpu renderer
- Phase 3: selection and clipboard
- Phase 4: scrollback and search
- Phase 5: platform polish
- Phase 6+: standalone, embedding, shell integration, performance, and polish

See [docs/phases.md](./docs/phases.md) for the full roadmap.

## Development Workflow

The repository now uses a staged branch model:

- `main`: completed phases only
- `dev`: active integration branch for the current phase
- `feature/*`: focused work branches created from `dev`

Normal development should branch from `dev` and merge back into `dev`. `main` should only receive deliberate phase-complete merges from `dev`.

## Documentation

- [docs/README.md](./docs/README.md)
- [docs/design.md](./docs/design.md)
- [docs/api-design.md](./docs/api-design.md)
- [docs/implementation.md](./docs/implementation.md)
- [docs/features.md](./docs/features.md)
- [docs/code-style.md](./docs/code-style.md)
- [docs/testing-strategy.md](./docs/testing-strategy.md)
- [docs/security-threat-model.md](./docs/security-threat-model.md)

## Status

Iris is currently documented in depth and organized around phased implementation work. The documentation set defines architecture, standards, security posture, performance targets, and release criteria for the build-out of the terminal platform.
