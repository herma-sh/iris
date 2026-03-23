## Objective
Describe the concrete change this PR delivers and why it is needed now.

## Scope
- List exactly what is included in this PR.
- Keep this focused and reviewable.

## API and Behavior Changes
Describe externally visible behavior changes and internal contract changes.

Use numbered subsections when helpful:
1. API surface change
2. Runtime behavior change
3. Error behavior change

## Backward Compatibility
State compatibility impact clearly:
- additive/non-breaking
- source-breaking
- behavior-breaking

If there is no compatibility impact, state that explicitly.

## Test Coverage
List tests added or updated with exact test names and locations.

Also summarize strategy:
- happy-path coverage
- error/edge coverage
- platform-specific coverage (if relevant)

## Verification
List commands actually run for this PR and whether they passed.

Minimum for substantial Rust changes:
- `cargo fmt --all`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --all`

Add benchmark commands only when the PR is benchmark/performance-specific.

## Files Changed
List the key files changed so reviewers can navigate quickly.

## Notes
- Keep this template high-detail and specific; avoid one-line summaries.
- If a section is not applicable, write `N/A` and explain why.
