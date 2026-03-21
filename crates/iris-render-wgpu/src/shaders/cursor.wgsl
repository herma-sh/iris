struct TextUniforms {
    resolution: vec2<f32>,
    cell_size: vec2<f32>,
    scroll_offset: f32,
    _padding: u32,
}

@group(0) @binding(0) var<uniform> uniforms: TextUniforms;

struct InstanceInput {
    @location(0) grid_position: vec2<f32>,
    @location(1) offset: vec2<f32>,
    @location(2) extent: vec2<f32>,
    @location(3) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

fn quad_position(vertex_index: u32) -> vec2<f32> {
    switch vertex_index {
        case 0u: {
            return vec2<f32>(0.0, 0.0);
        }
        case 1u: {
            return vec2<f32>(1.0, 0.0);
        }
        case 2u: {
            return vec2<f32>(0.0, 1.0);
        }
        case 3u: {
            return vec2<f32>(0.0, 1.0);
        }
        case 4u: {
            return vec2<f32>(1.0, 0.0);
        }
        default: {
            return vec2<f32>(1.0, 1.0);
        }
    }
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    instance: InstanceInput,
) -> VertexOutput {
    let corner = quad_position(vertex_index);
    let pixel_position = vec2<f32>(
        (instance.grid_position.x + instance.offset.x + corner.x * instance.extent.x) * uniforms.cell_size.x,
        (instance.grid_position.y + instance.offset.y + corner.y * instance.extent.y) * uniforms.cell_size.y + uniforms.scroll_offset,
    );
    let ndc = vec2<f32>(
        (pixel_position.x / uniforms.resolution.x) * 2.0 - 1.0,
        1.0 - (pixel_position.y / uniforms.resolution.y) * 2.0,
    );

    var output: VertexOutput;
    output.position = vec4<f32>(ndc, 0.0, 1.0);
    output.color = instance.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
