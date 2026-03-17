use crate::pty::{PortablePtyBackend, PtyBackend, PtyConfig};
use crate::Result;

/// Unix PTY backend backed by the native platform PTY system.
#[derive(Default)]
pub struct UnixPtyBackend {
    inner: PortablePtyBackend,
}

impl UnixPtyBackend {
    /// Create a Unix PTY backend using default settings.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let _backend = UnixPtyBackend::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl PtyBackend for UnixPtyBackend {
    /// Spawns a child process attached to a pseudo-terminal using the provided configuration.
    ///
    /// On success, the backend will have an active PTY connected to the spawned process.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut backend = UnixPtyBackend::new();
    /// let config = PtyConfig { /* populate as needed */ };
    /// backend.spawn(&config).unwrap();
    /// ```
    fn spawn(&mut self, config: &PtyConfig) -> Result<()> {
        self.inner.spawn(config)
    }

    /// Read data from the PTY into the provided buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut backend = UnixPtyBackend::new();
    /// let mut buf = [0u8; 1024];
    /// let _ = backend.read(&mut buf);
    /// ```
    ///
    /// # Returns
    ///
    /// `Ok(n)` with the number of bytes read into `buffer`, or an error.
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        self.inner.read(buffer)
    }

    /// Writes the provided bytes to the underlying PTY.
    ///
    /// Returns the number of bytes successfully written.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::platform::unix::UnixPtyBackend;
    ///
    /// let mut backend = UnixPtyBackend::new();
    /// let written = backend.write(b"hello").unwrap();
    /// assert_eq!(written, 5);
    /// ```
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        self.inner.write(data)
    }

    /// Resizes the child pseudoterminal to the specified number of rows and columns.
    ///
    /// Returns `Ok(())` if the resize operation succeeded, or an error if resizing failed.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut backend = UnixPtyBackend::new();
    /// backend.resize(24, 80).unwrap();
    /// ```
    fn resize(&mut self, rows: u16, cols: u16) -> Result<()> {
        self.inner.resize(rows, cols)
    }

    /// Checks whether the child process associated with this PTY is still running.
    ///
    /// # Returns
    ///
    /// `true` if the child process is running, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use iris_platform::platform::unix::UnixPtyBackend;
    ///
    /// let mut backend = UnixPtyBackend::new();
    /// // Without spawning a process this will typically return `Ok(false)`.
    /// let _ = backend.is_alive();
    /// ```
    fn is_alive(&mut self) -> Result<bool> {
        self.inner.is_alive()
    }

    /// Query the child process's exit code if it has terminated.
    ///
    /// # Returns
    ///
    /// `Ok(Some(code))` when the child has exited with the given exit code, `Ok(None)` if the child is still running, or `Err(_)` if an error occurred while retrieving the status.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut backend = UnixPtyBackend::new();
    /// // before spawning a process the exit status is typically None
    /// let status = backend.exit_status().unwrap();
    /// assert!(status.is_none());
    /// ```
    fn exit_status(&mut self) -> Result<Option<i32>> {
        self.inner.exit_status()
    }
}
