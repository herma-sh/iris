
use super::{GlyphBitmap, GlyphCache, GlyphKey, GlyphPlacement, RasterizedGlyph};
use crate::atlas::{AtlasConfig, AtlasSize};
use crate::error::Error;
use crate::renderer::{Renderer, RendererConfig};

#[test]
fn rasterized_glyph_exposes_borrowed_bitmap_view() {
    let glyph = RasterizedGlyph::new_with_placement(
        2,
        3,
        vec![0, 1, 2, 3, 4, 5],
        GlyphPlacement {
            left_px: 1,
            top_px: 2,
        },
    );
    let bitmap = glyph.as_bitmap();

    assert_eq!(glyph.width(), 2);
    assert_eq!(glyph.height(), 3);
    assert_eq!(glyph.data(), &[0, 1, 2, 3, 4, 5]);
    assert_eq!(
        glyph.placement(),
        GlyphPlacement {
            left_px: 1,
            top_px: 2
        }
    );
    assert_eq!(bitmap.width, 2);
    assert_eq!(bitmap.height, 3);
    assert_eq!(bitmap.data, &[0, 1, 2, 3, 4, 5]);
}

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
fn glyph_cache_rejects_mixed_api_reinsertion_with_different_placement() {
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
    let key = GlyphKey::new(10);

    cache
        .cache_glyph(
            &mut atlas,
            renderer.queue(),
            key,
            GlyphBitmap::new(4, 4, &[255; 16]),
        )
        .expect("first cache insertion should succeed");

    let result = cache.cache_glyph_with_placement(
        &mut atlas,
        renderer.queue(),
        key,
        GlyphBitmap::new(4, 4, &[255; 16]),
        GlyphPlacement {
            left_px: 1,
            top_px: 2,
        },
    );

    assert!(matches!(
        result,
        Err(Error::GlyphCachePlacementMismatch {
            key: 10,
            cached_left_px: 0,
            cached_top_px: 0,
            requested_left_px: 1,
            requested_top_px: 2,
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
