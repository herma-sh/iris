struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct PresentUniforms {
    frame_size: vec2<f32>,
    viewport_origin: vec2<f32>,
    scroll_offset: vec4<f32>,
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
    let viewport_size = uniforms.frame_size - uniforms.viewport_origin * vec2<f32>(2.0, 2.0);
    let shifted_pixels = uniforms.viewport_origin
        + vec2<f32>(input.uv.x * viewport_size.x, input.uv.y * viewport_size.y)
        - vec2<f32>(0.0, uniforms.scroll_offset.x);
    if any(shifted_pixels < vec2<f32>(0.0, 0.0))
        || shifted_pixels.x > uniforms.frame_size.x
        || shifted_pixels.y > uniforms.frame_size.y {
        return uniforms.background_color;
    }

    return textureSample(frame_texture, frame_sampler, shifted_pixels / uniforms.frame_size);
}
