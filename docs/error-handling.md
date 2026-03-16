# Iris Error Handling

Error handling strategy across all crates.

## Philosophy

### Errors Are Values

Errors are not exceptions. They are values that must be handled explicitly.

### No Panics in Production

Panics are for programmer errors (bugs). All other failures return `Result`.

### Context Matters

Errors should contain enough context to diagnose the problem without debugging.

---

## Error Types

### Core Error Type

Every crate has its own error type with `thiserror`:

```rust
// iris-core/src/error.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Grid error: {0}")]
    Grid(#[from] GridError),
    
    #[error("Parser error: {0}")]
    Parser(#[from] ParserError),
    
    #[error("Terminal error: {0}")]
    Terminal(#[from] TerminalError),
}

#[derive(Debug, Error)]
pub enum GridError {
    #[error("Invalid position: ({col}, {row}) exceeds grid size ({cols}, {rows})")]
    InvalidPosition {
        col: usize,
        row: usize,
        cols: usize,
        rows: usize,
    },
    
    #[error("Resize failed: {reason}")]
    ResizeFailed { reason: String },
}

#[derive(Debug, Error)]
pub enum ParserError {
    #[error("Invalid escape sequence: {0:?}")]
    InvalidEscape(Vec<u8>),
    
    #[error("Unexpected byte: {0:#02x} in state {1:?}")]
    UnexpectedByte(u8, ParserState),
    
    #[error("Buffer overflow: {0} bytes")]
    BufferOverflow(usize),
}
```

### Platform Error Type

```rust
// iris-platform/src/error.rs
#[derive(Debug, Error)]
pub enum Error {
    #[error("PTY error: {0}")]
    Pty(#[from] PtyError),
    
    #[error("Clipboard error: {0}")]
    Clipboard(#[from] ClipboardError),
    
    #[error("Font error: {0}")]
    Font(#[from] FontError),
}

#[derive(Debug, Error)]
pub enum PtyError {
    #[error("Failed to spawn process: {command}")]
    SpawnFailed { command: String },
    
    #[error("Process exited unexpectedly: {code}")]
    ProcessExited { code: Option<i32> },
    
    #[error("PTY read failed: {reason}")]
    ReadFailed { reason: String },
    
    #[error("PTY write failed: {reason}")]
    WriteFailed { reason: String },
    
    #[error("PTY resize failed: {reason}")]
    ResizeFailed { reason: String },
}
```

### Render Error Type

```rust
// iris-render-wgpu/src/error.rs
#[derive(Debug, Error)]
pub enum Error {
    #[error("GPU error: {0}")]
    Gpu(#[from] GpuError),
    
    #[error("Font error: {0}")]
    Font(#[from] FontError),
}

#[derive(Debug, Error)]
pub enum GpuError {
    #[error("No suitable GPU adapter found")]
    NoAdapter,
    
    #[error("GPU device creation failed: {reason}")]
    DeviceCreationFailed { reason: String },
    
    #[error("Surface creation failed: {reason}")]
    SurfaceFailed { reason: String },
    
    #[error("Shader compilation failed: {reason}")]
    ShaderFailed { reason: String },
}
```

---

## Error Propagation

### Use `?` Operator

```rust
// GOOD: Use ? for propagation
fn parse_and_write(&mut self, data: &[u8]) -> Result<(), Error> {
    self.parser.parse(data)?;
    self.grid.write(self.parser.output())?;
    Ok(())
}

// BAD: Manually match
fn parse_and_write(&mut self, data: &[u8]) -> Result<(), Error> {
    match self.parser.parse(data) {
        Ok(_) => {},
        Err(e) => return Err(e),
    }
    // ...
}
```

### Add Context At Boundaries

```rust
// Add context when crossing crate boundaries
fn handle_pty_output(&mut self, data: &[u8]) -> Result<(), Error> {
    self.parser.parse(data)
        .map_err(|e| Error::Parser(e))?;
    Ok(())
}

// Use .context() with anyhow for applications
fn main() -> Result<()> {
    let terminal = Terminal::new(80, 24)
        .context("Failed to create terminal")?;
    Ok(())
}
```

---

## Error Messages

### User-Facing Messages

```rust
// GOOD: Actionable message
#[error("Failed to spawn '{command}': {reason}. Check that the command exists and is executable.")]
SpawnFailed { command: String, reason: String },

// BAD: Developer-facing message
#[error("spawn failed: {0}")]
SpawnFailed(String),
```

### Include Relevant Context

```rust
// Grid error with coordinates
#[error("Cannot write at ({col}, {row}): grid is {cols}x{rows}")]
InvalidPosition { col: usize, row: usize, cols: usize, rows: usize }

// PTY error with command
#[error("Failed to spawn '{command}': {error}")]
SpawnFailed { command: String, error: String }
```

---

## Recoverable vs Unrecoverable

### Recoverable Errors

Return `Result<T, E>`:

```rust
// Recoverable: file not found
fn load_config(path: &Path) -> Result<Config, ConfigError> {
    std::fs::read_to_string(path)
        .map_err(|e| ConfigError::ReadFailed { path: path.display().to_string(), error: e })?
    // ...
}
```

### Unrecoverable Errors

Use `panic!` (but prefer returning Result):

```rust
// Only panic for programmer errors
fn get_cell(&self, col: usize, row: usize) -> &Cell {
    // This should never happen if code is correct
    // Prefer returning Option instead
    assert!(col < self.cols && row < self.rows, "Invalid cell access");
    &self.cells[row * self.cols + col]
}

// BETTER: Return Option
fn get_cell(&self, col: usize, row: usize) -> Option<&Cell> {
    if col < self.cols && row < self.rows {
        Some(&self.cells[row * self.cols + col])
    } else {
        None
    }
}
```

---

## Error Handling Patterns

### Fallible Constructor

```rust
impl Terminal {
    pub fn new(cols: usize, rows: usize) -> Result<Self, Error> {
        if cols == 0 || rows == 0 {
            return Err(Error::InvalidGridSize { cols, rows });
        }
        Ok(Self {
            grid: Grid::new(cols, rows),
            // ...
        })
    }
}
```

### Builder Pattern

```rust
impl PtyConfig {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
            env: HashMap::new(),
            // ...
        }
    }
    
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }
    
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }
}

// Usage
let config = PtyConfig::new("/bin/bash")
    .arg("-l")
    .env("TERM", "xterm-256color");
```

### Default with Fallback

```rust
impl Config {
    pub fn font(&self) -> &str {
        self.font.as_deref()
            .or_else(|| std::env::var("IRIS_FONT").ok().as_deref())
            .unwrap_or("JetBrains Mono")
    }
}
```

---

## Logging Errors

### Use `tracing` Crate

```rust
use tracing::{error, warn, info, debug};

fn spawn_process(&mut self, config: &PtyConfig) -> Result<PtyHandle, PtyError> {
    info!(command = %config.command, "Spawning process");
    
    match self.backend.spawn(config) {
        Ok(handle) => {
            debug!(pid = ?handle.pid(), "Process spawned");
            Ok(handle)
        }
        Err(e) => {
            error!(error = %e, command = %config.command, "Failed to spawn process");
            Err(e)
        }
    }
}
```

### Log Levels

| Level | Usage |
|-------|-------|
| ERROR | Application cannot continue |
| WARN | Unexpected but recoverable |
| INFO | Important lifecycle events |
| DEBUG | Debug information |
| TRACE | Detailed tracing |

---

## Testing Errors

### Test Error Conditions

```rust
#[test]
fn invalid_grid_size_returns_error() {
    let result = Terminal::new(0, 24);
    assert!(result.is_err());
    assert!(matches!(result, Err(Error::InvalidGridSize { .. })));
}

#[test]
fn spawn_invalid_command_returns_error() {
    let config = PtyConfig::new("/nonexistent/command");
    let result = pty.spawn(&config);
    assert!(result.is_err());
}

#[test]
fn pty_read_after_exit_returns_error() {
    let mut pty = Pty::new();
    pty.spawn(&PtyConfig::new("exit")).unwrap();
    pty.wait().unwrap();
    
    let result = pty.read(&mut buf);
    assert!(result.is_err());
}
```

### Test Error Messages

```rust
#[test]
fn error_message_is_helpful() {
    let error = GridError::InvalidPosition {
        col: 100,
        row: 50,
        cols: 80,
        rows: 24,
    };
    
    let message = error.to_string();
    assert!(message.contains("100"));
    assert!(message.contains("50"));
    assert!(message.contains("80"));
    assert!(message.contains("24"));
}
```

---

## Security Considerations

### No Secrets in Errors

```rust
// BAD: Leaks password
#[error("Failed to connect to {host} with password {password}")]
ConnectionFailed { host: String, password: String }

// GOOD: Sanitized
#[error("Failed to connect to {host}")]
ConnectionFailed { host: String }
```

### Sanitize Paths

```rust
// BAD: Leaks home directory
#[error("Config not found: {path}")]
ConfigNotFound { path: PathBuf }

// GOOD: Redact sensitive parts
#[error("Config not found: {path}")]
ConfigNotFound { path: String }

fn format_path(path: &Path) -> String {
    // Redact home directory
    if let Some(home) = dirs::home_dir() {
        path.strip_prefix(&home)
            .map(|p| format!("~{}", p.display()))
            .unwrap_or_else(|_| path.display().to_string())
    } else {
        path.display().to_string()
    }
}
```

---

## Crate-Specific Guidelines

### iris-core

- All errors derive `thiserror::Error`
- No `anyhow` in public API
- `Error` type aliased per module
- Specific error variants

### iris-platform

- Platform-specific errors wrapped
- Include system error code
- Document platform differences

### iris-render-wgpu

- GPU errors must be recoverable
- Provide fallback message
- Log GPU adapter info on error

### iris-standalone

- Use `anyhow` for application errors
- Convert library errors to user-friendly messages
- Exit codes for different error types