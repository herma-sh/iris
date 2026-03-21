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
    @location(3) glyph_offset: vec2<f32>,
    @location(4) glyph_extent: vec2<f32>,
    @location(5) fg_color: vec4<f32>,
    @location(6) bg_color: vec4<f32>,
    @location(7) cell_span: f32,
    @location(8) style_flags: u32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) atlas_min: vec2<f32>,
    @location(1) atlas_max: vec2<f32>,
    @location(2) glyph_offset: vec2<f32>,
    @location(3) glyph_extent: vec2<f32>,
    @location(4) local_pixel: vec2<f32>,
    @location(5) fg_color: vec4<f32>,
    @location(6) bg_color: vec4<f32>,
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
    let cell_extent = vec2<f32>(uniforms.cell_size.x * instance.cell_span, uniforms.cell_size.y);
    let local_pixel = corner * cell_extent;
    let pixel_position = vec2<f32>(
        instance.grid_position.x * uniforms.cell_size.x + local_pixel.x,
        instance.grid_position.y * uniforms.cell_size.y + local_pixel.y + uniforms.scroll_offset,
    );
    let ndc = vec2<f32>(
        (pixel_position.x / uniforms.resolution.x) * 2.0 - 1.0,
        1.0 - (pixel_position.y / uniforms.resolution.y) * 2.0,
    );

    var output: VertexOutput;
    output.position = vec4<f32>(ndc, 0.0, 1.0);
    output.atlas_min = instance.atlas_min;
    output.atlas_max = instance.atlas_max;
    output.glyph_offset = instance.glyph_offset;
    output.glyph_extent = instance.glyph_extent;
    output.local_pixel = local_pixel;
    output.fg_color = instance.fg_color;
    output.bg_color = instance.bg_color;

    let _unused_style_flags = instance.style_flags;

    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let glyph_min = input.glyph_offset;
    let glyph_max = input.glyph_offset + input.glyph_extent;
    let inside_glyph = all(input.local_pixel >= glyph_min) && all(input.local_pixel < glyph_max);

    var glyph_alpha = 0.0;
    if inside_glyph {
        let glyph_uv = (input.local_pixel - glyph_min) / max(input.glyph_extent, vec2<f32>(1.0, 1.0));
        let atlas_uv = mix(input.atlas_min, input.atlas_max, glyph_uv);
        glyph_alpha = textureSample(atlas_texture, atlas_sampler, atlas_uv).r;
    }

    return mix(input.bg_color, input.fg_color, glyph_alpha);
}
