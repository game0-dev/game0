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

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Style {
    pub(crate) size: Option<SizeStyle>,
    pub(crate) spacing: Option<SpacingStyle>,
    pub(crate) flex: Option<FlexStyle>,
    pub(crate) background: Option<BackgroundStyle>,
    pub(crate) border: Option<BorderStyle>,
    pub(crate) text: Option<TextStyle>,
    pub(crate) position: Option<PositionStyle>,
    pub(crate) overflow: Option<OverflowStyle>,
    pub(crate) effect: Option<EffectStyle>,
}

impl Style {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn w(mut self, value: impl Into<Length>) -> Self {
        self.size.get_or_insert_default().width = value.into();
        self
    }

    pub fn h(mut self, value: impl Into<Length>) -> Self {
        self.size.get_or_insert_default().height = value.into();
        self
    }

    pub fn min_w(mut self, value: impl Into<Length>) -> Self {
        self.size.get_or_insert_default().min_width = value.into();
        self
    }

    pub fn min_h(mut self, value: impl Into<Length>) -> Self {
        self.size.get_or_insert_default().min_height = value.into();
        self
    }

    pub fn max_w(mut self, value: impl Into<Length>) -> Self {
        self.size.get_or_insert_default().max_width = value.into();
        self
    }

    pub fn max_h(mut self, value: impl Into<Length>) -> Self {
        self.size.get_or_insert_default().max_height = value.into();
        self
    }

    pub fn margin(mut self, value: f32) -> Self {
        self.spacing.get_or_insert_default().margin = Edges::all(value);
        self
    }

    pub fn padding(mut self, value: f32) -> Self {
        self.spacing.get_or_insert_default().padding = Edges::all(value);
        self
    }

    pub fn gap(mut self, value: f32) -> Self {
        self.spacing.get_or_insert_default().gap = value;
        self
    }

    pub fn flex(mut self) -> Self {
        self.flex.get_or_insert_default().display = Display::Flex;
        self
    }

    pub fn row(mut self) -> Self {
        let flex = self.flex.get_or_insert_default();
        flex.display = Display::Flex;
        flex.direction = FlexDirection::Row;
        self
    }

    pub fn column(mut self) -> Self {
        let flex = self.flex.get_or_insert_default();
        flex.display = Display::Flex;
        flex.direction = FlexDirection::Column;
        self
    }

    pub fn align_items(mut self, value: AlignItems) -> Self {
        self.flex.get_or_insert_default().align_items = value;
        self
    }

    pub fn justify_content(mut self, value: JustifyContent) -> Self {
        self.flex.get_or_insert_default().justify_content = value;
        self
    }

    pub fn bg(mut self, color: Color) -> Self {
        self.background.get_or_insert_default().color = Some(color);
        self
    }

    pub fn border_color(mut self, color: Color) -> Self {
        self.border.get_or_insert_default().color = Some(color);
        self
    }

    pub fn border_width(mut self, value: f32) -> Self {
        self.border.get_or_insert_default().width = Edges::all(value);
        self
    }

    pub fn radius(mut self, value: f32) -> Self {
        self.border.get_or_insert_default().radius = Corners::all(value);
        self
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.text.get_or_insert_default().color = Some(color);
        self
    }

    pub fn font_size(mut self, value: f32) -> Self {
        self.text.get_or_insert_default().font_size = Some(value);
        self
    }

    pub fn font_family(mut self, value: impl Into<String>) -> Self {
        self.text.get_or_insert_default().font_family = Some(value.into());
        self
    }

    pub fn absolute(mut self) -> Self {
        self.position.get_or_insert_default().position = Position::Absolute;
        self
    }

    pub fn relative(mut self) -> Self {
        self.position.get_or_insert_default().position = Position::Relative;
        self
    }

    pub fn overflow(mut self, value: Overflow) -> Self {
        let overflow = self.overflow.get_or_insert_default();
        overflow.x = value;
        overflow.y = value;
        self
    }

    pub fn opacity(mut self, value: f32) -> Self {
        self.effect.get_or_insert_default().opacity = Some(value);
        self
    }

    pub fn merge(mut self, other: Style) -> Self {
        if other.size.is_some() {
            self.size = other.size;
        }
        if other.spacing.is_some() {
            self.spacing = other.spacing;
        }
        if other.flex.is_some() {
            self.flex = other.flex;
        }
        if other.background.is_some() {
            self.background = other.background;
        }
        if other.border.is_some() {
            self.border = other.border;
        }
        if other.text.is_some() {
            self.text = other.text;
        }
        if other.position.is_some() {
            self.position = other.position;
        }
        if other.overflow.is_some() {
            self.overflow = other.overflow;
        }
        if other.effect.is_some() {
            self.effect = other.effect;
        }

        self
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Edges {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl Edges {
    pub fn all(value: f32) -> Self {
        Self {
            left: value,
            right: value,
            top: value,
            bottom: value,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Corners {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl Corners {
    pub fn all(value: f32) -> Self {
        Self {
            top_left: value,
            top_right: value,
            bottom_right: value,
            bottom_left: value,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Length {
    #[default]
    Auto,
    Px(f32),
    Percent(f32),
}

impl Length {
    pub fn auto() -> Self {
        Self::Auto
    }

    pub fn px(value: f32) -> Self {
        Self::Px(value)
    }

    pub fn percent(value: f32) -> Self {
        Self::Percent(value)
    }
}

impl From<f32> for Length {
    fn from(value: f32) -> Self {
        Self::Px(value)
    }
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

impl Color {
    pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn rgb_u8(r: u8, g: u8, b: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0,
        }
    }
}
