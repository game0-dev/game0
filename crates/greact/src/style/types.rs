/// Dimension value: Auto, fixed pixels, or percentage.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Dimension {
    #[default]
    Auto,
    Px(f32),
    Percent(f32),
}

/// Flex direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlexDir {
    #[default]
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

/// Flex wrap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlexWrap {
    #[default]
    NoWrap,
    Wrap,
}

/// Justify content (main axis).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Justify {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
}

/// Align items (cross axis).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    #[default]
    Stretch,
    Start,
    Center,
    End,
}

/// CSS-like position mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Position {
    #[default]
    Relative,
    Absolute,
    Fixed,
}

/// Overflow behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Overflow {
    #[default]
    Visible,
    Hidden,
    Scroll,
}

/// Text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// RGBA colour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const TRANSPARENT: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };
    pub const RED: Self = Self {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const GREEN: Self = Self {
        r: 0.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };
    pub const BLUE: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn to_rgba(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::TRANSPARENT
    }
}

/// Box shadow parameters.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BoxShadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub spread: f32,
    pub color: Color,
}

/// 2D transform.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub translate_x: f32,
    pub translate_y: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub rotate: f32, // radians
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translate_x: 0.0,
            translate_y: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            rotate: 0.0,
        }
    }
}

impl Transform {
    pub fn translate(x: f32, y: f32) -> Self {
        Self {
            translate_x: x,
            translate_y: y,
            ..Default::default()
        }
    }

    pub fn scale(x: f32, y: f32) -> Self {
        Self {
            scale_x: x,
            scale_y: y,
            ..Default::default()
        }
    }
}

// Convenience helpers for declarative style syntax.
pub fn rgb(r: f32, g: f32, b: f32) -> Color {
    Color::rgb(r, g, b)
}

pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Color {
    Color::rgba(r, g, b, a)
}

pub fn px(v: f32) -> Dimension {
    Dimension::Px(v)
}

pub fn pct(v: f32) -> Dimension {
    Dimension::Percent(v)
}
