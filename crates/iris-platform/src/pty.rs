use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::PathBuf;

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};

use crate::error::{PtyError, Result};

/// PTY configuration used when spawning a child process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PtyConfig {
    /// Executable to launch.
    pub command: String,
    /// Command-line arguments.
    pub args: Vec<String>,
    /// Working directory for the process.
    pub working_dir: Option<PathBuf>,
    /// Environment overrides.
    pub env: Vec<(OsString, OsString)>,
    /// Initial terminal rows.
    pub rows: u16,
    /// Initial terminal columns.
    pub cols: u16,
}

impl Default for PtyConfig {
    /// Creates a default PTY configuration with a platform-specific shell and a standard terminal size.
    ///
    /// Defaults:
    /// - `command`: value returned by `default_shell()` (platform-appropriate shell)
    /// - `args`: empty
    /// - `working_dir`: `None`
    /// - `env`: empty
    /// - `rows`: 24
    /// - `cols`: 80
    ///
    /// # Examples
    ///
    /// ```
    /// let cfg = PtyConfig::default();
    /// assert_eq!(cfg.rows, 24);
    /// assert_eq!(cfg.cols, 80);
    /// assert!(cfg.working_dir.is_none());
    /// assert!(cfg.args.is_empty());
    /// ```
    fn default() -> Self {
        Self {
            command: default_shell(),
            args: Vec::new(),
            working_dir: None,
            env: Vec::new(),
            rows: 24,
            cols: 80,
        }
    }
}

impl PtyConfig {
    /// Create a `PtyConfig` with the given command and default settings.
    ///
    /// # Examples
    ///
    /// ```
    /// let cfg = PtyConfig::new("sh");
    /// assert_eq!(cfg.command, "sh");
    /// assert_eq!(cfg.rows, 24);
    /// assert_eq!(cfg.cols, 80);
    /// ```
    #[must_use]
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            ..Self::default()
        }
    }
}

/// PTY abstraction used by the higher-level terminal session.
pub trait PtyBackend: Send {
    /// Spawns a new process inside the PTY, replacing any previous child.
    fn spawn(&mut self, config: &PtyConfig) -> Result<()>;

    /// Reads PTY output into the provided buffer.
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize>;

    /// Writes bytes to the PTY input stream.
    fn write(&mut self, data: &[u8]) -> Result<usize>;

    /// Resizes the PTY viewport.
    fn resize(&mut self, rows: u16, cols: u16) -> Result<()>;

    /// Returns whether the child is still alive.
    fn is_alive(&mut self) -> Result<bool>;

    /// Returns the child exit status when available.
    fn exit_status(&mut self) -> Result<Option<i32>>;
}

/// Cross-platform PTY backend backed by the OS-native PTY implementation.
pub struct PortablePtyBackend {
    master: Option<Box<dyn MasterPty + Send>>,
    reader: Option<Box<dyn Read + Send>>,
    writer: Option<Box<dyn Write + Send>>,
    child: Option<Box<dyn Child + Send + Sync>>,
}

impl Default for PortablePtyBackend {
    /// Creates a default `PtyConfig` with a platform-appropriate shell, no arguments or environment overrides, no working directory, and a 24×80 terminal size.
    ///
    /// # Examples
    ///
    /// ```
    /// let cfg = PtyConfig::default();
    /// assert_eq!(cfg.rows, 24);
    /// assert_eq!(cfg.cols, 80);
    /// assert!(cfg.args.is_empty());
    /// ```
    fn default() -> Self {
        Self::new()
    }
}

impl PortablePtyBackend {
    /// Creates an empty PTY backend with no master, reader, writer, or child.
    ///
    /// # Examples
    ///
    /// ```
    /// let _backend = PortablePtyBackend::new();
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self {
            master: None,
            reader: None,
            writer: None,
            child: None,
        }
    }

    /// Get a mutable reference to the active PTY reader.
    ///
    /// Returns a mutable trait object implementing `Read + Send` when a reader is present; returns
    /// a `PtyError::NotActive` error if no reader is attached.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut backend = PortablePtyBackend::new();
    /// // No reader has been attached yet, so this should be an error.
    /// assert!(backend.reader_mut().is_err());
    /// ```
    fn reader_mut(&mut self) -> Result<&mut (dyn Read + Send + '_)> {
        match self.reader {
            Some(ref mut reader) => Ok(reader.as_mut()),
            None => Err(PtyError::NotActive.into()),
        }
    }

    /// Access the active PTY writer as a mutable trait object.
    ///
    /// Returns a mutable reference to the backend's writer so callers can write bytes to the PTY.
    ///
    /// # Errors
    ///
    /// Returns `PtyError::NotActive` if no writer is currently attached to the backend.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use crate::pty::PortablePtyBackend;
    /// # use std::io::Write;
    /// let mut backend = PortablePtyBackend::new();
    /// match backend.writer_mut() {
    ///     Ok(writer) => {
    ///         let _ = writer.write_all(b"echo hello\n");
    ///     }
    ///     Err(_) => {
    ///         // writer not active
    ///     }
    /// }
    /// ```
    fn writer_mut(&mut self) -> Result<&mut (dyn Write + Send + '_)> {
        match self.writer {
            Some(ref mut writer) => Ok(writer.as_mut()),
            None => Err(PtyError::NotActive.into()),
        }
    }

    /// Returns a mutable reference to the active master PTY or an error if no PTY is active.
    ///
    /// # Returns
    ///
    /// `Ok(&mut dyn MasterPty + Send)` when a master PTY is present, or an error (`PtyError::NotActive`) when none is stored.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::pty::PortablePtyBackend;
    ///
    /// let mut backend = PortablePtyBackend::new();
    /// // No PTY has been spawned yet, so accessing the master returns an error.
    /// assert!(backend.master_mut().is_err());
    /// ```
    fn master_mut(&mut self) -> Result<&mut (dyn MasterPty + Send + '_)> {
        match self.master {
            Some(ref mut master) => Ok(master.as_mut()),
            None => Err(PtyError::NotActive.into()),
        }
    }

    /// Get a mutable reference to the currently active child process.
    
    ///
    
    /// Returns `Ok(&mut dyn Child + Send + Sync)` when a child process is present,
    
    /// or `Err(PtyError::NotActive)` if no child has been spawned.
    
    ///
    
    /// # Examples
    
    ///
    
    /// ```
    
    /// let mut backend = PortablePtyBackend::new();
    
    /// // no child spawned yet
    
    /// assert!(backend.child_mut().is_err());
    
    /// ```
    fn child_mut(&mut self) -> Result<&mut (dyn Child + Send + Sync + '_)> {
        match self.child {
            Some(ref mut child) => Ok(child.as_mut()),
            None => Err(PtyError::NotActive.into()),
        }
    }
}

impl PtyBackend for PortablePtyBackend {
    /// Spawns a child process inside a new pseudo-terminal using the provided configuration.
    ///
    /// Initializes a PTY sized from `config`, launches the configured command on the slave side, and stores the master, reader, writer, and child handles in the backend.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::{PortablePtyBackend, PtyConfig};
    ///
    /// let mut backend = PortablePtyBackend::new();
    /// let config = PtyConfig::new("/bin/sh");
    /// backend.spawn(&config).unwrap();
    /// ```
    fn spawn(&mut self, config: &PtyConfig) -> Result<()> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: config.rows,
                cols: config.cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|error| PtyError::OpenFailed {
                reason: error.to_string(),
            })?;

        let mut command = CommandBuilder::new(&config.command);
        for argument in &config.args {
            command.arg(argument);
        }
        if let Some(working_dir) = &config.working_dir {
            command.cwd(working_dir);
        }
        for (key, value) in &config.env {
            command.env(key, value);
        }

        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|error| PtyError::SpawnFailed {
                command: config.command.clone(),
                reason: error.to_string(),
            })?;

        let master = pair.master;
        let reader = master
            .try_clone_reader()
            .map_err(|error| PtyError::ReadFailed {
                reason: error.to_string(),
            })?;
        let writer = master
            .take_writer()
            .map_err(|error| PtyError::WriteFailed {
                reason: error.to_string(),
            })?;

        self.master = Some(master);
        self.reader = Some(reader);
        self.writer = Some(writer);
        self.child = Some(child);
        Ok(())
    }

    /// Reads data from the PTY into `buffer`.
    ///
    /// # Returns
    ///
    /// `Ok(n)` with the number of bytes read into `buffer`, `Err` if the PTY is not active or an I/O error occurs (mapped to `PtyError::ReadFailed`).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let mut backend = PortablePtyBackend::new();
    /// let mut buf = [0u8; 1024];
    /// let _ = backend.read(&mut buf);
    /// ```
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        self.reader_mut()?.read(buffer).map_err(|error| {
            PtyError::ReadFailed {
                reason: error.to_string(),
            }
            .into()
        })
    }

    /// Write bytes to the PTY's input and flush the writer.
    ///
    /// Writes the provided byte slice into the backend's input stream and flushes the underlying writer.
    ///
    /// # Returns
    ///
    /// The number of bytes successfully written.
    ///
    /// # Examples
    ///
    /// ```
    /// // Given an active `PortablePtyBackend` with an attached child process and writer:
    /// // let mut backend = PortablePtyBackend::new(); // backend must be spawned/initialized first
    /// // let written = backend.write(b"ls -la\n").unwrap();
    /// // assert!(written > 0);
    /// ```
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        let writer = self.writer_mut()?;
        let written = writer.write(data).map_err(|error| PtyError::WriteFailed {
            reason: error.to_string(),
        })?;
        writer.flush().map_err(|error| PtyError::WriteFailed {
            reason: error.to_string(),
        })?;
        Ok(written)
    }

    /// Resize the master PTY to the specified terminal dimensions.
    ///
    /// Adjusts the PTY's character cell geometry to `rows` rows and `cols` columns.
    ///
    /// # Returns
    /// `Ok(())` if the resize succeeds, `Err(PtyError::ResizeFailed)` if the underlying PTY resize operation fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let mut backend = PortablePtyBackend::new();
    /// // After spawning a child, resize will update the terminal geometry for that session:
    /// let _ = backend.resize(24, 80);
    /// ```
    fn resize(&mut self, rows: u16, cols: u16) -> Result<()> {
        self.master_mut()?
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|error| {
                PtyError::ResizeFailed {
                    reason: error.to_string(),
                }
                .into()
            })
    }

    /// Check whether the spawned child process is still running.
    ///
    /// # Returns
    ///
    /// `true` if the child process has not exited, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// // `backend` is a `PortablePtyBackend` with a spawned child.
    /// // Handle the Result to determine liveness:
    /// if let Ok(true) = backend.is_alive() {
    ///     // child is still running
    /// } else {
    ///     // child has exited or an error occurred
    /// }
    /// ```
    fn is_alive(&mut self) -> Result<bool> {
        Ok(self.exit_status()?.is_none())
    }

    /// Query the child process for a non-blocking exit status.
    ///
    /// Returns `Some(exit_code)` when the child has exited — the exit code is converted to `i32`, using `i32::MAX` if the conversion fails; returns `None` if the child is still running. Errors are returned as `PtyError::StatusFailed`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let mut backend = PortablePtyBackend::new();
    /// match backend.exit_status() {
    ///     Ok(Some(code)) => println!("child exited with {}", code),
    ///     Ok(None) => println!("child is still running"),
    ///     Err(err) => eprintln!("failed to get status: {}", err),
    /// }
    /// ```
    fn exit_status(&mut self) -> Result<Option<i32>> {
        self.child_mut()?
            .try_wait()
            .map(|status| status.map(|exit| i32::try_from(exit.exit_code()).unwrap_or(i32::MAX)))
            .map_err(|error| {
                PtyError::StatusFailed {
                    reason: error.to_string(),
                }
                .into()
            })
    }
}

/// Selects the default interactive shell for the current platform.
///
/// On Windows, returns the value of the `ComSpec` environment variable if set, otherwise `"cmd.exe"`.
/// On non-Windows platforms, returns the value of the `SHELL` environment variable if set, otherwise `"sh"`.
///
/// # Examples
///
/// ```
/// let shell = default_shell();
/// assert!(!shell.is_empty());
/// ```
fn default_shell() -> String {
    #[cfg(target_os = "windows")]
    {
        std::env::var("ComSpec").unwrap_or_else(|_| "cmd.exe".to_string())
    }

    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string())
    }
}
