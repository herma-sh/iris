
use super::{AtlasAllocator, AtlasConfig, AtlasSize, GlyphAtlas};
use crate::error::Error;

#[test]
fn atlas_size_rejects_zero_width() {
    let result = AtlasSize::new(0, 32);
    assert!(matches!(
        result,
        Err(Error::InvalidAtlasSize {
            width: 0,
            height: 32
        })
    ));
}

#[test]
fn atlas_size_rejects_zero_height() {
    let result = AtlasSize::new(32, 0);
    assert!(matches!(
        result,
        Err(Error::InvalidAtlasSize {
            width: 32,
            height: 0
        })
    ));
}

#[test]
fn atlas_allocator_places_regions_in_rows() {
    let mut allocator = AtlasAllocator::new(AtlasSize::new(8, 8).expect("atlas size is valid"));

    let first = allocator.allocate(3, 2).expect("first region should fit");
    let second = allocator.allocate(2, 2).expect("second region should fit");

    assert_eq!((first.x, first.y), (0, 0));
    assert_eq!((second.x, second.y), (3, 0));
}

#[test]
fn atlas_allocator_wraps_to_the_next_row() {
    let mut allocator = AtlasAllocator::new(AtlasSize::new(8, 8).expect("atlas size is valid"));

    allocator.allocate(6, 2).expect("first region should fit");
    let wrapped = allocator.allocate(4, 3).expect("wrapped region should fit");

    assert_eq!((wrapped.x, wrapped.y), (0, 2));
}

#[test]
fn atlas_allocator_advances_by_the_tallest_item_in_a_row() {
    let mut allocator = AtlasAllocator::new(AtlasSize::new(10, 10).expect("atlas size is valid"));

    allocator.allocate(3, 5).expect("first region should fit");
    allocator.allocate(3, 2).expect("second region should fit");
    allocator.allocate(3, 3).expect("third region should fit");
    let wrapped = allocator.allocate(3, 1).expect("wrapped region should fit");

    assert_eq!((wrapped.x, wrapped.y), (0, 5));
}

#[test]
fn atlas_allocator_rejects_oversized_regions() {
    let mut allocator = AtlasAllocator::new(AtlasSize::new(8, 8).expect("atlas size is valid"));
    let result = allocator.allocate(9, 1);
    assert!(matches!(
        result,
        Err(Error::InvalidAtlasAllocation {
            width: 9,
            height: 1
        })
    ));
}

#[test]
fn atlas_allocator_rejects_zero_dimensions() {
    let mut allocator = AtlasAllocator::new(AtlasSize::new(8, 8).expect("atlas size is valid"));

    let zero_width = allocator.allocate(0, 1);
    assert!(matches!(
        zero_width,
        Err(Error::InvalidAtlasAllocation {
            width: 0,
            height: 1
        })
    ));

    let zero_height = allocator.allocate(1, 0);
    assert!(matches!(
        zero_height,
        Err(Error::InvalidAtlasAllocation {
            width: 1,
            height: 0
        })
    ));
}

#[test]
fn atlas_allocator_fills_exactly_before_reporting_full() {
    let mut allocator = AtlasAllocator::new(AtlasSize::new(4, 4).expect("atlas size is valid"));

    for _ in 0..4 {
        allocator.allocate(4, 1).expect("row should fit exactly");
    }

    let result = allocator.allocate(1, 1);
    assert!(matches!(
        result,
        Err(Error::AtlasFull {
            width: 1,
            height: 1
        })
    ));
}

#[test]
fn atlas_allocator_reports_full_atlas() {
    let mut allocator = AtlasAllocator::new(AtlasSize::new(4, 4).expect("atlas size is valid"));

    allocator.allocate(4, 2).expect("first row should fit");
    allocator.allocate(4, 2).expect("second row should fit");
    let result = allocator.allocate(1, 1);

    assert!(matches!(
        result,
        Err(Error::AtlasFull {
            width: 1,
            height: 1
        })
    ));
}

#[test]
fn glyph_atlas_rejects_invalid_upload_size() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
    let adapter = match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: None,
    })) {
        Some(adapter) => adapter,
        None => return,
    };
    let (device, queue) = match pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("iris-render-wgpu-atlas-test-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
        },
        None,
    )) {
        Ok(result) => result,
        Err(_) => return,
    };
    let mut atlas = GlyphAtlas::new(
        &device,
        AtlasConfig::new(AtlasSize::new(16, 16).expect("atlas size is valid")),
    )
    .expect("atlas should be created");
    let region = atlas.allocate(4, 4).expect("region should fit");

    let result = atlas.upload(&queue, region, &[0; 15]);
    assert!(matches!(
        result,
        Err(Error::InvalidAtlasUploadSize {
            expected: 16,
            actual: 15
        })
    ));
}
