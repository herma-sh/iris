# Iris

Iris is a Rust-first terminal platform for standalone and embedded use.

The project is being built in phases. `iris-core` now has Phase 1 parser coverage complete on `main`, and the next implementation phase is the GPU renderer in `iris-render-wgpu`.

Windows, Linux, and macOS are first-class targets. Windows is treated as a design-time priority.

## Current Status

- Phase 0 complete: core workspace and platform foundations
- Phase 1 complete: ANSI/VT parser, terminal behavior, integration coverage, and parser throughput target
- Phase 2 next: `wgpu` renderer, glyph atlas, damage-driven drawing, and visible text output
- VTtest is deferred until Phase 6, when Iris has a runnable standalone terminal binary that can host an interactive session

## Workspace

- `iris-core`: terminal state, parser, grid, damage tracking, and related tests
- `iris-platform`: PTY, clipboard, IME, keyboard, and font abstractions
- `iris-render-wgpu`: GPU renderer
- `iris-standalone`: standalone application entry point planned for a later phase
- `@iris/contract` and `@iris/react`: host-facing contracts and embedding adapters planned for later phases

Core rule: terminal behavior lives in Rust, not in the UI layer.

## Performance Targets

| Metric | Target |
|--------|--------|
| Input latency | `< 4ms` |
| Startup time | `< 30ms` aspirational, `< 100ms` release target |
| Scroll performance | `60fps` under heavy scrollback |
| Memory at 10k scrollback | `< 50MB` release target |

## Roadmap

Development is phased:

- Phase 0: foundation
- Phase 1: core parser
- Phase 2: `wgpu` renderer
- Phase 3: selection and clipboard
- Phase 4: scrollback and search
- Phase 5: platform polish
- Phase 6: standalone terminal application
- Later phases: embedding, shell integration, advanced rendering, performance, and polish

See [docs/phases.md](./docs/phases.md) for the full phase plan and [docs/phases/02.md](./docs/phases/02.md) for the active next phase.

## Development Workflow

- `main`: completed phases only
- `dev`: active integration branch
- `feature/*`: focused work branches created from `dev`

Normal development should branch from `dev` and merge back into `dev`. `main` should only receive deliberate phase-complete merges from `dev`.

## Documentation

- [docs/README.md](./docs/README.md)
- [docs/phases.md](./docs/phases.md)
- [docs/design.md](./docs/design.md)
- [docs/implementation.md](./docs/implementation.md)
- [docs/testing-strategy.md](./docs/testing-strategy.md)
- [docs/benchmarks.md](./docs/benchmarks.md)
- [docs/security-threat-model.md](./docs/security-threat-model.md)
