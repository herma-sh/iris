use super::{CursorPipeline, FullscreenPipeline, PresentPipeline, PresentUniforms, TextPipeline};
use crate::atlas::{AtlasConfig, AtlasSize};
use crate::cell::{CellColors, CellInstance, TextBuffers, TextUniforms};
use crate::cursor::{CursorBuffers, CursorInstance};
use crate::glyph::CachedGlyph;
use crate::renderer::{Renderer, RendererConfig};

const CLEARED_BGRA8_UNORM_SRGB_PIXEL: [u8; 4] = [0, 0, 0, 255];

#[test]
fn fullscreen_pipeline_tracks_requested_format() {
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

    let (device, _queue) = match pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("iris-render-wgpu-pipeline-test-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
        },
        None,
    )) {
        Ok(device) => device,
        Err(_) => return,
    };

    let pipeline = FullscreenPipeline::new(&device, wgpu::TextureFormat::Bgra8UnormSrgb);
    assert_eq!(pipeline.format(), wgpu::TextureFormat::Bgra8UnormSrgb);
}

#[test]
fn present_pipeline_tracks_requested_format_and_draws() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let source = renderer
        .create_texture_surface(crate::texture::TextureSurfaceConfig::new(
            crate::texture::TextureSurfaceSize::new(32, 32).expect("surface dimensions are valid"),
        ))
        .expect("source texture surface should be created");
    let destination = renderer
        .create_texture_surface(crate::texture::TextureSurfaceConfig::new(
            crate::texture::TextureSurfaceSize::new(32, 32).expect("surface dimensions are valid"),
        ))
        .expect("destination texture surface should be created");
    renderer.clear_texture_surface(&source, wgpu::Color::RED);
    let pipeline = PresentPipeline::new(renderer.device(), destination.format());
    let uniform_buffer = pipeline.create_uniform_buffer(renderer.device());
    renderer.queue().write_buffer(
        &uniform_buffer,
        0,
        PresentUniforms::new([32.0, 32.0], [0.0, 0.0], 0.0, [0.0, 0.0, 0.0, 1.0]).as_bytes(),
    );
    let uniform_bind_group = pipeline.create_uniform_bind_group(renderer.device(), &uniform_buffer);
    let bind_group = pipeline.create_texture_bind_group(renderer.device(), &source);
    let mut encoder = renderer
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("iris-render-wgpu-present-pipeline-test-encoder"),
        });

    pipeline.render(
        &mut encoder,
        destination.view(),
        &uniform_bind_group,
        &bind_group,
    );
    renderer.queue().submit(std::iter::once(encoder.finish()));

    assert_eq!(pipeline.format(), wgpu::TextureFormat::Bgra8UnormSrgb);
    let pixels = crate::test_support::read_texture_surface(&renderer, &destination);
    assert!(
        pixels
            .chunks_exact(CLEARED_BGRA8_UNORM_SRGB_PIXEL.len())
            .any(|pixel| pixel != CLEARED_BGRA8_UNORM_SRGB_PIXEL),
        "present draw should copy the sampled frame into the destination"
    );
}

#[test]
fn text_pipeline_tracks_requested_format() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let atlas = renderer
        .create_glyph_atlas(AtlasConfig::new(
            AtlasSize::new(32, 32).expect("atlas size is valid"),
        ))
        .expect("glyph atlas should be created");

    let pipeline = TextPipeline::new(
        renderer.device(),
        wgpu::TextureFormat::Bgra8UnormSrgb,
        &atlas,
    );

    assert_eq!(pipeline.format(), wgpu::TextureFormat::Bgra8UnormSrgb);
}

#[test]
fn text_pipeline_creates_uniform_bind_groups_and_draws() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let mut atlas = renderer
        .create_glyph_atlas(AtlasConfig::new(
            AtlasSize::new(32, 32).expect("atlas size is valid"),
        ))
        .expect("glyph atlas should be created");
    let region = atlas.allocate(4, 4).expect("region should fit");
    atlas
        .upload(renderer.queue(), region, &[255; 16])
        .expect("glyph upload should succeed");
    let mut buffers =
        TextBuffers::new(renderer.device(), 1).expect("text buffers should be created");
    let instance = CellInstance::from_cell(
        iris_core::cell::Cell::new('a'),
        0,
        0,
        CachedGlyph::new(region),
        atlas.size(),
        CellColors::new([1.0; 4], [0.0; 4]),
    )
    .expect("cell instance should be created");
    buffers.write_uniforms(
        renderer.queue(),
        &TextUniforms::new([64.0, 64.0], [8.0, 16.0], 0.0),
    );
    buffers
        .write_instances(renderer.device(), renderer.queue(), &[instance])
        .expect("instance upload should succeed");
    let pipeline = TextPipeline::new(
        renderer.device(),
        wgpu::TextureFormat::Bgra8UnormSrgb,
        &atlas,
    );
    let uniform_bind_group = pipeline.create_uniform_bind_group(renderer.device(), &buffers);
    let surface = renderer
        .create_texture_surface(crate::texture::TextureSurfaceConfig::new(
            crate::texture::TextureSurfaceSize::new(64, 64).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let mut encoder = renderer
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("iris-render-wgpu-text-pipeline-test-encoder"),
        });

    pipeline.render(
        &mut encoder,
        surface.view(),
        &uniform_bind_group,
        &atlas,
        &buffers,
        wgpu::Color::BLACK,
    );
    renderer.queue().submit(std::iter::once(encoder.finish()));

    let pixels = crate::test_support::read_texture_surface(&renderer, &surface);
    assert!(
        pixels
            .chunks_exact(CLEARED_BGRA8_UNORM_SRGB_PIXEL.len())
            .any(|pixel| pixel != CLEARED_BGRA8_UNORM_SRGB_PIXEL),
        "text draw should write pixels beyond the cleared black target"
    );
}

#[test]
fn text_pipeline_draws_with_zero_instances() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let atlas = renderer
        .create_glyph_atlas(AtlasConfig::new(
            AtlasSize::new(32, 32).expect("atlas size is valid"),
        ))
        .expect("glyph atlas should be created");
    let buffers = TextBuffers::new(renderer.device(), 1).expect("text buffers should be created");
    buffers.write_uniforms(
        renderer.queue(),
        &TextUniforms::new([64.0, 64.0], [8.0, 16.0], 0.0),
    );
    let pipeline = TextPipeline::new(
        renderer.device(),
        wgpu::TextureFormat::Bgra8UnormSrgb,
        &atlas,
    );
    let uniform_bind_group = pipeline.create_uniform_bind_group(renderer.device(), &buffers);
    let surface = renderer
        .create_texture_surface(crate::texture::TextureSurfaceConfig::new(
            crate::texture::TextureSurfaceSize::new(64, 64).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    let mut encoder = renderer
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("iris-render-wgpu-text-pipeline-empty-encoder"),
        });

    pipeline.render(
        &mut encoder,
        surface.view(),
        &uniform_bind_group,
        &atlas,
        &buffers,
        wgpu::Color::BLACK,
    );
    renderer.queue().submit(std::iter::once(encoder.finish()));

    let pixels = crate::test_support::read_texture_surface(&renderer, &surface);
    assert!(
        pixels
            .chunks_exact(CLEARED_BGRA8_UNORM_SRGB_PIXEL.len())
            .all(|pixel| pixel == CLEARED_BGRA8_UNORM_SRGB_PIXEL),
        "zero-instance text draw should leave the target at the pass clear color"
    );
}

#[test]
fn cursor_pipeline_tracks_requested_format_and_draws() {
    let _gpu_test_lock = crate::test_support::gpu_test_lock();
    let renderer = match pollster::block_on(Renderer::new(RendererConfig::default())) {
        Ok(renderer) => renderer,
        Err(crate::error::Error::NoAdapter) => return,
        Err(error) => panic!("renderer bootstrap failed unexpectedly: {error}"),
    };
    let text_buffers =
        TextBuffers::new(renderer.device(), 1).expect("text buffers should be created");
    text_buffers.write_uniforms(
        renderer.queue(),
        &TextUniforms::new([64.0, 64.0], [8.0, 16.0], 0.0),
    );
    let mut cursor_buffers = CursorBuffers::new(renderer.device());
    cursor_buffers.write_instance(
        renderer.queue(),
        Some(&CursorInstance {
            grid_position: [1.0, 1.0],
            offset: [0.0, 0.0],
            extent: [1.0, 1.0],
            color: [1.0, 0.0, 0.0, 1.0],
        }),
    );
    let pipeline = CursorPipeline::new(renderer.device(), wgpu::TextureFormat::Bgra8UnormSrgb);
    let uniform_bind_group = pipeline.create_uniform_bind_group(renderer.device(), &text_buffers);
    let surface = renderer
        .create_texture_surface(crate::texture::TextureSurfaceConfig::new(
            crate::texture::TextureSurfaceSize::new(64, 64).expect("surface dimensions are valid"),
        ))
        .expect("texture surface should be created");
    renderer.clear_texture_surface(&surface, wgpu::Color::BLACK);
    let mut encoder = renderer
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("iris-render-wgpu-cursor-pipeline-test-encoder"),
        });

    pipeline.render(
        &mut encoder,
        surface.view(),
        &uniform_bind_group,
        &cursor_buffers,
    );
    renderer.queue().submit(std::iter::once(encoder.finish()));

    assert_eq!(pipeline.format(), wgpu::TextureFormat::Bgra8UnormSrgb);
    let pixels = crate::test_support::read_texture_surface(&renderer, &surface);
    assert!(
        pixels
            .chunks_exact(CLEARED_BGRA8_UNORM_SRGB_PIXEL.len())
            .any(|pixel| pixel != CLEARED_BGRA8_UNORM_SRGB_PIXEL),
        "cursor draw should write pixels beyond the cleared black target"
    );
}
