# Iris wgpu Guide

## Purpose

Use this file for renderer work, shaders, GPU resources, and `wgpu`-specific debugging.

Primary references:

- [../docs/design.md](../docs/design.md)
- [../docs/implementation.md](../docs/implementation.md)
- [../docs/benchmarks.md](../docs/benchmarks.md)
- [../docs/phases/02.md](../docs/phases/02.md)
- [../docs/phases/09.md](../docs/phases/09.md)
- [../docs/phases/13.md](../docs/phases/13.md)

External references:

- [`wgpu` README](https://github.com/gfx-rs/wgpu)
- [`wgpu` CONTRIBUTING.md](https://github.com/gfx-rs/wgpu/blob/trunk/CONTRIBUTING.md)
- [`wgpu` testing guide](https://github.com/gfx-rs/wgpu/blob/trunk/docs/testing.md)

## Renderer Rules

- Keep renderer responsibilities limited to drawing from terminal state.
- Do not move PTY, clipboard, shell integration, or host orchestration into the render crate.
- Prefer damage-only updates over full redraws.
- Batch aggressively when correctness allows it.
- Keep glyph cache and atlas behavior explicit and measurable.

## Shader And Pipeline Rules

- Keep shader inputs and instance layouts documented and consistent with Rust-side structs.
- Treat validation warnings and GPU errors as real defects.
- Avoid clever shader complexity unless it is justified by measurement.
- Keep cursor, selection, and cell rendering behavior predictable and testable.

## Debugging And Environment

Useful environment variables when debugging `wgpu` issues:

```bash
WGPU_BACKEND=vulkan
WGPU_BACKEND=dx12
WGPU_BACKEND=metal
WGPU_ADAPTER_NAME=<substring>
DXC_PATH=<path-to-dxc>
```

Inference from `wgpu` docs: the exact variables and supported backends can evolve with `wgpu` releases, so confirm against upstream docs if renderer behavior or setup looks version-specific.

## Performance Expectations

- Track frame time, damage size, glyph cache misses, and startup or first-paint cost.
- Benchmark changes that affect batching, atlas allocation, resize, scrolling, or text shaping.
- Prefer changes that reduce per-frame CPU work and GPU submission overhead without obscuring correctness.

## Cross-Platform Notes

- Validate Windows, Linux, and macOS behavior whenever rendering changes are non-trivial.
- Watch for backend-specific differences in text rendering, surface creation, DPI scaling, and validation behavior.
- Do not assume a macOS-first path will generalize cleanly to DX12 or Vulkan.
