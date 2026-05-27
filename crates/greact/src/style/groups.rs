use super::types::*;

// ---------------------------------------------------------------------------
// Style groups -- each stored in its own SecondaryMap inside RenderTree.
// Only allocated for nodes that actually set properties in that group.
// ---------------------------------------------------------------------------

/// Size: width, height, min/max constraints.
#[derive(Debug, Clone, Default)]
pub struct SizeStyle {
    pub width: Dimension,
    pub height: Dimension,
    pub min_width: Dimension,
    pub max_width: Dimension,
    pub min_height: Dimension,
    pub max_height: Dimension,
}

/// Spacing: padding and margin (top, right, bottom, left).
#[derive(Debug, Clone, Default)]
pub struct SpacingStyle {
    pub padding: [f32; 4],
    pub margin: [f32; 4],
}

/// Flex layout properties.
#[derive(Debug, Clone)]
pub struct FlexStyle {
    pub direction: FlexDir,
    pub wrap: FlexWrap,
    pub justify: Justify,
    pub align_items: Align,
    pub align_self: Align,
    pub gap: f32,
    pub grow: f32,
    pub shrink: f32,
    pub basis: Dimension,
}

impl Default for FlexStyle {
    fn default() -> Self {
        Self {
            direction: FlexDir::Row,
            wrap: FlexWrap::NoWrap,
            justify: Justify::Start,
            align_items: Align::Stretch,
            align_self: Align::Stretch,
            gap: 0.0,
            grow: 0.0,
            shrink: 1.0,
            basis: Dimension::Auto,
        }
    }
}

/// Background visual.
#[derive(Debug, Clone, Default)]
pub struct BackgroundStyle {
    pub color: [f32; 4], // RGBA
}

/// Border visual.
#[derive(Debug, Clone, Default)]
pub struct BorderStyle {
    pub width: f32,
    pub color: [f32; 4],    // RGBA
    pub radius: [f32; 4],   // per-corner [tl, tr, br, bl]
}

/// Text rendering style.
#[derive(Debug, Clone)]
pub struct TextStyleGroup {
    pub font_size: f32,
    pub font_weight: u16,
    pub color: [f32; 4],
    pub line_height: f32,
    pub text_align: TextAlign,
}

impl Default for TextStyleGroup {
    fn default() -> Self {
        Self {
            font_size: 14.0,
            font_weight: 400,
            color: [0.0, 0.0, 0.0, 1.0],
            line_height: 1.2,
            text_align: TextAlign::Left,
        }
    }
}

/// Absolute/relative positioning.
#[derive(Debug, Clone, Default)]
pub struct PositionStyle {
    pub position: Position,
    pub top: Dimension,
    pub left: Dimension,
    pub right: Dimension,
    pub bottom: Dimension,
    pub z_index: i32,
}

/// Visual effects.
#[derive(Debug, Clone)]
pub struct EffectStyle {
    pub opacity: f32,
    pub box_shadow: Option<BoxShadow>,
    pub transform: Option<Transform>,
}

impl Default for EffectStyle {
    fn default() -> Self {
        Self {
            opacity: 1.0,
            box_shadow: None,
            transform: None,
        }
    }
}

/// Overflow / clipping.
#[derive(Debug, Clone)]
pub struct OverflowStyle {
    pub overflow_x: Overflow,
    pub overflow_y: Overflow,
    pub visibility: bool,
}

impl Default for OverflowStyle {
    fn default() -> Self {
        Self {
            overflow_x: Overflow::Visible,
            overflow_y: Overflow::Visible,
            visibility: true,
        }
    }
}
