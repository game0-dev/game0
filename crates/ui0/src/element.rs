use std::collections::{HashMap, HashSet};

use crate::app::EventCx;
use crate::reactive::{
    ForEachElement, Memo, ReactiveGraph, ReactiveRuntime, RegionState, ShowElement, Signal,
};
use crate::ui_tree::{
    AlignItems, Color, DirtyFlags, EventFlags, EventHandlers, ImageSource, ImageState,
    JustifyContent, Length, NodeId, Overflow, Style, UiNodeTag, UiTree,
};

pub enum Reactive<T> {
    Static(T),
    Dynamic(Box<dyn FnMut() -> T>),
}

pub trait IntoTextValue {
    fn into_text_value(self) -> Reactive<String>;
}

impl IntoTextValue for String {
    fn into_text_value(self) -> Reactive<String> {
        Reactive::Static(self)
    }
}

impl IntoTextValue for &str {
    fn into_text_value(self) -> Reactive<String> {
        Reactive::Static(self.to_string())
    }
}

impl<F, S> IntoTextValue for F
where
    F: FnMut() -> S + 'static,
    S: Into<String> + 'static,
{
    fn into_text_value(mut self) -> Reactive<String> {
        Reactive::Dynamic(Box::new(move || self().into()))
    }
}

impl IntoTextValue for Signal<String> {
    fn into_text_value(self) -> Reactive<String> {
        Reactive::Dynamic(Box::new(move || self.get()))
    }
}

impl IntoTextValue for Memo<String> {
    fn into_text_value(self) -> Reactive<String> {
        Reactive::Dynamic(Box::new(move || self.get()))
    }
}

pub trait IntoLengthValue {
    fn into_length_value(self) -> Reactive<Length>;
}

impl IntoLengthValue for Length {
    fn into_length_value(self) -> Reactive<Length> {
        Reactive::Static(self)
    }
}

impl IntoLengthValue for f32 {
    fn into_length_value(self) -> Reactive<Length> {
        Reactive::Static(self.into())
    }
}

impl<F, V> IntoLengthValue for F
where
    F: FnMut() -> V + 'static,
    V: Into<Length> + 'static,
{
    fn into_length_value(mut self) -> Reactive<Length> {
        Reactive::Dynamic(Box::new(move || self().into()))
    }
}

impl IntoLengthValue for Signal<Length> {
    fn into_length_value(self) -> Reactive<Length> {
        Reactive::Dynamic(Box::new(move || self.get()))
    }
}

impl IntoLengthValue for Signal<f32> {
    fn into_length_value(self) -> Reactive<Length> {
        Reactive::Dynamic(Box::new(move || self.get().into()))
    }
}

impl IntoLengthValue for Memo<Length> {
    fn into_length_value(self) -> Reactive<Length> {
        Reactive::Dynamic(Box::new(move || self.get()))
    }
}

impl IntoLengthValue for Memo<f32> {
    fn into_length_value(self) -> Reactive<Length> {
        Reactive::Dynamic(Box::new(move || self.get().into()))
    }
}

pub trait IntoColorValue {
    fn into_color_value(self) -> Reactive<Color>;
}

impl IntoColorValue for Color {
    fn into_color_value(self) -> Reactive<Color> {
        Reactive::Static(self)
    }
}

impl<F> IntoColorValue for F
where
    F: FnMut() -> Color + 'static,
{
    fn into_color_value(mut self) -> Reactive<Color> {
        Reactive::Dynamic(Box::new(move || self()))
    }
}

impl IntoColorValue for Signal<Color> {
    fn into_color_value(self) -> Reactive<Color> {
        Reactive::Dynamic(Box::new(move || self.get()))
    }
}

impl IntoColorValue for Memo<Color> {
    fn into_color_value(self) -> Reactive<Color> {
        Reactive::Dynamic(Box::new(move || self.get()))
    }
}

pub struct Element {
    pub(crate) tag: UiNodeTag,
    pub(crate) style: Style,
    pub(crate) text: Option<Reactive<String>>,
    pub(crate) image: Option<ImageSource>,
    pub(crate) events: EventHandlers,
    pub(crate) bindings: Vec<ElementBinding>,
    pub(crate) children: Vec<ElementChild>,
}

pub enum ElementChild {
    Static(Element),
    Dynamic(DynamicChild),
    Show(ShowElement),
    ForEach(ForEachElement),
}

pub struct DynamicChild {
    build: Box<dyn FnMut() -> Element>,
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

    pub(crate) fn fragment() -> Self {
        Self::new(UiNodeTag::Fragment)
    }

    pub fn child<C>(mut self, child: C) -> Self
    where
        C: IntoChild,
    {
        self.children.push(child.into_child());
        self
    }

    pub fn child_fn<F, E>(mut self, f: F) -> Self
    where
        F: FnMut() -> E + 'static,
        E: IntoElement + 'static,
    {
        self.children.push(DynamicChild::new(f).into_child());
        self
    }

    pub fn children<I, C>(mut self, children: I) -> Self
    where
        I: IntoIterator<Item = C>,
        C: IntoChild,
    {
        self.children
            .extend(children.into_iter().map(IntoChild::into_child));
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

    pub fn w<V>(mut self, value: V) -> Self
    where
        V: IntoLengthValue,
    {
        match value.into_length_value() {
            Reactive::Static(value) => self.style = self.style.w(value),
            Reactive::Dynamic(f) => self.bindings.push(ElementBinding::Width(f)),
        }
        self
    }

    pub fn bind_w<F, V>(self, f: F) -> Self
    where
        F: FnMut() -> V + 'static,
        V: Into<Length> + 'static,
    {
        self.w(f)
    }

    pub fn h<V>(mut self, value: V) -> Self
    where
        V: IntoLengthValue,
    {
        match value.into_length_value() {
            Reactive::Static(value) => self.style = self.style.h(value),
            Reactive::Dynamic(f) => self.bindings.push(ElementBinding::Height(f)),
        }
        self
    }

    pub fn bind_h<F, V>(self, f: F) -> Self
    where
        F: FnMut() -> V + 'static,
        V: Into<Length> + 'static,
    {
        self.h(f)
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

    pub fn align_self(mut self, value: AlignItems) -> Self {
        self.style = self.style.align_self(value);
        self
    }

    pub fn justify_content(mut self, value: JustifyContent) -> Self {
        self.style = self.style.justify_content(value);
        self
    }

    pub fn flex_grow(mut self, value: f32) -> Self {
        self.style = self.style.flex_grow(value);
        self
    }

    pub fn flex_shrink(mut self, value: f32) -> Self {
        self.style = self.style.flex_shrink(value);
        self
    }

    pub fn flex_basis(mut self, value: impl Into<Length>) -> Self {
        self.style = self.style.flex_basis(value);
        self
    }

    pub fn bg<V>(mut self, value: V) -> Self
    where
        V: IntoColorValue,
    {
        match value.into_color_value() {
            Reactive::Static(color) => self.style = self.style.bg(color),
            Reactive::Dynamic(f) => self.bindings.push(ElementBinding::Background(f)),
        }
        self
    }

    pub fn bind_bg<F>(self, f: F) -> Self
    where
        F: FnMut() -> Color + 'static,
    {
        self.bg(f)
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
        self.text = Some(f.into_text_value());
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

    pub fn left(mut self, value: impl Into<Length>) -> Self {
        self.style = self.style.left(value);
        self
    }

    pub fn right(mut self, value: impl Into<Length>) -> Self {
        self.style = self.style.right(value);
        self
    }

    pub fn top(mut self, value: impl Into<Length>) -> Self {
        self.style = self.style.top(value);
        self
    }

    pub fn bottom(mut self, value: impl Into<Length>) -> Self {
        self.style = self.style.bottom(value);
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

impl DynamicChild {
    fn new<F, E>(mut f: F) -> Self
    where
        F: FnMut() -> E + 'static,
        E: IntoElement + 'static,
    {
        Self {
            build: Box::new(move || f().into_element()),
        }
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

pub trait IntoChild {
    fn into_child(self) -> ElementChild;
}

impl IntoChild for Element {
    fn into_child(self) -> ElementChild {
        ElementChild::Static(self)
    }
}

impl IntoChild for String {
    fn into_child(self) -> ElementChild {
        ElementChild::Static(text(self))
    }
}

impl IntoChild for &str {
    fn into_child(self) -> ElementChild {
        ElementChild::Static(text(self))
    }
}

impl IntoChild for DynamicChild {
    fn into_child(self) -> ElementChild {
        ElementChild::Dynamic(self)
    }
}

impl IntoChild for ShowElement {
    fn into_child(self) -> ElementChild {
        ElementChild::Show(self)
    }
}

impl IntoChild for ForEachElement {
    fn into_child(self) -> ElementChild {
        ElementChild::ForEach(self)
    }
}

pub(crate) enum ElementBinding {
    Width(Box<dyn FnMut() -> Length>),
    Height(Box<dyn FnMut() -> Length>),
    Background(Box<dyn FnMut() -> Color>),
}

pub(crate) struct MountedRegion {
    pub(crate) parent: NodeId,
    pub(crate) nodes: Vec<NodeId>,
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

pub fn text<V>(value: V) -> Element
where
    V: IntoTextValue,
{
    let mut element = Element::new(UiNodeTag::Text);
    element.text = Some(value.into_text_value());
    element
}

pub(crate) fn mount_element(
    tree: &mut UiTree,
    reactive: &ReactiveRuntime,
    region: &mut MountedRegion,
    element: Element,
) {
    RegionRebuilder::new(tree, reactive).rebuild_region(region, vec![element.into_child()]);
}

struct RegionRebuilder<'a> {
    tree: &'a mut UiTree,
    reactive: &'a ReactiveRuntime,
}

impl<'a> RegionRebuilder<'a> {
    fn new(tree: &'a mut UiTree, reactive: &'a ReactiveRuntime) -> Self {
        Self { tree, reactive }
    }

    fn rebuild_region(&mut self, region: &mut MountedRegion, children: Vec<ElementChild>) {
        region.nodes =
            self.rebuild_children(region.parent, std::mem::take(&mut region.nodes), children);
    }

    fn rebuild_region_state(&mut self, region: &mut RegionState, children: Vec<ElementChild>) {
        let Some(parent) = region.parent else {
            return;
        };
        region.nodes = self.rebuild_children(parent, std::mem::take(&mut region.nodes), children);
    }

    fn rebuild_children(
        &mut self,
        parent: NodeId,
        old_children: Vec<NodeId>,
        children: Vec<ElementChild>,
    ) -> Vec<NodeId> {
        let new_len = children.len();
        let mut new_nodes = Vec::with_capacity(new_len);

        for (index, child) in children.into_iter().enumerate() {
            let existing = old_children.get(index).copied();
            let before = old_children
                .get(index + 1)
                .copied()
                .filter(|node| self.tree.node(*node).and_then(|node| node.parent) == Some(parent));
            new_nodes.push(self.rebuild_child(parent, existing, before, child));
        }

        for old_child in old_children.into_iter().skip(new_len) {
            if self.tree.node(old_child).and_then(|node| node.parent) == Some(parent) {
                self.tree.remove_subtree(old_child);
            }
        }

        new_nodes
    }

    fn rebuild_child(
        &mut self,
        parent: NodeId,
        existing: Option<NodeId>,
        before: Option<NodeId>,
        child: ElementChild,
    ) -> NodeId {
        match child {
            ElementChild::Static(element) => self.rebuild_node(parent, existing, before, element),
            ElementChild::Dynamic(child) => {
                let node = self.rebuild_fragment(parent, existing, before);
                self.mount_dynamic_child(node, child);
                node
            }
            ElementChild::Show(show) => {
                let node = self.rebuild_fragment(parent, existing, before);
                self.mount_show(node, show);
                node
            }
            ElementChild::ForEach(for_each) => {
                let node = self.rebuild_fragment(parent, existing, before);
                self.mount_for_each(node, for_each);
                node
            }
        }
    }

    fn rebuild_fragment(
        &mut self,
        parent: NodeId,
        existing: Option<NodeId>,
        before: Option<NodeId>,
    ) -> NodeId {
        let element = Element::fragment();
        self.rebuild_node(parent, existing, before, element)
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
        self.apply_bindings(node, bindings);

        let old_children = self.tree.children(node).to_vec();
        self.rebuild_children(node, old_children, children);
        node
    }

    fn create_node(&mut self, parent: NodeId, before: Option<NodeId>, tag: UiNodeTag) -> NodeId {
        let node = self.tree.create_node(tag);
        self.tree.insert_child_before(parent, node, before);
        node
    }

    fn sync_text(&mut self, node: NodeId, text: Option<Reactive<String>>) {
        match text {
            Some(Reactive::Static(text)) => {
                self.tree.set_text(node, text);
            }
            Some(Reactive::Dynamic(mut f)) => {
                let owner = ReactiveGraph::current_owner();
                self.reactive.create_effect(owner, move |tree| {
                    tree.set_text(node, f());
                });
            }
            None => {
                if self.tree.text_content.remove(node).is_some() {
                    self.tree.mark_dirty(
                        node,
                        DirtyFlags::TEXT | DirtyFlags::LAYOUT | DirtyFlags::PAINT,
                    );
                }
            }
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

    fn apply_bindings(&mut self, node: NodeId, bindings: Vec<ElementBinding>) {
        let owner = ReactiveGraph::current_owner();
        for binding in bindings {
            match binding {
                ElementBinding::Width(mut f) => {
                    self.reactive.create_effect(owner, move |tree| {
                        tree.set_width(node, f());
                    });
                }
                ElementBinding::Height(mut f) => {
                    self.reactive.create_effect(owner, move |tree| {
                        tree.set_height(node, f());
                    });
                }
                ElementBinding::Background(mut f) => {
                    self.reactive.create_effect(owner, move |tree| {
                        tree.set_background(node, f());
                    });
                }
            }
        }
    }

    fn mount_dynamic_child(&mut self, parent: NodeId, mut child: DynamicChild) {
        let reactive = self.reactive.clone();
        let parent_owner = ReactiveGraph::current_owner();
        let mut region = RegionState::new(parent);
        self.reactive.create_effect(parent_owner, move |tree| {
            dispose_region(tree, &reactive, &mut region);
            let owner = reactive.create_child_owner(parent_owner);
            region.owner = Some(owner);
            reactive.enter(owner, || {
                RegionRebuilder::new(tree, &reactive)
                    .rebuild_region_state(&mut region, vec![(child.build)().into_child()]);
            });
        });
    }

    fn mount_show(&mut self, parent: NodeId, mut show: ShowElement) {
        let reactive = self.reactive.clone();
        let parent_owner = ReactiveGraph::current_owner();
        let mut region = RegionState::new(parent);
        let mut current_branch = None;
        self.reactive.create_effect(parent_owner, move |tree| {
            let next_branch = (show.condition)();
            if current_branch == Some(next_branch) {
                return;
            }
            current_branch = Some(next_branch);
            dispose_region(tree, &reactive, &mut region);
            let owner = reactive.create_child_owner(parent_owner);
            region.owner = Some(owner);
            let child = if next_branch {
                Some((show.then_view)())
            } else {
                show.fallback_view.as_mut().map(|fallback| fallback())
            };
            if let Some(child) = child {
                reactive.enter(owner, || {
                    RegionRebuilder::new(tree, &reactive)
                        .rebuild_region_state(&mut region, vec![child.into_child()]);
                });
            }
        });
    }

    fn mount_for_each(&mut self, parent: NodeId, mut for_each: ForEachElement) {
        let reactive = self.reactive.clone();
        let parent_owner = ReactiveGraph::current_owner();
        self.reactive.create_effect(parent_owner, move |tree| {
            let items = (for_each.build)();
            let next_order = items
                .iter()
                .filter(|item| !item.empty)
                .map(|item| item.key.clone())
                .collect::<Vec<_>>();
            let next_keys = items
                .iter()
                .filter(|item| !item.empty)
                .map(|item| item.key.clone())
                .collect::<HashSet<_>>();

            let removed = for_each
                .state
                .rows
                .keys()
                .filter(|key| !next_keys.contains(*key))
                .cloned()
                .collect::<Vec<_>>();
            for key in removed {
                if let Some(mut region) = for_each.state.rows.remove(&key) {
                    dispose_region(tree, &reactive, &mut region);
                }
            }

            if items.iter().any(|item| item.empty) {
                if for_each.state.empty.is_none() {
                    for_each.state.empty = Some(RegionState::new(parent));
                }
            } else if let Some(mut empty) = for_each.state.empty.take() {
                dispose_region(tree, &reactive, &mut empty);
            }

            for item in items {
                if item.empty {
                    if let Some(region) = for_each.state.empty.as_mut() {
                        if region.owner.is_none() {
                            let owner = reactive.create_child_owner(parent_owner);
                            region.owner = Some(owner);
                            reactive.enter(owner, || {
                                RegionRebuilder::new(tree, &reactive)
                                    .rebuild_region_state(region, vec![item.element.into_child()]);
                            });
                        }
                    }
                    continue;
                }

                let region = for_each
                    .state
                    .rows
                    .entry(item.key.clone())
                    .or_insert_with(|| RegionState::new(parent));
                if region.owner.is_none() {
                    let owner = reactive.create_child_owner(parent_owner);
                    region.owner = Some(owner);
                    reactive.enter(owner, || {
                        RegionRebuilder::new(tree, &reactive)
                            .rebuild_region_state(region, vec![item.element.into_child()]);
                    });
                }
            }

            reorder_for_each_nodes(tree, parent, &for_each.state.rows, &next_order);
        });
    }
}

fn dispose_region(tree: &mut UiTree, reactive: &ReactiveRuntime, region: &mut RegionState) {
    if let Some(owner) = region.owner.take() {
        reactive.dispose_owner(owner);
    }
    for node in std::mem::take(&mut region.nodes) {
        tree.remove_subtree(node);
    }
}

fn reorder_for_each_nodes(
    tree: &mut UiTree,
    parent: NodeId,
    rows: &HashMap<String, RegionState>,
    keys: &[String],
) {
    let ordered = keys
        .iter()
        .filter_map(|key| rows.get(key))
        .flat_map(|region| region.nodes.iter().copied())
        .collect::<Vec<_>>();
    for node in ordered {
        tree.insert_child_before(parent, node, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reactive::{for_each, memo, show, signal, ReactiveRuntime};
    use crate::ui_tree::{
        BackgroundStyle, Display, FlexDirection, FlexStyle, SizeStyle, SpacingStyle, StyleFlags,
    };

    fn component() -> Element {
        div().child(text("Component"))
    }

    fn mount(
        tree: &mut UiTree,
        reactive: &ReactiveRuntime,
        region: &mut MountedRegion,
        element: Element,
    ) {
        reactive.enter(reactive.root_owner(), || {
            mount_element(tree, reactive, region, element);
        });
        reactive.flush(tree);
    }

    fn root_region(tree: &UiTree) -> MountedRegion {
        MountedRegion::new(tree.root())
    }

    #[test]
    fn mount_creates_single_root_child() {
        let mut tree = UiTree::new();
        let reactive = ReactiveRuntime::new();
        let mut region = root_region(&tree);

        mount(
            &mut tree,
            &reactive,
            &mut region,
            div().child(text("Hello")),
        );

        assert_eq!(region.nodes.len(), 1);
        assert_eq!(tree.node(region.nodes[0]).unwrap().tag, UiNodeTag::Div);
    }

    #[test]
    fn mount_reuses_node_when_tag_matches() {
        let mut tree = UiTree::new();
        let reactive = ReactiveRuntime::new();
        let mut region = root_region(&tree);
        mount(&mut tree, &reactive, &mut region, div().child(text("One")));
        let first = region.nodes[0];

        mount(&mut tree, &reactive, &mut region, div().child(text("Two")));

        assert_eq!(region.nodes, [first]);
        assert_eq!(tree.debug_dump(), "root\n  div\n    text: Two\n");
    }

    #[test]
    fn mount_replaces_node_when_tag_changes() {
        let mut tree = UiTree::new();
        let reactive = ReactiveRuntime::new();
        let mut region = root_region(&tree);
        mount(&mut tree, &reactive, &mut region, div());
        let first = region.nodes[0];

        mount(&mut tree, &reactive, &mut region, button());

        let second = region.nodes[0];
        assert_ne!(first, second);
        assert!(tree.node(first).is_none());
        assert_eq!(tree.node(second).unwrap().tag, UiNodeTag::Button);
    }

    #[test]
    fn mount_removes_extra_children() {
        let mut tree = UiTree::new();
        let reactive = ReactiveRuntime::new();
        let mut region = root_region(&tree);
        mount(
            &mut tree,
            &reactive,
            &mut region,
            div().child(text("One")).child(text("Two")),
        );
        let parent = region.nodes[0];
        assert_eq!(tree.children(parent).len(), 2);

        mount(&mut tree, &reactive, &mut region, div().child(text("One")));

        assert_eq!(tree.children(parent).len(), 1);
        assert_eq!(tree.debug_dump(), "root\n  div\n    text: One\n");
    }

    #[test]
    fn mount_updates_text_without_replacing_node() {
        let mut tree = UiTree::new();
        let reactive = ReactiveRuntime::new();
        let mut region = root_region(&tree);
        mount(&mut tree, &reactive, &mut region, text("One"));
        let node = region.nodes[0];

        mount(&mut tree, &reactive, &mut region, text("Two"));

        assert_eq!(region.nodes, [node]);
        assert_eq!(tree.text(node), Some("Two"));
    }

    #[test]
    fn style_chain_sets_side_tables_and_flags() {
        let mut tree = UiTree::new();
        let reactive = ReactiveRuntime::new();
        let mut region = root_region(&tree);

        mount(
            &mut tree,
            &reactive,
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
    fn dynamic_text_updates_without_replacing_node() {
        let mut tree = UiTree::new();
        let reactive = ReactiveRuntime::new();
        let mut region = root_region(&tree);
        let value = reactive.enter(reactive.root_owner(), || signal(1));
        let text_value = value.clone();

        mount(
            &mut tree,
            &reactive,
            &mut region,
            text(move || format!("value {}", text_value.get())),
        );
        let node = region.nodes[0];
        assert_eq!(tree.text(node), Some("value 1"));

        value.set(2);
        reactive.flush(&mut tree);

        assert_eq!(region.nodes, [node]);
        assert_eq!(tree.text(node), Some("value 2"));
    }

    #[test]
    fn dynamic_style_updates_side_table() {
        let mut tree = UiTree::new();
        let reactive = ReactiveRuntime::new();
        let mut region = root_region(&tree);
        let height = reactive.enter(reactive.root_owner(), || signal(10.0));
        let height_value = height.clone();

        mount(
            &mut tree,
            &reactive,
            &mut region,
            div().h(move || height_value.get()),
        );
        let node = region.nodes[0];
        assert_eq!(
            tree.size_styles.get(node).map(|style| style.height),
            Some(Length::Px(10.0))
        );

        height.set(20.0);
        reactive.flush(&mut tree);

        assert_eq!(
            tree.size_styles.get(node).map(|style| style.height),
            Some(Length::Px(20.0))
        );
    }

    #[test]
    fn child_fn_replaces_dynamic_region() {
        let mut tree = UiTree::new();
        let reactive = ReactiveRuntime::new();
        let mut region = root_region(&tree);
        let flag = reactive.enter(reactive.root_owner(), || signal(false));
        let flag_value = flag.clone();

        mount(
            &mut tree,
            &reactive,
            &mut region,
            div().child_fn(move || {
                if flag_value.get() {
                    text("on")
                } else {
                    text("off")
                }
            }),
        );
        assert_eq!(
            tree.debug_dump(),
            "root\n  div\n    fragment\n      text: off\n"
        );

        flag.set(true);
        reactive.flush(&mut tree);

        assert_eq!(
            tree.debug_dump(),
            "root\n  div\n    fragment\n      text: on\n"
        );
    }

    #[test]
    fn show_only_switches_when_condition_changes() {
        let mut tree = UiTree::new();
        let reactive = ReactiveRuntime::new();
        let mut region = root_region(&tree);
        let value = reactive.enter(reactive.root_owner(), || signal(1));
        let condition = value.clone();

        mount(
            &mut tree,
            &reactive,
            &mut region,
            div().child(
                show(move || condition.get() > 5)
                    .then(|| text("big"))
                    .fallback(|| text("small")),
            ),
        );
        let fragment = tree.children(region.nodes[0])[0];
        let small = tree.children(fragment)[0];

        value.set(2);
        reactive.flush(&mut tree);
        assert_eq!(tree.children(fragment), &[small]);

        value.set(6);
        reactive.flush(&mut tree);
        assert_eq!(
            tree.debug_dump(),
            "root\n  div\n    fragment\n      text: big\n"
        );
    }

    #[test]
    fn signal_and_memo_are_dynamic_text_inputs() {
        let mut tree = UiTree::new();
        let reactive = ReactiveRuntime::new();
        let mut region = root_region(&tree);
        let name = reactive.enter(reactive.root_owner(), || signal("A".to_string()));
        let name_for_memo = name.clone();
        let label = reactive.enter(reactive.root_owner(), || {
            memo(move || format!("Name {}", name_for_memo.get()))
        });

        mount(&mut tree, &reactive, &mut region, text(label));
        let node = region.nodes[0];
        assert_eq!(tree.text(node), Some("Name A"));

        name.set("B".to_string());
        reactive.flush(&mut tree);

        assert_eq!(tree.text(node), Some("Name B"));
    }

    #[test]
    fn for_each_reorders_rows_by_key() {
        let mut tree = UiTree::new();
        let reactive = ReactiveRuntime::new();
        let mut region = root_region(&tree);
        let items = reactive.enter(reactive.root_owner(), || signal(vec![1, 2]));
        let items_for_view = items.clone();

        mount(
            &mut tree,
            &reactive,
            &mut region,
            div().child(
                for_each(move || items_for_view.get())
                    .key(|item| *item)
                    .row(|item| text(format!("row {item}")))
                    .empty(|| text("empty")),
            ),
        );
        assert_eq!(
            tree.debug_dump(),
            "root\n  div\n    fragment\n      text: row 1\n      text: row 2\n"
        );

        items.set(vec![2, 1]);
        reactive.flush(&mut tree);

        assert_eq!(
            tree.debug_dump(),
            "root\n  div\n    fragment\n      text: row 2\n      text: row 1\n"
        );
    }

    #[test]
    fn component_function_is_just_element_factory() {
        let mut tree = UiTree::new();
        let reactive = ReactiveRuntime::new();
        let mut region = root_region(&tree);

        mount(&mut tree, &reactive, &mut region, component());

        assert_eq!(tree.debug_dump(), "root\n  div\n    text: Component\n");
    }
}
