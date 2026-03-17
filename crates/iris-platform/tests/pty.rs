use std::time::{Duration, Instant};

use iris_platform::platform::PlatformPtyBackend;
use iris_platform::{PtyBackend, PtyConfig};

/// Verifies that spawning a PTY produces an initial output containing the sentinel marker.
///
/// Spawns a `PlatformPtyBackend` with a configuration that emits `__IRIS_PTY__` and asserts the captured output contains that marker.
///
/// # Examples
///
/// ```
/// let mut backend = PlatformPtyBackend::default();
/// let config = spawn_output_config();
/// backend.spawn(&config).unwrap();
/// let output = read_until_contains(&mut backend, "__IRIS_PTY__", std::time::Duration::from_secs(5));
/// assert!(output.contains("__IRIS_PTY__"));
/// ```
#[test]
fn pty_spawn_returns_handle() {
    let mut backend = PlatformPtyBackend::default();
    let config = spawn_output_config();

    backend.spawn(&config).unwrap();

    let output = read_until_contains(&mut backend, "__IRIS_PTY__", Duration::from_secs(5));
    assert!(output.contains("__IRIS_PTY__"));
}

/// Verifies that data written to a PTY is echoed back by the spawned process.
///
/// # Examples
///
/// ```
/// let mut backend = PlatformPtyBackend::default();
/// let config = round_trip_config();
/// backend.spawn(&config).unwrap();
/// backend.write(input_payload().as_bytes()).unwrap();
/// let output = read_until_contains(&mut backend, "__IRIS_ECHO__", std::time::Duration::from_secs(5));
/// assert!(output.contains("__IRIS_ECHO__hello-from-iris"));
/// ```
#[test]
fn pty_read_write_works() {
    let mut backend = PlatformPtyBackend::default();
    let config = round_trip_config();

    backend.spawn(&config).unwrap();
    backend.write(input_payload().as_bytes()).unwrap();

    let output = read_until_contains(&mut backend, "__IRIS_ECHO__", Duration::from_secs(5));
    assert!(output.contains("__IRIS_ECHO__hello-from-iris"));
}

/// Reads from a PTY backend until the accumulated output contains `needle` or the `timeout` elapses.
///
/// The function continuously reads available bytes from `backend`, accumulates them, and returns as soon as the accumulated text contains `needle`. If the backend closes before the needle is seen, or the timeout is reached, the function returns the accumulated output up to that point. The returned string is produced using UTF-8 lossless conversion with replacement for invalid sequences.
///
/// # Examples
///
/// ```no_run
/// use std::time::Duration;
/// // `backend` should be a test or platform-specific PtyBackend implementation.
/// // let mut backend: Box<dyn iris_platform::PtyBackend> = ...;
/// // let out = read_until_contains(&mut *backend, "__IRIS_PTY__", Duration::from_secs(5));
/// // assert!(out.contains("__IRIS_PTY__"));
/// ```
fn read_until_contains(backend: &mut dyn PtyBackend, needle: &str, timeout: Duration) -> String {
    let start = Instant::now();
    let mut output = Vec::new();
    let mut buffer = [0_u8; 4096];

    while start.elapsed() < timeout {
        let read = backend.read(&mut buffer).unwrap();
        if read == 0 {
            if !backend.is_alive().unwrap() {
                break;
            }
            continue;
        }

        output.extend_from_slice(&buffer[..read]);
        let text = String::from_utf8_lossy(&output).to_string();
        if text.contains(needle) {
            return text;
        }
    }

    String::from_utf8_lossy(&output).to_string()
}

/// Create a PTY configuration that runs the system shell to print the sentinel `__IRIS_PTY__`.
///
/// This Windows-specific helper produces a `PtyConfig` whose command is the system shell and
/// whose arguments instruct the shell to execute `echo __IRIS_PTY__`.
///
/// # Examples
///
/// ```
/// let cfg = spawn_output_config();
/// assert_eq!(cfg.args[0], "/C");
/// assert!(cfg.args[1].contains("__IRIS_PTY__"));
/// ```
#[cfg(target_os = "windows")]
fn spawn_output_config() -> PtyConfig {
    let mut config = PtyConfig::new(shell_command());
    config.args = vec!["/C".into(), "echo __IRIS_PTY__".into()];
    config
}

/// Create a `PtyConfig` that launches the system shell to print the sentinel `__IRIS_PTY__`.
///
/// # Examples
///
/// ```
/// let cfg = spawn_output_config();
/// // cfg will run the default shell with arguments that print "__IRIS_PTY__\n"
/// ```
#[cfg(not(target_os = "windows"))]
fn spawn_output_config() -> PtyConfig {
    let mut config = PtyConfig::new(shell_command());
    config.args = vec!["-lc".into(), "printf '__IRIS_PTY__\\n'".into()];
    config
}

/// Builds a PTY configuration that runs PowerShell and echoes a single input line prefixed with `__IRIS_ECHO__`.
///
/// The returned `PtyConfig` will invoke `powershell.exe` with arguments that read one line from stdin
/// and write it back prefixed by the marker.
///
/// # Examples
///
/// ```
/// let cfg = round_trip_config();
/// assert!(cfg.program.ends_with("powershell.exe") || cfg.program == "powershell.exe");
/// assert!(cfg.args.iter().any(|a| a.contains("__IRIS_ECHO__") || a.contains("-NoLogo")));
/// ```
#[cfg(target_os = "windows")]
fn round_trip_config() -> PtyConfig {
    let mut config = PtyConfig::new("powershell.exe");
    config.args = vec![
        "-NoLogo".into(),
        "-NoProfile".into(),
        "-Command".into(),
        "$line = [Console]::In.ReadLine(); Write-Output \"__IRIS_ECHO__$line\"".into(),
    ];
    config
}

/// Builds a PtyConfig that reads a single line from stdin and echoes it prefixed with `__IRIS_ECHO__` using the system shell.
///
/// # Returns
///
/// A `PtyConfig` configured to invoke the shell with arguments that read one line and print it prefixed with `__IRIS_ECHO__`.
///
/// # Examples
///
/// ```
/// let cfg = round_trip_config();
/// assert!(cfg.args.iter().any(|a| a == "-lc"));
/// assert!(cfg.args.iter().any(|a| a.contains("__IRIS_ECHO__")));
/// ```
#[cfg(not(target_os = "windows"))]
fn round_trip_config() -> PtyConfig {
    let mut config = PtyConfig::new(shell_command());
    config.args = vec![
        "-lc".into(),
        "IFS= read -r line; printf '__IRIS_ECHO__%s\\n' \"$line\"".into(),
    ];
    config
}

/// Windows-specific input payload including a CRLF line ending.
///
/// Returns the payload string terminated with `\r\n`.
///
/// # Examples
///
/// ```
/// assert_eq!(input_payload(), "hello-from-iris\r\n");
/// ```
#[cfg(target_os = "windows")]
fn input_payload() -> &'static str {
    "hello-from-iris\r\n"
}

/// Payload sent to the spawned PTY for round-trip tests.
///
/// The string is terminated with a newline to simulate an entered line.
///
/// # Examples
///
/// ```
/// assert_eq!(input_payload(), "hello-from-iris\n");
/// ```
#[cfg(not(target_os = "windows"))]
fn input_payload() -> &'static str {
    "hello-from-iris\n"
}

/// Resolve the system shell command used on Windows.
///
/// Prefers the `ComSpec` environment variable and falls back to `"cmd.exe"` if the variable is unset.
///
/// # Examples
///
/// ```
/// let cmd = shell_command();
/// assert!(!cmd.is_empty());
/// ```
#[cfg(target_os = "windows")]
fn shell_command() -> String {
    std::env::var("ComSpec").unwrap_or_else(|_| "cmd.exe".to_string())
}

/// Resolve the user's shell command for non-Windows platforms.
///
/// # Returns
///
/// The value of the `SHELL` environment variable if set, otherwise the string `"sh"`.
///
/// # Examples
///
/// ```
/// let cmd = shell_command();
/// assert!(!cmd.is_empty());
/// ```
fn
fn shell_command() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string())
}
