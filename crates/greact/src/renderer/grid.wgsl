@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4<f32> {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );
    return vec4<f32>(positions[idx], 0.0, 1.0);
}

struct GridUniforms {
    zoom: f32,
    pan_world_x: f32,
    pan_world_y: f32,
    viewport_width: f32,
    viewport_height: f32,
    base_world_step: f32,
    dot_radius_px: f32,
    target_screen_step_px: f32,
    background_color: vec4<f32>,
    dot_color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: GridUniforms;

@fragment
fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let zoom = max(uniforms.zoom, 0.0001);
    let world_pos = (pos.xy / zoom) + vec2<f32>(uniforms.pan_world_x, uniforms.pan_world_y);

    let ratio = uniforms.target_screen_step_px / (uniforms.base_world_step * zoom);
    let level = round(log2(max(ratio, 0.0001)));
    let world_step = uniforms.base_world_step * pow(2.0, level);
    let half_step = world_step * 0.5;

    let grid_local = fract((world_pos + vec2<f32>(half_step, half_step)) / world_step) - vec2<f32>(0.5, 0.5);
    let dist_world = length(grid_local * world_step);
    let dist_screen = dist_world * zoom;

    let edge0 = uniforms.dot_radius_px - 0.5;
    let edge1 = uniforms.dot_radius_px + 0.5;
    let alpha = 1.0 - smoothstep(edge0, edge1, dist_screen);

    let background = vec4<f32>(0.985, 0.987, 0.993, 1.0);
    let dot = vec4<f32>(0.78, 0.80, 0.86, 1.0);
    return mix(background, dot, clamp(alpha, 0.0, 1.0));
}
