# Iris

Iris is the embeddable terminal platform for [Herma](../../README.md).

## Overview

Iris is a cross-platform terminal emulator designed for two modes:

1. **Standalone** - A native terminal application (like Alacritty, WezTerm, Ghostty)
2. **Embedded** - A terminal surface embedded in Hermes via Tauri

**Windows, Linux, and macOS are all first-class targets. Windows receives explicit priority.**

## Packages

Iris is published as `@iris/*` packages:

| Package | Description |
|---------|-------------|
| `@iris/core` | Terminal state, parser, buffer (no windowing dependencies) |
| `@iris/platform` | PTY, clipboard, IME, fonts (platform-specific) |
| `@iris/render` | GPU rendering via wgpu |
| `@iris/standalone` | Binary entry point for standalone terminal |

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
- **Zero-allocation hot path** - No Vec::push in parsing/rendering

### Crate Structure

```
crates/
  iris-core/           # No windowing/render dependencies
  iris-platform/       # PTY, clipboard, IME, fonts
  iris-render-wgpu/    # GPU rendering
  iris-standalone/     # winit binary
```

## Success Criteria

Iris v1 is complete when:

1. ✅ Runs standalone on Windows, Linux, macOS
2. ✅ Renders at 60fps with smooth scrolling
3. ✅ Passes vttest basic sequences
4. ✅ Handles real-world workloads (tmux, htop, vim)
5. ✅ Input latency < 16ms
6. ✅ Memory < 50MB at 10k scrollback
7. ✅ Embeds into Hermes via Tauri
8. ✅ "Open with Iris" works on Windows