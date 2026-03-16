# Iris Release Criteria

Quality gates and shipping requirements.

## Release Types

| Type | Cadence | Stability | Changes |
|------|---------|-----------|---------|
| Major (X.0.0) | Breaking changes | Highest approval | API breaks |
| Minor (0.X.0) | Feature releases | Feature freeze | New features |
| Patch (0.0.X) | Bug fixes | Anytime | Bug fixes only |

---

## Alpha Releases

**Purpose**: Early testing, feedback collection.

**Criteria:**

| Criterion | Requirement |
|-----------|-------------|
| Compiles | On all target platforms |
| Basic tests pass | Unit tests |
| Core functionality | PTY, parser, grid work |
| Documentation | README exists |

**No blocking bugs required. Alpha is for testing.**

---

## Beta Releases

**Purpose**: Wider testing, stability polish.

**Criteria:**

| Criterion | Requirement |
|-----------|-------------|
| All tests pass | Unit + integration |
| No compiler warnings | `cargo clippy -- -D warnings` |
| Performance baselines | No regression > 10% |
| Documentation | All public API documented |
| Platform testing | Windows, macOS, Linux |
| Known issues | Documented in release notes |

**Blocking bugs:**

- [ ] No crashes on normal use
- [ ] No data loss bugs
- [ ] No security vulnerabilities

---

## Release Candidate (RC)

**Purpose**: Final testing before stable.

**Criteria:**

| Criterion | Requirement |
|-----------|-------------|
| All tests pass | Including conformance |
| Performance verified | All benchmarks meet target |
| Manual QA complete | All platforms tested |
| Documentation complete | User guide, API docs |
| Conformance tested | vttest passes |

**Blocking bugs:**

- [ ] No crashes
- [ ] No hangs
- [ ] No data corruption
- [ ] No security issues
- [ ] No performance regressions

---

## Stable Release

### Must Have

**Code Quality:**

- [ ] No compiler warnings
- [ ] No clippy warnings
- [ ] Code formatted (`cargo fmt`)
- [ ] All public API documented
- [ ] CHANGELOG updated

**Tests:**

- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] All benchmarks within threshold
- [ ] vttest passes
- [ ] No skipped tests

**Platforms:**

- [ ] Windows 10/11 verified
- [ ] macOS 12+ verified
- [ ] Linux (Ubuntu 22.04) verified
- [ ] Linux (Arch rolling) verified

**Performance:**

- [ ] Startup time < 100ms
- [ ] Input latency < 10ms (target < 4ms)
- [ ] Scroll FPS >= 60
- [ ] Memory (10k lines) < 100MB (target < 50MB)

**Security:**

- [ ] No secrets in binary
- [ ] No unsafe without SAFETY comment
- [ ] Input validation complete
- [ ] No known CVEs in dependencies

### Should Have

**Documentation:**

- [ ] User guide
- [ ] Configuration reference
- [ ] Keyboard shortcut list
- [ ] Migration guide (from other terminals)

**UX:**

- [ ] First-run experience
- [ ] Error messages are helpful
- [ ] Settings are discoverable
- [ ] Keyboard internationalization

---

## Pre-Release Checklist

### 1 Week Before

- [ ] Feature freeze
- [ ] Focus on bugs and polish
- [ ] Update CHANGELOG
- [ ] Update documentation
- [ ] Create release notes draft

### 3 Days Before

- [ ] Run full test suite
- [ ] Run complete manual QA
- [ ] Verify all platforms
- [ ] Check performance baselines
- [ ] Security audit

### 1 Day Before

- [ ] Final bug triage
- [ ] Update version numbers
- [ ] Create git tag
- [ ] Build release binaries
- [ ] Test installers

### Release Day

- [ ] Push tag to GitHub
- [ ] Create GitHub release
- [ ] Upload binaries
- [ ] Announce release
- [ ] Update website

---

## Platform-Specific Criteria

### Windows

| Criterion | Requirement |
|-----------|-------------|
| ConPTY | Works on Windows 10 1809+ |
| WSL | Functional integration |
| PowerShell | Full support |
| High DPI | Scales correctly |
| Clipboard | Copy/paste works |
| Context menu | "Open with Iris" works |

**Testing:**

- [ ] Install MSI
- [ ] Install portable
- [ ] Update from previous version
- [ ] Uninstall cleans up

### macOS

| Criterion | Requirement |
|-----------|-------------|
| Metal | Renders correctly |
| Retina | Scales correctly |
| Touch Bar | Works (if supported) |
| Keyboard | All shortcuts work |
| Clipboard | Copy/paste works |

**Testing:**

- [ ] Install DMG
- [ ] Notarization (if applicable)
- [ ] Update from previous version
- [ ] Uninstall

### Linux

| Criterion | Requirement |
|-----------|-------------|
| X11 | Clipboard, selection |
| Wayland | Clipboard, selection |
| Font config | Discovers fonts |
| Themes | Matches desktop |

**Testing:**

- [ ] Install .deb
- [ ] Install .rpm
- [ ] Install AppImage
- [ ] Install from source

---

## Conformance Testing

### vttest

Must pass all sections of vttest:

- [ ] Test of cursor movements
- [ ] Test of screen features
- [ ] Test of character sets
- [ ] Test of screen colors
- [ ] Test of scroll regions
- [ ] Test of status line
- [ ] Test of double width/high height
- [ ] Test of soft character sets

### Real-World Applications

Must work correctly with:

- [ ] vim / neovim
- [ ] emacs
- [ ] tmux
- [ ] htop / btop
- [ ] git (diff, log)
- [ ] docker (attach, logs)
- [ ] cargo build
- [ ] pytest
- [ ] ssh (remote sessions)

### Edge Cases

- [ ] Very long lines (10000+ chars)
- [ ] Rapid output (cat large file)
- [ ] Binary data
- [ ] Invalid UTF-8
- [ ] Control characters
- [ ] Resize during output
- [ ] Signal handling (SIGWINCH)

---

## Performance Gates

### Hard Limits (Must Pass)

| Metric | Limit | Target |
|--------|-------|--------|
| Startup time | < 500ms | < 100ms |
| Input latency | < 50ms | < 10ms |
| Scroll FPS | > 30 fps | 60 fps |
| Memory (10k) | < 200MB | < 50MB |
| Binary size | < 50MB | < 20MB |

### Regression Limits (Must Not Exceed)

| Metric | Max Regression |
|--------|---------------|
| Startup time | +20% |
| Input latency | +20% |
| Scroll FPS | -10fps |
| Memory | +20% |
| Binary size | +10% |

---

## Security Requirements

### Dependency Audit

```bash
cargo audit
```

- [ ] No known CVEs in dependencies
- [ ] Dependencies at latest stable version
- [ ] License compatibility verified

### Static Analysis

```bash
cargo clippy --all-targets -- -D warnings
```

- [ ] No warnings

### Dynamic Analysis

**AddressSanitizer:**

```bash
RUSTFLAGS="-Z sanitizer=address" cargo test
```

- [ ] No memory safety issues

**ThreadSanitizer:**

```bash
RUSTFLAGS="-Z sanitizer=thread" cargo test
```

- [ ] No data races

---

## Quality Metrics

### Code Coverage

| Crate | Minimum |
|-------|---------|
| iris-core | 85% |
| iris-platform | 75% |
| iris-render-wgpu | 65% |
| Overall | 75% |

### Documentation Coverage

| Type | Minimum |
|------|---------|
| Public API | 100% |
| Public types | 100% |
| Examples | All public functions |
| Error types | 100% |

### Performance Metrics

All benchmarks must meet targets. Track over time:

| Metric | Target | Trend |
|--------|--------|-------|
| Parser throughput | > 100MB/s | Improving |
| Grid write | < 1µs | Stable |
| Render latency | < 5ms | Improving |

---

## Post-Release

### Monitoring (First Week)

- [ ] GitHub issues
- [ ] Crash reports
- [ ] Performance complaints
- [ ] Installation issues

### Hotfixes

If critical bug found:

1. Create hotfix branch
2. Fix bug with tests
3. Run abbreviated checklist:
   - [ ] Tests pass
   - [ ] Platform tests
   - [ ] No new bugs
4. Release as patch version

### Post-Mortem (After Major Release)

- What went well?
- What went wrong?
- What can improve?
- Update process based on learnings