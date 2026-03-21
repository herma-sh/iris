struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct PresentUniforms {
    frame_size: vec2<f32>,
    scroll_offset: f32,
    _padding: u32,
    background_color: vec4<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: PresentUniforms;
@group(1) @binding(0) var frame_texture: texture_2d<f32>;
@group(1) @binding(1) var frame_sampler: sampler;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(3.0, 1.0),
    );
    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 2.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(2.0, 0.0),
    );

    var output: VertexOutput;
    output.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    output.uv = uvs[vertex_index];
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let shifted_uv = input.uv - vec2<f32>(0.0, uniforms.scroll_offset / uniforms.frame_size.y);
    if any(shifted_uv < vec2<f32>(0.0, 0.0)) || any(shifted_uv > vec2<f32>(1.0, 1.0)) {
        return uniforms.background_color;
    }

    return textureSample(frame_texture, frame_sampler, shifted_uv);
}
