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

/// Platform font provider with deterministic fallback behavior.
#[derive(Clone, Debug)]
pub struct PlatformFontProvider {
    catalog: Vec<FontInfo>,
}

impl PlatformFontProvider {
    /// Creates a platform-font provider using the built-in platform catalog.
    #[must_use]
    pub fn new() -> Self {
        Self {
            catalog: platform_font_catalog(),
        }
    }

    /// Creates a provider with an explicit font catalog.
    ///
    /// This is primarily used by tests and deterministic host integration.
    #[must_use]
    pub fn with_catalog(catalog: Vec<FontInfo>) -> Self {
        Self { catalog }
    }

    fn find_family(&self, family: &str) -> Option<FontInfo> {
        self.catalog
            .iter()
            .find(|font| font.family.eq_ignore_ascii_case(family))
            .cloned()
    }
}

impl Default for PlatformFontProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl FontProvider for PlatformFontProvider {
    fn enumerate(&self) -> Result<Vec<FontInfo>> {
        Ok(self.catalog.clone())
    }

    fn fallback_for(&self, character: char) -> Result<Option<FontInfo>> {
        if self.catalog.is_empty() {
            return Ok(None);
        }

        let preferred = if is_emoji(character) {
            preferred_emoji_families()
        } else if is_cjk(character) {
            preferred_cjk_families()
        } else {
            preferred_monospace_families()
        };

        for family in preferred {
            if let Some(font) = self.find_family(family) {
                return Ok(Some(font));
            }
        }

        Ok(self.catalog.first().cloned())
    }
}

/// Placeholder font provider for environments that do not expose fonts.
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

fn platform_font_catalog() -> Vec<FontInfo> {
    #[cfg(target_os = "windows")]
    {
        return vec![
            font("Cascadia Mono"),
            font("Consolas"),
            font("Segoe UI Emoji"),
            font("Microsoft YaHei UI"),
            font("Yu Gothic UI"),
        ];
    }

    #[cfg(target_os = "macos")]
    {
        return vec![
            font("SF Mono"),
            font("Menlo"),
            font("Apple Color Emoji"),
            font("PingFang SC"),
            font("Hiragino Sans"),
        ];
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        return vec![
            font("DejaVu Sans Mono"),
            font("Noto Sans Mono"),
            font("Noto Color Emoji"),
            font("Noto Sans CJK SC"),
            font("Noto Sans CJK JP"),
        ];
    }

    #[allow(unreachable_code)]
    Vec::new()
}

const fn preferred_monospace_families() -> &'static [&'static str] {
    &[
        "Cascadia Mono",
        "Consolas",
        "SF Mono",
        "Menlo",
        "DejaVu Sans Mono",
        "Noto Sans Mono",
    ]
}

const fn preferred_cjk_families() -> &'static [&'static str] {
    &[
        "Microsoft YaHei UI",
        "Yu Gothic UI",
        "PingFang SC",
        "Hiragino Sans",
        "Noto Sans CJK SC",
        "Noto Sans CJK JP",
    ]
}

const fn preferred_emoji_families() -> &'static [&'static str] {
    &["Segoe UI Emoji", "Apple Color Emoji", "Noto Color Emoji"]
}

fn font(family: &str) -> FontInfo {
    FontInfo {
        family: family.to_string(),
        style: None,
    }
}

fn is_cjk(character: char) -> bool {
    let cp = character as u32;
    (0x3000..=0x303F).contains(&cp)
        || (0x3400..=0x4DBF).contains(&cp)
        || (0x4E00..=0x9FFF).contains(&cp)
        || (0xF900..=0xFAFF).contains(&cp)
        || (0xFF00..=0xFFEF).contains(&cp)
        || (0x3040..=0x30FF).contains(&cp)
        || (0xAC00..=0xD7AF).contains(&cp)
}

fn is_emoji(character: char) -> bool {
    let cp = character as u32;
    (0x1F300..=0x1FAFF).contains(&cp)
        || (0x1F1E6..=0x1F1FF).contains(&cp)
        || (0x2600..=0x27BF).contains(&cp)
}

#[cfg(test)]
mod tests {
    use super::{FontInfo, FontProvider, PlatformFontProvider};

    fn custom_provider() -> PlatformFontProvider {
        PlatformFontProvider::with_catalog(vec![
            FontInfo {
                family: "JetBrains Mono".to_string(),
                style: Some("Regular".to_string()),
            },
            FontInfo {
                family: "Noto Color Emoji".to_string(),
                style: Some("Regular".to_string()),
            },
            FontInfo {
                family: "Noto Sans CJK SC".to_string(),
                style: Some("Regular".to_string()),
            },
        ])
    }

    #[test]
    fn platform_provider_enumerates_catalog() {
        let provider = custom_provider();
        let fonts = provider.enumerate().unwrap();
        assert_eq!(fonts.len(), 3);
        assert_eq!(fonts[0].family, "JetBrains Mono");
    }

    #[test]
    fn platform_provider_prefers_emoji_fallback() {
        let provider = custom_provider();
        let fallback = provider.fallback_for('\u{1F600}').unwrap().unwrap();
        assert_eq!(fallback.family, "Noto Color Emoji");
    }

    #[test]
    fn platform_provider_prefers_cjk_fallback() {
        let provider = custom_provider();
        let fallback = provider.fallback_for('\u{6F22}').unwrap().unwrap();
        assert_eq!(fallback.family, "Noto Sans CJK SC");
    }

    #[test]
    fn platform_provider_treats_fullwidth_forms_as_cjk() {
        let provider = custom_provider();
        let fallback = provider.fallback_for('\u{FF01}').unwrap().unwrap();
        assert_eq!(fallback.family, "Noto Sans CJK SC");
    }

    #[test]
    fn platform_provider_treats_regional_indicators_as_emoji() {
        let provider = custom_provider();
        let fallback = provider.fallback_for('\u{1F1FA}').unwrap().unwrap();
        assert_eq!(fallback.family, "Noto Color Emoji");
    }

    #[test]
    fn platform_provider_falls_back_to_first_when_no_family_matches() {
        let provider = PlatformFontProvider::with_catalog(vec![FontInfo {
            family: "Custom Font".to_string(),
            style: Some("Regular".to_string()),
        }]);
        let fallback = provider.fallback_for('A').unwrap().unwrap();
        assert_eq!(fallback.family, "Custom Font");
    }
}
