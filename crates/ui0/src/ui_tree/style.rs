bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct StyleFlags: u32 {
        const SIZE       = 1 << 0;
        const SPACING    = 1 << 1;
        const FLEX       = 1 << 2;
        const BACKGROUND = 1 << 3;
        const BORDER     = 1 << 4;
        const TEXT       = 1 << 5;
        const POSITION   = 1 << 6;
        const OVERFLOW   = 1 << 7;
        const EFFECT     = 1 << 8;
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SizeStyle {
    pub width: Length,
    pub height: Length,
    pub min_width: Length,
    pub min_height: Length,
    pub max_width: Length,
    pub max_height: Length,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SpacingStyle {
    pub margin: Edges,
    pub padding: Edges,
    pub gap: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlexStyle {
    pub display: Display,
    pub direction: FlexDirection,
    pub align_items: AlignItems,
    pub justify_content: JustifyContent,
}

impl Default for FlexStyle {
    fn default() -> Self {
        Self {
            display: Display::Block,
            direction: FlexDirection::Row,
            align_items: AlignItems::Stretch,
            justify_content: JustifyContent::Start,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BackgroundStyle {
    pub color: Option<Color>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BorderStyle {
    pub color: Option<Color>,
    pub width: Edges,
    pub radius: Corners,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TextStyle {
    pub color: Option<Color>,
    pub font_size: Option<f32>,
    pub font_family: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PositionStyle {
    pub position: Position,
}

impl Default for PositionStyle {
    fn default() -> Self {
        Self {
            position: Position::Relative,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverflowStyle {
    pub x: Overflow,
    pub y: Overflow,
}

impl Default for OverflowStyle {
    fn default() -> Self {
        Self {
            x: Overflow::Visible,
            y: Overflow::Visible,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EffectStyle {
    pub opacity: Option<f32>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Edges {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Corners {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Length {
    #[default]
    Auto,
    Px(f32),
    Percent(f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Display {
    None,
    Block,
    Flex,
}

impl Default for Display {
    fn default() -> Self {
        Self::Block
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexDirection {
    Row,
    Column,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignItems {
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JustifyContent {
    Start,
    Center,
    End,
    SpaceBetween,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    Relative,
    Absolute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overflow {
    Visible,
    Hidden,
    Scroll,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}
