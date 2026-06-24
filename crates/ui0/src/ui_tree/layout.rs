use std::collections::HashSet;

use slotmap::{Key, KeyData};
use taffy::geometry::{Point, Rect, Size};
use taffy::{
    compute_block_layout, compute_cached_layout, compute_flexbox_layout, compute_grid_layout,
    compute_hidden_layout, compute_leaf_layout, compute_root_layout, round_layout, AvailableSpace,
    Cache, CacheTree, Dimension, Layout, LayoutBlockContainer, LayoutFlexboxContainer,
    LayoutGridContainer, LayoutInput, LayoutOutput, LayoutPartialTree, LengthPercentage,
    LengthPercentageAuto, NodeId as TaffyNodeId, Style as TaffyStyle, TraversePartialTree,
    TraverseTree,
};

use super::{
    AlignItems, DirtyFlags, Display, FlexDirection, JustifyContent, LayoutRect, Length, NodeId,
    Overflow, Position, UiNodeTag, UiTree,
};

pub(crate) struct LayoutNodeState {
    pub(crate) style: TaffyStyle,
    pub(crate) children: Vec<NodeId>,
    pub(crate) cache: Cache,
    pub(crate) unrounded: Layout,
    pub(crate) final_layout: Layout,
    pub(crate) rect: LayoutRect,
}

impl Default for LayoutNodeState {
    fn default() -> Self {
        Self {
            style: TaffyStyle::DEFAULT,
            children: Vec::new(),
            cache: Cache::new(),
            unrounded: Layout::with_order(0),
            final_layout: Layout::with_order(0),
            rect: LayoutRect::default(),
        }
    }
}

impl UiTree {
    pub fn compute_layout(&mut self, width: f32, height: f32) -> bool {
        let root = self.root;
        let mut pass = LayoutPass::new(self);
        pass.invalidate_dirty_caches();
        compute_root_layout(
            &mut pass,
            to_taffy_node(root),
            Size {
                width: AvailableSpace::Definite(width),
                height: AvailableSpace::Definite(height),
            },
        );
        round_layout(&mut pass, to_taffy_node(root));
        let changed = pass.write_absolute_rects(root);
        pass.clear_layout_dirty();
        changed
    }

    pub(crate) fn initialize_layout_node(&mut self, node: NodeId) {
        if !self.is_layout_node(node) {
            return;
        }
        let style = self.build_taffy_style(node);
        let state = self.layout_states.entry(node).unwrap().or_default();
        state.style = style;
    }

    pub(crate) fn sync_layout_style(&mut self, node: NodeId) {
        if !self.is_layout_node(node) {
            return;
        }
        let style = self.build_taffy_style(node);
        let state = self.layout_states.entry(node).unwrap().or_default();
        if state.style != style {
            state.style = style;
            state.cache.clear();
        }
    }

    pub(crate) fn sync_layout_children_from_structure_parent(&mut self, node: NodeId) {
        let Some(layout_node) = self.nearest_layout_node_from(Some(node)) else {
            return;
        };
        self.rebuild_layout_children(layout_node);
        self.invalidate_layout_from(layout_node);
    }

    pub(crate) fn invalidate_layout_from(&mut self, node: NodeId) {
        let mut current = self.nearest_layout_node_from(Some(node));
        let mut cleared = HashSet::new();
        while let Some(node) = current {
            if cleared.insert(node) {
                if let Some(state) = self.layout_states.get_mut(node) {
                    state.cache.clear();
                }
            }
            current = self.nearest_layout_parent(node);
        }
    }

    fn rebuild_layout_children(&mut self, node: NodeId) {
        let children = self.flattened_layout_children(node);
        let state = self.layout_states.entry(node).unwrap().or_default();
        if state.children != children {
            state.children = children;
        }
    }

    fn flattened_layout_children(&self, node: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        for child in self.children(node) {
            self.push_flattened_layout_child(*child, &mut out);
        }
        out
    }

    fn push_flattened_layout_child(&self, node: NodeId, out: &mut Vec<NodeId>) {
        let Some(node_ref) = self.node(node) else {
            return;
        };
        if node_ref.tag == UiNodeTag::Fragment {
            for child in self.children(node) {
                self.push_flattened_layout_child(*child, out);
            }
        } else {
            out.push(node);
        }
    }

    fn is_layout_node(&self, node: NodeId) -> bool {
        self.node(node)
            .map(|node| node.tag != UiNodeTag::Fragment)
            .unwrap_or(false)
    }

    fn nearest_layout_node_from(&self, mut current: Option<NodeId>) -> Option<NodeId> {
        while let Some(node) = current {
            if self.is_layout_node(node) {
                return Some(node);
            }
            current = self.node(node).and_then(|node| node.parent);
        }
        None
    }

    fn nearest_layout_parent(&self, node: NodeId) -> Option<NodeId> {
        self.node(node)
            .and_then(|node| node.parent)
            .and_then(|parent| self.nearest_layout_node_from(Some(parent)))
    }

    fn build_taffy_style(&self, node: NodeId) -> TaffyStyle {
        let mut style = TaffyStyle {
            display: taffy_display(
                self.flex_styles
                    .get(node)
                    .map(|style| style.display)
                    .unwrap_or(Display::Block),
            ),
            ..TaffyStyle::DEFAULT
        };

        if let Some(size) = self.size_styles.get(node) {
            style.size = Size {
                width: dimension(size.width),
                height: dimension(size.height),
            };
            style.min_size = Size {
                width: dimension(size.min_width),
                height: dimension(size.min_height),
            };
            style.max_size = Size {
                width: dimension(size.max_width),
                height: dimension(size.max_height),
            };
        }

        if let Some(spacing) = self.spacing_styles.get(node) {
            style.margin = Rect {
                left: length_auto(Length::Px(spacing.margin.left)),
                right: length_auto(Length::Px(spacing.margin.right)),
                top: length_auto(Length::Px(spacing.margin.top)),
                bottom: length_auto(Length::Px(spacing.margin.bottom)),
            };
            style.padding = Rect {
                left: length(Length::Px(spacing.padding.left)),
                right: length(Length::Px(spacing.padding.right)),
                top: length(Length::Px(spacing.padding.top)),
                bottom: length(Length::Px(spacing.padding.bottom)),
            };
            style.gap = Size {
                width: LengthPercentage::length(spacing.gap),
                height: LengthPercentage::length(spacing.gap),
            };
        }

        if let Some(border) = self.border_styles.get(node) {
            style.border = Rect {
                left: LengthPercentage::length(border.width.left),
                right: LengthPercentage::length(border.width.right),
                top: LengthPercentage::length(border.width.top),
                bottom: LengthPercentage::length(border.width.bottom),
            };
        }

        if let Some(flex) = self.flex_styles.get(node) {
            style.display = taffy_display(flex.display);
            style.flex_direction = taffy_flex_direction(flex.direction);
            style.align_items = Some(taffy_align_items(flex.align_items));
            style.justify_content = Some(taffy_justify_content(flex.justify_content));
            style.align_self = flex.align_self.map(taffy_align_items);
            style.flex_grow = flex.flex_grow;
            style.flex_shrink = flex.flex_shrink;
            style.flex_basis = dimension(flex.flex_basis);
        }

        if let Some(position) = self.position_styles.get(node) {
            style.position = match position.position {
                Position::Relative => taffy::Position::Relative,
                Position::Absolute => taffy::Position::Absolute,
            };
            style.inset = Rect {
                left: length_auto(position.inset.left),
                right: length_auto(position.inset.right),
                top: length_auto(position.inset.top),
                bottom: length_auto(position.inset.bottom),
            };
        }

        if let Some(overflow) = self.overflow_styles.get(node) {
            style.overflow = Point {
                x: taffy_overflow(overflow.x),
                y: taffy_overflow(overflow.y),
            };
        }

        style
    }
}

struct LayoutPass<'a> {
    tree: &'a mut UiTree,
}

impl<'a> LayoutPass<'a> {
    fn new(tree: &'a mut UiTree) -> Self {
        Self { tree }
    }

    fn invalidate_dirty_caches(&mut self) {
        let dirty_nodes = self
            .tree
            .layout_states
            .iter()
            .filter_map(|(node, _)| {
                self.tree
                    .node(node)
                    .map(|node| node.dirty.contains(DirtyFlags::LAYOUT))
                    .unwrap_or(false)
                    .then_some(node)
            })
            .collect::<Vec<_>>();

        for node in dirty_nodes {
            self.tree.invalidate_layout_from(node);
        }
    }

    fn node_from_id(&self, node_id: TaffyNodeId) -> NodeId {
        from_taffy_node(node_id)
    }

    fn node_from_id_mut(&mut self, node_id: TaffyNodeId) -> NodeId {
        from_taffy_node(node_id)
    }

    fn measure_leaf(
        &self,
        node: NodeId,
        known: Size<Option<f32>>,
        available: Size<AvailableSpace>,
    ) -> Size<f32> {
        match self.tree.node(node).map(|node| node.tag) {
            Some(UiNodeTag::Text) => self.measure_text(node, known, available),
            Some(UiNodeTag::Img) => Size {
                width: known.width.unwrap_or(0.0),
                height: known.height.unwrap_or(0.0),
            },
            _ => Size {
                width: known.width.unwrap_or(0.0),
                height: known.height.unwrap_or(0.0),
            },
        }
    }

    fn measure_text(
        &self,
        node: NodeId,
        known: Size<Option<f32>>,
        available: Size<AvailableSpace>,
    ) -> Size<f32> {
        let text = self.tree.text(node).unwrap_or("");
        let font_size = self
            .tree
            .text_styles
            .get(node)
            .and_then(|style| style.font_size)
            .unwrap_or(14.0);
        let char_width = font_size * 0.5;
        let line_height = font_size * 1.2;
        let natural_width = text.chars().count() as f32 * char_width;
        let max_width = match available.width {
            AvailableSpace::Definite(width) => width,
            AvailableSpace::MinContent => char_width,
            AvailableSpace::MaxContent => natural_width,
        };
        let measured_width = known.width.unwrap_or_else(|| natural_width.min(max_width));
        let lines = if natural_width > 0.0 && max_width.is_finite() && max_width > 0.0 {
            (natural_width / max_width).ceil().max(1.0)
        } else {
            1.0
        };
        Size {
            width: measured_width,
            height: known.height.unwrap_or(line_height * lines),
        }
    }

    fn write_absolute_rects(&mut self, root: NodeId) -> bool {
        self.write_absolute_rect(root, 0.0, 0.0)
    }

    fn clear_layout_dirty(&mut self) {
        for (_, node) in self.tree.nodes.iter_mut() {
            node.dirty.remove(DirtyFlags::LAYOUT);
        }
    }

    fn write_absolute_rect(&mut self, node: NodeId, parent_x: f32, parent_y: f32) -> bool {
        let (rect, changed) = {
            let Some(state) = self.tree.layout_states.get_mut(node) else {
                return false;
            };
            let layout = state.final_layout;
            let rect = LayoutRect {
                x: parent_x + layout.location.x,
                y: parent_y + layout.location.y,
                width: layout.size.width,
                height: layout.size.height,
            };
            let changed = state.rect != rect;
            state.rect = rect;
            (rect, changed)
        };
        if changed {
            self.tree.mark_dirty(node, DirtyFlags::PAINT);
        }

        let mut any_changed = changed;
        let children = self
            .tree
            .layout_states
            .get(node)
            .map(|state| state.children.clone())
            .unwrap_or_default();
        for child in children {
            any_changed |= self.write_absolute_rect(child, rect.x, rect.y);
        }
        any_changed
    }
}

struct ChildIter(std::vec::IntoIter<TaffyNodeId>);

impl Iterator for ChildIter {
    type Item = TaffyNodeId;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl TraversePartialTree for LayoutPass<'_> {
    type ChildIter<'b>
        = ChildIter
    where
        Self: 'b;

    fn child_ids(&self, parent_node_id: TaffyNodeId) -> Self::ChildIter<'_> {
        let parent = self.node_from_id(parent_node_id);
        let children = self
            .tree
            .layout_states
            .get(parent)
            .map(|state| {
                state
                    .children
                    .iter()
                    .copied()
                    .map(to_taffy_node)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        ChildIter(children.into_iter())
    }

    fn child_count(&self, parent_node_id: TaffyNodeId) -> usize {
        self.tree
            .layout_states
            .get(self.node_from_id(parent_node_id))
            .map(|state| state.children.len())
            .unwrap_or(0)
    }

    fn get_child_id(&self, parent_node_id: TaffyNodeId, child_index: usize) -> TaffyNodeId {
        let parent = self.node_from_id(parent_node_id);
        to_taffy_node(self.tree.layout_states[parent].children[child_index])
    }
}

impl TraverseTree for LayoutPass<'_> {}

impl LayoutPartialTree for LayoutPass<'_> {
    type CoreContainerStyle<'b>
        = &'b TaffyStyle
    where
        Self: 'b;

    type CustomIdent = String;

    fn get_core_container_style(&self, node_id: TaffyNodeId) -> Self::CoreContainerStyle<'_> {
        &self.tree.layout_states[self.node_from_id(node_id)].style
    }

    fn set_unrounded_layout(&mut self, node_id: TaffyNodeId, layout: &Layout) {
        let node = self.node_from_id_mut(node_id);
        self.tree.layout_states[node].unrounded = *layout;
    }

    fn compute_child_layout(&mut self, node_id: TaffyNodeId, inputs: LayoutInput) -> LayoutOutput {
        compute_cached_layout(self, node_id, inputs, |tree, node_id, inputs| {
            let node = tree.node_from_id(node_id);
            let style = tree.tree.layout_states[node].style.clone();
            if style.display == taffy::Display::None {
                return compute_hidden_layout(tree, node_id);
            }
            match tree.tree.node(node).map(|node| node.tag) {
                Some(UiNodeTag::Text | UiNodeTag::Img) => compute_leaf_layout(
                    inputs,
                    &style,
                    |_val, _basis| 0.0,
                    |known, available| tree.measure_leaf(node, known, available),
                ),
                _ => match style.display {
                    taffy::Display::Flex => compute_flexbox_layout(tree, node_id, inputs),
                    taffy::Display::Grid => compute_grid_layout(tree, node_id, inputs),
                    taffy::Display::Block => compute_block_layout(tree, node_id, inputs, None),
                    taffy::Display::None => compute_hidden_layout(tree, node_id),
                },
            }
        })
    }
}

impl CacheTree for LayoutPass<'_> {
    fn cache_get(&self, node_id: TaffyNodeId, input: &LayoutInput) -> Option<LayoutOutput> {
        self.tree.layout_states[self.node_from_id(node_id)]
            .cache
            .get(input)
    }

    fn cache_store(
        &mut self,
        node_id: TaffyNodeId,
        input: &LayoutInput,
        layout_output: LayoutOutput,
    ) {
        let node = self.node_from_id_mut(node_id);
        self.tree.layout_states[node]
            .cache
            .store(input, layout_output);
    }

    fn cache_clear(&mut self, node_id: TaffyNodeId) {
        let node = self.node_from_id_mut(node_id);
        self.tree.layout_states[node].cache.clear();
    }
}

impl LayoutFlexboxContainer for LayoutPass<'_> {
    type FlexboxContainerStyle<'b>
        = &'b TaffyStyle
    where
        Self: 'b;

    type FlexboxItemStyle<'b>
        = &'b TaffyStyle
    where
        Self: 'b;

    fn get_flexbox_container_style(&self, node_id: TaffyNodeId) -> Self::FlexboxContainerStyle<'_> {
        &self.tree.layout_states[self.node_from_id(node_id)].style
    }

    fn get_flexbox_child_style(&self, child_node_id: TaffyNodeId) -> Self::FlexboxItemStyle<'_> {
        &self.tree.layout_states[self.node_from_id(child_node_id)].style
    }
}

impl LayoutGridContainer for LayoutPass<'_> {
    type GridContainerStyle<'b>
        = &'b TaffyStyle
    where
        Self: 'b;

    type GridItemStyle<'b>
        = &'b TaffyStyle
    where
        Self: 'b;

    fn get_grid_container_style(&self, node_id: TaffyNodeId) -> Self::GridContainerStyle<'_> {
        &self.tree.layout_states[self.node_from_id(node_id)].style
    }

    fn get_grid_child_style(&self, child_node_id: TaffyNodeId) -> Self::GridItemStyle<'_> {
        &self.tree.layout_states[self.node_from_id(child_node_id)].style
    }
}

impl LayoutBlockContainer for LayoutPass<'_> {
    type BlockContainerStyle<'b>
        = &'b TaffyStyle
    where
        Self: 'b;

    type BlockItemStyle<'b>
        = &'b TaffyStyle
    where
        Self: 'b;

    fn get_block_container_style(&self, node_id: TaffyNodeId) -> Self::BlockContainerStyle<'_> {
        &self.tree.layout_states[self.node_from_id(node_id)].style
    }

    fn get_block_child_style(&self, child_node_id: TaffyNodeId) -> Self::BlockItemStyle<'_> {
        &self.tree.layout_states[self.node_from_id(child_node_id)].style
    }
}

impl taffy::RoundTree for LayoutPass<'_> {
    fn get_unrounded_layout(&self, node_id: TaffyNodeId) -> Layout {
        self.tree.layout_states[self.node_from_id(node_id)].unrounded
    }

    fn set_final_layout(&mut self, node_id: TaffyNodeId, layout: &Layout) {
        let node = self.node_from_id_mut(node_id);
        self.tree.layout_states[node].final_layout = *layout;
    }
}

fn to_taffy_node(node: NodeId) -> TaffyNodeId {
    TaffyNodeId::from(node.data().as_ffi())
}

fn from_taffy_node(node: TaffyNodeId) -> NodeId {
    KeyData::from_ffi(u64::from(node)).into()
}

fn dimension(value: Length) -> Dimension {
    match value {
        Length::Auto => Dimension::auto(),
        Length::Px(value) => Dimension::length(value),
        Length::Percent(value) => Dimension::percent(value),
    }
}

fn length(value: Length) -> LengthPercentage {
    match value {
        Length::Auto => LengthPercentage::length(0.0),
        Length::Px(value) => LengthPercentage::length(value),
        Length::Percent(value) => LengthPercentage::percent(value),
    }
}

fn length_auto(value: Length) -> LengthPercentageAuto {
    match value {
        Length::Auto => LengthPercentageAuto::auto(),
        Length::Px(value) => LengthPercentageAuto::length(value),
        Length::Percent(value) => LengthPercentageAuto::percent(value),
    }
}

fn taffy_display(value: Display) -> taffy::Display {
    match value {
        Display::None => taffy::Display::None,
        Display::Block => taffy::Display::Block,
        Display::Flex => taffy::Display::Flex,
    }
}

fn taffy_flex_direction(value: FlexDirection) -> taffy::FlexDirection {
    match value {
        FlexDirection::Row => taffy::FlexDirection::Row,
        FlexDirection::Column => taffy::FlexDirection::Column,
    }
}

fn taffy_align_items(value: AlignItems) -> taffy::AlignItems {
    match value {
        AlignItems::Start => taffy::AlignItems::FLEX_START,
        AlignItems::Center => taffy::AlignItems::CENTER,
        AlignItems::End => taffy::AlignItems::FLEX_END,
        AlignItems::Stretch => taffy::AlignItems::STRETCH,
    }
}

fn taffy_justify_content(value: JustifyContent) -> taffy::JustifyContent {
    match value {
        JustifyContent::Start => taffy::JustifyContent::FLEX_START,
        JustifyContent::Center => taffy::JustifyContent::CENTER,
        JustifyContent::End => taffy::JustifyContent::FLEX_END,
        JustifyContent::SpaceBetween => taffy::JustifyContent::SPACE_BETWEEN,
    }
}

fn taffy_overflow(value: Overflow) -> taffy::Overflow {
    match value {
        Overflow::Visible => taffy::Overflow::Visible,
        Overflow::Hidden => taffy::Overflow::Hidden,
        Overflow::Scroll => taffy::Overflow::Scroll,
    }
}
