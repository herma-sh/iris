# Iris

Iris is the terminal platform documented in this repository.

## Overview

Iris is a cross-platform terminal emulator built for two deployment modes:

1. **Standalone** - A native terminal application
2. **Embedded** - A terminal surface hosted inside another application

Windows, Linux, and macOS are all first-class targets, with Windows treated as an explicit design-time priority.

## Workspace Crates

The current repository is organized as a Rust workspace:

| Crate | Description |
|-------|-------------|
| `iris-core` | Terminal state, parser, buffer, damage tracking, and tests |
| `iris-platform` | PTY, clipboard, IME, and font abstractions |
| `iris-render-wgpu` | GPU rendering via `wgpu` |
| `iris-standalone` | Planned standalone binary entry point in a later phase |

## Design Reference

> **Screenshot Reference**: `chrome_H2F5YRWQmB.png` - Visual design target for Iris.
> This file contains the target visual appearance. Agents with image capabilities
> should reference this for typography, spacing, color, and chrome decisions.

## Documentation Index

| Document | Purpose |
|----------|---------|
| [engineering-brief.md](./engineering-brief.md) | Product vision, naming, and strategic context |
| [features.md](./features.md) | Feature list for embedded and standalone modes |
| [design.md](./design.md) | Architecture, crate structure, and design decisions |
| [research.md](./research.md) | Lessons from other terminals, common problems, solutions |
| [implementation.md](./implementation.md) | Core components, code patterns, technical details |
| [phases.md](./phases.md) | Development phases and timeline |
| [code-style.md](./code-style.md) | Coding standards: minimal, fast, maintainable, readable |
| [rust-best-practices.md](./rust-best-practices.md) | Iris-specific Rust standards derived from Canonical Rust best practices |
| [rust-api-guidelines.md](./rust-api-guidelines.md) | Iris-specific Rust API design standards derived from the Rust API Guidelines |
| [innovation.md](./innovation.md) | Emerging features and unique differentiators |
| [security-threat-model.md](./security-threat-model.md) | Security threats and mitigations |

## Quick Links

### Performance Targets

| Metric | Target | Benchmark |
|--------|--------|-----------|
| Input latency | < 4ms | Ghostty ~8ms |
| Startup time | < 30ms | Ghostty <50ms |
| Scroll FPS | 60fps @ 10M lines | Alacritty 60fps @ 1M |
| Memory (10k scrollback) | < 25MB | Alacritty ~30MB |

### Key Design Decisions

- **wgpu for rendering** - Works on Vulkan/DX12/Metal, enables both standalone and embedded
- **Buffer-first architecture** - Parser needs somewhere to write; grid model comes first
- **Cell grid with attribute runs** - Efficient GPU batching, memory savings
- **Lock-free reads** - Arc snapshots for concurrent render without mutex
- **Zero-allocation hot path** - No hidden heap churn in parser and render hot loops

### Crate Structure

```text
crates/
  iris-core/           # No windowing/render dependencies
  iris-platform/       # PTY, clipboard, IME, fonts
  iris-render-wgpu/    # GPU rendering
  iris-standalone/     # standalone binary (Phase 6)
```

## Success Criteria

Iris v1 is complete when:

1. Runs standalone on Windows, Linux, and macOS.
2. Renders at 60fps with smooth scrolling.
3. Passes VTtest basic sequences once the standalone binary can host an interactive session.
4. Handles real-world workloads such as `tmux`, `htop`, and `vim`.
5. Keeps input latency below 16ms.
6. Stays below 50MB at 10k lines of scrollback.
7. Embeds cleanly into host applications through the documented integration surface.
8. Supports platform-native launch and shell workflows on Windows.
