use smallvec::SmallVec;

use super::{EventFlags, StyleFlags};

slotmap::new_key_type! {
    pub struct NodeId;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiNodeTag {
    Root,
    Div,
    Span,
    Text,
    Button,
    Img,
}

impl UiNodeTag {
    pub(crate) fn debug_name(self) -> &'static str {
        match self {
            Self::Root => "root",
            Self::Div => "div",
            Self::Span => "span",
            Self::Text => "text",
            Self::Button => "button",
            Self::Img => "img",
        }
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct DirtyFlags: u16 {
        const STRUCTURE = 1 << 0;
        const STYLE     = 1 << 1;
        const LAYOUT    = 1 << 2;
        const PRE_PAINT = 1 << 3;
        const PAINT     = 1 << 4;
        const COMPOSITE = 1 << 5;
        const TEXT      = 1 << 6;
        const EVENTS    = 1 << 7;
    }
}

pub struct UiNode {
    pub tag: UiNodeTag,
    pub parent: Option<NodeId>,
    pub children: SmallVec<[NodeId; 1]>,
    pub style_flags: StyleFlags,
    pub event_flags: EventFlags,
    pub dirty: DirtyFlags,
}

impl UiNode {
    pub fn new(tag: UiNodeTag) -> Self {
        Self {
            tag,
            parent: None,
            children: SmallVec::new(),
            style_flags: StyleFlags::empty(),
            event_flags: EventFlags::empty(),
            dirty: DirtyFlags::all(),
        }
    }
}
