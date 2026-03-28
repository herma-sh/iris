# Iris Agent Brief

## Purpose

Iris is the Rust-first terminal platform for Hermes. It must work as both:

- a standalone terminal binary
- an embeddable terminal surface inside Hermes and `imux`

Treat Hermes integration as the strategic priority, even when implementing standalone features. Keep the terminal engine independent from host-shell UI concerns.

Primary references:

- [../docs/README.md](../docs/README.md)
- [../docs/engineering-brief.md](../docs/engineering-brief.md)
- [../docs/design.md](../docs/design.md)
- [../docs/api-design.md](../docs/api-design.md)

## Default Engineering Stance

- Prefer Rust for parser, grid, PTY, rendering, platform integration, performance-sensitive logic, and correctness-critical code.
- Keep TypeScript limited to host contracts, bindings, and thin React integration.
- Keep React limited to the embedding surface. Do not move terminal intelligence into React components.
- Treat Windows, Linux, and macOS as first-class. Windows is never a later compatibility pass.
- Optimize only after correctness is established and measured.

## Architectural Boundaries

- `iris-core`: terminal state, parser, buffer, selection, search, themes, events. No windowing or renderer dependencies.
- `iris-platform`: PTY, clipboard, IME, fonts, keyboard normalization, platform APIs behind traits.
- `iris-render-wgpu`: rendering only. Depends on `iris-core`, not platform internals.
- `iris-standalone` or app entrypoints: compose core, platform, and renderer.
- `@iris/contract` and `@iris/react`: host-facing API and thin embedding layer only.

See:

- [../docs/design.md](../docs/design.md)
- [../docs/implementation.md](../docs/implementation.md)
- [../docs/api-design.md](../docs/api-design.md)

## How To Work

1. Start with the relevant docs before changing code.
2. If you are on `main` or `dev`, create a focused `feature/*` branch before editing files.
3. Identify the active delivery phase and use the matching phase spec for scope and acceptance criteria.
4. Make the smallest coherent change that preserves crate boundaries.
5. Add or update tests, benchmarks, docs, and logging/security behavior when the change affects them.
6. Verify with formatting, linting, tests, and phase-appropriate checks.
7. Open or update a PR targeting `dev` and use `./.github/PULL_REQUEST_TEMPLATE.md`.

## Document Map

Use these documents by concern:

- Product goals and positioning: [../docs/engineering-brief.md](../docs/engineering-brief.md), [../docs/features.md](../docs/features.md)
- Architecture and crate layout: [../docs/design.md](../docs/design.md), [../docs/implementation.md](../docs/implementation.md), [../docs/api-design.md](../docs/api-design.md)
- Coding standards: [../docs/code-style.md](../docs/code-style.md), [../docs/error-handling.md](../docs/error-handling.md)
- Testing and performance: [../docs/testing-strategy.md](../docs/testing-strategy.md), [../docs/benchmarks.md](../docs/benchmarks.md), [../docs/release-criteria.md](../docs/release-criteria.md), [../docs/review-checklist.md](../docs/review-checklist.md)
- Security and observability: [../docs/security-threat-model.md](../docs/security-threat-model.md), [../docs/logging-strategy.md](../docs/logging-strategy.md)
- Competitive context and future ideas: [../docs/research.md](../docs/research.md), [../docs/innovation.md](../docs/innovation.md)
- Phase sequencing: [../docs/phases.md](../docs/phases.md)

## Phase References

Consult the specific phase file when implementing work in that area:

- Foundation: [../docs/phases/00.md](../docs/phases/00.md)
- Parser and ANSI/VT behavior: [../docs/phases/01.md](../docs/phases/01.md)
- wgpu renderer: [../docs/phases/02.md](../docs/phases/02.md)
- Selection and clipboard: [../docs/phases/03.md](../docs/phases/03.md)
- Scrollback and search: [../docs/phases/04.md](../docs/phases/04.md)
- Platform polish, DPI, IME, fonts: [../docs/phases/05.md](../docs/phases/05.md)
- Standalone app and CLI: [../docs/phases/06.md](../docs/phases/06.md)
- Hermes and Tauri integration: [../docs/phases/07.md](../docs/phases/07.md)
- Shell integration and OSC 133: [../docs/phases/08.md](../docs/phases/08.md)
- Graphics, hyperlinks, advanced rendering: [../docs/phases/09.md](../docs/phases/09.md)
- Performance optimization: [../docs/phases/10.md](../docs/phases/10.md)
- Inspector and debugging: [../docs/phases/11.md](../docs/phases/11.md)
- Bidirectional text: [../docs/phases/12.md](../docs/phases/12.md)
- Deep color and HDR work: [../docs/phases/13.md](../docs/phases/13.md)
- Predictive echo: [../docs/phases/14.md](../docs/phases/14.md)
- Quake mode and release polish: [../docs/phases/15.md](../docs/phases/15.md)

## Decision Rules

- If a change violates crate boundaries, redesign it.
- If a change adds hot-path allocations, cloning, or unnecessary indirection, redesign it.
- If behavior differs by platform, make the difference explicit in `iris-platform`.
- If a feature is novel but baseline correctness, latency, or readability is weak, defer the novelty.
- If a question is about exact protocol or sequence behavior, consult the relevant phase doc plus [../docs/api-design.md](../docs/api-design.md) before implementing.

## Definition Of Done

Work is not done when code merely compiles. It is done when:

- the change matches the active phase or documented scope
- tests and benchmarks appropriate to the change exist or are updated
- logging, security, and error-handling implications are addressed
- the implementation still reads clearly and preserves Iris's architecture
