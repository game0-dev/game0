#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LayoutRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextContent {
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageState {
    pub source: ImageSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageSource {
    Asset(String),
    Texture(String),
    Path(String),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InteractionState {
    pub hovered: bool,
    pub pressed: bool,
    pub focus_visible: bool,
    pub disabled: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ScrollState {
    pub offset_x: f32,
    pub offset_y: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub content_width: f32,
    pub content_height: f32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextInputState {
    pub value: String,
}
