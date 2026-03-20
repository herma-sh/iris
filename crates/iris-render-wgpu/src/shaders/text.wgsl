struct TextUniforms {
    resolution: vec2<f32>,
    cell_size: vec2<f32>,
    scroll_offset: f32,
    _padding: u32,
}

@group(0) @binding(0) var<uniform> uniforms: TextUniforms;
@group(1) @binding(0) var atlas_texture: texture_2d<f32>;
@group(1) @binding(1) var atlas_sampler: sampler;

struct InstanceInput {
    @location(0) grid_position: vec2<f32>,
    @location(1) atlas_min: vec2<f32>,
    @location(2) atlas_max: vec2<f32>,
    @location(3) fg_color: vec4<f32>,
    @location(4) bg_color: vec4<f32>,
    @location(5) cell_span: f32,
    @location(6) style_flags: u32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) fg_color: vec4<f32>,
    @location(2) bg_color: vec4<f32>,
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

fn quad_uv(vertex_index: u32, atlas_min: vec2<f32>, atlas_max: vec2<f32>) -> vec2<f32> {
    switch vertex_index {
        case 0u: {
            return vec2<f32>(atlas_min.x, atlas_min.y);
        }
        case 1u: {
            return vec2<f32>(atlas_max.x, atlas_min.y);
        }
        case 2u: {
            return vec2<f32>(atlas_min.x, atlas_max.y);
        }
        case 3u: {
            return vec2<f32>(atlas_min.x, atlas_max.y);
        }
        case 4u: {
            return vec2<f32>(atlas_max.x, atlas_min.y);
        }
        default: {
            return vec2<f32>(atlas_max.x, atlas_max.y);
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
        (instance.grid_position.x + corner.x * instance.cell_span) * uniforms.cell_size.x,
        (instance.grid_position.y + corner.y) * uniforms.cell_size.y + uniforms.scroll_offset,
    );
    let ndc = vec2<f32>(
        (pixel_position.x / uniforms.resolution.x) * 2.0 - 1.0,
        1.0 - (pixel_position.y / uniforms.resolution.y) * 2.0,
    );

    var output: VertexOutput;
    output.position = vec4<f32>(ndc, 0.0, 1.0);
    output.tex_coords = quad_uv(vertex_index, instance.atlas_min, instance.atlas_max);
    output.fg_color = instance.fg_color;
    output.bg_color = instance.bg_color;

    let _unused_style_flags = instance.style_flags;

    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let glyph = textureSample(atlas_texture, atlas_sampler, input.tex_coords).r;
    return mix(input.bg_color, input.fg_color, glyph);
}
