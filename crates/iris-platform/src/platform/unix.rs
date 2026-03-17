use crate::pty::{PortablePtyBackend, PtyBackend, PtyConfig};
use crate::Result;

/// Unix PTY backend backed by the native platform PTY system.
#[derive(Default)]
pub struct UnixPtyBackend {
    inner: PortablePtyBackend,
}

impl UnixPtyBackend {
    /// Creates a new Unix PTY backend.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl PtyBackend for UnixPtyBackend {
    fn spawn(&mut self, config: &PtyConfig) -> Result<()> {
        self.inner.spawn(config)
    }

    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        self.inner.read(buffer)
    }

    fn read_to_end(&mut self) -> Result<Vec<u8>> {
        self.inner.read_to_end()
    }

    fn write(&mut self, data: &[u8]) -> Result<usize> {
        self.inner.write(data)
    }

    fn close_stdin(&mut self) -> Result<()> {
        self.inner.close_stdin()
    }

    fn resize(&mut self, rows: u16, cols: u16) -> Result<()> {
        self.inner.resize(rows, cols)
    }

    fn is_alive(&mut self) -> Result<bool> {
        self.inner.is_alive()
    }

    fn exit_status(&mut self) -> Result<Option<i32>> {
        self.inner.exit_status()
    }
}
