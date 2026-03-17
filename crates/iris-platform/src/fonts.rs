use crate::error::Result;

/// Minimal font metadata surfaced by the platform layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontInfo {
    /// User-facing font family name.
    pub family: String,
    /// Optional style name such as Regular or Bold.
    pub style: Option<String>,
}

/// Font enumeration and fallback lookup abstraction.
pub trait FontProvider {
    /// Returns the available fonts known to the implementation.
    fn enumerate(&self) -> Result<Vec<FontInfo>>;

    /// Returns a best-effort fallback family for the provided character.
    fn fallback_for(&self, character: char) -> Result<Option<FontInfo>>;
}

/// Placeholder font provider for phase 0.
#[derive(Debug, Default)]
pub struct NoopFontProvider;

impl FontProvider for NoopFontProvider {
    fn enumerate(&self) -> Result<Vec<FontInfo>> {
        Ok(Vec::new())
    }

    fn fallback_for(&self, _character: char) -> Result<Option<FontInfo>> {
        Ok(None)
    }
}
