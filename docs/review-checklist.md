# Iris Code Review Checklist

Standards for reviewing code changes.

## Before Review

### Author Checklist

Before requesting review, author must verify:

- [ ] Code compiles without warnings (`cargo build --all-features`)
- [ ] Tests pass (`cargo test --all`)
- [ ] Clippy passes (`cargo clippy --all-targets -- -D warnings`)
- [ ] Format is correct (`cargo fmt -- --check`)
- [ ] Documentation is updated
- [ ] CHANGELOG has entry (for user-facing changes)
- [ ] PR description follows `./.github/PULL_REQUEST_TEMPLATE.md`
- [ ] PR includes: Objective, Scope, API and Behavior Changes, Backward Compatibility, Test Coverage, Verification, Files Changed
- [ ] Any non-applicable template section is marked `N/A` with a reason

---

## General Review Checklist

### Correctness

- [ ] Does the code do what it's supposed to do?
- [ ] Are edge cases handled?
- [ ] Are error cases handled?
- [ ] Are there off-by-one errors?
- [ ] Are there potential null/None dereferences?

### Code Quality

- [ ] Is the code readable?
- [ ] Are names descriptive?
- [ ] Is the code DRY (Don't Repeat Yourself)?
- [ ] Is the code SOLID?
- [ ] Are functions focused and small?
- [ ] Is there unnecessary complexity?

### Performance

- [ ] Are there allocations in hot paths?
- [ ] Are there unnecessary clones?
- [ ] Are there inefficient loops?
- [ ] Is memory managed properly?
- [ ] Are there potential performance regressions?

### Safety

- [ ] Is `unsafe` code documented with SAFETY comments?
- [ ] Are invariants maintained?
- [ ] Is there potential for UB (undefined behavior)?
- [ ] Are panic cases handled gracefully?

### Testing

- [ ] Are there unit tests for new functionality?
- [ ] Do tests cover edge cases?
- [ ] Are tests maintainable?
- [ ] Do existing tests still pass?

### Documentation

- [ ] Is public API documented?
- [ ] Are complex algorithms explained?
- [ ] Are invariants documented?
- [ ] Is README updated if needed?

---

## Crate-Specific Checklists

### iris-core

**No windowing/rendering dependencies allowed:**

- [ ] No `wgpu` imports
- [ ] No `winit` imports
- [ ] No platform-specific imports
- [ ] All dependencies are core-compatible

**Zero-allocation hot paths:**

- [ ] Parser `parse_byte` does not allocate
- [ ] Grid `write` does not allocate
- [ ] No `Vec::push` in hot loops
- [ ] No `String` creation in hot paths

**Thread safety:**

- [ ] All public types are `Send + Sync` or documented why not
- [ ] Lock usage is documented
- [ ] No deadlocks possible

### iris-platform

**Platform abstraction:**

- [ ] All platform code behind traits
- [ ] Windows code in `windows.rs`
- [ ] Unix code in `unix.rs`
- [ ] macOS code in `macos.rs`

**Unsafe code:**

- [ ] All `unsafe` blocks have SAFETY comments
- [ ] FFI boundaries are safe
- [ ] Platform API usage is correct

### iris-render-wgpu

**GPU safety:**

- [ ] No GPU validation errors in debug builds
- [ ] Uniform buffers don't overflow
- [ ] All shaders compile
- [ ] Texture dimensions are valid

**Frame pacing:**

- [ ] No dropped frames at 60fps
- [ ] VSync is respected
- [ ] Resize doesn't cause GPU hangs

---

## Hot Path Checklist

For code in hot paths (parser, grid, render):

### Allocation

```rust
// ❌ DON'T: Allocate in hot path
fn parse(&mut self, data: &[u8]) {
    let mut buf = Vec::new();  // Allocates!
    buf.extend(data);
}

// ✅ DO: Use pre-allocated buffers
fn parse(&mut self, data: &[u8]) {
    self.buffer.extend_from_slice(data);  // Pre-allocated
}
```

Check:

- [ ] No heap allocations
- [ ] No `clone()` calls
- [ ] No `String` creation
- [ ] No `Vec` growth

### Bounds Checking

```rust
// ❌ DON'T: Bounds check twice
fn cell(&self, col: usize, row: usize) -> &Cell {
    let idx = self.index(col, row);  // Bounds check
    &self.cells[idx]  // Bounds check again
}

// ✅ DO: Single bounds check
fn cell(&self, col: usize, row: usize) -> Option<&Cell> {
    self.cells.get(row * self.cols + col)  // Single check
}

// ✅ OK: Unchecked when safe
fn cell_unchecked(&self, col: usize, row: usize) -> &Cell {
    // SAFETY: col < self.cols && row < self.rows guaranteed by caller
    unsafe { self.cells.get_unchecked(row * self.cols + col) }
}
```

Check:

- [ ] Minimal bounds checks
- [ ] `get_unchecked` has SAFETY comment
- [ ] Not trading safety for minor speed

### Cache Efficiency

```rust
// ❌ DON'T: Poor cache access
fn process_grid(&self) {
    for col in 0..self.cols {
        for row in 0..self.rows {  // Row-major stored row-wise
            self.cells[row * self.cols + col];  // Cache unfriendly
        }
    }
}

// ✅ DO: Cache-friendly access
fn process_grid(&self) {
    for row in 0..self.rows {
        for col in 0..self.cols {  // Row-wise access
            self.cells[row * self.cols + col];  // Cache friendly
        }
    }
}
```

Check:

- [ ] Row-major access pattern
- [ ] Struct-of-arrays for hot data
- [ ] Array-of-structs for cold data

---

## Test Review Checklist

### Test Quality

- [ ] Tests are readable
- [ ] Tests are not brittle
- [ ] Tests use realistic data
- [ ] Tests test behavior, not implementation
- [ ] Tests are independent
- [ ] Tests prefer concrete backends/real data when those paths exist or are expected soon
- [ ] Any mock-data tests justify why real-backend coverage is not yet practical

### Test Coverage

- [ ] Happy path tested
- [ ] Error path tested
- [ ] Edge cases tested
- [ ] Boundary conditions tested

### Test Performance

- [ ] Tests don't sleep (use proper synchronization)
- [ ] Tests don't depend on timing
- [ ] Tests run fast (< 100ms each)

---

## Security Review Checklist

### Input Validation

- [ ] PTY input is validated
- [ ] ANSI sequences are bounded
- [ ] Unicode is validated
- [ ] File paths are canonicalized

### Privilege Boundaries

- [ ] No privilege escalation
- [ ] Sandbox boundaries respected
- [ ] Sensitive operations logged

### Secrets

- [ ] No secrets in logs
- [ ] Secrets cleared after use
- [ ] No secrets in config files (use keychain)

---

## Documentation Review

### Public API

- [ ] Every public type has docs
- [ ] Every public function has docs
- [ ] Examples are provided
- [ ] Panics are documented
- [ ] Errors are documented

### Code Comments

- [ ] Comments explain "why" not "what"
- [ ] Complex code is explained
- [ ] TODOs have issue numbers
- [ ] FIXMEs are urgent

### Module Docs

- [ ] Module has overview
- [ ] Key types are highlighted
- [ ] Common patterns are shown

---

## Performance Review

### Startup Time

- [ ] No blocking operations in init
- [ ] Lazy initialization where possible
- [ ] Font loading is deferred

### Memory

- [ ] No memory leaks
- [ ] Buffers are reused
- [ ] Large objects are freed promptly

### Rendering

- [ ] Damage tracking is used
- [ ] No full redraws unless needed
- [ ] GPU resources are pooled

---

## Cross-Platform Review

### Windows

- [ ] Paths use `\` or are normalized
- [ ] Line endings handled correctly
- [ ] ConPTY quirks tested

### macOS

- [ ] Metal is used correctly
- [ ] Retina scaling handled
- [ ] App bundle is valid

### Linux

- [ ] X11 clipboard works
- [ ] Wayland clipboard works
- [ ] Font discovery works

---

## Review Process

### 1. Automated Checks

CI must pass before review:

```
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all
cargo bench -- --save-baseline pr
```

### 2. Manual Review

1. Read PR description
   - Confirm the PR template sections are present and detailed.
   - Confirm verification commands listed were actually run.
2. Review changed files in order:
   - Core types first
   - Implementation
   - Tests
   - Documentation
3. Leave comments
4. Request changes or approve

### 3. Review Comments

Use conventional prefixes:

| Prefix | Meaning |
|--------|---------|
| `[NIT]` | Minor style issue, non-blocking |
| `[MUST]` | Required change |
| `[SHOULD]` | Strongly recommended |
| `[QUESTION]` | Seeking clarification |
| `[SUGGEST]` | Optional improvement |

### 4. Approval

Celebrate clean code:

- `[LGTM]` - Looks good to merge
- `[LGTM] with [NIT]` - Merge with minor fixes
