use bytemuck::{Pod, Zeroable};
use wgpu::{TextureFormat, VertexAttribute, VertexBufferLayout, VertexStepMode};

use crate::ui_tree::Color;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct RectUniform {
    pub(crate) screen_size: [f32; 2],
    pub(crate) _pad: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct RectInstance {
    pub(crate) rect: [f32; 4],
    pub(crate) fill: [f32; 4],
    pub(crate) border_color: [f32; 4],
    pub(crate) border_width: [f32; 4],
    pub(crate) radius: [f32; 4],
}

impl RectInstance {
    const ATTRS: [VertexAttribute; 5] = wgpu::vertex_attr_array![
        0 => Float32x4,
        1 => Float32x4,
        2 => Float32x4,
        3 => Float32x4,
        4 => Float32x4,
    ];

    pub(crate) fn layout<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<RectInstance>() as u64,
            step_mode: VertexStepMode::Instance,
            attributes: &Self::ATTRS,
        }
    }
}

pub(crate) fn preferred_surface_format(formats: &[TextureFormat]) -> TextureFormat {
    formats
        .iter()
        .copied()
        .find(TextureFormat::is_srgb)
        .unwrap_or(formats[0])
}

pub(crate) fn color_to_array(color: Color, opacity: f32) -> [f32; 4] {
    [
        srgb_to_linear(color.r),
        srgb_to_linear(color.g),
        srgb_to_linear(color.b),
        color.a * opacity,
    ]
}

pub(crate) fn srgb_to_linear(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}
