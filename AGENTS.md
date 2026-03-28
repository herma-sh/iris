# Iris Agent Guide

This file is the entrypoint for coding agents working in the Iris repository.

Start here, then load the focused guidance in `.agents/`.

## Read Order

1. [README.md](./README.md)
2. [.agents/agent.md](./.agents/agent.md)
3. [.agents/rules.md](./.agents/rules.md)

Load these when relevant:

- Rust implementation guidance: [.agents/rust.md](./.agents/rust.md)
- `wgpu` and renderer guidance: [.agents/wgpu.md](./.agents/wgpu.md)
- Testing and verification guidance: [.agents/testing.md](./.agents/testing.md)

## Working Model

- Keep the root context light and use it as a map.
- Prefer linking to source-of-truth docs over duplicating them.
- Follow the active phase in [docs/phases.md](./docs/phases.md) and the matching file in [docs/phases](./docs/phases).
- Update [CHANGELOG.md](./CHANGELOG.md) with every meaningful change.

## Branch And PR Workflow (Required)

- Do not work directly on `main` or `dev`.
- Before making code or docs changes, create/switch to a `feature/*` branch from `dev`.
- Keep each branch focused on one coherent change set.
- Open or update a pull request targeting `dev` after verification completes.
- Use `./.github/PULL_REQUEST_TEMPLATE.md` for every PR description.
- If a PR cannot be opened (for example, missing auth/remote), report the blocker and provide exact commands to complete it.

## Source Documents

- Architecture: [docs/design.md](./docs/design.md), [docs/api-design.md](./docs/api-design.md), [docs/implementation.md](./docs/implementation.md)
- Standards: [docs/code-style.md](./docs/code-style.md), [docs/error-handling.md](./docs/error-handling.md)
- Rust standards: [docs/rust-best-practices.md](./docs/rust-best-practices.md)
- Rust API standards: [docs/rust-api-guidelines.md](./docs/rust-api-guidelines.md)
- Performance and testing: [docs/benchmarks.md](./docs/benchmarks.md), [docs/testing-strategy.md](./docs/testing-strategy.md)
- Security and logging: [docs/security-threat-model.md](./docs/security-threat-model.md), [docs/logging-strategy.md](./docs/logging-strategy.md)

## Rule

If a deeper document exists for the task, read that document instead of guessing.
