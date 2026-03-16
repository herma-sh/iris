# Iris Engineering Brief

## Summary

Iris is the embeddable terminal platform for Hermes.

It is not a standalone terminal product. It exists to make Hermes the best cross-platform SSH and terminal workspace for Windows, Linux, and macOS, while remaining reusable as an open source terminal component for other software.

Within Hermes, Iris is consumed by `imux`, the integrated workspace shell/window that brings together terminal, git, browser, and operator workflow surfaces.

Ghostty is a quality benchmark for speed, smoothness, restraint, and rendering feel. Iris is not a Ghostty fork, not a Ghostty skin, and not a Ghostty asset port.

## Naming model

- `Hermes`: the product and company-facing application
- `Iris`: the embeddable terminal platform
- `imux`: the primary Hermes workspace shell/window that hosts Iris alongside git, browser, and related tools

## Stack position

Iris should be Rust-heavy by default.

The stack bias is:

- `Rust` for terminal core, platform integration, renderer-adjacent logic, performance-sensitive behavior, and OS-facing correctness
- `TypeScript` for host contracts, bindings, and thin frontend integration
- `React` only as the embedding surface inside `imux`, not as the place where terminal intelligence lives
- `Tauri` as the desktop host bridge while Iris is embedded inside Hermes

This keeps speed, correctness, and cross-platform behavior in Rust while preserving clean integration with the existing Hermes application shell.

## Product definition

Iris should be understood as:

- a cross-platform terminal technology stack
- an embeddable terminal surface and host API
- a Hermes-owned terminal experience designed for SSH, tmux, and agent-driven workflows
- a foundational engine consumed by `imux` and potentially by other host software later
- a Rust-first system with a minimal TypeScript host layer

Iris should not be understood as:

- a separate desktop terminal application
- a one-off visual theme over `xterm.js`
- a macOS-first renderer patched later for Windows
- a frontend-heavy terminal implementation where core behavior lives in React

## Why this exists

Hermes is trying to win a category where terminal quality is part of the product, not a commodity detail.

For Hermes, terminal quality directly affects:

- trust during long SSH sessions
- speed and perceived latency
- readability under dense operator workflows
- session resilience during reconnects and resizes
- confidence that local and remote shells behave consistently

If Hermes wants to be the best cross-platform SSH and terminal agent workspace, it needs a terminal layer that it can tune deeply instead of treating as a black box forever.

## Product principles

### 1. Embeddable first

Iris is built for embedding inside Hermes.

Everything should be designed as a hostable module:

- clear lifecycle
- explicit input and output contracts
- no assumptions about owning the entire window or app shell
- no dependency on standalone settings screens or terminal-app concepts
- compatibility with `imux` as the primary host shell inside Hermes

### 2. Cross-platform from the start

Windows, Linux, and macOS are all first-class targets.

That means:

- no renderer architecture that assumes Cocoa or Linux desktop primitives
- no keybinding model that silently privileges one platform
- no clipboard, IME, font, or shell behavior left as "fix later"

### 3. Hermes-native identity

Iris has its own visual language, assets, interaction details, and host API.

The bar is premium, restrained, and fast. The visual direction should feel deliberate and original rather than nostalgic or derivative.

### 4. Quality before novelty

The terminal should feel reliable before it feels clever.

Priority order:

1. correctness
2. responsiveness
3. readability
4. interaction polish
5. advanced effects

Implementation bias:

- if it affects terminal behavior, rendering quality, responsiveness, parsing, selection, search, or platform correctness, prefer `Rust`
- if it affects host composition, props, events, theming controls, or `imux` integration, use `TypeScript`

### 5. Measured performance

"Smooth" is not a vibe target alone. It must be measured.

Iris should define and track:

- time to first rendered prompt
- input-to-paint latency
- scroll performance under large scrollback
- resize stability
- memory cost at realistic scrollback sizes
- dropped frames under heavy output

## Non-goals

- shipping a standalone Iris desktop app
- cloning Ghostty UI or reusing Ghostty assets
- replacing Hermes SSH transport with a full in-house SSH implementation in the near term
- rebuilding every terminal concern at once before Hermes gets product value
- chasing exotic rendering features before baseline operator workflows are excellent

## Core promise

Iris should eventually give Hermes a terminal that feels:

- visually premium
- operationally trustworthy
- fast under real SSH and tmux usage
- native on Windows, Linux, and macOS
- extensible for agent-centric workflows

## Architecture boundaries

Iris should be split into four layers.

### 1. Terminal core

Owns terminal behavior that should not depend on Hermes UI or a specific host surface.

Scope:

- terminal state model
- buffer and scrollback behavior
- selection model
- search model
- theme model
- parser and escape-sequence handling strategy
- performance instrumentation hooks

The core must avoid direct dependency on React, Tauri window primitives, or app-specific session UX.

### 2. Platform layer

Owns OS-specific integration.

Scope:

- PTY integration
- keyboard event normalization
- clipboard integration
- IME behavior
- font discovery or fallback metadata
- accessibility hooks
- per-platform shell affordances where needed

This is where Windows-specific correctness has to be explicit rather than bolted on later.

### 3. Render layer

Owns drawing and viewport behavior.

Scope:

- glyph rendering strategy
- cursor rendering
- selection rendering
- scroll viewport behavior
- damage or repaint strategy
- text measurement and layout hooks

This layer must be replaceable. Hermes should not be welded forever to the first renderer implementation.

### 4. Host API

Owns the boundary Hermes consumes.

Scope:

- create and destroy terminal instances
- attach data streams
- send input
- resize
- focus and selection commands
- copy and paste hooks
- theme and font updates
- search controls
- telemetry events

Hermes should talk to Iris through this API, not through renderer-specific internals.

## Suggested monorepo shape

This does not need to happen in one migration, but the end state should look roughly like this:

```text
packages/
  terminal-contract/   shared TypeScript contracts for the host API
  terminal-react/      React host components for Hermes
  terminal-theme/      Hermes Terminal themes, typography, and tokens

crates/
  iris-core/           terminal core logic and shared runtime types
  iris-platform/       PTY/platform integration
  iris-render/         renderer implementation(s)
```

Notes:

- `terminal-contract` keeps the Hermes frontend from binding itself to a renderer implementation.
- `terminal-react` should remain thin. It is a host adapter, not the terminal engine.
- `terminal-theme` can stay frontend-friendly, but theme semantics should still map cleanly to Rust-owned terminal capabilities.
- the Rust crates can start life inside `apps/desktop/src-tauri` and only be extracted once the boundaries are stable

## Current and target stack

Current practical stack:

- `Rust` in Tauri for PTY and system integration
- `TypeScript` and `React` in Hermes for the host UI
- `xterm.js` as the current renderer

Target Iris stack:

- `Rust` for the majority of the terminal platform
- `TypeScript` for contracts and bindings
- `React` for a thin host component inside `imux`
- `xterm.js` only as an interim renderer during migration, if it remains useful behind the Iris boundary

## Hermes integration model

Hermes remains responsible for:

- tab and workspace management
- SSH and tmux session flows
- server metadata and reconnect UX
- agent and tool workflow orchestration
- desktop shell layout and navigation

Iris becomes responsible for:

- terminal presentation
- terminal interaction fidelity
- terminal state and rendering behavior
- terminal-level telemetry

`imux` becomes responsible for:

- composing Iris with git, browser, files, and workflow sidecars
- window-level navigation and pane choreography
- integrated operator context around the terminal rather than inside the terminal engine

The split matters because Hermes is a workspace product. Iris is one of its foundational engines.

## Migration strategy

The migration should be staged. Do not freeze product progress while trying to write the "final" terminal engine.

### Phase 0: Stabilize the current boundary

Current Hermes already has:

- Rust PTY/session management
- a React terminal workspace
- `xterm.js` rendering

First step:

- wrap the current terminal surface behind a Hermes-owned interface
- remove direct `xterm.js` assumptions from higher-level workspace code
- define terminal events, commands, theme inputs, and metrics explicitly
- keep `xterm.js` as an implementation detail rather than a platform identity

This phase reduces coupling immediately and creates a safe seam for future renderer work.

### Phase 1: Premium Hermes terminal shell

Before deeper engine work, improve the user-visible quality bar:

- typography
- cursor and selection styling
- spacing and density
- tab strip and pane transitions
- startup and reconnect feel
- scrollback defaults
- theme consistency

This is the fastest route to a terminal that already feels significantly more premium.

### Phase 2: Instrumentation and stress testing

Build repeatable measurement before swapping the renderer.

Add:

- render timing
- input latency measurement
- scroll stress fixtures
- resize stress fixtures
- heavy ANSI output fixtures
- Windows-specific behavior tests

Without this phase, "better than before" will be guesswork.

### Phase 3: Renderer spike

After the host API and instrumentation exist, prototype alternate renderer paths behind the same boundary.

Goals:

- compare `xterm.js` against a Hermes-owned render path
- identify the actual bottlenecks
- validate Windows behavior early
- measure whether a deeper native path is justified

This phase should be treated as a spike, not an immediate rewrite.

### Phase 4: Expand Iris core ownership

Only after the spike proves value should Iris absorb more of the stack:

- parser ownership
- buffer ownership
- search and selection ownership
- richer telemetry
- renderer-specific optimization work
- more of the rendering path in Rust where the measurements justify it

## Quality bar

Iris should target the following characteristics:

- immediate visual clarity at first prompt
- no jarring reflow on resize
- responsive typing under remote latency
- stable scrolling under large output
- strong copy and selection behavior
- deliberate, restrained UI chrome
- clean font rendering and spacing at default settings

## Windows-specific requirements

Windows cannot be a follow-up compatibility pass.

Iris must treat these as design-time concerns:

- PowerShell, PowerShell 7, and `cmd.exe` behavior
- PTY behavior through the platform abstraction
- keyboard semantics that differ from Unix terminals
- font fallback and glyph coverage
- clipboard edge cases
- IME and composition handling
- high-DPI rendering behavior

If Windows quality is left to late-stage patching, the architecture will drift toward macOS-first assumptions and stay expensive forever.

## Design direction

Iris should feel:

- calm under pressure
- dense but readable
- tactile without decorative noise
- technical and premium rather than retro

Design implications:

- custom assets and original visual language
- typography chosen for shell readability first
- low-chrome surfaces with precise contrast
- motion used to communicate state, not to decorate
- no visual borrowing that makes the product feel derivative

## Open source posture

Open source is compatible with the strategy, but Hermes remains the primary customer.

That means:

- the API should be clean enough for external embedding
- the implementation should not depend on Hermes-only assumptions
- roadmap decisions should still prioritize Hermes product value first

Hermes should be the flagship integration that proves Iris works in a demanding real product.

## Build order

Recommended order:

1. define the Iris boundary inside Hermes
2. create contracts for terminal commands, events, themes, and metrics
3. refactor the current `xterm.js` integration behind that boundary
4. improve visible shell quality and baseline interaction polish
5. add repeatable performance and behavior fixtures
6. run a renderer spike behind the same API
7. decide whether deeper terminal-core ownership is justified

## Decision rule

Hermes should not replace working terminal infrastructure just because a lower-level approach sounds more powerful.

Iris takes on deeper ownership only when one of these is true:

- the current renderer is measurably limiting smoothness or correctness
- a Hermes-specific workflow needs capabilities the current stack cannot support cleanly
- the abstraction layer is already stable enough that a deeper implementation can land without destabilizing the product

## Short version

Iris is a Hermes-owned, embeddable, cross-platform terminal platform.

It exists to give Hermes first-class terminal quality on Windows, Linux, and macOS without turning the company into a standalone terminal-app project.

The right first move is not a rewrite. It is to create the boundary, improve the visible experience, measure the bottlenecks, and then replace deeper layers only where the data says it matters.
