#![cfg(not(target_os = "windows"))]

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use iris_platform::platform::PlatformPtyBackend;
use iris_platform::{PtyBackend, PtyConfig};

#[test]
fn pty_spawn_returns_handle() {
    let mut backend = PlatformPtyBackend::default();
    let config = spawn_output_config();
    backend.spawn(&config).unwrap();
    backend.close_stdin().unwrap();

    let output = read_until_contains(backend, "__IRIS_PTY__", Duration::from_secs(5));
    assert!(output.contains("__IRIS_PTY__"));
}

#[test]
fn pty_read_write_works() {
    let mut backend = PlatformPtyBackend::default();
    let config = round_trip_config();
    backend.spawn(&config).unwrap();
    backend.write(input_payload().as_bytes()).unwrap();
    backend.close_stdin().unwrap();

    let output = read_until_contains(
        backend,
        "__IRIS_ECHO__hello-from-iris",
        Duration::from_secs(5),
    );
    assert!(output.contains("__IRIS_ECHO__hello-from-iris"));
}

fn read_until_contains(
    mut backend: impl PtyBackend + 'static,
    needle: &str,
    timeout: Duration,
) -> String {
    let (sender, receiver) = mpsc::channel();
    let needle = needle.to_string();

    thread::spawn(move || {
        let mut buffer = [0_u8; 1024];
        loop {
            match backend.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    if sender.send(buffer[..read].to_vec()).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let mut output = Vec::new();
    loop {
        match receiver.recv_timeout(timeout) {
            Ok(chunk) => {
                output.extend_from_slice(&chunk);
                let text = String::from_utf8_lossy(&output);
                if text.contains(&needle) {
                    return text.into_owned();
                }
            }
            Err(_) => return String::from_utf8_lossy(&output).into_owned(),
        }
    }
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
