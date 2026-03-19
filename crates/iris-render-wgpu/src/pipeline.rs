use crate::atlas::GlyphAtlas;
use crate::cell::{CellInstance, TextBuffers};

/// Minimal fullscreen pipeline used to bootstrap shader and render-pipeline
/// wiring before cell, glyph, and atlas rendering land.
#[derive(Debug)]
pub struct FullscreenPipeline {
    pipeline: wgpu::RenderPipeline,
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

/// Text pipeline bootstrap using atlas-backed glyph sampling and per-cell instances.
#[derive(Debug)]
pub struct TextPipeline {
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
    ) {
        let instance_count = u32::try_from(buffers.instance_count()).unwrap_or(u32::MAX);
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("iris-render-wgpu-text-pass"),
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
        render_pass.set_bind_group(0, uniform_bind_group, &[]);
        render_pass.set_bind_group(1, atlas.bind_group(), &[]);
        render_pass.set_vertex_buffer(0, buffers.instance_buffer().slice(..));
        render_pass.draw(0..6, 0..instance_count);
    }
}

#[cfg(test)]
mod tests {
    use super::{FullscreenPipeline, TextPipeline};
    use crate::atlas::{AtlasConfig, AtlasSize};
    use crate::cell::{CellColors, CellInstance, TextBuffers, TextUniforms};
    use crate::glyph::CachedGlyph;
    use crate::renderer::{Renderer, RendererConfig};

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
        );
        renderer.queue().submit(std::iter::once(encoder.finish()));
    }
}
