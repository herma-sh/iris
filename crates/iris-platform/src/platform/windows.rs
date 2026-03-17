use crate::pty::{PortablePtyBackend, PtyBackend, PtyConfig};
use crate::Result;

/// Windows PTY backend backed by ConPTY through the portable PTY system.
#[derive(Default)]
pub struct ConPtyBackend {
    inner: PortablePtyBackend,
}

impl ConPtyBackend {
    /// Creates a new ConPTY-backed PTY backend.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl PtyBackend for ConPtyBackend {
    fn spawn(&mut self, config: &PtyConfig) -> Result<()> {
        self.inner.spawn(config)
    }

    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        self.inner.read(buffer)
    }

    fn write(&mut self, data: &[u8]) -> Result<usize> {
        self.inner.write(data)
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

/// Default Windows PTY backend alias.
pub type PlatformPtyBackend = ConPtyBackend;
