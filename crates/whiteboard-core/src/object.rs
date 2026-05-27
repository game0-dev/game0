pub type ObjectId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    Rect,
    Ellipse,
    Line,
    Arrow,
    Text,
    Image,
}

#[derive(Debug, Clone, Copy)]
pub struct ObjectStyle {
    pub fill: [f32; 4],
    pub stroke: [f32; 4],
    pub stroke_width: f32,
    pub opacity: f32,
}

impl Default for ObjectStyle {
    fn default() -> Self {
        Self {
            fill: [0.94, 0.95, 0.99, 1.0],
            stroke: [0.40, 0.45, 0.58, 1.0],
            stroke_width: 1.0,
            opacity: 1.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WhiteboardObject {
    pub id: ObjectId,
    pub kind: ObjectKind,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub rotation: f32,
    pub text: String,
    pub image_src: Option<String>,
    pub style: ObjectStyle,
}

impl WhiteboardObject {
    pub fn bounds(&self) -> (f32, f32, f32, f32) {
        (self.x, self.y, self.width.max(1.0), self.height.max(1.0))
    }
}
