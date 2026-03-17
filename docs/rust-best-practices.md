# Iris Rust Best Practices

This document adopts and localizes Rust best-practice guidance for Iris.

Primary external reference:

- [Canonical Rust best practices](https://canonical.github.io/rust-best-practices/introduction.html)

Use this document as the Iris-specific interpretation of that guidance. If the upstream guide evolves, prefer keeping Iris locally consistent unless the team explicitly updates this file.

## Precedence

This document is an adopted standard, not an independent authority above Iris design constraints.

If there is tension:

1. [api-design.md](./api-design.md) governs Iris public API shape and crate boundaries.
2. [code-style.md](./code-style.md) governs clarity, minimalism, and hot-path cost.
3. This document supplies the default Rust engineering baseline for choices not already constrained by Iris-specific docs.

## Scope

These rules apply to Rust code across Iris, with the strongest enforcement in:

- `iris-core`
- `iris-platform`
- `iris-render-wgpu`
- `iris-standalone`

They complement:

- [code-style.md](./code-style.md)
- [error-handling.md](./error-handling.md)
- [design.md](./design.md)
- [implementation.md](./implementation.md)

## Adopted Rules

## Naming Discipline

- Prefer names that match the surrounding project vocabulary.
- Use names that make argument order and intent obvious.
- Keep names concise, but not cryptic.
- Keep word order consistent across APIs.
- Avoid unnecessary abbreviations unless they are already well-established in the codebase.

## Import Discipline

- Do not use wildcard imports in production code.
- Do not use broad preludes as a shortcut in production code.
- Avoid importing enum variants with `*`.
- The common unit-test exception is acceptable: `use super::*;`.
- If a type name is too long for a small local scope, use a short, clear local alias rather than obscuring the type entirely.

## Cosmetic And Structural Discipline

- Use blank lines semantically, not decoratively.
- Group strongly related statements together.
- Keep unrelated code blocks separate.
- Prefer simple top-to-bottom flow over clever helper closures when possible.
- Keep helper types as local as reasonably possible to reduce namespace pollution.
- For foreign API or serde-only types, prefer local helper structs instead of letting remote schemas govern core domain types.

## Function And Type Discipline

- Prefer constructors or builders for object-like public structs.
- For plain config-style transfer structs, public fields are acceptable when there is a reasonable `Default` and the exposed shape is intentionally part of the contract.
- Consider `#[non_exhaustive]` on public config or data-transfer structs that may grow.
- Prefer free functions when no state is needed and the pattern fits the surrounding module.
- Keep functions focused on one responsibility.

## Error And Panic Discipline

- Do not panic in production code for recoverable failures.
- Default to `Result` or `Option`.
- Avoid `.unwrap()` in production paths.
- Use `?`, `ok_or`, `ok_or_else`, and pattern matching instead of unwrapping.
- If a panic is ever unavoidable, treat it as an internal fault and make that explicit.
- `.unwrap()` remains acceptable in tests and similar narrow scopes where failure is programmer-only and directly useful.

## Unsafe Discipline

- Minimize unsafe usage.
- Do not use `unsafe` merely because it may be faster.
- Require measurement before accepting `unsafe` for performance reasons.
- Keep unsafe scope as small as possible.
- Every `unsafe` block and `unsafe fn` must document preconditions with a `SAFETY:` comment.

## Comment Discipline

- Public doc comments should start with a concise sentence that explains when and why the item is used, not just what it mechanically does.
- Comments should reduce ambiguity, not narrate obvious code.
- Use marker comments such as `// test types.` or `// serde types.` when local helper types are intentionally placed at the bottom of a function.
- Keep comments accurate when code changes.

## Iris-Specific Interpretation

For Iris, the Canonical guidance matters most in these ways:

- `iris-core` should stay explicit, small-scoped, and easy to audit.
- Parser, grid, and renderer hot paths should avoid abstraction that hides cost.
- Public terminal and platform APIs should use stable naming and predictable structure.
- Serde-facing or external-host shapes should not leak into the core terminal model unless there is a strong reason.
- Unsafe code needs especially high scrutiny because Iris targets PTY, OS integration, and GPU code.

Compatibility notes with Iris docs:

- "Prefer functions over methods when state is not needed" from [code-style.md](./code-style.md) remains the Iris default for non-object-like helpers.
- Constructors and builders remain valid for object-like public types and configuration entrypoints.
- Minimal code still wins over boilerplate trait impls or abstraction added only to satisfy style purity.

## Enforcement

When reviewing Rust code in Iris, check at minimum:

- naming consistency
- absence of wildcard imports in production code
- panic avoidance in production paths
- small, justified unsafe blocks with `SAFETY:` comments
- local containment of helper types and remote API mapping types
- constructors or builders where public object-like structs would otherwise expose brittle internals

See also:

- [testing-strategy.md](./testing-strategy.md)
- [review-checklist.md](./review-checklist.md)
- [release-criteria.md](./release-criteria.md)
