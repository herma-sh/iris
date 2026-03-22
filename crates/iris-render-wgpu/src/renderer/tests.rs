
use super::{Renderer, RendererConfig};
use crate::atlas::{AtlasConfig, AtlasSize};
use crate::cell::{CellColors, CellInstance, TextUniforms};
use crate::error::Error;
use crate::glyph::{GlyphBitmap, GlyphKey};
use crate::pipeline::PresentUniforms;
use crate::texture::{TextureSurfaceConfig, TextureSurfaceSize};
use crate::theme::Theme;
use iris_core::damage::DamageRegion;
use iris_core::grid::{Grid, GridSize};

const CLEARED_BGRA8_UNORM_SRGB_PIXEL: [u8; 4] = [0, 0, 0, 255];

#[test]
fn renderer_config_defaults_are_headless_safe() {
    let config = RendererConfig::default();
    assert_eq!(config.required_features, wgpu::Features::empty());
    assert_eq!(config.required_limits, wgpu::Limits::default());
    assert!(!config.force_fallback_adapter);
}

#[test]
fn renderer_bootstrap_creates_a_texture_surface() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };

    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(64, 32).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");

    assert_eq!(surface.size().width, 64);
    assert_eq!(surface.size().height, 32);

    renderer.clear_texture_surface(&surface, wgpu::Color::BLACK);
}

#[test]
fn renderer_reports_request_device_error_for_unsupported_features() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    let adapter = match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: None,
    })) {
        Some(adapter) => adapter,
        None => return,
    };

    let unsupported_features = wgpu::Features::all() & !adapter.features();
    if unsupported_features.is_empty() {
        return;
    }

    let result = pollster::block_on(Renderer::new(RendererConfig {
        required_features: unsupported_features,
        ..RendererConfig::default()
    }));

    assert!(matches!(
        result,
        Err(Error::RequestDevice { .. }) | Err(Error::NoAdapter)
    ));
}

#[test]
fn renderer_rejects_texture_surfaces_without_render_attachment_usage() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };

    let result = renderer.create_texture_surface(TextureSurfaceConfig {
        size: TextureSurfaceSize::new(64, 32).expect("surface dimensions are valid"),
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        usage: wgpu::TextureUsages::COPY_SRC,
    });

    assert!(matches!(result, Err(Error::InvalidTextureSurfaceUsage)));
}

#[test]
fn renderer_creates_and_draws_the_fullscreen_pipeline() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };

    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(64, 64).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let pipeline = renderer.create_fullscreen_pipeline(surface.format());

    assert_eq!(pipeline.format(), surface.format());
    renderer.draw_fullscreen_pipeline_to_texture_surface(&pipeline, &surface);
}

#[test]
fn renderer_creates_a_glyph_atlas() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };

    let atlas = renderer
        .create_glyph_atlas(AtlasConfig::new(
            AtlasSize::new(128, 64).expect("atlas size is valid"),
        ))
        .expect("glyph atlas should be created");

    assert_eq!(atlas.size().width, 128);
    assert_eq!(atlas.size().height, 64);
    assert_eq!(atlas.format(), wgpu::TextureFormat::R8Unorm);
}

#[test]
fn renderer_caches_a_glyph_in_the_atlas() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let mut atlas = renderer
        .create_glyph_atlas(AtlasConfig::new(
            AtlasSize::new(64, 64).expect("atlas size is valid"),
        ))
        .expect("glyph atlas should be created");
    let mut cache = renderer.create_glyph_cache();

    let entry = renderer
        .cache_glyph(
            &mut cache,
            &mut atlas,
            GlyphKey::new(42),
            GlyphBitmap::new(4, 4, &[255; 16]),
        )
        .expect("glyph should be cached");

    assert_eq!(entry.region().width, 4);
    assert_eq!(entry.region().height, 4);
    assert_eq!(cache.len(), 1);
    assert!(cache.contains(GlyphKey::new(42)));
}

#[test]
fn renderer_creates_and_updates_text_buffers() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let mut buffers = renderer
        .create_text_buffers(1)
        .expect("text buffers should be created");
    let instance = CellInstance::from_cell(
        iris_core::cell::Cell::new('a'),
        2,
        3,
        crate::glyph::CachedGlyph::new(crate::atlas::AtlasRegion {
            x: 0,
            y: 0,
            width: 8,
            height: 16,
        }),
        AtlasSize::new(32, 32).expect("atlas size is valid"),
        CellColors::new([1.0; 4], [0.0; 4]),
    )
    .expect("cell should encode into an instance");

    renderer.write_text_uniforms(
        &buffers,
        &TextUniforms::new([800.0, 600.0], [9.0, 18.0], 10.0),
    );
    renderer
        .write_text_instances(&mut buffers, &[instance, instance])
        .expect("text instances should upload");

    assert_eq!(buffers.instance_count(), 2);
    assert!(buffers.instance_capacity() >= 2);
}

#[test]
fn renderer_creates_and_draws_the_text_pipeline() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let mut atlas = renderer
        .create_glyph_atlas(AtlasConfig::new(
            AtlasSize::new(64, 64).expect("atlas size is valid"),
        ))
        .expect("glyph atlas should be created");
    let region = atlas.allocate(4, 4).expect("region should fit");
    atlas
        .upload(renderer.queue(), region, &[255; 16])
        .expect("glyph upload should succeed");
    let mut buffers = renderer
        .create_text_buffers(1)
        .expect("text buffers should be created");
    let instance = CellInstance::from_cell(
        iris_core::cell::Cell::new('a'),
        1,
        1,
        crate::glyph::CachedGlyph::new(region),
        atlas.size(),
        CellColors::new([1.0; 4], [0.0; 4]),
    )
    .expect("cell should encode into an instance");
    renderer.write_text_uniforms(&buffers, &TextUniforms::new([64.0, 64.0], [8.0, 16.0], 0.0));
    renderer
        .write_text_instances(&mut buffers, &[instance])
        .expect("instance upload should succeed");
    let pipeline = renderer.create_text_pipeline(wgpu::TextureFormat::Bgra8UnormSrgb, &atlas);
    let uniform_bind_group = renderer.create_text_uniform_bind_group(&pipeline, &buffers);
    let surface = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(64, 64).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");

    renderer.draw_text_pipeline_to_texture_surface(
        &pipeline,
        &uniform_bind_group,
        &atlas,
        &buffers,
        &surface,
    );

    let pixels = crate::test_support::read_texture_surface(&renderer, &surface);
    assert!(
        pixels
            .chunks_exact(CLEARED_BGRA8_UNORM_SRGB_PIXEL.len())
            .any(|pixel| pixel != CLEARED_BGRA8_UNORM_SRGB_PIXEL),
        "renderer text draw helper should write pixels beyond the cleared black target"
    );
}

#[test]
fn renderer_creates_and_draws_the_present_pipeline() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let source = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(32, 32).expect("surface dimensions are valid"),
        ))
        .expect("source texture surface should be created");
    let destination = renderer
        .create_texture_surface(TextureSurfaceConfig::new(
            TextureSurfaceSize::new(32, 32).expect("surface dimensions are valid"),
        ))
        .expect("destination texture surface should be created");
    renderer.clear_texture_surface(&source, wgpu::Color::RED);
    let pipeline = renderer.create_present_pipeline(destination.format());
    let uniform_buffer = pipeline.create_uniform_buffer(renderer.device());
    let uniform_bind_group = pipeline.create_uniform_bind_group(renderer.device(), &uniform_buffer);
    renderer.queue().write_buffer(
        &uniform_buffer,
        0,
        PresentUniforms::new([32.0, 32.0], [0.0, 0.0], 0.0, [0.0, 0.0, 0.0, 1.0]).as_bytes(),
    );
    let bind_group = pipeline.create_texture_bind_group(renderer.device(), &source);

    renderer.draw_present_pipeline_to_texture_surface(
        &pipeline,
        &uniform_bind_group,
        &bind_group,
        &destination,
    );

    let pixels = crate::test_support::read_texture_surface(&renderer, &destination);
    assert!(
        pixels
            .chunks_exact(CLEARED_BGRA8_UNORM_SRGB_PIXEL.len())
            .any(|pixel| pixel != CLEARED_BGRA8_UNORM_SRGB_PIXEL),
        "present pipeline helper should write pixels into the destination target"
    );
}

#[test]
fn renderer_encodes_text_instances_from_grid_damage() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let atlas = renderer
        .create_glyph_atlas(AtlasConfig::new(
            AtlasSize::new(64, 64).expect("atlas size is valid"),
        ))
        .expect("glyph atlas should be created");
    let mut grid = Grid::new(GridSize { rows: 1, cols: 2 }).expect("grid should be created");
    grid.write(0, 0, iris_core::cell::Cell::new('a'))
        .expect("grid write should succeed");
    let mut instances = Vec::new();

    renderer
        .encode_text_instances_for_damage(
            &mut instances,
            &grid,
            &[DamageRegion::new(0, 0, 0, 0)],
            &atlas,
            &Theme::default(),
            |_| {
                Some(crate::glyph::CachedGlyph::new(crate::atlas::AtlasRegion {
                    x: 0,
                    y: 0,
                    width: 8,
                    height: 16,
                }))
            },
        )
        .expect("damage should encode");

    assert_eq!(instances.len(), 1);
    assert_eq!(instances[0].grid_position, [0.0, 0.0]);
}
