use std::time::{Duration, Instant};

use iris_platform::platform::PlatformPtyBackend;
use iris_platform::{PtyBackend, PtyConfig};

#[test]
fn pty_spawn_returns_handle() {
    let mut backend = PlatformPtyBackend::default();
    let config = spawn_output_config();

    backend.spawn(&config).unwrap();

    let output = read_until_contains(&mut backend, "__IRIS_PTY__", Duration::from_secs(5));
    assert!(output.contains("__IRIS_PTY__"));
}

#[test]
fn pty_read_write_works() {
    let mut backend = PlatformPtyBackend::default();
    let config = round_trip_config();

    backend.spawn(&config).unwrap();
    backend.write(input_payload().as_bytes()).unwrap();

    let output = read_until_contains(&mut backend, "__IRIS_ECHO__", Duration::from_secs(5));
    assert!(output.contains("__IRIS_ECHO__hello-from-iris"));
}

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

#[cfg(target_os = "windows")]
fn spawn_output_config() -> PtyConfig {
    let mut config = PtyConfig::new(shell_command());
    config.args = vec!["/C".into(), "echo __IRIS_PTY__".into()];
    config
}

#[cfg(not(target_os = "windows"))]
fn spawn_output_config() -> PtyConfig {
    let mut config = PtyConfig::new(shell_command());
    config.args = vec!["-lc".into(), "printf '__IRIS_PTY__\\n'".into()];
    config
}

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

#[cfg(not(target_os = "windows"))]
fn round_trip_config() -> PtyConfig {
    let mut config = PtyConfig::new(shell_command());
    config.args = vec![
        "-lc".into(),
        "IFS= read -r line; printf '__IRIS_ECHO__%s\\n' \"$line\"".into(),
    ];
    config
}

#[cfg(target_os = "windows")]
fn input_payload() -> &'static str {
    "hello-from-iris\r\n"
}

#[cfg(not(target_os = "windows"))]
fn input_payload() -> &'static str {
    "hello-from-iris\n"
}

#[cfg(target_os = "windows")]
fn shell_command() -> String {
    std::env::var("ComSpec").unwrap_or_else(|_| "cmd.exe".to_string())
}

#[cfg(not(target_os = "windows"))]
fn shell_command() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string())
}
