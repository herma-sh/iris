use iris_core::cell::Cell;

use super::{
    blank_glyph, validate_glyph_dimension, FontRasterizer, FontRasterizerConfig,
    MAX_RASTERIZED_GLYPH_DIMENSION,
};
use crate::error::Error;

#[test]
fn font_rasterizer_rejects_non_positive_sizes() {
    let result = FontRasterizer::new(FontRasterizerConfig {
        font_size_px: 0.0,
        ..FontRasterizerConfig::default()
    });

    assert!(matches!(result, Err(Error::InvalidFontSize { size: 0.0 })));
}

#[test]
fn font_rasterizer_rejects_nan_sizes() {
    let result = FontRasterizer::new(FontRasterizerConfig {
        font_size_px: f32::NAN,
        ..FontRasterizerConfig::default()
    });

    assert!(matches!(result, Err(Error::InvalidFontSize { size }) if size.is_nan()));
}

#[test]
fn blank_glyph_is_transparent_single_pixel() {
    let glyph = blank_glyph();

    assert_eq!(glyph.width(), 1);
    assert_eq!(glyph.height(), 1);
    assert_eq!(glyph.data(), &[0]);
}

#[test]
fn glyph_dimension_validation_rejects_pathological_sizes() {
    let result = validate_glyph_dimension(MAX_RASTERIZED_GLYPH_DIMENSION + 1, "width", 'A');

    assert!(matches!(
        result,
        Err(Error::GlyphRasterizationFailed { .. })
    ));
}

#[test]
fn font_rasterizer_loads_a_system_font_and_rasterizes_ascii() {
    let mut rasterizer = match FontRasterizer::new(FontRasterizerConfig::default()) {
        Ok(rasterizer) => rasterizer,
        Err(Error::NoUsableSystemFont) => return,
        Err(error) => panic!("font rasterizer failed unexpectedly: {error}"),
    };

    let glyph = rasterizer
        .rasterize_cell(Cell::new('A'))
        .expect("ASCII rasterization should succeed")
        .expect("ASCII glyph should produce a bitmap");

    assert!(!rasterizer.loaded_family_names().is_empty());
    assert!(glyph.width() > 0);
    assert!(glyph.height() > 0);
    assert!(!glyph.data().is_empty());
}

#[test]
fn font_rasterizer_returns_a_transparent_glyph_for_blank_cells() {
    let mut rasterizer = match FontRasterizer::new(FontRasterizerConfig::default()) {
        Ok(rasterizer) => rasterizer,
        Err(Error::NoUsableSystemFont) => return,
        Err(error) => panic!("font rasterizer failed unexpectedly: {error}"),
    };

    let glyph = rasterizer
        .rasterize_cell(Cell::new(' '))
        .expect("blank glyph rasterization should succeed")
        .expect("blank glyph should be returned");

    assert_eq!(glyph.width(), 1);
    assert_eq!(glyph.height(), 1);
    assert_eq!(glyph.data(), &[0]);
}

#[test]
fn font_rasterizer_best_effort_rasterizes_cjk_when_available() {
    let mut rasterizer = match FontRasterizer::new(FontRasterizerConfig {
        primary_family: Some("Courier New".to_string()),
        ..FontRasterizerConfig::default()
    }) {
        Ok(rasterizer) => rasterizer,
        Err(Error::NoUsableSystemFont) => return,
        Err(error) => panic!("font rasterizer failed unexpectedly: {error}"),
    };

    let fallback_candidate = ['\u{4E2D}', '\u{6F22}', '\u{3042}', '\u{D55C}']
        .into_iter()
        .find(|character| rasterizer.rasterize_cell(Cell::new(*character)).is_ok());

    if let Some(character) = fallback_candidate {
        let glyph = rasterizer
            .rasterize_cell(Cell::new(character))
            .expect("CJK rasterization should succeed")
            .expect("CJK glyph should be returned");

        assert!(glyph.width() > 0);
        assert!(glyph.height() > 0);
    }
}

#[test]
fn font_rasterizer_best_effort_rasterizes_emoji_when_available() {
    let mut rasterizer = match FontRasterizer::new(FontRasterizerConfig {
        primary_family: Some("Courier New".to_string()),
        ..FontRasterizerConfig::default()
    }) {
        Ok(rasterizer) => rasterizer,
        Err(Error::NoUsableSystemFont) => return,
        Err(error) => panic!("font rasterizer failed unexpectedly: {error}"),
    };

    let fallback_candidate = ['\u{1F600}', '\u{1F680}', '\u{1F44D}', '\u{1F4A1}']
        .into_iter()
        .find(|character| rasterizer.rasterize_cell(Cell::new(*character)).is_ok());

    if let Some(character) = fallback_candidate {
        let glyph = rasterizer
            .rasterize_cell(Cell::new(character))
            .expect("emoji rasterization should succeed")
            .expect("emoji glyph should be returned");

        assert!(glyph.width() > 0);
        assert!(glyph.height() > 0);
    }
}

#[test]
fn font_rasterizer_returns_error_when_no_font_can_map_a_character() {
    let mut rasterizer = FontRasterizer::new_empty_for_tests(14.0);

    let result = rasterizer.rasterize_cell(Cell::new('A'));

    assert!(matches!(
        result,
        Err(Error::GlyphRasterizationFailed { .. })
    ));
}
