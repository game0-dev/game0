struct QuadInstance {
    border_width: f32,
    shadow_blur: f32,
    position: vec2<f32>,
    size: vec2<f32>,
    shadow_offset: vec2<f32>,
    radius: vec4<f32>,
    background_color: vec4<f32>,
    border_color: vec4<f32>,
    shadow_color: vec4<f32>,
};

// Vertex output structure
struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) v_position: vec2<f32>,
    @location(1) v_size: vec2<f32>,
    @location(2) v_radius: vec4<f32>,
    @location(3) v_background_color: vec4<f32>,
    @location(4) v_border_color: vec4<f32>,
    @location(5) v_shadow_color: vec4<f32>,
    @location(6) v_border_width: f32,
    @location(7) v_shadow_offset: vec2<f32>,
    @location(8) v_shadow_blur: f32,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @location(0) border_width: f32,
    @location(1) shadow_blur: f32,
    @location(2) position: vec2<f32>,
    @location(3) size: vec2<f32>,
    @location(4) shadow_offset: vec2<f32>,
    @location(5) radius: vec4<f32>,
    @location(6) background_color: vec4<f32>,
    @location(7) border_color: vec4<f32>,
    @location(8) shadow_color: vec4<f32>,
) -> VertexOutput {
    // Unit rectangle vertices
    let vertices = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 1.0)
    );

    // Calculate world position
    let world_pos = position + vertices[vertex_index] * size;

    // Convert to NDC space (-1 to 1)
    let ndc_pos = vec2<f32>(
        (world_pos.x / 1024.0) * 2.0 - 1.0,
        1.0 - (world_pos.y / 768.0) * 2.0
    );

    var result: VertexOutput;
    result.pos = vec4<f32>(ndc_pos, 0.0, 1.0);
    result.v_position = position;
    result.v_size = size;
    result.v_radius = radius;
    result.v_background_color = background_color;
    result.v_border_color = border_color;
    result.v_shadow_color = shadow_color;
    result.v_border_width = border_width;
    result.v_shadow_offset = shadow_offset;
    result.v_shadow_blur = shadow_blur;
    
    return result;
}

// Calculate rectangle SDF with rounded corners
fn sdf_rectangle(p: vec2<f32>, size: vec2<f32>, radius: vec4<f32>) -> f32 {
    // Calculate half size
    let half_size = size / 2.0;
    
    // Calculate distance to rectangle boundary
    let d = abs(p) - half_size;
    
    // Calculate corner radii
    let corner_radii = min(radius, half_size.xyxy);
    
    // Apply different radius for each corner
    let corner_factor = select(
        select(
            corner_radii.x,  // Top-left
            corner_radii.y,  // Top-right
            p.x > 0.0
        ),
        select(
            corner_radii.z,  // Bottom-right
            corner_radii.w,  // Bottom-left
            p.x > 0.0
        ),
        p.y > 0.0
    );
    
    // Calculate rounded rectangle SDF
    let d_rect = length(max(d, vec2<f32>(0.0))) + min(max(d.x, d.y), 0.0);
    return d_rect - corner_factor;
}

@fragment
fn fs_main(
    @builtin(position) pos: vec4<f32>,
    @location(0) v_position: vec2<f32>,
    @location(1) v_size: vec2<f32>,
    @location(2) v_radius: vec4<f32>,
    @location(3) v_background_color: vec4<f32>,
    @location(4) v_border_color: vec4<f32>,
    @location(5) v_shadow_color: vec4<f32>,
    @location(6) v_border_width: f32,
    @location(7) v_shadow_offset: vec2<f32>,
    @location(8) v_shadow_blur: f32,
) -> @location(0) vec4<f32> {
    // Calculate vector from rectangle center to pixel
    let rect_center = v_position + v_size / 2.0;
    let p = pos.xy - rect_center;
    
    // Calculate shadow
    let shadow_p = p - v_shadow_offset;
    let shadow_dist = sdf_rectangle(shadow_p, v_size, v_radius);
    let shadow_factor = 1.0 - smoothstep(-v_shadow_blur, 0.0, shadow_dist);
    let shadow = v_shadow_color * shadow_factor;

    // Calculate rectangle boundary
    let rect_dist = sdf_rectangle(p, v_size, v_radius);
    
    // Determine if inside rectangle (including border)
    let is_inside = rect_dist <= v_border_width;
    
    // Determine if inside border
    let is_border = rect_dist > 0.0 && rect_dist <= v_border_width;
    
    var color = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    if (is_inside) {
        if (is_border) {
            color = v_border_color;
        } else {
            color = v_background_color;
        }
    }
    
    // Mix shadow and rectangle color
    // Shadow only appears outside the rectangle
    let final_color = mix(shadow, color, step(0.0, -rect_dist));

    return final_color;
}