# Iris Rust API Guidelines

This document adopts and localizes the Rust API Guidelines for Iris.

Primary external reference:

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Rust API Guidelines Checklist](https://rust-lang.github.io/api-guidelines/checklist.html)

Use this document for designing public Rust APIs in Iris crates. It complements:

- [rust-best-practices.md](./rust-best-practices.md)
- [code-style.md](./code-style.md)
- [error-handling.md](./error-handling.md)
- [api-design.md](./api-design.md)

## Precedence

This document is an adopted standard, interpreted through Iris's architecture and performance constraints.

If there is tension:

1. [api-design.md](./api-design.md) governs crate boundaries, core trait shape, and terminal contract structure.
2. [code-style.md](./code-style.md) governs simplicity, explicitness, and hot-path cost.
3. This document provides the default Rust public-API baseline where the Iris-specific docs do not require a narrower rule.

## Scope

These rules matter most for:

- public items in `iris-core`
- public traits and types in `iris-platform`
- public renderer-facing APIs in `iris-render-wgpu`
- public CLI or configuration surfaces in `iris-standalone`

## Naming And Conventions

- Follow standard Rust naming conventions.
- Keep word order consistent across related APIs.
- Use standard conversion naming such as `as_*`, `to_*`, and `into_*` only when the semantics match Rust conventions.
- Use Rust-conventional getter names.
- Collection and iterator methods should follow the expected `iter`, `iter_mut`, and `into_iter` patterns.
- Iterator type names should match the methods that produce them.

## Interoperability

- Implement common standard traits when they make semantic sense and do not add noisy or misleading surface area.
- Public types should commonly consider `Clone`, `Debug`, `Default`, `Eq`, `PartialEq`, `Ord`, `PartialOrd`, `Hash`, and `Display` where appropriate.
- Prefer standard conversion traits such as `From`, `TryFrom`, `AsRef`, and `AsMut`.
- Collection-like types should consider `FromIterator` and `Extend`.
- Types should be `Send` and `Sync` where feasible and semantically correct.
- Error types should be meaningful, composable, and well-behaved.

## Documentation

- Crate-level docs should explain what the crate is for and how it is used.
- Public items should have examples when reasonable.
- Rustdoc examples should use idiomatic modern error handling.
- Function docs should call out error, panic, and safety behavior where relevant.
- Release notes and changelog entries should capture significant public changes.
- Rustdoc should hide irrelevant implementation detail when it does not help callers.

## Predictability

- Constructors should be inherent associated functions.
- Functions with a natural receiver should be methods.
- Do not use out-parameters.
- Avoid surprising operator overloads.
- Avoid implementing `Deref` or `DerefMut` unless the type is genuinely acting like a smart pointer.

## Flexibility

- Let the caller control allocations and copies where practical.
- Use generics when they reduce unnecessary input restrictions without obscuring behavior, increasing API fragility, or hurting hot-path clarity.
- Expose intermediate results when doing so avoids duplicate work for callers.
- Keep traits object-safe if dynamic dispatch is a realistic use case.

## Type Safety

- Prefer domain-specific types over ambiguous booleans or loosely meaningful `Option` parameters.
- Use newtypes when they encode real semantic distinctions.
- Use `bitflags` for flag sets instead of fake enums.
- Use builders when constructing complex public values with many optional settings.

## Dependability

- Validate arguments at API boundaries.
- Avoid APIs that can silently accept invalid state without checking.
- Destructors must not fail.
- If cleanup can block or fail in a meaningful way, provide an explicit method rather than relying on `Drop`.

## Debuggability

- All public types should implement `Debug` unless there is a strong reason not to.
- `Debug` output should be informative rather than empty or intentionally opaque without reason.

## Future-Proofing

- Prefer private fields on public structs unless there is a strong reason not to.
- Consider sealed traits when downstream implementations would make future evolution difficult.
- Use newtypes to hide implementation details that should remain changeable.
- Avoid exposing more structure than the public contract truly needs.

## Iris-Specific Interpretation

For Iris, these guidelines are especially important because:

- `iris-core` should feel stable and unsurprising to embedders and internal callers.
- platform traits in `iris-platform` need clear ownership and future evolution room.
- renderer APIs should expose only what composition requires, not internal GPU details.
- terminal-facing types should encode meaning precisely because correctness and cross-platform behavior matter more than brevity.

Compatibility notes with Iris docs:

- Public fields can still be acceptable for intentionally data-shaped config or transport structs when that matches [api-design.md](./api-design.md) and keeps the API clearer.
- "Implement expected traits" does not override Iris minimalism; add traits when they help real callers.
- Genericity is not a goal by itself; [code-style.md](./code-style.md) still rejects abstraction for abstraction's sake.

## Review Checklist

When reviewing public Rust API changes in Iris, check:

- naming consistency and standard Rust conventions
- trait implementations that callers will reasonably expect
- examples and crate-level docs where applicable
- constructor, method, and conversion predictability
- avoidance of `bool` parameters or weakly typed argument lists
- argument validation and error behavior
- `Debug` coverage for public types
- room for future evolution without immediate breaking changes
