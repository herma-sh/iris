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

#[cfg(test)]
mod tests {
    use super::FullscreenPipeline;

    #[test]
    fn fullscreen_pipeline_tracks_requested_format() {
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
}
