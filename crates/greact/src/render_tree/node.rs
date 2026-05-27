use smallvec::SmallVec;

slotmap::new_key_type! {
    /// Stable identifier for a node in the render tree.
    pub struct NodeId;
}

/// Element kind.  Determines default behaviour (focusable, scrollable, …)
/// but does **not** change the stored data – all nodes share the same
/// `RenderNode` struct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ElementTag {
    Div,
    Text,
    Icon,
    Button,
    Image,
    Input,
    Canvas,
    Scroll,
}

/// Display mode (kept inline on `RenderNode` because it is read on almost
/// every layout/render pass).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Display {
    #[default]
    Flex,
    Block,
    None,
}

bitflags::bitflags! {
    /// Bitfield on each `RenderNode` indicating which style-group
    /// `SecondaryMap`s contain data for this node.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct StyleFlags: u32 {
        const SIZE       = 1 << 0;
        const SPACING    = 1 << 1;
        const FLEX       = 1 << 2;
        const BACKGROUND = 1 << 3;
        const BORDER     = 1 << 4;
        const TEXT_STYLE = 1 << 5;
        const POSITION   = 1 << 6;
        const EFFECT     = 1 << 7;
        const OVERFLOW   = 1 << 8;
    }
}

bitflags::bitflags! {
    /// Per-node dirty flags used to schedule incremental work.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct DirtyFlags: u8 {
        const STYLE  = 1 << 0;
        const LAYOUT = 1 << 1;
        const PAINT  = 1 << 2;
    }
}

/// Core per-node data.  Kept intentionally small (~48 B) so that the main
/// `SlotMap` is cache-friendly during tree traversals.  All heavyweight data
/// (styles, events, text) lives in separate `SecondaryMap`s.
pub struct RenderNode {
    pub tag: ElementTag,
    pub display: Display,
    pub style_flags: StyleFlags,
    pub dirty: DirtyFlags,
    pub children: SmallVec<[NodeId; 4]>,
    pub parent: Option<NodeId>,
}

impl RenderNode {
    pub fn new(tag: ElementTag) -> Self {
        Self {
            tag,
            display: Display::default(),
            style_flags: StyleFlags::empty(),
            dirty: DirtyFlags::all(), // new nodes are fully dirty
            children: SmallVec::new(),
            parent: None,
        }
    }
}
