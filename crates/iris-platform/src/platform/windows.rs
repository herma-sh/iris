use crate::pty::{PortablePtyBackend, PtyBackend, PtyConfig};
use crate::Result;

/// Windows PTY backend backed by ConPTY through the portable PTY system.
#[derive(Default)]
pub struct ConPtyBackend {
    inner: PortablePtyBackend,
}

impl ConPtyBackend {
    /// Constructs a new ConPTY-backed PTY backend.
    ///
    /// # Examples
    ///
    /// ```
    /// let _backend = ConPtyBackend::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl PtyBackend for ConPtyBackend {
    /// Spawns a ConPTY-backed pseudoterminal using the provided configuration.
    ///
    /// The backend will create and attach a child pseudoterminal and associated
    /// process according to `config`.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut backend = ConPtyBackend::new();
    /// let config = PtyConfig::default();
    /// backend.spawn(&config).expect("failed to spawn pty");
    /// ```
    fn
    fn spawn(&mut self, config: &PtyConfig) -> Result<()> {
        self.inner.spawn(config)
    }

    /// Reads up to `buffer.len()` bytes from the PTY into `buffer`.
    ///
    /// # Returns
    ///
    /// `Ok(n)` with the number of bytes written into `buffer` on success, `Err` on failure.
    ///
    /// # Examples
    ///
    /// ```
    /// # use crate::ConPtyBackend;
    /// let mut backend = ConPtyBackend::new();
    /// let mut buf = [0u8; 1024];
    /// let n = backend.read(&mut buf).unwrap();
    /// assert!(n <= buf.len());
    /// ```
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        self.inner.read(buffer)
    }

    /// Writes the provided bytes into the PTY and returns how many bytes were written.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut backend = ConPtyBackend::new();
    /// // In real use, the PTY should be spawned before writing.
    /// let n = backend.write(b"hello").unwrap();
    /// assert!(n <= 5);
    /// ```
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        self.inner.write(data)
    }

    /// Resizes the underlying pseudo-terminal to the specified number of rows and columns.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut backend = ConPtyBackend::new();
    /// // Resize to 24 rows and 80 columns; backend must be spawned before some backends require it.
    /// let res = backend.resize(24, 80);
    /// assert!(res.is_ok());
    /// ```
    fn resize(&mut self, rows: u16, cols: u16) -> Result<()> {
        self.inner.resize(rows, cols)
    }

    /// Check whether the pseudoterminal process is still running.
    ///
    /// # Returns
    ///
    /// `true` if the pseudoterminal process is running, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut backend = ConPtyBackend::new();
    /// let alive = backend.is_alive().unwrap();
    /// // `alive` is a bool indicating whether the process is running
    /// assert!(alive == alive);
    /// ```
    fn is_alive(&mut self) -> Result<bool> {
        self.inner.is_alive()
    }

    /// Get the child process's exit status if it has terminated.
    ///
    /// # Returns
    ///
    /// `Ok(Some(code))` with the process exit code if the child has exited, `Ok(None)` if the child is still running, or an error if retrieving the status failed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let mut backend = ConPtyBackend::new();
    /// // spawn and interact with the child process here...
    /// let status = backend.exit_status().unwrap();
    /// if let Some(code) = status {
    ///     println!("process exited with code {}", code);
    /// } else {
    ///     println!("process is still running");
    /// }
    /// ```
    fn exit_status(&mut self) -> Result<Option<i32>> {
        self.inner.exit_status()
    }
}

/// Default Windows PTY backend alias.
pub type PlatformPtyBackend = ConPtyBackend;
