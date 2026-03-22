use crate::atlas::GlyphAtlas;
use crate::cell::{CellInstance, TextBuffers};

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
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/text.wgsl").into()),
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
