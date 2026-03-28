/// Baseline DPI for scale-factor calculations.
pub const BASELINE_DPI: f32 = 96.0;

/// Normalized display scale value used for DPI-aware sizing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DpiScale {
    factor: f32,
}

impl DpiScale {
    /// Creates a scale from a raw OS DPI value.
    ///
    /// Invalid or zero values fall back to 1.0.
    #[must_use]
    pub fn from_dpi(dpi: u32) -> Self {
        Self::from_scale_factor(dpi as f32 / BASELINE_DPI)
    }

    /// Creates a scale from a raw scale-factor value.
    ///
    /// Invalid, non-finite, or non-positive values fall back to 1.0.
    #[must_use]
    pub fn from_scale_factor(factor: f32) -> Self {
        if factor.is_finite() && factor > 0.0 {
            Self { factor }
        } else {
            Self { factor: 1.0 }
        }
    }

    /// Returns the normalized scale factor.
    #[must_use]
    pub const fn factor(self) -> f32 {
        self.factor
    }

    /// Scales a font size in logical pixels.
    #[must_use]
    pub fn scale_font(self, logical_size: f32) -> f32 {
        logical_size * self.factor
    }

    /// Converts logical pixels into physical pixels.
    #[must_use]
    pub fn logical_to_physical(self, logical_px: f32) -> f32 {
        logical_px * self.factor
    }

    /// Converts physical pixels into logical pixels.
    #[must_use]
    pub fn physical_to_logical(self, physical_px: f32) -> f32 {
        physical_px / self.factor
    }
}

impl Default for DpiScale {
    fn default() -> Self {
        Self { factor: 1.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::DpiScale;

    #[test]
    fn dpi_scale_from_dpi_matches_expected_factor() {
        let scale = DpiScale::from_dpi(144);
        assert_eq!(scale.factor(), 1.5);
    }

    #[test]
    fn dpi_scale_rejects_invalid_factors() {
        let zero = DpiScale::from_scale_factor(0.0);
        let negative = DpiScale::from_scale_factor(-2.0);
        let nan = DpiScale::from_scale_factor(f32::NAN);
        assert_eq!(zero.factor(), 1.0);
        assert_eq!(negative.factor(), 1.0);
        assert_eq!(nan.factor(), 1.0);
    }

    #[test]
    fn dpi_scale_converts_logical_and_physical_pixels() {
        let scale = DpiScale::from_scale_factor(2.0);
        assert_eq!(scale.scale_font(12.0), 24.0);
        assert_eq!(scale.logical_to_physical(10.0), 20.0);
        assert_eq!(scale.physical_to_logical(20.0), 10.0);
    }
}
