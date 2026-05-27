#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolKind {
    Select,
    Pan,
    Rect,
    Ellipse,
    Line,
    Arrow,
    Text,
    Image,
}

impl ToolKind {
    pub fn label(self) -> &'static str {
        match self {
            ToolKind::Select => "Select",
            ToolKind::Pan => "Pan",
            ToolKind::Rect => "Rect",
            ToolKind::Ellipse => "Ellipse",
            ToolKind::Line => "Line",
            ToolKind::Arrow => "Arrow",
            ToolKind::Text => "Text",
            ToolKind::Image => "Image",
        }
    }
}
