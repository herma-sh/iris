# Iris

Iris is a Rust-first terminal platform for standalone and embedded use.

Windows, Linux, and macOS are first-class targets. Windows is treated as a design-time priority.

## What Exists Today

- `iris-core` handles terminal state, ANSI/VT parsing, screen buffers, damage tracking, and terminal behavior
- parser and terminal behavior are covered with unit and integration tests, including realistic redraw streams
- parser throughput is benchmarked and currently clears the documented target
- `iris-platform` provides the platform abstraction layer for PTY, clipboard, IME, keyboard, and fonts
- `iris-render-wgpu` exists as the renderer crate boundary and is the next major implementation area

VTtest is still deferred because Iris does not yet have a runnable terminal application that can host an interactive session.

## Testing And Verification

Current coverage includes:

- parser state-machine unit tests
- terminal behavior unit tests
- integration tests for chunked redraw streams, OSC flows, and terminal-style screen updates
- parser throughput benchmarking through the shipped parser-to-terminal path

Standard verification commands:

```bash
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all
cargo bench -p iris-core --bench parser_throughput
```

## Workspace

- `iris-core`: terminal state, parser, grid, damage tracking, and related tests
- `iris-platform`: PTY, clipboard, IME, keyboard, and font abstractions
- `iris-render-wgpu`: GPU renderer
- `iris-standalone`: standalone application entry point
- `@iris/contract` and `@iris/react`: host-facing contracts and embedding adapters

Core rule: terminal behavior lives in Rust, not in the UI layer.

## What We Are Doing Now

The current focus is making terminal state visible on screen.

That means building out `iris-render-wgpu` so it can:

- initialize `wgpu` device and surface state
- load fonts and measure cell geometry
- rasterize glyphs into a texture atlas
- render terminal cells, colors, and cursor state
- update only damaged regions instead of redrawing everything
- provide a real visual harness for renderer verification

## Performance Targets

| Metric | Target |
|--------|--------|
| Input latency | `< 4ms` |
| Startup time | `< 30ms` aspirational, `< 100ms` release target |
| Scroll performance | `60fps` under heavy scrollback |
| Memory at 10k scrollback | `< 50MB` release target |

## Development Workflow

- `main`: stabilized milestone branch
- `dev`: active integration branch
- `feature/*`: focused work branches created from `dev`

Normal development must branch from `dev` and merge back into `dev` via pull request. Do not commit feature work directly to `main` or `dev`. `main` should only receive deliberate promotion merges from `dev`.

## Documentation

- [docs/README.md](./docs/README.md)
- [docs/design.md](./docs/design.md)
- [docs/implementation.md](./docs/implementation.md)
- [docs/testing-strategy.md](./docs/testing-strategy.md)
- [docs/benchmarks.md](./docs/benchmarks.md)
- [docs/security-threat-model.md](./docs/security-threat-model.md)
- [CHANGELOG.md](./CHANGELOG.md)
