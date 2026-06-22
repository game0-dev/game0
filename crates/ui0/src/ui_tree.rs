mod event;
mod node;
mod state;
mod style;

use slotmap::{SecondaryMap, SlotMap};

use crate::app::Application;

pub use event::{ClickHandler, EventFlags, EventHandlers};
pub use node::{DirtyFlags, NodeId, UiNode, UiNodeTag};
pub use state::{
    ImageSource, ImageState, InteractionState, LayoutRect, ScrollState, TextContent, TextInputState,
};
pub use style::{
    AlignItems, BackgroundStyle, BorderStyle, Color, Corners, Display, Edges, EffectStyle,
    FlexDirection, FlexStyle, JustifyContent, Length, Overflow, OverflowStyle, Position,
    PositionStyle, SizeStyle, SpacingStyle, StyleFlags, TextStyle,
};

pub struct UiTree<A: Application> {
    nodes: SlotMap<NodeId, UiNode>,
    root: NodeId,

    pub(crate) size_styles: SecondaryMap<NodeId, SizeStyle>,
    pub(crate) spacing_styles: SecondaryMap<NodeId, SpacingStyle>,
    pub(crate) flex_styles: SecondaryMap<NodeId, FlexStyle>,
    pub(crate) background_styles: SecondaryMap<NodeId, BackgroundStyle>,
    pub(crate) border_styles: SecondaryMap<NodeId, BorderStyle>,
    pub(crate) text_styles: SecondaryMap<NodeId, TextStyle>,
    pub(crate) position_styles: SecondaryMap<NodeId, PositionStyle>,
    pub(crate) overflow_styles: SecondaryMap<NodeId, OverflowStyle>,
    pub(crate) effect_styles: SecondaryMap<NodeId, EffectStyle>,

    pub(crate) text_content: SecondaryMap<NodeId, TextContent>,
    pub(crate) image_states: SecondaryMap<NodeId, ImageState>,
    pub(crate) event_handlers: SecondaryMap<NodeId, EventHandlers<A>>,
    pub(crate) interaction_states: SecondaryMap<NodeId, InteractionState>,
    pub(crate) scroll_states: SecondaryMap<NodeId, ScrollState>,
    pub(crate) text_input_states: SecondaryMap<NodeId, TextInputState>,

    pub(crate) layout_rects: SecondaryMap<NodeId, LayoutRect>,
}

impl<A: Application> UiTree<A> {
    pub fn new() -> Self {
        let mut nodes = SlotMap::with_key();
        let root = nodes.insert(UiNode::new(UiNodeTag::Root));
        Self {
            nodes,
            root,
            size_styles: SecondaryMap::new(),
            spacing_styles: SecondaryMap::new(),
            flex_styles: SecondaryMap::new(),
            background_styles: SecondaryMap::new(),
            border_styles: SecondaryMap::new(),
            text_styles: SecondaryMap::new(),
            position_styles: SecondaryMap::new(),
            overflow_styles: SecondaryMap::new(),
            effect_styles: SecondaryMap::new(),
            text_content: SecondaryMap::new(),
            image_states: SecondaryMap::new(),
            event_handlers: SecondaryMap::new(),
            interaction_states: SecondaryMap::new(),
            scroll_states: SecondaryMap::new(),
            text_input_states: SecondaryMap::new(),
            layout_rects: SecondaryMap::new(),
        }
    }

    pub fn root(&self) -> NodeId {
        self.root
    }

    pub fn create_node(&mut self, tag: UiNodeTag) -> NodeId {
        self.nodes.insert(UiNode::new(tag))
    }

    pub fn append_child(&mut self, parent: NodeId, child: NodeId) {
        self.insert_child_before(parent, child, None);
    }

    pub fn insert_child_before(&mut self, parent: NodeId, child: NodeId, before: Option<NodeId>) {
        if !self.nodes.contains_key(parent) || !self.nodes.contains_key(child) || parent == child {
            return;
        }

        self.detach(child);

        if let Some(child_node) = self.nodes.get_mut(child) {
            child_node.parent = Some(parent);
        }

        if let Some(parent_node) = self.nodes.get_mut(parent) {
            let insert_at = before
                .and_then(|before| parent_node.children.iter().position(|node| *node == before))
                .unwrap_or(parent_node.children.len());
            parent_node.children.insert(insert_at, child);
        }
        self.mark_dirty(
            parent,
            DirtyFlags::STRUCTURE | DirtyFlags::LAYOUT | DirtyFlags::PAINT,
        );
    }

    pub fn detach(&mut self, node: NodeId) {
        if node == self.root {
            return;
        }

        let Some(parent) = self.nodes.get(node).and_then(|node| node.parent) else {
            return;
        };

        if let Some(parent_node) = self.nodes.get_mut(parent) {
            if let Some(index) = parent_node.children.iter().position(|child| *child == node) {
                parent_node.children.remove(index);
            }
        }
        if let Some(node) = self.nodes.get_mut(node) {
            node.parent = None;
        }
        self.mark_dirty(
            parent,
            DirtyFlags::STRUCTURE | DirtyFlags::LAYOUT | DirtyFlags::PAINT,
        );
    }

    pub fn remove_subtree(&mut self, node: NodeId) {
        if node == self.root || !self.nodes.contains_key(node) {
            return;
        }

        let parent = self.nodes.get(node).and_then(|node| node.parent);
        self.detach(node);
        self.remove_subtree_inner(node);
        if let Some(parent) = parent {
            self.mark_dirty(
                parent,
                DirtyFlags::STRUCTURE | DirtyFlags::LAYOUT | DirtyFlags::PAINT,
            );
        }
    }

    pub fn node(&self, node: NodeId) -> Option<&UiNode> {
        self.nodes.get(node)
    }

    pub fn node_mut(&mut self, node: NodeId) -> Option<&mut UiNode> {
        self.nodes.get_mut(node)
    }

    pub fn children(&self, node: NodeId) -> &[NodeId] {
        self.nodes
            .get(node)
            .map(|node| node.children.as_slice())
            .unwrap_or(&[])
    }

    pub fn set_text(&mut self, node: NodeId, value: impl Into<String>) -> bool {
        let value = value.into();
        if self.text_content.get(node).map(|text| text.value.as_str()) == Some(value.as_str()) {
            return false;
        }
        self.text_content.insert(node, TextContent { value });
        self.mark_dirty(
            node,
            DirtyFlags::TEXT | DirtyFlags::LAYOUT | DirtyFlags::PAINT,
        );
        true
    }

    pub fn text(&self, node: NodeId) -> Option<&str> {
        self.text_content.get(node).map(|text| text.value.as_str())
    }

    pub fn mark_dirty(&mut self, node: NodeId, dirty: DirtyFlags) {
        if let Some(node) = self.nodes.get_mut(node) {
            node.dirty |= dirty;
        }
    }

    pub fn clear_dirty(&mut self) {
        for (_, node) in self.nodes.iter_mut() {
            node.dirty = DirtyFlags::empty();
        }
    }

    pub fn debug_dump(&self) -> String {
        let mut out = String::new();
        self.debug_node(self.root, 0, &mut out);
        out
    }

    fn remove_subtree_inner(&mut self, node: NodeId) {
        let children = self
            .nodes
            .get(node)
            .map(|node| node.children.iter().copied().collect::<Vec<_>>())
            .unwrap_or_default();

        for child in children {
            self.remove_subtree_inner(child);
        }

        self.remove_side_tables(node);
        self.nodes.remove(node);
    }

    fn remove_side_tables(&mut self, node: NodeId) {
        self.size_styles.remove(node);
        self.spacing_styles.remove(node);
        self.flex_styles.remove(node);
        self.background_styles.remove(node);
        self.border_styles.remove(node);
        self.text_styles.remove(node);
        self.position_styles.remove(node);
        self.overflow_styles.remove(node);
        self.effect_styles.remove(node);
        self.text_content.remove(node);
        self.image_states.remove(node);
        self.event_handlers.remove(node);
        self.interaction_states.remove(node);
        self.scroll_states.remove(node);
        self.text_input_states.remove(node);
        self.layout_rects.remove(node);
    }

    fn debug_node(&self, node: NodeId, depth: usize, out: &mut String) {
        let Some(node_ref) = self.nodes.get(node) else {
            return;
        };
        for _ in 0..depth {
            out.push_str("  ");
        }
        out.push_str(node_ref.tag.debug_name());
        if let Some(text) = self.text(node) {
            out.push_str(": ");
            out.push_str(text);
        }
        out.push('\n');

        for child in &node_ref.children {
            self.debug_node(*child, depth + 1, out);
        }
    }
}

impl<A: Application> Default for UiTree<A> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestApp;

    impl Application for TestApp {}

    #[test]
    fn new_tree_has_root() {
        let tree = UiTree::<TestApp>::new();
        let root = tree.root();

        assert_eq!(tree.node(root).map(|node| node.tag), Some(UiNodeTag::Root));
        assert_eq!(tree.node(root).unwrap().parent, None);
        assert!(tree.children(root).is_empty());
    }

    #[test]
    fn append_child_sets_parent_and_children() {
        let mut tree = UiTree::<TestApp>::new();
        let div = tree.create_node(UiNodeTag::Div);

        tree.append_child(tree.root(), div);

        assert_eq!(tree.node(div).unwrap().parent, Some(tree.root()));
        assert_eq!(tree.children(tree.root()), &[div]);
    }

    #[test]
    fn append_child_moves_from_old_parent() {
        let mut tree = UiTree::<TestApp>::new();
        let old_parent = tree.create_node(UiNodeTag::Div);
        let new_parent = tree.create_node(UiNodeTag::Div);
        let child = tree.create_node(UiNodeTag::Span);
        tree.append_child(tree.root(), old_parent);
        tree.append_child(tree.root(), new_parent);
        tree.append_child(old_parent, child);

        tree.append_child(new_parent, child);

        assert!(tree.children(old_parent).is_empty());
        assert_eq!(tree.children(new_parent), &[child]);
        assert_eq!(tree.node(child).unwrap().parent, Some(new_parent));
    }

    #[test]
    fn insert_child_before_orders_children() {
        let mut tree = UiTree::<TestApp>::new();
        let parent = tree.create_node(UiNodeTag::Div);
        let first = tree.create_node(UiNodeTag::Span);
        let second = tree.create_node(UiNodeTag::Button);
        tree.append_child(tree.root(), parent);
        tree.append_child(parent, second);

        tree.insert_child_before(parent, first, Some(second));

        assert_eq!(tree.children(parent), &[first, second]);
    }

    #[test]
    fn remove_subtree_cleans_side_tables() {
        let mut tree = UiTree::<TestApp>::new();
        let parent = tree.create_node(UiNodeTag::Div);
        let text = tree.create_node(UiNodeTag::Text);
        tree.append_child(tree.root(), parent);
        tree.append_child(parent, text);
        tree.set_text(text, "Save");
        tree.size_styles.insert(parent, SizeStyle::default());
        tree.layout_rects.insert(text, LayoutRect::default());

        tree.remove_subtree(parent);

        assert!(tree.node(parent).is_none());
        assert!(tree.node(text).is_none());
        assert!(tree.text_content.get(text).is_none());
        assert!(tree.size_styles.get(parent).is_none());
        assert!(tree.layout_rects.get(text).is_none());
        assert!(tree.children(tree.root()).is_empty());
    }

    #[test]
    fn set_text_marks_text_layout_paint_dirty() {
        let mut tree = UiTree::<TestApp>::new();
        let text = tree.create_node(UiNodeTag::Text);
        tree.clear_dirty();

        assert!(tree.set_text(text, "Hello"));
        let dirty = tree.node(text).unwrap().dirty;
        assert!(dirty.contains(DirtyFlags::TEXT));
        assert!(dirty.contains(DirtyFlags::LAYOUT));
        assert!(dirty.contains(DirtyFlags::PAINT));

        tree.clear_dirty();
        assert!(!tree.set_text(text, "Hello"));
        assert!(tree.node(text).unwrap().dirty.is_empty());
    }

    #[test]
    fn mark_dirty_sets_flags_on_node() {
        let mut tree = UiTree::<TestApp>::new();
        let div = tree.create_node(UiNodeTag::Div);
        tree.clear_dirty();

        tree.mark_dirty(div, DirtyFlags::STYLE | DirtyFlags::PAINT);

        assert_eq!(
            tree.node(div).unwrap().dirty,
            DirtyFlags::STYLE | DirtyFlags::PAINT
        );
    }

    #[test]
    fn debug_dump_outputs_stable_web_like_tree() {
        let mut tree = UiTree::<TestApp>::new();
        let div = tree.create_node(UiNodeTag::Div);
        let button = tree.create_node(UiNodeTag::Button);
        let text = tree.create_node(UiNodeTag::Text);
        tree.append_child(tree.root(), div);
        tree.append_child(div, button);
        tree.append_child(button, text);
        tree.set_text(text, "Save");

        assert_eq!(
            tree.debug_dump(),
            "root\n  div\n    button\n      text: Save\n"
        );
    }
}
