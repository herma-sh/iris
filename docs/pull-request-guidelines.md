# Pull Request Guidelines

Standards for production-grade Iris pull request descriptions.

## Required Template

All PRs must use the repository template:

- `./.github/PULL_REQUEST_TEMPLATE.md`

Do not replace it with a short freeform summary.

## Required Detail Level

Each required section must include concrete, reviewable detail:

1. `Objective`
   - What problem this PR solves now.
2. `Scope`
   - Exactly what is included in this PR.
3. `API and Behavior Changes`
   - Public API changes.
   - Runtime behavior changes.
   - Error handling changes.
4. `Backward Compatibility`
   - Explicitly classify impact (`additive`, `source-breaking`, `behavior-breaking`, or `none`).
5. `Test Coverage`
   - Exact tests added/updated (file paths + test names).
   - Coverage strategy summary (happy path, errors, edges, platform specifics when relevant).
6. `Verification`
   - Exact commands run and pass/fail status.
   - Include benchmarks only when the PR is benchmark/performance-specific.
7. `Files Changed`
   - Key files reviewers should inspect first.

## Section Applicability Rule

If a section does not apply, do not omit it. Mark it `N/A` and provide a one-line reason.

## Quality Bar

A PR description is not complete if it only states:

- high-level bullets with no behavior details
- no test names
- no verification commands
- no compatibility statement

The description must be sufficient for a reviewer to understand what changed, how it was validated, and what compatibility guarantees are being made without reverse-engineering the diff first.
