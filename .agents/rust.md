# Iris Rust Guide

## Purpose

Use this file for Rust implementation work in Iris.

Primary references:

- [../docs/code-style.md](../docs/code-style.md)
- [../docs/error-handling.md](../docs/error-handling.md)
- [../docs/design.md](../docs/design.md)
- [../docs/implementation.md](../docs/implementation.md)
- [../docs/rust-best-practices.md](../docs/rust-best-practices.md)
- [../docs/rust-api-guidelines.md](../docs/rust-api-guidelines.md)

External reference:

- [Canonical Rust best practices](https://canonical.github.io/rust-best-practices/introduction.html)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)

## Rust Defaults

- Use stable, readable Rust first.
- Optimize only after measurement.
- Prefer explicit data flow and small focused types.
- Keep `iris-core` dependency-light.
- Use `thiserror` for library error types.
- Use `anyhow` only in binary or application layers.
- Avoid wildcard imports in production code.
- Prefer naming that matches existing local vocabulary and consistent word order.
- Keep helper types local when they only serve one function or one remote API mapping.
- Follow standard Rust API conventions for constructors, getters, iterators, conversions, and expected trait implementations.

## API Shape

- Public APIs must be documented.
- Prefer domain types over ambiguous primitive-heavy signatures.
- Return `Result` or `Option` instead of panicking.
- Keep public crate boundaries clear and narrow.
- Prefer constructors or builders for object-like public structs.
- For config-style data transfer structs, public fields are acceptable if `Default` is reasonable.
- Prefer standard conversion traits such as `From`, `TryFrom`, `AsRef`, and `AsMut` where semantics fit.
- Avoid `bool` arguments or weakly typed option bundles when a domain type would be clearer.
- Ensure public types implement `Debug` unless there is a strong reason not to.

## Hot Path Rules

- Parser, grid, and render-adjacent hot paths must avoid allocations.
- Avoid `String` creation, `clone`, hidden `Vec` growth, and unnecessary trait indirection in hot loops.
- Prefer pre-allocated buffers, direct indexing, and fixed-capacity structures where appropriate.
- Mark hot paths clearly in code when they are performance-sensitive.

## Safety

- Avoid `unsafe` in `iris-core` unless it is justified by measurement and cannot be replaced by safe Rust.
- Every `unsafe` block requires a `SAFETY:` comment that names the invariant being relied on.
- Avoid widening unsafe scope; keep it small and local.

## Error Handling

- Add context at crate boundaries.
- Keep error messages actionable and sanitized.
- Do not expose secrets, passwords, tokens, key material, or sensitive paths in errors.
- Replace `.unwrap()` with `?`, `ok_or`, `ok_or_else`, or pattern matching in production paths.

## Crate Expectations

- `iris-core`: parser, buffer, damage, selection, scrollback, search, themes, events
- `iris-platform`: platform traits and OS-specific implementations
- `iris-render-wgpu`: renderer and GPU resources only
- `iris-standalone`: composition, configuration, CLI, lifecycle

## Common Verification

Run these after meaningful Rust changes:

```bash
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all
```

Add benchmarks and platform checks when the change touches hot paths or OS integration.
