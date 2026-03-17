#[cfg(any(target_os = "linux", target_os = "macos"))]
mod unix;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
pub use unix::MacOsPtyBackend as PlatformPtyBackend;
#[cfg(target_os = "linux")]
pub use unix::UnixPtyBackend as PlatformPtyBackend;
#[cfg(target_os = "windows")]
pub use windows::{ConPtyBackend, PlatformPtyBackend};
