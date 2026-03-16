# Iris Logging Strategy

Logging, tracing, and metrics approach.

## Philosophy

### Logging Levels

| Level | Purpose | Example |
|-------|---------|---------|
| ERROR | Application cannot continue | Fatal error, crash |
| WARN | Unexpected but recoverable | Fallback used, retry needed |
| INFO | Important lifecycle events | Process started, connected |
| DEBUG | Debug information | Internal state, decisions |
| TRACE | Detailed tracing | Every function call, data flow |

### Rules

1. **No secrets in logs** - Passwords, keys, tokens never logged
2. **Structured logging** - Use `tracing` spans and fields
3. **Performance first** - Logging should not impact performance
4. **Log for debugging** - Logs should help diagnose issues

---

## Crate: `tracing`

Use `tracing` for all logging:

```toml
[dependencies]
tracing = "0.1"
```

### Basic Usage

```rust
use tracing::{info, warn, error, debug, trace};

// Simple message
info!("Terminal started");

// With fields
info!(cols = 80, rows = 24, "Grid initialized");

// Structured span
#[tracing::instrument]
fn parse_byte(&mut self, byte: u8) {
    trace!(byte = format!("{:#02x}", byte), "Parsing byte");
    // ...
}
```

---

## Structured Logging

### Spans

```rust
use tracing::{info_span, instrument};

// Create span
let span = info_span!("terminal", id = %terminal.id());
let _enter = span.enter();

// Or use instrument
#[instrument(skip(self))]
pub fn spawn(&mut self, config: &PtyConfig) -> Result<PtyHandle, PtyError> {
    debug!(command = %config.command, "Spawning process");
    // ...
}
```

### Fields

```rust
// Structured fields
info!(
    terminal.id = %id,
    cols = grid.cols,
    rows = grid.rows,
    "Terminal created"
);

// Nested objects
debug!(
    cursor = ?self.cursor,
    "Cursor state"
);
```

---

## Logging Points

### Startup

```rust
info!(
    version = env!("CARGO_PKG_VERSION"),
    platform = ?Platform::current(),
    "Iris starting"
);

debug!(
    config = ?self.config,
    "Configuration loaded"
);
```

### PTY Events

```rust
#[instrument(skip(self))]
pub fn spawn(&mut self, config: &PtyConfig) -> Result<PtyHandle, PtyError> {
    info!(
        command = %config.command,
        args = ?config.args,
        "Spawning PTY process"
    );
    
    let handle = self.backend.spawn(config)?;
    
    info!(
        pid = ?handle.pid(),
        "Process spawned"
    );
    
    Ok(handle)
}

pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, PtyError> {
    let n = self.backend.read(buf)?;
    
    trace!(
        bytes = n,
        "PTY read"
    );
    
    Ok(n)
}
```

### Parser Events

```rust
#[instrument(skip(self, data))]
pub fn parse(&mut self, data: &[u8]) {
    trace!(
        len = data.len(),
        "Parsing input"
    );
    
    for byte in data {
        self.parse_byte(*byte);
    }
}

fn handle_csi(&mut self, params: &[i64], final: u8) {
    debug!(
        params = ?params,
        final = format!("{:#02x}", final),
        "CSI sequence"
    );
}
```

### Grid Events

```rust
#[instrument(skip(self))]
pub fn resize(&mut self, cols: usize, rows: usize) {
    info!(
        old_cols = self.cols,
        old_rows = self.rows,
        new_cols = cols,
        new_rows = rows,
        "Grid resize"
    );
    
    // Resize logic...
}
```

### Render Events

```rust
pub fn render(&mut self, grid: &Grid) -> Result<(), RenderError> {
    let damage = grid.take_damage();
    
    trace!(
        damaged_rows = damage.len(),
        "Rendering damaged regions"
    );
    
    // Render logic...
    
    debug!(
        frame_time = ?elapsed,
        "Frame rendered"
    );
}
```

---

## Performance

### Avoid Hot Path Logging

```rust
// BAD: Logs every byte in hot path
fn parse_byte(&mut self, byte: u8) {
    trace!(byte = format!("{:#02x}", byte)); // TOO VERBOSE
}

// GOOD: Sample logging in hot path
fn parse(&mut self, data: &[u8]) {
    trace!(len = data.len(), "Parsing input"); // Log once
    
    if self.sample_counter % 1000 == 0 {
        debug!(
            parsed_bytes = self.parsed_bytes,
            "Parser progress"
        );
    }
    self.sample_counter += 1;
}
```

### Level Guard

```rust
// Expensive operations only at debug/trace
if tracing::enabled!(tracing::Level::DEBUG) {
    let state = self.expensive_debug_info();
    debug!(state = ?state, "Detailed state");
}
```

---

## Configuration

### Environment Variables

```bash
# Log level
RUST_LOG=iris_core=debug,iris_platform=info

# Log format
RUST_LOG_FORMAT=json

# Log file
IRIS_LOG_FILE=/var/log/iris.log
```

### Runtime Configuration

```rust
pub fn init_logging(config: &LoggingConfig) {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(config.level)
        .with_target(config.show_target)
        .with_thread_ids(config.show_thread_ids)
        .with_file(config.show_file)
        .with_line_number(config.show_line_numbers);
    
    if config.json_format {
        subscriber.json().init();
    } else {
        subscriber.init();
    }
}
```

---

## Log Format

### Default Format

```
2024-03-16T10:30:00.123Z INFO iris_core::terminal: Terminal started cols=80 rows=24
2024-03-16T10:30:00.456Z DEBUG iris_platform::pty: Process spawned command="/bin/zsh" pid=12345
```

### JSON Format

```json
{
  "timestamp": "2024-03-16T10:30:00.123Z",
  "level": "INFO",
  "target": "iris_core::terminal",
  "message": "Terminal started",
  "fields": {
    "cols": 80,
    "rows": 24
  }
}
```

---

## Metrics

### Counters

```rust
use tracing::Counter;

lazy_static! {
    static ref BYTES_PARSED: Counter = Counter::new("iris_bytes_parsed");
    static ref FRAMES_RENDERED: Counter = Counter::new("iris_frames_rendered");
}

pub fn parse(&mut self, data: &[u8]) {
    BYTES_PARSED.increment(data.len() as u64);
    // ...
}
```

### Histograms

```rust
use tracing::Histogram;

lazy_static! {
    static ref PARSE_LATENCY: Histogram = Histogram::new("iris_parse_latency_ms");
    static ref RENDER_LATENCY: Histogram = Histogram::new("iris_render_latency_ms");
}

pub fn render(&mut self, grid: &Grid) {
    let start = std::time::Instant::now();
    // ... render ...
    RENDER_LATENCY.record(start.elapsed().as_millis() as u64);
}
```

---

## Integration

### With Sentry

```rust
// Optional: Sentry integration
#[cfg(feature = "sentry")]
pub fn init_sentry(dsn: &str) {
    let _guard = sentry::init((
        dsn,
        sentry::ClientOptions {
            release: Some(env!("CARGO_PKG_VERSION").into()),
            ..Default::default()
        },
    ));
    
    tracing_subscriber::fmt()
        .with_env_filter()
        .finish();
}
```

### With file logging

```rust
pub fn init_file_logging(path: &Path) -> Result<(), std::io::Error> {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    
    tracing_subscriber::fmt()
        .with_writer(file)
        .with_max_level(tracing::Level::DEBUG)
        .init();
    
    Ok(())
}
```

---

## Debugging

### Enable Verbose Logging

```bash
# Debug level for all crates
RUST_LOG=debug iris

# Trace level for specific crate
RUST_LOG=iris_core=trace,iris_platform=debug iris

# Multiple modules
RUST_LOG=iris_core::parser=trace,iris_render::glyph=debug iris
```

### Log File Location

| Platform | Location |
|----------|----------|
| Windows | `%APPDATA%\iris\logs\iris.log` |
| macOS | `~/Library/Logs/iris/iris.log` |
| Linux | `~/.local/share/iris/logs/iris.log` |

---

## Security

### Never Log

- Passwords
- SSH keys
- Session tokens
- API keys
- Credit card numbers
- Personal data (PII)

### Sanitize

```rust
fn sanitize_command(cmd: &str) -> String {
    // Remove potential passwords from URLs
    cmd.replace_regex(r"://([^:]+):([^@]+)@", "://$1:****@")
}

fn sanitize_env(env: &HashMap<String, String>) -> HashMap<String, String> {
    env.iter()
        .map(|(k, v)| {
            if k.contains("PASSWORD") || k.contains("TOKEN") || k.contains("SECRET") {
                (k.clone(), "****".to_string())
            } else {
                (k.clone(), v.clone())
            }
        })
        .collect()
}
```

---

## Testing

### Verify Logging

```rust
#[test]
fn logs_terminal_start() {
    use tracing_subscriber::layer::SubscriberExt;
    
    let (layer, handle) = tracing_subscriber::layer::with(|ctx| {
        // Collect log records
    });
    
    tracing::subscriber::with_default(layer, || {
        let terminal = Terminal::new(80, 24).unwrap();
        // Verify info log was created
    });
}
```