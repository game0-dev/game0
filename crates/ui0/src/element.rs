use crate::app::EventCx;
use crate::ui_tree::{
    AlignItems, Color, DirtyFlags, EventFlags, EventHandlers, ImageSource, ImageState,
    JustifyContent, Length, NodeId, Overflow, Style, UiNodeTag, UiTree,
};

pub struct Element {
    pub(crate) tag: UiNodeTag,
    pub(crate) style: Style,
    pub(crate) text: Option<String>,
    pub(crate) image: Option<ImageSource>,
    pub(crate) events: EventHandlers,
    pub(crate) bindings: Vec<ElementBinding>,
    pub(crate) children: Vec<Element>,
}

impl Element {
    pub fn new(tag: UiNodeTag) -> Self {
        Self {
            tag,
            style: Style::new(),
            text: None,
            image: None,
            events: EventHandlers::default(),
            bindings: Vec::new(),
            children: Vec::new(),
        }
    }

    pub fn child<E>(mut self, child: E) -> Self
    where
        E: IntoElement,
    {
        self.children.push(child.into_element());
        self
    }

    pub fn children<I, E>(mut self, children: I) -> Self
    where
        I: IntoIterator<Item = E>,
        E: IntoElement,
    {
        self.children
            .extend(children.into_iter().map(IntoElement::into_element));
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = self.style.merge(style);
        self
    }

    pub fn on_click<F>(mut self, handler: F) -> Self
    where
        F: for<'a> FnMut(&mut EventCx<'a>) + 'static,
    {
        self.events.click = Some(Box::new(handler));
        self
    }

    pub fn w(mut self, value: impl Into<Length>) -> Self {
        self.style = self.style.w(value);
        self
    }

    pub fn bind_w<F, V>(mut self, f: F) -> Self
    where
        F: FnMut() -> V + 'static,
        V: Into<Length> + 'static,
    {
        self.bindings.push(ElementBinding::width(f));
        self
    }

    pub fn h(mut self, value: impl Into<Length>) -> Self {
        self.style = self.style.h(value);
        self
    }

    pub fn bind_h<F, V>(mut self, f: F) -> Self
    where
        F: FnMut() -> V + 'static,
        V: Into<Length> + 'static,
    {
        self.bindings.push(ElementBinding::height(f));
        self
    }

    pub fn min_w(mut self, value: impl Into<Length>) -> Self {
        self.style = self.style.min_w(value);
        self
    }

    pub fn min_h(mut self, value: impl Into<Length>) -> Self {
        self.style = self.style.min_h(value);
        self
    }

    pub fn max_w(mut self, value: impl Into<Length>) -> Self {
        self.style = self.style.max_w(value);
        self
    }

    pub fn max_h(mut self, value: impl Into<Length>) -> Self {
        self.style = self.style.max_h(value);
        self
    }

    pub fn margin(mut self, value: f32) -> Self {
        self.style = self.style.margin(value);
        self
    }

    pub fn padding(mut self, value: f32) -> Self {
        self.style = self.style.padding(value);
        self
    }

    pub fn gap(mut self, value: f32) -> Self {
        self.style = self.style.gap(value);
        self
    }

    pub fn flex(mut self) -> Self {
        self.style = self.style.flex();
        self
    }

    pub fn row(mut self) -> Self {
        self.style = self.style.row();
        self
    }

    pub fn column(mut self) -> Self {
        self.style = self.style.column();
        self
    }

    pub fn align_items(mut self, value: AlignItems) -> Self {
        self.style = self.style.align_items(value);
        self
    }

    pub fn justify_content(mut self, value: JustifyContent) -> Self {
        self.style = self.style.justify_content(value);
        self
    }

    pub fn bg(mut self, color: Color) -> Self {
        self.style = self.style.bg(color);
        self
    }

    pub fn bind_bg<F>(mut self, f: F) -> Self
    where
        F: FnMut() -> Color + 'static,
    {
        self.bindings.push(ElementBinding::Background(Box::new(f)));
        self
    }

    pub fn border_color(mut self, color: Color) -> Self {
        self.style = self.style.border_color(color);
        self
    }

    pub fn border_width(mut self, value: f32) -> Self {
        self.style = self.style.border_width(value);
        self
    }

    pub fn radius(mut self, value: f32) -> Self {
        self.style = self.style.radius(value);
        self
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.style = self.style.text_color(color);
        self
    }

    pub fn font_size(mut self, value: f32) -> Self {
        self.style = self.style.font_size(value);
        self
    }

    pub fn font_family(mut self, value: impl Into<String>) -> Self {
        self.style = self.style.font_family(value);
        self
    }

    pub fn bind_text<F, S>(mut self, f: F) -> Self
    where
        F: FnMut() -> S + 'static,
        S: Into<String> + 'static,
    {
        self.bindings.push(ElementBinding::text(f));
        self
    }

    pub fn absolute(mut self) -> Self {
        self.style = self.style.absolute();
        self
    }

    pub fn relative(mut self) -> Self {
        self.style = self.style.relative();
        self
    }

    pub fn overflow(mut self, value: Overflow) -> Self {
        self.style = self.style.overflow(value);
        self
    }

    pub fn opacity(mut self, value: f32) -> Self {
        self.style = self.style.opacity(value);
        self
    }
}

pub trait IntoElement {
    fn into_element(self) -> Element;
}

impl IntoElement for Element {
    fn into_element(self) -> Element {
        self
    }
}

impl IntoElement for String {
    fn into_element(self) -> Element {
        text(self)
    }
}

impl IntoElement for &str {
    fn into_element(self) -> Element {
        text(self)
    }
}

pub(crate) enum ElementBinding {
    Text(Box<dyn FnMut() -> String>),
    Width(Box<dyn FnMut() -> Length>),
    Height(Box<dyn FnMut() -> Length>),
    Background(Box<dyn FnMut() -> Color>),
}

impl ElementBinding {
    fn text<F, S>(mut f: F) -> Self
    where
        F: FnMut() -> S + 'static,
        S: Into<String> + 'static,
    {
        Self::Text(Box::new(move || f().into()))
    }

    fn width<F, V>(mut f: F) -> Self
    where
        F: FnMut() -> V + 'static,
        V: Into<Length> + 'static,
    {
        Self::Width(Box::new(move || f().into()))
    }

    fn height<F, V>(mut f: F) -> Self
    where
        F: FnMut() -> V + 'static,
        V: Into<Length> + 'static,
    {
        Self::Height(Box::new(move || f().into()))
    }
}

pub(crate) struct MountedRegion {
    parent: NodeId,
    nodes: Vec<NodeId>,
}

impl MountedRegion {
    pub(crate) fn new(parent: NodeId) -> Self {
        Self {
            parent,
            nodes: Vec::new(),
        }
    }
}

pub fn div() -> Element {
    Element::new(UiNodeTag::Div)
}

pub fn span() -> Element {
    Element::new(UiNodeTag::Span)
}

pub fn button() -> Element {
    Element::new(UiNodeTag::Button)
}

pub fn img(source: ImageSource) -> Element {
    let mut element = Element::new(UiNodeTag::Img);
    element.image = Some(source);
    element
}

pub fn text(value: impl Into<String>) -> Element {
    let mut element = Element::new(UiNodeTag::Text);
    element.text = Some(value.into());
    element
}

pub(crate) fn mount_element(tree: &mut UiTree, region: &mut MountedRegion, element: Element) {
    RegionRebuilder::new(tree).rebuild_region(region, vec![element]);
}

struct RegionRebuilder<'a> {
    tree: &'a mut UiTree,
}

impl<'a> RegionRebuilder<'a> {
    fn new(tree: &'a mut UiTree) -> Self {
        Self { tree }
    }

    fn rebuild_region(&mut self, region: &mut MountedRegion, children: Vec<Element>) {
        region.nodes =
            self.rebuild_children(region.parent, std::mem::take(&mut region.nodes), children);
    }

    fn rebuild_children(
        &mut self,
        parent: NodeId,
        old_children: Vec<NodeId>,
        children: Vec<Element>,
    ) -> Vec<NodeId> {
        let new_len = children.len();
        let mut new_nodes = Vec::with_capacity(new_len);

        for (index, child) in children.into_iter().enumerate() {
            let existing = old_children.get(index).copied();
            let before = old_children
                .get(index + 1)
                .copied()
                .filter(|node| self.tree.node(*node).and_then(|node| node.parent) == Some(parent));
            new_nodes.push(self.rebuild_node(parent, existing, before, child));
        }

        for old_child in old_children.into_iter().skip(new_len) {
            if self.tree.node(old_child).and_then(|node| node.parent) == Some(parent) {
                self.tree.remove_subtree(old_child);
            }
        }

        new_nodes
    }

    fn rebuild_node(
        &mut self,
        parent: NodeId,
        existing: Option<NodeId>,
        before: Option<NodeId>,
        element: Element,
    ) -> NodeId {
        let Element {
            tag,
            style,
            text,
            image,
            events,
            bindings,
            children,
        } = element;

        let node = match existing {
            Some(existing) if self.tree.node(existing).map(|node| node.tag) == Some(tag) => {
                existing
            }
            Some(existing) => {
                self.tree.remove_subtree(existing);
                self.create_node(parent, before, tag)
            }
            None => self.create_node(parent, before, tag),
        };

        self.sync_text(node, text);
        self.sync_image(node, image);
        self.sync_events(node, events);
        self.tree.apply_style(node, style);
        self.apply_bindings_once(node, bindings);

        let old_children = self.tree.children(node).to_vec();
        self.rebuild_children(node, old_children, children);
        node
    }

    fn create_node(&mut self, parent: NodeId, before: Option<NodeId>, tag: UiNodeTag) -> NodeId {
        let node = self.tree.create_node(tag);
        self.tree.insert_child_before(parent, node, before);
        node
    }

    fn sync_text(&mut self, node: NodeId, text: Option<String>) {
        if let Some(text) = text {
            self.tree.set_text(node, text);
        } else if self.tree.text_content.remove(node).is_some() {
            self.tree.mark_dirty(
                node,
                DirtyFlags::TEXT | DirtyFlags::LAYOUT | DirtyFlags::PAINT,
            );
        }
    }

    fn sync_image(&mut self, node: NodeId, image: Option<ImageSource>) {
        match image {
            Some(source) => {
                if self.tree.image_states.get(node).map(|state| &state.source) != Some(&source) {
                    self.tree.image_states.insert(node, ImageState { source });
                    self.tree
                        .mark_dirty(node, DirtyFlags::LAYOUT | DirtyFlags::PAINT);
                }
            }
            None => {
                if self.tree.image_states.remove(node).is_some() {
                    self.tree
                        .mark_dirty(node, DirtyFlags::LAYOUT | DirtyFlags::PAINT);
                }
            }
        }
    }

    fn sync_events(&mut self, node: NodeId, events: EventHandlers) {
        if events.click.is_some() {
            self.tree.event_handlers.insert(node, events);
            if let Some(node) = self.tree.node_mut(node) {
                node.event_flags.insert(EventFlags::CLICK);
                node.dirty.insert(DirtyFlags::EVENTS);
            }
        } else {
            let removed = self.tree.event_handlers.remove(node).is_some();
            if let Some(node) = self.tree.node_mut(node) {
                let had_flag = node.event_flags.contains(EventFlags::CLICK);
                node.event_flags.remove(EventFlags::CLICK);
                if removed || had_flag {
                    node.dirty.insert(DirtyFlags::EVENTS);
                }
            }
        }
    }

    fn apply_bindings_once(&mut self, node: NodeId, bindings: Vec<ElementBinding>) {
        for mut binding in bindings {
            match &mut binding {
                ElementBinding::Text(f) => {
                    self.tree.set_text(node, f());
                }
                ElementBinding::Width(f) => {
                    self.tree.set_width(node, f());
                }
                ElementBinding::Height(f) => {
                    self.tree.set_height(node, f());
                }
                ElementBinding::Background(f) => {
                    self.tree.set_background(node, f());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui_tree::{
        BackgroundStyle, Display, FlexDirection, FlexStyle, SizeStyle, SpacingStyle, StyleFlags,
    };

    fn component() -> Element {
        div().child(text("Component"))
    }

    fn mount(tree: &mut UiTree, region: &mut MountedRegion, element: Element) {
        mount_element(tree, region, element);
    }

    fn root_region(tree: &UiTree) -> MountedRegion {
        MountedRegion::new(tree.root())
    }

    #[test]
    fn mount_creates_single_root_child() {
        let mut tree = UiTree::new();
        let mut region = root_region(&tree);

        mount(&mut tree, &mut region, div().child(text("Hello")));

        assert_eq!(region.nodes.len(), 1);
        assert_eq!(tree.node(region.nodes[0]).unwrap().tag, UiNodeTag::Div);
    }

    #[test]
    fn mount_reuses_node_when_tag_matches() {
        let mut tree = UiTree::new();
        let mut region = root_region(&tree);
        mount(&mut tree, &mut region, div().child(text("One")));
        let first = region.nodes[0];

        mount(&mut tree, &mut region, div().child(text("Two")));

        assert_eq!(region.nodes, [first]);
        assert_eq!(tree.debug_dump(), "root\n  div\n    text: Two\n");
    }

    #[test]
    fn mount_replaces_node_when_tag_changes() {
        let mut tree = UiTree::new();
        let mut region = root_region(&tree);
        mount(&mut tree, &mut region, div());
        let first = region.nodes[0];

        mount(&mut tree, &mut region, button());

        let second = region.nodes[0];
        assert_ne!(first, second);
        assert!(tree.node(first).is_none());
        assert_eq!(tree.node(second).unwrap().tag, UiNodeTag::Button);
    }

    #[test]
    fn mount_removes_extra_children() {
        let mut tree = UiTree::new();
        let mut region = root_region(&tree);
        mount(
            &mut tree,
            &mut region,
            div().child(text("One")).child(text("Two")),
        );
        let parent = region.nodes[0];
        assert_eq!(tree.children(parent).len(), 2);

        mount(&mut tree, &mut region, div().child(text("One")));

        assert_eq!(tree.children(parent).len(), 1);
        assert_eq!(tree.debug_dump(), "root\n  div\n    text: One\n");
    }

    #[test]
    fn mount_updates_text_without_replacing_node() {
        let mut tree = UiTree::new();
        let mut region = root_region(&tree);
        mount(&mut tree, &mut region, text("One"));
        let node = region.nodes[0];

        mount(&mut tree, &mut region, text("Two"));

        assert_eq!(region.nodes, [node]);
        assert_eq!(tree.text(node), Some("Two"));
    }

    #[test]
    fn style_chain_sets_side_tables_and_flags() {
        let mut tree = UiTree::new();
        let mut region = root_region(&tree);

        mount(
            &mut tree,
            &mut region,
            div()
                .w(100.0)
                .h(40.0)
                .padding(8.0)
                .row()
                .bg(Color::rgb_u8(10, 20, 30)),
        );

        let node = region.nodes[0];
        assert_eq!(
            tree.size_styles.get(node),
            Some(&SizeStyle {
                width: Length::Px(100.0),
                height: Length::Px(40.0),
                ..SizeStyle::default()
            })
        );
        assert_eq!(
            tree.spacing_styles.get(node),
            Some(&SpacingStyle {
                padding: crate::Edges::all(8.0),
                ..SpacingStyle::default()
            })
        );
        assert_eq!(
            tree.flex_styles.get(node),
            Some(&FlexStyle {
                display: Display::Flex,
                direction: FlexDirection::Row,
                ..FlexStyle::default()
            })
        );
        assert_eq!(
            tree.background_styles.get(node),
            Some(&BackgroundStyle {
                color: Some(Color::rgb_u8(10, 20, 30))
            })
        );
        assert!(tree
            .node(node)
            .unwrap()
            .style_flags
            .contains(StyleFlags::SIZE | StyleFlags::SPACING | StyleFlags::FLEX));
    }

    #[test]
    fn style_rebuild_replaces_whole_style_groups() {
        let mut tree = UiTree::new();
        let mut region = root_region(&tree);
        mount(&mut tree, &mut region, div().w(100.0).h(40.0));
        let node = region.nodes[0];

        mount(&mut tree, &mut region, div().w(200.0));

        assert_eq!(
            tree.size_styles.get(node),
            Some(&SizeStyle {
                width: Length::Px(200.0),
                height: Length::Auto,
                ..SizeStyle::default()
            })
        );
    }

    #[test]
    fn style_setter_preserves_unrelated_fields() {
        let mut tree = UiTree::new();
        let mut region = root_region(&tree);
        mount(&mut tree, &mut region, div().w(100.0).h(40.0));
        let node = region.nodes[0];

        tree.set_width(node, 200.0);

        assert_eq!(
            tree.size_styles.get(node),
            Some(&SizeStyle {
                width: Length::Px(200.0),
                height: Length::Px(40.0),
                ..SizeStyle::default()
            })
        );
    }

    #[test]
    fn element_shortcut_style_matches_explicit_style() {
        let mut shortcut_tree = UiTree::new();
        let mut explicit_tree = UiTree::new();
        let mut shortcut_region = root_region(&shortcut_tree);
        let mut explicit_region = root_region(&explicit_tree);

        mount(
            &mut shortcut_tree,
            &mut shortcut_region,
            div().w(100.0).h(40.0),
        );
        mount(
            &mut explicit_tree,
            &mut explicit_region,
            div().style(Style::new().w(100.0).h(40.0)),
        );

        let shortcut = shortcut_region.nodes[0];
        let explicit = explicit_region.nodes[0];
        assert_eq!(
            shortcut_tree.size_styles.get(shortcut),
            explicit_tree.size_styles.get(explicit)
        );
    }

    #[test]
    fn bind_style_runs_once_before_reactive_runtime_exists() {
        let mut tree = UiTree::new();
        let mut region = root_region(&tree);

        mount(&mut tree, &mut region, div().w(100.0).bind_w(|| 200.0));

        let node = region.nodes[0];
        assert_eq!(
            tree.size_styles.get(node),
            Some(&SizeStyle {
                width: Length::Px(200.0),
                ..SizeStyle::default()
            })
        );
    }

    #[test]
    fn on_click_sets_event_handler_and_flag() {
        let mut tree = UiTree::new();
        let mut region = root_region(&tree);

        mount(&mut tree, &mut region, button().on_click(|_| {}));

        let node = region.nodes[0];
        assert!(tree.event_handlers.get(node).unwrap().click.is_some());
        assert!(tree
            .node(node)
            .unwrap()
            .event_flags
            .contains(EventFlags::CLICK));
    }

    #[test]
    fn component_function_is_just_element_factory() {
        let mut tree = UiTree::new();
        let mut region = root_region(&tree);

        mount(&mut tree, &mut region, component());

        assert_eq!(tree.debug_dump(), "root\n  div\n    text: Component\n");
    }
}
