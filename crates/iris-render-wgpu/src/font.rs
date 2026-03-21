use std::collections::HashMap;

use fontdb::{Database, Family, Query, Stretch, Style, Weight};
use iris_core::cell::Cell;

use crate::error::{Error, Result};
use crate::glyph::RasterizedGlyph;

const MAX_RASTERIZED_GLYPH_DIMENSION: u32 = 512;
const MAX_FONT_DATA_BYTES: usize = 32 * 1024 * 1024;

/// Configuration for system-font-backed glyph rasterization.
#[derive(Clone, Debug, PartialEq)]
pub struct FontRasterizerConfig {
    /// Requested primary family name.
    pub primary_family: Option<String>,
    /// Explicit fallback families tried before scanning the wider system database.
    pub fallback_families: Vec<String>,
    /// Glyph rasterization size in pixels.
    pub font_size_px: f32,
}

impl Default for FontRasterizerConfig {
    fn default() -> Self {
        Self {
            primary_family: None,
            fallback_families: Vec::new(),
            font_size_px: 14.0,
        }
    }
}

/// System font loader and glyph rasterizer used by the renderer bootstrap.
pub struct FontRasterizer {
    database: Database,
    loaded_faces: Vec<LoadedFace>,
    fallback_cache: HashMap<char, Option<usize>>,
    font_size_px: f32,
}

struct LoadedFace {
    id: fontdb::ID,
    family_name: String,
    font: fontdue::Font,
}

impl FontRasterizer {
    /// Creates a rasterizer backed by best-effort system font loading.
    pub fn new(config: FontRasterizerConfig) -> Result<Self> {
        if config.font_size_px.is_nan() || config.font_size_px <= 0.0 {
            return Err(Error::InvalidFontSize {
                size: config.font_size_px,
            });
        }

        let mut database = Database::new();
        database.load_system_fonts();
        let mut rasterizer = Self {
            database,
            loaded_faces: Vec::new(),
            fallback_cache: HashMap::new(),
            font_size_px: config.font_size_px,
        };

        if let Some(primary_family) = config.primary_family.as_deref() {
            rasterizer.load_named_family(primary_family)?;
        }

        rasterizer.load_monospace_family()?;

        for family in &config.fallback_families {
            rasterizer.load_named_family(family)?;
        }

        if rasterizer.loaded_faces.is_empty() {
            rasterizer.load_first_usable_face()?;
        }

        if rasterizer.loaded_faces.is_empty() {
            return Err(Error::NoUsableSystemFont);
        }

        Ok(rasterizer)
    }

    /// Returns the configured rasterization size in pixels.
    #[must_use]
    pub const fn font_size_px(&self) -> f32 {
        self.font_size_px
    }

    /// Rasterizes a terminal cell using the currently loaded font set and
    /// best-effort fallback discovery.
    pub fn rasterize_cell(&mut self, cell: Cell) -> Result<Option<RasterizedGlyph>> {
        if cell.width.columns() == 0 {
            return Ok(None);
        }

        if cell.character == ' ' {
            return Ok(Some(blank_glyph()));
        }

        let face_index = self.face_index_for_character(cell.character)?;
        let (metrics, bitmap) = self.loaded_faces[face_index]
            .font
            .rasterize(cell.character, self.font_size_px);

        let width = u32::try_from(metrics.width).map_err(|_| Error::GlyphRasterizationFailed {
            reason: format!(
                "glyph width {} for {:?} did not fit into u32",
                metrics.width, cell.character
            ),
        })?;
        let height =
            u32::try_from(metrics.height).map_err(|_| Error::GlyphRasterizationFailed {
                reason: format!(
                    "glyph height {} for {:?} did not fit into u32",
                    metrics.height, cell.character
                ),
            })?;
        validate_glyph_dimension(width, "width", cell.character)?;
        validate_glyph_dimension(height, "height", cell.character)?;

        if width == 0 || height == 0 || bitmap.is_empty() {
            return Ok(Some(blank_glyph()));
        }

        Ok(Some(RasterizedGlyph::new(width, height, bitmap)))
    }

    /// Returns the family names currently loaded into the rasterizer.
    #[must_use]
    pub fn loaded_family_names(&self) -> Vec<&str> {
        self.loaded_faces
            .iter()
            .map(|face| face.family_name.as_str())
            .collect()
    }

    fn face_index_for_character(&mut self, character: char) -> Result<usize> {
        if let Some(cached_face_index) = self.fallback_cache.get(&character).copied() {
            return cached_face_index.ok_or_else(|| Error::GlyphRasterizationFailed {
                reason: format!("no system font contained {:?}", character),
            });
        }

        if let Some(index) = self
            .loaded_faces
            .iter()
            .position(|face| face.font.lookup_glyph_index(character) != 0)
        {
            self.fallback_cache.insert(character, Some(index));
            return Ok(index);
        }

        self.load_fallback_face_for_character(character)?;

        match self
            .loaded_faces
            .iter()
            .position(|face| face.font.lookup_glyph_index(character) != 0)
        {
            Some(index) => {
                self.fallback_cache.insert(character, Some(index));
                Ok(index)
            }
            None => {
                self.fallback_cache.insert(character, None);
                Err(Error::GlyphRasterizationFailed {
                    reason: format!("no system font contained {:?}", character),
                })
            }
        }
    }

    fn load_named_family(&mut self, family_name: &str) -> Result<()> {
        let families = [Family::Name(family_name)];
        self.load_query_face(&families)
    }

    fn load_monospace_family(&mut self) -> Result<()> {
        let families = [Family::Monospace];
        self.load_query_face(&families)
    }

    fn load_query_face(&mut self, families: &[Family<'_>]) -> Result<()> {
        let query = Query {
            families,
            weight: Weight::NORMAL,
            stretch: Stretch::Normal,
            style: Style::Normal,
        };

        if let Some(id) = self.database.query(&query) {
            self.load_face(id)?;
        }

        Ok(())
    }

    fn load_first_usable_face(&mut self) -> Result<()> {
        let ids: Vec<_> = self.database.faces().map(|face| face.id).collect();
        for id in ids {
            if self.try_load_face(id)?.is_some() {
                return Ok(());
            }
        }

        Ok(())
    }

    fn load_fallback_face_for_character(&mut self, character: char) -> Result<()> {
        let ids: Vec<_> = self.database.faces().map(|face| face.id).collect();
        for id in ids {
            if self.is_face_loaded(id) {
                continue;
            }

            let Some(face) = self.try_load_face(id)? else {
                continue;
            };

            if face.font.lookup_glyph_index(character) != 0 {
                let index = self.loaded_faces.len();
                self.loaded_faces.push(face);
                self.fallback_cache.insert(character, Some(index));
                return Ok(());
            }
        }

        self.fallback_cache.insert(character, None);
        Ok(())
    }

    fn load_face(&mut self, id: fontdb::ID) -> Result<()> {
        if self.is_face_loaded(id) {
            return Ok(());
        }

        if let Some(face) = self.try_load_face(id)? {
            self.loaded_faces.push(face);
        }

        Ok(())
    }

    fn try_load_face(&self, id: fontdb::ID) -> Result<Option<LoadedFace>> {
        let family_name = self
            .database
            .face(id)
            .and_then(|face| face.families.first())
            .map_or_else(
                || format!("font-{id:?}"),
                |(family_name, _)| family_name.clone(),
            );

        let Some(font_result) = self.database.with_face_data(id, |data, face_index| {
            if data.len() > MAX_FONT_DATA_BYTES {
                return Err(Error::FontDataTooLarge {
                    family: family_name.clone(),
                    size: data.len(),
                });
            }

            fontdue::Font::from_bytes(
                data,
                fontdue::FontSettings {
                    collection_index: face_index,
                    ..fontdue::FontSettings::default()
                },
            )
            .map_err(|error| Error::FontLoadFailed {
                family: family_name.clone(),
                reason: error.to_string(),
            })
        }) else {
            return Err(Error::FontDataUnavailable {
                family: family_name,
            });
        };

        let font = font_result?;

        Ok(Some(LoadedFace {
            id,
            family_name,
            font,
        }))
    }

    fn is_face_loaded(&self, id: fontdb::ID) -> bool {
        self.loaded_faces.iter().any(|face| face.id == id)
    }

    #[cfg(test)]
    fn new_empty_for_tests(font_size_px: f32) -> Self {
        Self {
            database: Database::new(),
            loaded_faces: Vec::new(),
            fallback_cache: HashMap::new(),
            font_size_px,
        }
    }
}

#[must_use]
fn blank_glyph() -> RasterizedGlyph {
    RasterizedGlyph::new(1, 1, vec![0])
}

fn validate_glyph_dimension(dimension: u32, axis: &str, character: char) -> Result<()> {
    if dimension > MAX_RASTERIZED_GLYPH_DIMENSION {
        return Err(Error::GlyphRasterizationFailed {
            reason: format!(
                "glyph {} {} for {:?} exceeded the maximum {}",
                axis, dimension, character, MAX_RASTERIZED_GLYPH_DIMENSION
            ),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
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
}
