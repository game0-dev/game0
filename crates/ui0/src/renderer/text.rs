use glyphon::Color as GlyphColor;

use crate::ui_tree::Color;

pub(crate) struct TextNodeState {
    pub(crate) buffer: glyphon::Buffer,
    pub(crate) text: String,
    pub(crate) font_size: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

pub(crate) fn glyph_color(color: Color, opacity: f32) -> GlyphColor {
    GlyphColor::rgba(
        (color.r.clamp(0.0, 1.0) * 255.0) as u8,
        (color.g.clamp(0.0, 1.0) * 255.0) as u8,
        (color.b.clamp(0.0, 1.0) * 255.0) as u8,
        ((color.a * opacity).clamp(0.0, 1.0) * 255.0) as u8,
    )
}
