struct Viewport {
    size: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> u_viewport: Viewport;

struct RectInstance {
    border_width: f32,
    shadow_blur: f32,
    shadow_spread: f32,
    _pad0: f32,
    position: vec2<f32>,
    size: vec2<f32>,
    shape_origin: vec2<f32>,
    shape_size: vec2<f32>,
    shadow_offset: vec2<f32>,
    radius: vec4<f32>,
    background_color: vec4<f32>,
    border_color: vec4<f32>,
    shadow_color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) local_pos: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) radius: vec4<f32>,
    @location(3) bg: vec4<f32>,
    @location(4) border: vec4<f32>,
    @location(5) border_width: f32,
    @location(6) shadow_blur: f32,
    @location(7) shadow_spread: f32,
    @location(8) shadow_offset: vec2<f32>,
    @location(9) shadow_color: vec4<f32>,
    @location(10) shape_origin: vec2<f32>,
    @location(11) shape_size: vec2<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @location(0) border_width: f32,
    @location(1) shadow_blur: f32,
    @location(2) shadow_spread: f32,
    @location(3) _pad0: f32,
    @location(4) position: vec2<f32>,
    @location(5) size: vec2<f32>,
    @location(6) shape_origin: vec2<f32>,
    @location(7) shape_size: vec2<f32>,
    @location(8) shadow_offset: vec2<f32>,
    @location(9) radius: vec4<f32>,
    @location(10) background_color: vec4<f32>,
    @location(11) border_color: vec4<f32>,
    @location(12) shadow_color: vec4<f32>,
) -> VertexOutput {
    let vertices = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 1.0)
    );

    let uv = vertices[vertex_index];
    let world_pos = position + uv * size;
    let ndc_pos = vec2<f32>(
        (world_pos.x / max(u_viewport.size.x, 1.0)) * 2.0 - 1.0,
        1.0 - (world_pos.y / max(u_viewport.size.y, 1.0)) * 2.0
    );

    var out: VertexOutput;
    out.pos = vec4<f32>(ndc_pos, 0.0, 1.0);
    out.local_pos = uv * size;
    out.size = size;
    out.radius = radius;
    out.bg = background_color;
    out.border = border_color;
    out.border_width = border_width;
    out.shadow_blur = shadow_blur;
    out.shadow_spread = shadow_spread;
    out.shadow_offset = shadow_offset;
    out.shadow_color = shadow_color;
    out.shape_origin = shape_origin;
    out.shape_size = shape_size;
    return out;
}

fn sd_rounded_rect(p: vec2<f32>, size: vec2<f32>, radius: vec4<f32>) -> f32 {
    let c = size * 0.5;
    let q = p - c;
    let top = select(radius.x, radius.y, q.x > 0.0);
    let bottom = select(radius.w, radius.z, q.x > 0.0);
    let r = max(0.0, select(bottom, top, q.y < 0.0));

    // Correct rounded-rect SDF: shrink core by current corner radius.
    let half = max(c - vec2<f32>(r, r), vec2<f32>(0.0, 0.0));
    let d = abs(q) - half;
    return length(max(d, vec2<f32>(0.0, 0.0))) + min(max(d.x, d.y), 0.0) - r;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let shape_pos = in.local_pos - in.shape_origin;
    let aa = 1.0;
    let bw = max(in.border_width, 0.0);

    // Outer silhouette.
    let outer_dist = sd_rounded_rect(shape_pos, in.shape_size, in.radius);
    let outer_cov = 1.0 - smoothstep(-aa, aa, outer_dist);

    // Inner silhouette (inset by border width, with radius reduced by border width).
    let inner_size = max(in.shape_size - vec2<f32>(2.0 * bw, 2.0 * bw), vec2<f32>(1.0, 1.0));
    let inner_radius = max(in.radius - vec4<f32>(bw, bw, bw, bw), vec4<f32>(0.0, 0.0, 0.0, 0.0));
    let inner_pos = shape_pos - vec2<f32>(bw, bw);
    let inner_dist = sd_rounded_rect(inner_pos, inner_size, inner_radius);
    let inner_cov = 1.0 - smoothstep(-aa, aa, inner_dist);

    // Border is exactly between outer and inner silhouette; no seam with fill.
    let fill_alpha = inner_cov;
    let border_alpha = clamp(outer_cov - inner_cov, 0.0, 1.0);
    let shape_alpha = clamp(fill_alpha + border_alpha, 0.0, 1.0);
    let shape_rgb = in.bg.rgb * fill_alpha + in.border.rgb * border_alpha;
    let shape = vec4<f32>(shape_rgb, shape_alpha);

    let spread = in.shadow_spread;
    let shadow_size = in.shape_size + vec2<f32>(spread * 2.0, spread * 2.0);
    let shadow_radius = max(
        in.radius + vec4<f32>(spread, spread, spread, spread),
        vec4<f32>(0.0, 0.0, 0.0, 0.0)
    );
    let shadow_pos = shape_pos - in.shadow_offset + vec2<f32>(spread, spread);
    let shadow_dist = sd_rounded_rect(shadow_pos, shadow_size, shadow_radius);
    let blur = max(in.shadow_blur, 0.0);
    let outside = max(shadow_dist, 0.0);

    // CSS-like soft shadow profile: gaussian falloff outside silhouette.
    // sigma ~= blur * 0.5 (tuned visually closer to browser box-shadow).
    let sigma = max(blur * 0.5, 0.5);
    var shadow_alpha = exp(-0.5 * pow(outside / sigma, 2.0));
    if blur <= 0.0 {
        shadow_alpha = select(0.0, 1.0, shadow_dist <= 0.0);
    }
    shadow_alpha = clamp(shadow_alpha, 0.0, 1.0);
    let shadow = vec4<f32>(in.shadow_color.rgb, in.shadow_color.a * shadow_alpha);

    let out_a = shape.a + shadow.a * (1.0 - shape.a);
    if out_a <= 0.0001 {
        discard;
    }
    let out_rgb = shape.rgb * shape.a + shadow.rgb * shadow.a * (1.0 - shape.a);
    return vec4<f32>(out_rgb / out_a, out_a);
}
