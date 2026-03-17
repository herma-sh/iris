#[cfg(unix)]
mod unix;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(unix)]
pub use unix::{UnixPtyBackend, UnixPtyBackend as PlatformPtyBackend};
#[cfg(target_os = "windows")]
pub use windows::{ConPtyBackend, PlatformPtyBackend};
