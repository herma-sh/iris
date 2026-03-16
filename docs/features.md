# Iris Features

Features planned for Iris terminal emulator, covering both embedded and standalone modes.

## UX Philosophy

Iris prioritizes speed and discoverability. Every action should be achievable multiple ways.

### Core UX Principles

| Principle | Implementation |
|-----------|---------------|
| **Multiple paths** | Every action has mouse, keyboard, and menu paths |
| **Sensible defaults** | Works great out of the box, customize later |
| **Progressive disclosure** | Simple first, power features discoverable |
| **Instant feedback** | No action should feel laggy |
| **Reversible** | Undo for most operations |

### Interaction Philosophy

**The 80/20 rule:** 80% of users use 20% of features. Make that 20% instant. The other 80% should be discoverable but not intrusive.

| User Type | Needs | Priority |
|-----------|-------|----------|
| Novice | Clear defaults, discoverability | Sensible defaults, tooltips |
| Power user | Speed, keyboard everything | Customizable shortcuts, leader keys |
| Admin | SSH, automation | Connection profiles, scripting |

---

## Copy and Paste

Multiple methods for different workflows. All should work seamlessly.

### Keyboard Methods

| Action | Default (Windows/Linux) | Default (macOS) | Customizable |
|--------|-------------------------|-----------------|--------------|
| Copy | Ctrl+Shift+C | Cmd+C | Yes |
| Copy (alternate) | Ctrl+C (when no selection) | Cmd+C | Yes |
| Paste | Ctrl+Shift+V | Cmd+V | Yes |
| Paste (alternate) | Ctrl+V | Cmd+V | Yes |
| Copy selection | Ctrl+Shift+C | Cmd+C | Yes |
| Copy command output | Ctrl+Alt+C | Cmd+Option+C | Yes |

### Mouse Methods

| Action | Method | Notes |
|--------|--------|-------|
| Select | Click + drag | Character-by-character |
| Select word | Double-click | Selects word under cursor |
| Select line | Triple-click | Selects entire line |
| Select block | Alt/Option + drag | Rectangular selection |
| Copy selection | Release mouse | Auto-copies selection (configurable) |
| Paste | Middle-click | Pastes primary selection |
| Paste | Right-click → Paste | Context menu |
| Copy (context) | Right-click → Copy | Context menu |
| Extend selection | Shift + click | Extend from anchor point |

### Selection Behaviors

| Behavior | Description | Configurable |
|----------|-------------|--------------|
| Auto-copy on select | Automatically copy selected text to clipboard | Yes |
| Copy includes newline | Include trailing newline when copying line | Yes |
| Trailing whitespace | Trim or preserve trailing whitespace | Yes |
| Bracketed paste | Paste as bracketed paste mode | Yes |
| Multiline warning | Warn when pasting multiline content | Yes |

### Clipboard History (Post-v1)

| Feature | Description |
|---------|-------------|
| Ring buffer | Last N copies stored |
| Search | Fuzzy search through clipboard history |
| Pinning | Pin frequently used clips |

---

## Navigation

### Keyboard Navigation

| Action | Default | Alternate |
|--------|---------|-----------|
| Scroll up | Shift+PgUp | Ctrl+Shift+Up |
| Scroll down | Shift+PgDn | Ctrl+Shift+Down |
| Scroll to top | Shift+Home | Ctrl+Home |
| Scroll to bottom | Shift+End | Ctrl+End |
| Next prompt | Ctrl+Shift+Down | Ctrl+Down (vi mode) |
| Previous prompt | Ctrl+Shift+Up | Ctrl+Up (vi mode) |
| Next tab | Ctrl+Tab | Ctrl+PgDn |
| Previous tab | Ctrl+Shift+Tab | Ctrl+PgUp |
| Go to tab N | Ctrl+1-9 | Alt+1-9 |

### Mouse Navigation

| Action | Method |
|--------|--------|
| Scroll | Mouse wheel |
| Scroll faster | Ctrl + wheel |
| Click to focus | Click pane/tab |
| Drag divider | Resize panes |
| Tab bar | Click to switch |

---

## Speed Optimizations

### Instant Actions

These should feel instantaneous (< 16ms):

| Action | Target |
|--------|--------|
| Type to screen | < 4ms |
| Scroll | < 16ms per frame |
| Tab switch | < 50ms |
| New tab | < 100ms |
| Search start | < 50ms to first result |
| Copy | < 10ms |
| Paste | < 10ms |

### Perceived Speed

| Technique | Implementation |
|-----------|---------------|
| Predictive echo | Show typed chars before PTY echo (SSH) |
| Optimistic UI | Update UI before operation completes |
| Background loading | Tabs load in background, ready when clicked |
| Instant feedback | Visual/auditory confirmation immediately |

### Workflow Shortcuts

| Action | Shortcut | Behavior |
|--------|----------|----------|
| Smart paste | Ctrl+Shift+V | Detects URLs, paths, commands |
| Quick open | Ctrl+Click URL | Opens in default browser |
| Quick edit | Ctrl+E | Edit current command in $EDITOR |
| Repeat command | Ctrl+R, Enter | Re-run last command |
| Clear scrollback | Ctrl+L (shell) / Ctrl+Shift+K (terminal) | Clear buffer |

---

## Accessibility

| Feature | Description |
|---------|-------------|
| High contrast | High contrast color schemes |
| Screen reader | Announce cell content, selection, prompts |
| Keyboard only | All functions accessible via keyboard |
| Focus indicators | Clear visual focus for keyboard navigation |
| Font scaling | Scale fonts independently of DPI |
| Reduced motion | Disable animations for motion sensitivity |
| Color blind modes | Alternative color schemes for color blindness |

---

## Discoverability

### First Run Experience

| Element | Purpose |
|---------|---------|
| Welcome modal | Quick overview of key features |
| Keyboard cheat sheet | Modal with common shortcuts (Ctrl+? to open) |
| Configuration hints | Tooltips on hover for settings |
| Examples | Sample connections, profiles |

### Inline Help

| Feature | How |
|---------|-----|
| Shortcuts | Show shortcut in context menu |
| Command palette | Searchable commands with descriptions |
| Status bar | Show current mode, key hints |
| Tooltips | Hover for explanations |

### Progressive Learning

| Level | Features |
|-------|----------|
| Level1 | Basic terminal, copy/paste, tabs |
| Level 2 | Splits, profiles, SSH |
| Level 3 | Keybindings, themes, advanced search |
| Level 4 | Snippets, macros, scripting |

---

## Core Terminal Features

### Performance

| Feature | Description | Priority |
|---------|-------------|----------|
| GPU Rendering | wgpu-based rendering (Metal/Vulkan/DX12) | Phase 2 |
| Sub-4ms Input Latency | Target < 4ms key-to-screen | Phase 2 |
| 60fps Scrolling | Smooth scrolling with 10M+ line scrollback | Phase 2 |
| Damage Tracking | Only render changed regions | Phase 0 |
| Zero-Copy Parsing | Direct PTY read to grid, no intermediate buffers | Phase 0 |
| Fixed-Size Ring Buffer | Pre-allocated scrollback, no allocation in hot path | Phase 0 |

### Text Rendering

| Feature | Description | Priority |
|---------|-------------|----------|
| True Color (24-bit) | Full 16M color support | Phase 1 |
| Deep Color | Support for >24-bit color pipelines (future-proof) | Post-v1 |
| Wide Color Gamut | Display-P3, Rec.2020 color spaces | Post-v1 |
| Unicode & Grapheme Clusters | Correct handling of multi-codepoint emojis, combining marks | Phase 1 |
| Ligatures | Programming ligatures (Fira Code, JetBrains Mono) | Phase 2 |
| Bidirectional Text | RTL language support (Arabic, Hebrew) | Phase 5 |
| Font Fallback Chain | Automatic fallback for missing glyphs | Phase 5 |
| High DPI | Per-monitor DPI scaling | Phase 5 |

### Keybinds and Shortcuts

Fully customizable keyboard shortcuts for all actions.

| Feature | Description | Priority |
|---------|-------------|----------|
| Custom Keybindings | Remap any action to any key | Phase 2 |
| Multi-Chord Shortcuts | Multi-key sequences (e.g., Ctrl+K, Ctrl+C) | Phase 2 |
| Leader Key | Leader key prefix for shortcuts (like tmux/vim) | Phase 6 |
| Shortcut Profiles | Save and switch shortcut sets | Post-v1 |
| Platform Defaults | Sensible defaults per platform (Mac vs Windows/Linux) | Phase 2 |
| Shortcut Conflicts | Warn and resolve conflicting shortcuts | Phase 2 |
| Import/Export | Import/export keybinding configurations | Post-v1 |

#### Default Shortcuts (Customizable)

| Action | Windows/Linux | macOS |
|--------|---------------|-------|
| New Tab | Ctrl+Shift+T | Cmd+T |
| Close Tab | Ctrl+Shift+W | Cmd+W |
| New Split Horizontal | Ctrl+Shift+D | Cmd+D |
| New Split Vertical | Ctrl+Shift+E | Cmd+Shift+D |
| Navigate Splits | Alt+Arrow | Cmd+Option+Arrow |
| Zoom Split | Ctrl+Shift+Enter | Cmd+Shift+Enter |
| Find | Ctrl+Shift+F | Cmd+F |
| Copy | Ctrl+Shift+C | Cmd+C |
| Paste | Ctrl+Shift+V | Cmd+V |
| Clear Scrollback | Ctrl+Shift+K | Cmd+K |
| Search History | Ctrl+R | Cmd+R |
| Next/Previous Tab | Ctrl+Tab / Ctrl+Shift+Tab | Cmd+Shift+[ / Cmd+Shift+] |
| Split Focus | Alt+1-9 | Cmd+Option+1-9 |

### Interaction

| Feature | Description | Priority |
|---------|-------------|----------|
| Mouse Selection | Character, line, and block selection | Phase 3 |
| Multi-Click Selection | Double/triple click for word/line | Phase 3 |
| Bracketed Paste | Proper paste handling with warning for multiline | Phase 1 |
| Copy/Paste | Clipboard integration with format options | Phase 3 |
| Search | Forward/backward search in scrollback with highlighting | Phase 4 |
| Hyperlinks | OSC 8 clickable links with hover preview | Phase 1 |

### Shell Integration (OSC 133)

| Feature | Description | Priority |
|---------|-------------|----------|
| Prompt Detection | Mark prompts for semantic understanding | Phase 2 |
| Jump to Prompt | Navigate between command prompts | Phase 2 |
| Copy Command Output | Select entire command output with single action | Phase 2 |
| Command Finished Notification | Notify when long-running command completes | Phase 2 |
| Shell Integration Injection | Auto-configure bash/zsh/fish/pwsh | Phase 2 |

---

## Connection Management (Standalone Mode)

Iris standalone shares connection management with Hermes for a consistent experience.

### Connection Types

| Type | Description | Priority |
|------|-------------|----------|
| Local Shell | Native shell (bash, zsh, fish, pwsh, cmd) | Phase 0 |
| SSH | Secure remote connections | Phase 6 |
| Serial | Serial port connections (embedded development) | Post-v1 |
| WSL | Windows Subsystem for Linux integration | Phase 6 |

### SSH Features

| Feature | Description | Priority |
|---------|-------------|----------|
| Connection Profiles | Save and manage SSH connections | Phase 6 |
| Jump Hosts | Multi-hop SSH through bastion hosts | Phase 6 |
| Port Forwarding | Local, remote, and dynamic forwarding | Phase 6 |
| X11 Forwarding | GUI application forwarding | Post-v1 |
| Agent Forwarding | SSH agent forwarding with Pageant/OpenSSH Agent | Phase 6 |
| SFTP Integration | File transfer within terminal | Post-v1 |
| Password Manager | Encrypted storage for SSH secrets | Phase 6 |
| Login Scripts | Execute scripts on connection | Phase 6 |

### SSH Tunneling

| Feature | Description | Priority |
|---------|-------------|----------|
| Local Port Forward | Forward local port to remote server (`-L`) | Phase 6 |
| Remote Port Forward | Forward remote port to local (`-R`) | Phase 6 |
| Dynamic Port Forward | SOCKS proxy (`-D`) | Post-v1 |
| Tunnel Management UI | Visual management of active tunnels | Phase 6 |
| Tunnel Profiles | Save common tunnel configurations | Phase 6 |

### Serial Port Features

| Feature | Description | Priority |
|---------|-------------|----------|
| Port Selection | Auto-detect and select serial ports | Post-v1 |
| Baud Rate | Standard and custom baud rates | Post-v1 |
| Data/Stop/Parity Bits | Full serial configuration | Post-v1 |
| Flow Control | RTS/CTS, XON/XOFF | Post-v1 |
| Line Endings | CR, LF, CRLF conversion | Post-v1 |
| Hex Output | Display raw bytes as hex | Post-v1 |
| Auto-Reconnect | Reconnect on disconnect | Post-v1 |

---

## Session Management

### Tabs and Panes (Standalone Mode)

| Feature | Description | Priority |
|---------|-------------|----------|
| Tabs | Multiple terminal tabs | Phase 6 |
| Split Panes | Horizontal and vertical splits | Phase 6 |
| Nested Splits | Arbitrarily nested panes | Phase 6 |
| Session Restore | Restore tabs/panes on restart | Phase 6 |
| Tab Positioning | Tabs on any window edge | Post-v1 |

### Smart Tabs

Smart tabs automatically route connections to existing tabs based on context, reducing duplicates and improving workflow.

| Feature | Description | Priority |
|---------|-------------|----------|
| Auto-Detect Existing | Detect if connection already open, focus existing tab | Phase 6 |
| Profile Matching | Match by hostname, user, port combination | Phase 6 |
| Working Directory Match | Match local tabs by current directory | Phase 6 |
| Smart Reconnect | Reconnect to existing session instead of new shell | Phase 6 |
| Tab Naming | Auto-name tabs based on connection/host/directory | Phase 6 |
| Tab Groups | Group related tabs (e.g., all tabs for a project) | Post-v1 |
| Pinning | Pin important tabs to prevent accidental close | Phase 6 |

### Persistent History

Command history that persists across sessions, with intelligent search and recall.

| Feature | Description | Priority |
|---------|-------------|----------|
| Cross-Session History | Commands saved between sessions | Phase 4 |
| Synchronized History | History synced across tabs (optional) | Phase 6 |
| History Search | Fuzzy search through command history | Phase 4 |
| History by Directory | Per-directory command history | Phase 6 |
| History by Host | Separate SSH history per host | Phase 6 |
| History Exclusions | Exclude sensitive commands from history | Phase 4 |
| History Timestamps | Record when commands were executed | Phase 4 |
| History Export | Export history to file | Post-v1 |
| History Analytics | Most-used commands, frequent directories | Post-v1 |

### Split Management

| Feature | Description | Priority |
|---------|-------------|----------|
| Split Navigation | Move between panes with keyboard | Phase 6 |
| Split Resize | Resize panes with keyboard shortcuts | Phase 6 |
| Split Focus Remember | Remember which pane had focus | Phase 6 |
| Zoom Split | Temporarily maximize a pane | Phase 6 |
| Split Profiles | Save and restore split layouts | Phase 6 |
| Even Splits | Distribute panes evenly | Phase 6 |
| Split Labels | Label panes for quick navigation | Post-v1 |

### Quake Mode (Standalone Mode)

| Feature | Description | Priority |
|---------|-------------|----------|
| Dropdown Terminal | Terminal drops from screen edge | Post-v1 |
| Global Hotkey | Toggle terminal visibility system-wide | Post-v1 |
| Always on Top | Float above other windows | Post-v1 |

---

## Display and Theming

| Feature | Description | Priority |
|---------|-------------|----------|
| Color Schemes | Built-in themes + custom scheme support | Phase 2 |
| Font Selection | Custom fonts with fallback chain | Phase 2 |
| Font Ligatures | Toggle programming ligatures | Phase 2 |
| Background Opacity | Translucent terminal background | Phase 2 |
| Background Image | Custom background images | Post-v1 |
| Padding/Margins | Adjustable window padding | Phase 2 |
| Cursor Styles | Block, underline, bar; blinking options | Phase 2 |

---

## Security

| Feature | Description | Priority |
|---------|-------------|----------|
| Encrypted Secrets | AES-256 encrypted password storage | Phase 6 |
| Master Password | Optional master password for secret store | Phase 6 |
| SSH Key Management | Import and manage SSH keys | Phase 6 |
| Known Hosts | Manage SSH known hosts with fingerprint verification | Phase 6 |
| Bracketed Paste Warning | Warn on multiline paste | Phase 1 |
| URL Security | Show full URL before opening OSC 8 links | Phase 1 |

---

## Productivity

| Feature | Description | Priority |
|---------|-------------|----------|
| Command Palette | Quick access to all commands (Ctrl+Shift+P) | Post-v1 |
| Profiles | Save terminal configurations | Phase 6 |
| Snippets | Save and insert frequently used commands | Post-v1 |
| Working Directory | Remember or specify startup directory | Phase 6 |
| Custom Shell Args | Pass arguments to shell on startup | Phase 6 |
| Window Transparency | Adjustable window opacity | Phase 2 |

---

## Integrated with Hermes (Embedded Mode)

These features come from Hermes when Iris is embedded:

| Feature | Provided By |
|---------|-------------|
| SSH Tunnel Management | Hermes |
| Connection Profiles | Hermes |
| Key Management | Hermes |
| Workspace Tabs | Hermes |
| Multi-pane Workspace | Hermes |
| Configuration Sync | Hermes |

---

## Inspector and Debugging (Inspired by Ghostty)

| Feature | Description | Priority |
|---------|-------------|----------|
| Cell Inspector | Show cell attributes under cursor | Post-v1 |
| Color Palette | Display active 16/256/true color palette | Post-v1 |
| OSC 133 Overlay | Visualize prompt boundaries | Phase 2 |
| Protocol Monitor | View raw escape sequences | Post-v1 |
| Performance Metrics | FPS, input latency, memory usage | Post-v1 |
| Font Metrics | Show loaded fonts and fallbacks | Post-v1 |

---

## Platform-Specific Features

### Windows

| Feature | Description | Priority |
|---------|-------------|----------|
| ConPTY Integration | Windows pseudoconsole | Phase 5 |
| WSL Integration | Direct WSL shell launch | Phase 6 |
| PowerShell Support | Full PowerShell and pwsh support | Phase 0 |
| Windows Context Menu | "Open with Iris" in Explorer | Phase 6 |
| Jump Lists | Recent connections in taskbar | Post-v1 |

### macOS

| Feature | Description | Priority |
|---------|-------------|----------|
| Metal Rendering | Native GPU via Metal | Phase 2 |
| Touch Bar | Customizable touch bar shortcuts | Post-v1 |
| Quick Terminal | Dropdown from menu bar | Post-v1 |
| Services Integration | macOS Services menu | Post-v1 |

### Linux

| Feature | Description | Priority |
|---------|-------------|----------|
| GTK Integration | Native GTK4 chrome | Post-v1 |
| Desktop Entry | .desktop file for application menu | Phase 6 |
| Terminal Emulation | Proper TERM and terminfo | Phase 1 |

---

## Protocol Support

### Standard Protocols

| Protocol | Description | Priority |
|----------|-------------|----------|
| ANSI/VT100 | Basic escape sequences | Phase 1 |
| VT220 | Extended VT features | Phase 1 |
| VT420 | Advanced VT features | Phase 1 |
| XTerm | XTerm extensions | Phase 1 |
| ECMA-48 | Standard control functions | Phase 1 |

### Modern Extensions

| Protocol | Description | Priority |
|----------|-------------|----------|
| OSC 8 | Hyperlinks | Phase 1 |
| OSC 133 | Shell integration markers | Phase 2 |
| OSC 52 | Clipboard operations | Phase 3 |
| OSC 9/777 | Desktop notifications | Phase 2 |
| DECSET 2026 | Synchronized output | Phase 1 |
| Kitty Graphics | Inline images | Phase 4 |

### Keyboards Protocols

| Protocol | Description | Priority |
|----------|-------------|----------|
| Kitty Keyboard | Extended key events | Phase 5 |
| FixTerms Keyboard | Legacy key handling | Phase 1 |

---

## Configuration

### Configuration File

```toml
# ~/.config/iris/config.toml

[theme]
name = "dracula"
font = "JetBrains Mono"
font_size = 14
line_height = 1.2

[shell]
program = "/bin/zsh"
args = ["-l"]

[scrollback]
limit = 10000

[ssh]
# SSH profiles defined here
[[ssh.profiles]]
name = "production"
host = "prod.example.com"
user = "admin"
port = 22

[shortcuts]
# Custom keybindings
```

### Command Line

```bash
iris                          # Open new terminal window
iris -e "command"             # Run command
iris --profile "production"    # Use saved profile
iris --ssh user@host          # SSH connection
iris --serial /dev/ttyUSB0    # Serial connection
iris --help                   # Show help
```

---

## Comparison with Other Terminals

| Feature | Iris | Ghostty | Alacritty | WezTerm | Tabby |
|---------|------|---------|-----------|---------|-------|
| Windows Native | ✅ | ❌ | ✅ | ✅ | ✅ |
| macOS Native | ✅ | ✅ | ✅ | ✅ | ✅ |
| Linux Native | ✅ | ✅ | ✅ | ✅ | ✅ |
| Embedded Mode | ✅ | ❌ | ❌ | ❌ | ❌ |
| SSH Client | ✅ | ❌ | ❌ | ❌ | ✅ |
| Serial Port | ✅ | ❌ | ❌ | ❌ | ✅ |
| Metal Rendering | ✅ | ✅ | ❌ | ✅ | ❌ |
| <4ms Latency | Target | ✅ | ✅ | ✅ | ❓ |
| Built-in Multiplexer | Standalone | ✅ | ❌ | ✅ | ✅ |
| Ligatures | ✅ | ✅ | ✅ | ✅ | ✅ |
| Smart Tabs | ✅ | ❌ | ❌ | ❌ | ✅ |
| Persistent History | ✅ | ❌ | ❌ | ❌ | ✅ |
| Custom Keybinds | ✅ | ✅ | ✅ | ✅ | ✅ |
| 24-bit Color | ✅ | ✅ | ✅ | ✅ | ✅ |
| Deep Color Pipeline | Planned | ❌ | ❌ | ❌ | ❌ |

---

## Feature Phases Summary

| Phase | Focus | Key Features |
|-------|-------|---------------|
| 0 | Foundation | Grid, parser, PTY, Windows support |
| 1 | Core Parser | ANSI/VT, OSC 8, DECSET 2026 |
| 2 | Renderer | GPU rendering, themes, fonts, OSC 133 |
| 3 | Selection | Mouse selection, clipboard, OSC 52 |
| 4 | Scrollback | Ring buffer, search, Kitty graphics |
| 5 | Platform | IME, high DPI, keyboard protocols |
| 6 | Standalone | SSH, serial, tabs, profiles, sessions |
| Post-v1 | Polish | Quake mode, inspector, bidi, X11 forward |