struct RectUniform {
    screen_size: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> rect_uniform: RectUniform;

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) rect: vec4<f32>,
    @location(1) fill: vec4<f32>,
    @location(2) border_color: vec4<f32>,
    @location(3) border_width: vec4<f32>,
    @location(4) radius: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) local_pos: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) fill: vec4<f32>,
    @location(3) border_color: vec4<f32>,
    @location(4) border_width: vec4<f32>,
    @location(5) radius: vec4<f32>,
};

fn rect_corner(index: u32) -> vec2<f32> {
    if index == 0u {
        return vec2<f32>(0.0, 0.0);
    }
    if index == 1u {
        return vec2<f32>(1.0, 0.0);
    }
    if index == 2u {
        return vec2<f32>(0.0, 1.0);
    }
    if index == 3u {
        return vec2<f32>(0.0, 1.0);
    }
    if index == 4u {
        return vec2<f32>(1.0, 0.0);
    }
    return vec2<f32>(1.0, 1.0);
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    let corner = rect_corner(input.vertex_index);
    let pixel_position = input.rect.xy + corner * input.rect.zw;
    let ndc = vec2<f32>(
        pixel_position.x / rect_uniform.screen_size.x * 2.0 - 1.0,
        1.0 - pixel_position.y / rect_uniform.screen_size.y * 2.0,
    );

    var out: VertexOutput;
    out.clip_position = vec4<f32>(ndc, 0.0, 1.0);
    out.local_pos = corner * input.rect.zw;
    out.size = input.rect.zw;
    out.fill = input.fill;
    out.border_color = input.border_color;
    out.border_width = input.border_width;
    out.radius = input.radius;
    return out;
}

fn corner_radius(local_pos: vec2<f32>, size: vec2<f32>, radius: vec4<f32>) -> f32 {
    let left = local_pos.x < size.x * 0.5;
    let top = local_pos.y < size.y * 0.5;
    if left && top {
        return radius.x;
    }
    if !left && top {
        return radius.y;
    }
    if !left && !top {
        return radius.z;
    }
    return radius.w;
}

fn rounded_box_distance(local_pos: vec2<f32>, size: vec2<f32>, radius: vec4<f32>) -> f32 {
    let half_size = size * 0.5;
    let p = local_pos - half_size;
    let max_radius = min(half_size.x, half_size.y);
    let r = clamp(corner_radius(local_pos, size, radius), 0.0, max_radius);
    let q = abs(p) - (half_size - vec2<f32>(r, r));
    return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - r;
}

fn sdf_coverage(dist: f32) -> f32 {
    let aa = max(fwidth(dist), 0.75);
    return 1.0 - smoothstep(-aa, aa, dist);
}

fn straight_from_premul(premul: vec3<f32>, alpha: f32) -> vec4<f32> {
    if alpha <= 0.0001 {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
    return vec4<f32>(premul / alpha, alpha);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let dist = rounded_box_distance(input.local_pos, input.size, input.radius);
    let outer_alpha = sdf_coverage(dist);

    let border = max(max(input.border_width.x, input.border_width.y), max(input.border_width.z, input.border_width.w));
    if border <= 0.0 {
        return vec4<f32>(input.fill.rgb, input.fill.a * outer_alpha);
    }

    let inner_size = max(input.size - vec2<f32>(border * 2.0, border * 2.0), vec2<f32>(0.0, 0.0));
    let inner_pos = input.local_pos - vec2<f32>(border, border);
    let inner_radius = max(input.radius - vec4<f32>(border, border, border, border), vec4<f32>(0.0, 0.0, 0.0, 0.0));
    let inner_dist = rounded_box_distance(inner_pos, inner_size, inner_radius);
    let fill_coverage = min(sdf_coverage(inner_dist), outer_alpha);
    let border_coverage = max(outer_alpha - fill_coverage, 0.0);

    let fill_alpha = input.fill.a * fill_coverage;
    let border_alpha = input.border_color.a * border_coverage;
    let out_alpha = fill_alpha + border_alpha * (1.0 - fill_alpha);
    let premul = input.fill.rgb * fill_alpha + input.border_color.rgb * border_alpha * (1.0 - fill_alpha);
    return straight_from_premul(premul, out_alpha);
}
