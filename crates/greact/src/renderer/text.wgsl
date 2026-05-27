struct Viewport {
    size: vec2<f32>,
    atlas_size: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> u_view: Viewport;

@group(0) @binding(1)
var t_glyph: texture_2d<f32>;

@group(0) @binding(2)
var s_glyph: sampler;

struct GlyphInstance {
    position: vec2<f32>,
    size: vec2<f32>,
    atlas_origin: vec2<f32>,
    atlas_size: vec2<f32>,
    color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @location(0) position: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) atlas_origin: vec2<f32>,
    @location(3) atlas_size: vec2<f32>,
    @location(4) color: vec4<f32>,
) -> VertexOutput {
    let quad = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 1.0)
    );

    let local = quad[vertex_index];
    let world = position + local * size;
    let ndc = vec2<f32>(
        (world.x / max(u_view.size.x, 1.0)) * 2.0 - 1.0,
        1.0 - (world.y / max(u_view.size.y, 1.0)) * 2.0
    );

    let uv = (atlas_origin + local * atlas_size) / max(u_view.atlas_size, vec2<f32>(1.0, 1.0));

    var out: VertexOutput;
    out.pos = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = uv;
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let alpha = textureSample(t_glyph, s_glyph, in.uv).r;
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
