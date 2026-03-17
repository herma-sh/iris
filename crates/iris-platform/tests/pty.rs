#![cfg(not(target_os = "windows"))]

use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use iris_platform::platform::PlatformPtyBackend;
use iris_platform::{PtyBackend, PtyConfig};

#[test]
fn pty_spawn_returns_handle() {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut backend = PlatformPtyBackend::default();
        let config = spawn_output_config();
        backend.spawn(&config).unwrap();
        backend.close_stdin().unwrap();
        let _status = wait_for_exit(&mut backend, Duration::from_secs(5));
        let output = String::from_utf8_lossy(&backend.read_to_end().unwrap()).to_string();
        let _ = sender.send(output);
    });

    let output = receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap_or_default();
    assert!(output.contains("__IRIS_PTY__"));
}

#[test]
fn pty_read_write_works() {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut backend = PlatformPtyBackend::default();
        let config = round_trip_config();
        backend.spawn(&config).unwrap();
        backend.write(input_payload().as_bytes()).unwrap();
        backend.close_stdin().unwrap();
        let _status = wait_for_exit(&mut backend, Duration::from_secs(5));
        let output = String::from_utf8_lossy(&backend.read_to_end().unwrap()).to_string();
        let _ = sender.send(output);
    });

    let output = receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap_or_default();
    assert!(output.contains("__IRIS_ECHO__hello-from-iris"));
}

fn wait_for_exit(backend: &mut dyn PtyBackend, timeout: Duration) -> Option<i32> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        match backend.exit_status() {
            Ok(Some(status)) => return Some(status),
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(_) => return None,
        }
    }
    None
}

fn spawn_output_config() -> PtyConfig {
    let mut config = PtyConfig::new(shell_command());
    config.args = vec!["-c".into(), "printf '__IRIS_PTY__\\n'".into()];
    config
}

fn round_trip_config() -> PtyConfig {
    let mut config = PtyConfig::new(shell_command());
    config.args = vec![
        "-c".into(),
        "IFS= read -r line; printf '__IRIS_ECHO__%s\\n' \"$line\"".into(),
    ];
    config
}

fn input_payload() -> &'static str {
    "hello-from-iris\n"
}

fn shell_command() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string())
}
