use std::collections::HashMap;

use crate::atlas::{AtlasRegion, GlyphAtlas};
use crate::error::{Error, Result};

/// Placement offsets for positioning a glyph bitmap inside a terminal cell.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GlyphPlacement {
    /// Horizontal glyph offset in pixels from the cell's left edge.
    pub left_px: i32,
    /// Vertical glyph offset in pixels from the cell's top edge.
    pub top_px: i32,
}

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

/// Owned single-channel glyph bitmap produced by a rasterizer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RasterizedGlyph {
    width: u32,
    height: u32,
    data: Vec<u8>,
    placement: GlyphPlacement,
}

impl RasterizedGlyph {
    /// Creates an owned glyph bitmap ready for atlas upload.
    #[must_use]
    pub fn new(width: u32, height: u32, data: Vec<u8>) -> Self {
        Self {
            width,
            height,
            data,
            placement: GlyphPlacement::default(),
        }
    }

    /// Creates an owned glyph bitmap with explicit placement offsets.
    #[must_use]
    pub fn new_with_placement(
        width: u32,
        height: u32,
        data: Vec<u8>,
        placement: GlyphPlacement,
    ) -> Self {
        Self {
            width,
            height,
            data,
            placement,
        }
    }

    /// Returns the bitmap width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Returns the bitmap height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Returns the owned bitmap bytes.
    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Returns the glyph placement offsets used when drawing inside a cell.
    #[must_use]
    pub const fn placement(&self) -> GlyphPlacement {
        self.placement
    }

    /// Borrows the owned bitmap as an atlas-upload descriptor.
    #[must_use]
    pub fn as_bitmap(&self) -> GlyphBitmap<'_> {
        GlyphBitmap::new(self.width, self.height, &self.data)
    }
}

/// Atlas-backed cache entry for a rasterized glyph variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CachedGlyph {
    region: AtlasRegion,
    placement: GlyphPlacement,
}

impl CachedGlyph {
    /// Creates a cached glyph entry for the provided atlas region.
    #[must_use]
    pub const fn new(region: AtlasRegion) -> Self {
        Self {
            region,
            placement: GlyphPlacement {
                left_px: 0,
                top_px: 0,
            },
        }
    }

    /// Creates a cached glyph entry for the provided atlas region and placement.
    #[must_use]
    pub const fn with_placement(region: AtlasRegion, placement: GlyphPlacement) -> Self {
        Self { region, placement }
    }

    /// Returns the atlas region occupied by this cached glyph.
    #[must_use]
    pub const fn region(self) -> AtlasRegion {
        self.region
    }

    /// Returns the cached glyph placement offsets.
    #[must_use]
    pub const fn placement(self) -> GlyphPlacement {
        self.placement
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
        self.cache_glyph_with_placement(atlas, queue, key, bitmap, GlyphPlacement::default())
    }

    /// Caches a glyph bitmap with explicit placement offsets.
    pub fn cache_glyph_with_placement(
        &mut self,
        atlas: &mut GlyphAtlas,
        queue: &wgpu::Queue,
        key: GlyphKey,
        bitmap: GlyphBitmap<'_>,
        placement: GlyphPlacement,
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
            if entry.placement != placement {
                return Err(Error::GlyphCachePlacementMismatch {
                    key: key.value(),
                    cached_left_px: entry.placement.left_px,
                    cached_top_px: entry.placement.top_px,
                    requested_left_px: placement.left_px,
                    requested_top_px: placement.top_px,
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

        let entry = CachedGlyph::with_placement(region, placement);
        self.entries.insert(key, entry);

        Ok(entry)
    }
}

#[cfg(test)]
#[path = "test/glyph/tests.rs"]
mod tests;
