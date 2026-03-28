# Iris Coding Rules

## Source Of Truth

Follow these documents when coding:

- Project-facing overview and release history: [../README.md](../README.md), [../CHANGELOG.md](../CHANGELOG.md)
- Architecture and API: [../docs/design.md](../docs/design.md), [../docs/api-design.md](../docs/api-design.md), [../docs/implementation.md](../docs/implementation.md)
- Style and correctness: [../docs/code-style.md](../docs/code-style.md), [../docs/error-handling.md](../docs/error-handling.md), [../docs/rust-best-practices.md](../docs/rust-best-practices.md), [../docs/rust-api-guidelines.md](../docs/rust-api-guidelines.md)
- Testing and review: [../docs/testing-strategy.md](../docs/testing-strategy.md), [../docs/review-checklist.md](../docs/review-checklist.md), [../docs/release-criteria.md](../docs/release-criteria.md)
- Security and logging: [../docs/security-threat-model.md](../docs/security-threat-model.md), [../docs/logging-strategy.md](../docs/logging-strategy.md)
- Scope and acceptance for current work: [../docs/phases.md](../docs/phases.md) and the matching file in [../docs/phases](../docs/phases)

If documents conflict:

1. Follow the active phase spec for scope and acceptance criteria.
2. Follow `api-design.md` and `code-style.md` for Iris-specific API, simplicity, and performance tradeoffs.
3. Treat `rust-best-practices.md` and `rust-api-guidelines.md` as adopted Rust defaults that must be interpreted through Iris-specific docs, not override them.
4. Treat research and innovation docs as guidance, not license to bypass the core rules.

## Core Rules

- Correctness first, then benchmark, then optimize.
- No panics in production paths. Use `Result` or `Option`.
- No `unwrap`, `expect`, or unchecked assumptions outside tests, benches, and tightly justified initialization paths.
- No wildcard imports in Rust production code.
- No hidden allocations in hot paths.
- No renderer, windowing, or host-framework dependencies inside `iris-core`.
- No platform-specific behavior leaking outside `iris-platform`.
- No host code reaching through renderer internals instead of the defined Iris API.
- No undocumented `unsafe`. Every `unsafe` block needs a precise `SAFETY:` comment.
- No surprising public Rust APIs when standard constructors, conversions, iterators, getters, or trait implementations would be expected.

## Crate Boundary Rules

### `iris-core`

- Own terminal state, parser, buffer, damage tracking, selection, scrollback, search, themes, and events.
- Stay dependency-light.
- Avoid `unsafe` entirely unless there is a demonstrated, measured need.
- Keep parser and grid operations allocation-free on the hot path.

### `iris-platform`

- Wrap ConPTY, Unix PTY, clipboard, IME, font discovery, and keyboard normalization behind traits.
- Keep OS-specific code isolated in platform modules.
- Make platform differences explicit and testable.

### `iris-render-wgpu`

- Render from `iris-core` state and damage information.
- Prefer GPU batching and damage-only updates.
- Keep glyph caching, atlas management, and cursor rendering in the render layer.
- Do not pull PTY, clipboard, or host-shell concerns into the renderer.

### Host Layer

- Use `@iris/contract` for terminal commands, events, themes, and metrics.
- Keep `@iris/react` thin. It is an adapter, not the terminal engine.
- Preserve embeddability for Hermes and future hosts.

## Performance Rules

- Mark hot paths clearly.
- Avoid `Vec::push`, `String` creation, `clone`, regex use, or heap growth in parser and render hot loops unless proven acceptable by measurement.
- Prefer pre-allocated buffers, fixed-capacity structures, and direct indexing.
- Use damage tracking, not full redraws, unless correctness requires otherwise.
- Benchmark any change that touches parser throughput, render latency, resize/reflow, startup time, or memory.

See:

- [../docs/code-style.md](../docs/code-style.md)
- [../docs/benchmarks.md](../docs/benchmarks.md)
- [../docs/phases/10.md](../docs/phases/10.md)

## API And Error Rules

- Public APIs must be documented.
- Public Rust APIs should follow the local Rust API guideline document.
- Error types should be specific, contextual, and built with `thiserror` in libraries.
- Do not leak secrets, passwords, keys, tokens, or sensitive paths in errors.
- Add context at crate or application boundaries.
- Use `anyhow` only in application/binary layers, not in public library APIs.
- Prefer standard conversion traits over ad hoc conversion methods when semantics match.
- Prefer domain-specific argument types over `bool` switches and ambiguous `Option` parameters.
- Public Rust types should generally implement `Debug`.

## Comment Rules

- Comments explain why, invariants, tradeoffs, or protocol behavior.
- Do not narrate obvious code.
- `TODO` and `FIXME` must be specific and actionable. Prefer an issue reference.
- `SAFETY:` comments are mandatory for every `unsafe` block.
- Public Rust doc comments should begin by explaining when or why the item is used, not only what it mechanically does.

## Documentation Rules

- Keep the root `README.md` focused on Iris only.
- Do not mention specific external parent products or named host integrations in the root `README.md` unless explicitly requested.
- Update `CHANGELOG.md` for every meaningful project update in the same change set.
- Use the project phase versioning scheme in `CHANGELOG.md`:
- Phase 0 maps to `0.0.1`.
- Phase 1 maps to `0.1.0`.
- For later phases, map phase `N` to `0.N.0`.
- Do not assume code signing is required for local, development, alpha, or preview builds unless the release docs are explicitly tightened.

## Git And PR Rules

- Never implement work directly on `main` or `dev`.
- Before changing files, switch to a focused `feature/*` branch created from `dev`.
- Keep branch scope reviewable; avoid mixing unrelated changes.
- Open or update a PR to `dev` for every meaningful change set.
- Never merge a PR without explicit user approval in the current conversation. Opening/updating the PR is the default handoff point.
- PR descriptions must use `./.github/PULL_REQUEST_TEMPLATE.md` with all required sections completed (or marked `N/A` with a reason).
- If a PR cannot be opened due to environment constraints (for example missing GitHub auth), document the blocker and provide exact follow-up commands.

## Logging Rules

- Use structured `tracing`.
- Log lifecycle events, important state changes, warnings, and recoverable failures.
- Do not log every byte or cell update in hot paths.
- Never log secrets or raw sensitive payloads.
- Sanitize commands, environment variables, URLs, and file paths before logging.

## Security Rules

- Treat PTY output and terminal escape sequences as hostile input.
- Enforce bounds on escape sequences, arguments, nesting, and line lengths.
- Filter or gate sensitive OSC operations such as clipboard writes and notifications.
- Canonicalize and validate paths before file access.
- Do not store passwords in config.
- Prefer strict host key checking for SSH and safe tunnel defaults.
- Keep security-sensitive defaults enabled unless the task explicitly changes them.

See:

- [../docs/security-threat-model.md](../docs/security-threat-model.md)
- [../docs/error-handling.md](../docs/error-handling.md)

## Testing Rules

- New behavior requires unit tests.
- Cross-module behavior requires integration tests.
- Hot-path changes require benchmarks or explicit benchmark impact review.
- Parser and terminal behavior changes should consider malformed input and `vttest` compatibility.
- Cross-platform features need platform-specific verification notes.
- Do not merge code that only works on one platform when the feature is documented as cross-platform.
- Prefer tests that use concrete backends and real/captured data.
- Do not add mock-data tests for behavior that will soon be covered by a real backend path; add or defer to real-backend tests.
- If mocks are used, document why real-backend coverage is not yet practical.

Minimum verification set for substantial work:

- `cargo fmt -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --all`

Add as needed:

- `cargo bench`
- `cargo audit`
- platform-specific manual checks
- `vttest` and real-application compatibility checks

## Review Rules

- Review for correctness, boundary violations, performance regressions, unsafe usage, missing tests, and security issues first.
- Flag hidden allocations, extra abstraction, and host/render/core coupling.
- Require acceptance criteria from the active phase doc to be satisfied or explicitly deferred.
- Keep PRs focused and small enough to review coherently.
- PR descriptions must follow the repository template in `./.github/PULL_REQUEST_TEMPLATE.md`.
- PR descriptions must include concrete detail for objective, scope, API/behavior changes, backward compatibility, tests, verification commands run, and key files changed.
- If a section is not applicable, mark it `N/A` with a one-line reason instead of omitting it.

## Task Routing

When deeper context is needed, read the matching doc directly instead of guessing:

- Protocol behavior: [../docs/phases/01.md](../docs/phases/01.md), [../docs/phases/08.md](../docs/phases/08.md), [../docs/phases/09.md](../docs/phases/09.md)
- Renderer behavior: [../docs/phases/02.md](../docs/phases/02.md), [../docs/phases/09.md](../docs/phases/09.md), [../docs/phases/13.md](../docs/phases/13.md)
- Platform behavior: [../docs/phases/05.md](../docs/phases/05.md), [../docs/features.md](../docs/features.md)
- Hermes embedding: [../docs/phases/07.md](../docs/phases/07.md)
- Performance work: [../docs/benchmarks.md](../docs/benchmarks.md), [../docs/phases/10.md](../docs/phases/10.md)
- Inspector/debugging: [../docs/phases/11.md](../docs/phases/11.md)
- Accessibility, bidi, and polish: [../docs/features.md](../docs/features.md), [../docs/phases/12.md](../docs/phases/12.md), [../docs/phases/15.md](../docs/phases/15.md)
