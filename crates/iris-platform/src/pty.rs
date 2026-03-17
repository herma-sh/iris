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
    /// Creates a config for the provided command.
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

    /// Reads the remaining PTY output until EOF.
    fn read_to_end(&mut self) -> Result<Vec<u8>>;

    /// Writes bytes to the PTY input stream.
    fn write(&mut self, data: &[u8]) -> Result<usize>;

    /// Closes the PTY stdin stream, signaling EOF to the child.
    fn close_stdin(&mut self) -> Result<()>;

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
    fn default() -> Self {
        Self::new()
    }
}

impl PortablePtyBackend {
    /// Creates an empty PTY backend.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            master: None,
            reader: None,
            writer: None,
            child: None,
        }
    }

    fn reader_mut(&mut self) -> Result<&mut (dyn Read + Send + '_)> {
        match self.reader {
            Some(ref mut reader) => Ok(reader.as_mut()),
            None => Err(PtyError::NotActive.into()),
        }
    }

    fn writer_mut(&mut self) -> Result<&mut (dyn Write + Send + '_)> {
        match self.writer {
            Some(ref mut writer) => Ok(writer.as_mut()),
            None => Err(PtyError::NotActive.into()),
        }
    }
}

impl PtyBackend for PortablePtyBackend {
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

    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        self.reader_mut()?.read(buffer).map_err(|error| {
            PtyError::ReadFailed {
                reason: error.to_string(),
            }
            .into()
        })
    }

    fn read_to_end(&mut self) -> Result<Vec<u8>> {
        let mut output = Vec::new();
        self.reader_mut()?
            .read_to_end(&mut output)
            .map_err(|error| PtyError::ReadFailed {
                reason: error.to_string(),
            })?;
        Ok(output)
    }

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

    fn close_stdin(&mut self) -> Result<()> {
        self.writer.take();
        Ok(())
    }

    fn resize(&mut self, rows: u16, cols: u16) -> Result<()> {
        let master = match self.master {
            Some(ref mut master) => master,
            None => return Err(PtyError::NotActive.into()),
        };

        master
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

    fn is_alive(&mut self) -> Result<bool> {
        Ok(self.exit_status()?.is_none())
    }

    fn exit_status(&mut self) -> Result<Option<i32>> {
        let child = match self.child {
            Some(ref mut child) => child,
            None => return Err(PtyError::NotActive.into()),
        };

        child
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
