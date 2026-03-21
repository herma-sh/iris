use crate::atlas::GlyphAtlas;
use crate::cell::{CellInstance, TextBuffers};
use crate::cursor::{CursorBuffers, CursorInstance};
use crate::texture::TextureSurface;

/// Minimal fullscreen pipeline used to bootstrap shader and render-pipeline
/// wiring before cell, glyph, and atlas rendering land.
#[derive(Debug)]
pub struct FullscreenPipeline {
    pipeline: wgpu::RenderPipeline,
    format: wgpu::TextureFormat,
}

/// Fullscreen textured presentation pipeline used to draw a cached frame.
#[derive(Debug)]
pub struct PresentPipeline {
    pipeline: wgpu::RenderPipeline,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    format: wgpu::TextureFormat,
}

impl FullscreenPipeline {
    /// Creates a fullscreen triangle pipeline for the requested render-target format.
    #[must_use]
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("iris-render-wgpu-fullscreen-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/fullscreen.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("iris-render-wgpu-fullscreen-layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("iris-render-wgpu-fullscreen-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

        Self { pipeline, format }
    }

    /// Returns the render-target format this pipeline was built for.
    #[must_use]
    pub const fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    pub(crate) fn render(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("iris-render-wgpu-fullscreen-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        render_pass.set_pipeline(&self.pipeline);
        render_pass.draw(0..3, 0..1);
    }
}

impl PresentPipeline {
    /// Creates a fullscreen textured presentation pipeline for the requested format.
    #[must_use]
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("iris-render-wgpu-present-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/present.wgsl").into()),
        });
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("iris-render-wgpu-present-layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("iris-render-wgpu-present-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            // The cached frame is already fully rendered; nearest preserves exact pixels.
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("iris-render-wgpu-present-pipeline-layout"),
            bind_group_layouts: &[&texture_bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("iris-render-wgpu-present-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

        Self {
            pipeline,
            texture_bind_group_layout,
            sampler,
            format,
        }
    }

    /// Returns the render-target format this pipeline was built for.
    #[must_use]
    pub const fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// Creates the bind group used to sample a cached frame texture.
    #[must_use]
    pub fn create_texture_bind_group(
        &self,
        device: &wgpu::Device,
        texture_surface: &TextureSurface,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("iris-render-wgpu-present-bind-group"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(texture_surface.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    pub(crate) fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        texture_bind_group: &wgpu::BindGroup,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("iris-render-wgpu-present-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, texture_bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}

/// Text pipeline bootstrap using atlas-backed glyph sampling and per-cell instances.
#[derive(Debug)]
pub struct TextPipeline {
    pipeline: wgpu::RenderPipeline,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
}

/// Cursor overlay pipeline for block, underline, and bar cursor shapes.
#[derive(Debug)]
pub struct CursorPipeline {
    pipeline: wgpu::RenderPipeline,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
}

impl TextPipeline {
    /// Creates a text pipeline for the requested render-target format and atlas layout.
    #[must_use]
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, atlas: &GlyphAtlas) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("iris-render-wgpu-text-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/text.wgsl").into()),
        });
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("iris-render-wgpu-text-uniform-layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("iris-render-wgpu-text-layout"),
            bind_group_layouts: &[&uniform_bind_group_layout, atlas.bind_group_layout()],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("iris-render-wgpu-text-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[CellInstance::layout()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

        Self {
            pipeline,
            uniform_bind_group_layout,
            format,
        }
    }

    /// Returns the render-target format this pipeline was built for.
    #[must_use]
    pub const fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// Creates the uniform bind group used by the text pipeline.
    #[must_use]
    pub fn create_uniform_bind_group(
        &self,
        device: &wgpu::Device,
        buffers: &TextBuffers,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("iris-render-wgpu-text-uniform-bind-group"),
            layout: &self.uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffers.uniform_buffer().as_entire_binding(),
            }],
        })
    }

    pub(crate) fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        uniform_bind_group: &wgpu::BindGroup,
        atlas: &GlyphAtlas,
        buffers: &TextBuffers,
        clear_color: wgpu::Color,
    ) {
        self.render_with_load_op(
            encoder,
            view,
            uniform_bind_group,
            atlas,
            buffers,
            wgpu::LoadOp::Clear(clear_color),
        );
    }

    pub(crate) fn render_with_load_op(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        uniform_bind_group: &wgpu::BindGroup,
        atlas: &GlyphAtlas,
        buffers: &TextBuffers,
        load_op: wgpu::LoadOp<wgpu::Color>,
    ) {
        // Cursor overlays upload at most one instance, so the count always fits in `u32`.
        let instance_count = buffers.instance_count() as u32;
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("iris-render-wgpu-text-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: load_op,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, uniform_bind_group, &[]);
        render_pass.set_bind_group(1, atlas.bind_group(), &[]);
        render_pass.set_vertex_buffer(0, buffers.instance_buffer().slice(..));
        render_pass.draw(0..6, 0..instance_count);
    }
}

impl CursorPipeline {
    /// Creates a cursor overlay pipeline for the requested render-target format.
    #[must_use]
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("iris-render-wgpu-cursor-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/cursor.wgsl").into()),
        });
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("iris-render-wgpu-cursor-uniform-layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("iris-render-wgpu-cursor-layout"),
            bind_group_layouts: &[&uniform_bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("iris-render-wgpu-cursor-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[CursorInstance::layout()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

        Self {
            pipeline,
            uniform_bind_group_layout,
            format,
        }
    }

    /// Returns the render-target format this pipeline was built for.
    #[must_use]
    pub const fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// Creates the uniform bind group used by the cursor pipeline.
    #[must_use]
    pub fn create_uniform_bind_group(
        &self,
        device: &wgpu::Device,
        buffers: &TextBuffers,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("iris-render-wgpu-cursor-uniform-bind-group"),
            layout: &self.uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffers.uniform_buffer().as_entire_binding(),
            }],
        })
    }

    pub(crate) fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        uniform_bind_group: &wgpu::BindGroup,
        buffers: &CursorBuffers,
    ) {
        let instance_count = u32::try_from(buffers.instance_count()).unwrap_or(u32::MAX);
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("iris-render-wgpu-cursor-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, buffers.instance_buffer().slice(..));
        render_pass.draw(0..6, 0..instance_count);
    }
}

#[cfg(test)]
mod tests {
    use super::{CursorPipeline, FullscreenPipeline, PresentPipeline, TextPipeline};
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
        let adapter =
            match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
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
                crate::texture::TextureSurfaceSize::new(32, 32)
                    .expect("surface dimensions are valid"),
            ))
            .expect("source texture surface should be created");
        let destination = renderer
            .create_texture_surface(crate::texture::TextureSurfaceConfig::new(
                crate::texture::TextureSurfaceSize::new(32, 32)
                    .expect("surface dimensions are valid"),
            ))
            .expect("destination texture surface should be created");
        renderer.clear_texture_surface(&source, wgpu::Color::RED);
        let pipeline = PresentPipeline::new(renderer.device(), destination.format());
        let bind_group = pipeline.create_texture_bind_group(renderer.device(), &source);
        let mut encoder =
            renderer
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("iris-render-wgpu-present-pipeline-test-encoder"),
                });

        pipeline.render(&mut encoder, destination.view(), &bind_group);
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
                crate::texture::TextureSurfaceSize::new(64, 64)
                    .expect("surface dimensions are valid"),
            ))
            .expect("texture surface should be created");
        let mut encoder =
            renderer
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
        let buffers =
            TextBuffers::new(renderer.device(), 1).expect("text buffers should be created");
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
                crate::texture::TextureSurfaceSize::new(64, 64)
                    .expect("surface dimensions are valid"),
            ))
            .expect("texture surface should be created");
        let mut encoder =
            renderer
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
        let uniform_bind_group =
            pipeline.create_uniform_bind_group(renderer.device(), &text_buffers);
        let surface = renderer
            .create_texture_surface(crate::texture::TextureSurfaceConfig::new(
                crate::texture::TextureSurfaceSize::new(64, 64)
                    .expect("surface dimensions are valid"),
            ))
            .expect("texture surface should be created");
        renderer.clear_texture_surface(&surface, wgpu::Color::BLACK);
        let mut encoder =
            renderer
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
}
