use std::collections::HashMap;

use crate::atlas::{AtlasRegion, GlyphAtlas};
use crate::error::{Error, Result};

/// Stable cache key for a rasterized glyph variant.
///
/// Callers should derive this from the inputs that affect rasterization, such
/// as font identity, glyph identifier, size, and style selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GlyphKey(u64);

impl GlyphKey {
    /// Creates a glyph cache key from a caller-defined stable value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the underlying cache key value.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// Borrowed single-channel glyph bitmap ready for atlas upload.
#[derive(Clone, Copy, Debug)]
pub struct GlyphBitmap<'a> {
    /// Bitmap width in pixels.
    pub width: u32,
    /// Bitmap height in pixels.
    pub height: u32,
    /// Bitmap bytes in row-major order.
    pub data: &'a [u8],
}

impl<'a> GlyphBitmap<'a> {
    /// Creates a borrowed glyph bitmap descriptor.
    #[must_use]
    pub const fn new(width: u32, height: u32, data: &'a [u8]) -> Self {
        Self {
            width,
            height,
            data,
        }
    }
}

/// Atlas-backed cache entry for a rasterized glyph variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CachedGlyph {
    region: AtlasRegion,
}

impl CachedGlyph {
    /// Creates a cached glyph entry for the provided atlas region.
    #[must_use]
    pub const fn new(region: AtlasRegion) -> Self {
        Self { region }
    }

    /// Returns the atlas region occupied by this cached glyph.
    #[must_use]
    pub const fn region(self) -> AtlasRegion {
        self.region
    }
}

/// CPU-side glyph cache that tracks atlas residency.
#[derive(Debug, Default)]
pub struct GlyphCache {
    entries: HashMap<GlyphKey, CachedGlyph>,
}

impl GlyphCache {
    /// Creates an empty glyph cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns whether the cache contains no glyphs.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the number of cached glyphs.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns whether the cache already contains the requested glyph key.
    #[must_use]
    pub fn contains(&self, key: GlyphKey) -> bool {
        self.entries.contains_key(&key)
    }

    /// Returns the cached glyph entry for the provided key.
    #[must_use]
    pub fn get(&self, key: GlyphKey) -> Option<&CachedGlyph> {
        self.entries.get(&key)
    }

    /// Caches a glyph bitmap in the provided atlas and returns its atlas entry.
    pub fn cache_glyph(
        &mut self,
        atlas: &mut GlyphAtlas,
        queue: &wgpu::Queue,
        key: GlyphKey,
        bitmap: GlyphBitmap<'_>,
    ) -> Result<CachedGlyph> {
        if let Some(entry) = self.entries.get(&key).copied() {
            if entry.region.width != bitmap.width || entry.region.height != bitmap.height {
                return Err(Error::GlyphCacheEntryMismatch {
                    key: key.value(),
                    cached_width: entry.region.width,
                    cached_height: entry.region.height,
                    requested_width: bitmap.width,
                    requested_height: bitmap.height,
                });
            }

            return Ok(entry);
        }

        let expected = bitmap
            .width
            .checked_mul(bitmap.height)
            .and_then(|bytes| usize::try_from(bytes).ok())
            .ok_or(Error::InvalidAtlasUploadSize {
                expected: usize::MAX,
                actual: bitmap.data.len(),
            })?;
        if bitmap.data.len() != expected {
            return Err(Error::InvalidAtlasUploadSize {
                expected,
                actual: bitmap.data.len(),
            });
        }

        let region = atlas.allocate(bitmap.width, bitmap.height)?;
        atlas.upload(queue, region, bitmap.data)?;

        let entry = CachedGlyph::new(region);
        self.entries.insert(key, entry);

        Ok(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::{GlyphBitmap, GlyphCache, GlyphKey};
    use crate::atlas::{AtlasConfig, AtlasSize};
    use crate::error::Error;
    use crate::renderer::{Renderer, RendererConfig};

    #[test]
    fn glyph_cache_starts_empty() {
        let cache = GlyphCache::new();

        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
        assert!(!cache.contains(GlyphKey::new(1)));
    }

    #[test]
    fn glyph_cache_reuses_existing_entries() {
        let _gpu_test_lock = crate::test_support::gpu_test_lock();
        let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
            Ok(renderer) => renderer,
            Err(Error::NoAdapter) => return,
            Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
        };
        let mut atlas = renderer
            .create_glyph_atlas(AtlasConfig::new(
                AtlasSize::new(32, 32).expect("atlas size is valid"),
            ))
            .expect("glyph atlas should be created");
        let mut cache = GlyphCache::new();
        let key = GlyphKey::new(7);
        let bitmap = GlyphBitmap::new(4, 4, &[255; 16]);

        let first = cache
            .cache_glyph(&mut atlas, renderer.queue(), key, bitmap)
            .expect("first cache insertion should succeed");
        let second = cache
            .cache_glyph(&mut atlas, renderer.queue(), key, bitmap)
            .expect("second cache insertion should reuse the existing entry");

        assert_eq!(first, second);
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(key), Some(&first));
    }

    #[test]
    fn glyph_cache_rejects_conflicting_reinsertions() {
        let _gpu_test_lock = crate::test_support::gpu_test_lock();
        let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
            Ok(renderer) => renderer,
            Err(Error::NoAdapter) => return,
            Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
        };
        let mut atlas = renderer
            .create_glyph_atlas(AtlasConfig::new(
                AtlasSize::new(32, 32).expect("atlas size is valid"),
            ))
            .expect("glyph atlas should be created");
        let mut cache = GlyphCache::new();
        let key = GlyphKey::new(9);

        cache
            .cache_glyph(
                &mut atlas,
                renderer.queue(),
                key,
                GlyphBitmap::new(4, 4, &[255; 16]),
            )
            .expect("first cache insertion should succeed");

        let result = cache.cache_glyph(
            &mut atlas,
            renderer.queue(),
            key,
            GlyphBitmap::new(5, 4, &[255; 20]),
        );

        assert!(matches!(
            result,
            Err(Error::GlyphCacheEntryMismatch {
                key: 9,
                cached_width: 4,
                cached_height: 4,
                requested_width: 5,
                requested_height: 4,
            })
        ));
    }

    #[test]
    fn glyph_cache_rejects_invalid_upload_size_without_consuming_atlas_space() {
        let _gpu_test_lock = crate::test_support::gpu_test_lock();
        let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
            Ok(renderer) => renderer,
            Err(Error::NoAdapter) => return,
            Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
        };
        let mut atlas = renderer
            .create_glyph_atlas(AtlasConfig::new(
                AtlasSize::new(4, 4).expect("atlas size is valid"),
            ))
            .expect("glyph atlas should be created");
        let mut cache = GlyphCache::new();

        let result = cache.cache_glyph(
            &mut atlas,
            renderer.queue(),
            GlyphKey::new(11),
            GlyphBitmap::new(4, 4, &[0; 15]),
        );

        assert!(matches!(
            result,
            Err(Error::InvalidAtlasUploadSize {
                expected: 16,
                actual: 15,
            })
        ));
        assert!(cache.is_empty());
        assert!(atlas.allocate(4, 4).is_ok());
    }

    #[test]
    fn glyph_cache_rejects_zero_dimension_bitmaps() {
        let _gpu_test_lock = crate::test_support::gpu_test_lock();
        let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
            Ok(renderer) => renderer,
            Err(Error::NoAdapter) => return,
            Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
        };
        let mut atlas = renderer
            .create_glyph_atlas(AtlasConfig::new(
                AtlasSize::new(8, 8).expect("atlas size is valid"),
            ))
            .expect("glyph atlas should be created");
        let mut cache = GlyphCache::new();

        let result = cache.cache_glyph(
            &mut atlas,
            renderer.queue(),
            GlyphKey::new(12),
            GlyphBitmap::new(0, 1, &[]),
        );

        assert!(matches!(
            result,
            Err(Error::InvalidAtlasAllocation {
                width: 0,
                height: 1,
            })
        ));
        assert!(cache.is_empty());
    }

    #[test]
    fn glyph_cache_reports_when_the_atlas_is_full() {
        let _gpu_test_lock = crate::test_support::gpu_test_lock();
        let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
            Ok(renderer) => renderer,
            Err(Error::NoAdapter) => return,
            Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
        };
        let mut atlas = renderer
            .create_glyph_atlas(AtlasConfig::new(
                AtlasSize::new(4, 4).expect("atlas size is valid"),
            ))
            .expect("glyph atlas should be created");
        let mut cache = GlyphCache::new();

        cache
            .cache_glyph(
                &mut atlas,
                renderer.queue(),
                GlyphKey::new(13),
                GlyphBitmap::new(4, 4, &[255; 16]),
            )
            .expect("first glyph should fill the atlas");

        let result = cache.cache_glyph(
            &mut atlas,
            renderer.queue(),
            GlyphKey::new(14),
            GlyphBitmap::new(1, 1, &[255; 1]),
        );

        assert!(matches!(
            result,
            Err(Error::AtlasFull {
                width: 1,
                height: 1,
            })
        ));
        assert_eq!(cache.len(), 1);
    }
}
