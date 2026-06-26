use std::ops::Range;

use crate::ui_tree::{
    Color, Corners, DirtyFlags, Edges, LayoutRect, NodeId, Overflow, SurfaceColorSpace,
    SurfaceSource, UiNodeTag, UiTree,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct RectId(pub(crate) usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct TextId(pub(crate) usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ImageId(pub(crate) usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct SurfaceId(pub(crate) usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ClipId(pub(crate) usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct TransformId(pub(crate) usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct OpacityId(pub(crate) usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct LayerId(pub(crate) usize);

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PaintCommand {
    DrawRect(RectId),
    DrawText(TextId),
    DrawImage(ImageId),
    DrawSurface(SurfaceId),
    PushClip(ClipId),
    PopClip,
    PushTransform(TransformId),
    PopTransform,
    PushOpacity(OpacityId),
    PopOpacity,
    BeginLayer(LayerId),
    EndLayer,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RectPrimitive {
    pub(crate) node: NodeId,
    pub(crate) rect: LayoutRect,
    pub(crate) fill: Color,
    pub(crate) border_color: Color,
    pub(crate) border_width: Edges,
    pub(crate) radius: Corners,
    pub(crate) opacity: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TextPrimitive {
    pub(crate) node: NodeId,
    pub(crate) rect: LayoutRect,
    pub(crate) text: String,
    pub(crate) color: Color,
    pub(crate) font_size: f32,
    pub(crate) opacity: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ImagePrimitive {
    pub(crate) node: NodeId,
    pub(crate) rect: LayoutRect,
    pub(crate) opacity: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SurfacePrimitive {
    pub(crate) node: NodeId,
    pub(crate) source: SurfaceSource,
    pub(crate) rect: LayoutRect,
    pub(crate) clip: Option<ClipId>,
    pub(crate) transform: Option<TransformId>,
    pub(crate) color_space: SurfaceColorSpace,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ClipState {
    pub(crate) rect: LayoutRect,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TransformState {
    pub(crate) translate_x: f32,
    pub(crate) translate_y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct OpacityState {
    pub(crate) opacity: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LayerState {
    pub(crate) bounds: LayoutRect,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct PaintScene {
    pub(crate) commands: Vec<PaintCommand>,
    pub(crate) rects: Vec<RectPrimitive>,
    pub(crate) texts: Vec<TextPrimitive>,
    pub(crate) images: Vec<ImagePrimitive>,
    pub(crate) surfaces: Vec<SurfacePrimitive>,
    pub(crate) clips: Vec<ClipState>,
    pub(crate) transforms: Vec<TransformState>,
    pub(crate) opacities: Vec<OpacityState>,
    pub(crate) layers: Vec<LayerState>,
}

impl PaintScene {
    pub(crate) fn build(tree: &mut UiTree, previous: Option<&PaintScene>) -> Self {
        let mut builder = PaintSceneBuilder {
            tree,
            previous,
            scene: PaintScene::default(),
            clip_stack: Vec::new(),
            transform_stack: Vec::new(),
            opacity_stack: Vec::new(),
            accumulated_opacity: 1.0,
        };
        builder.paint_node(builder.tree.root());
        builder.scene
    }

    pub(crate) fn command_len(&self) -> usize {
        self.commands.len()
    }

    pub(crate) fn replay_range(&mut self, previous: &PaintScene, range: Range<usize>) {
        for command in &previous.commands[range] {
            let command = match command {
                PaintCommand::DrawRect(id) => {
                    let next = self.push_rect(previous.rects[id.0].clone());
                    PaintCommand::DrawRect(next)
                }
                PaintCommand::DrawText(id) => {
                    let next = self.push_text(previous.texts[id.0].clone());
                    PaintCommand::DrawText(next)
                }
                PaintCommand::DrawImage(id) => {
                    let next = self.push_image(previous.images[id.0].clone());
                    PaintCommand::DrawImage(next)
                }
                PaintCommand::DrawSurface(id) => {
                    let next = self.push_surface(previous.surfaces[id.0]);
                    PaintCommand::DrawSurface(next)
                }
                PaintCommand::PushClip(id) => {
                    let next = self.push_clip(previous.clips[id.0]);
                    PaintCommand::PushClip(next)
                }
                PaintCommand::PushTransform(id) => {
                    let next = self.push_transform(previous.transforms[id.0]);
                    PaintCommand::PushTransform(next)
                }
                PaintCommand::PushOpacity(id) => {
                    let next = self.push_opacity(previous.opacities[id.0]);
                    PaintCommand::PushOpacity(next)
                }
                PaintCommand::BeginLayer(id) => {
                    let next = self.push_layer(previous.layers[id.0]);
                    PaintCommand::BeginLayer(next)
                }
                PaintCommand::PopClip => PaintCommand::PopClip,
                PaintCommand::PopTransform => PaintCommand::PopTransform,
                PaintCommand::PopOpacity => PaintCommand::PopOpacity,
                PaintCommand::EndLayer => PaintCommand::EndLayer,
            };
            self.commands.push(command);
        }
    }

    pub(crate) fn push_rect(&mut self, primitive: RectPrimitive) -> RectId {
        let id = RectId(self.rects.len());
        self.rects.push(primitive);
        id
    }

    pub(crate) fn push_text(&mut self, primitive: TextPrimitive) -> TextId {
        let id = TextId(self.texts.len());
        self.texts.push(primitive);
        id
    }

    pub(crate) fn push_image(&mut self, primitive: ImagePrimitive) -> ImageId {
        let id = ImageId(self.images.len());
        self.images.push(primitive);
        id
    }

    pub(crate) fn push_surface(&mut self, primitive: SurfacePrimitive) -> SurfaceId {
        let id = SurfaceId(self.surfaces.len());
        self.surfaces.push(primitive);
        id
    }

    pub(crate) fn push_clip(&mut self, state: ClipState) -> ClipId {
        let id = ClipId(self.clips.len());
        self.clips.push(state);
        id
    }

    pub(crate) fn push_transform(&mut self, state: TransformState) -> TransformId {
        let id = TransformId(self.transforms.len());
        self.transforms.push(state);
        id
    }

    pub(crate) fn push_opacity(&mut self, state: OpacityState) -> OpacityId {
        let id = OpacityId(self.opacities.len());
        self.opacities.push(state);
        id
    }

    pub(crate) fn push_layer(&mut self, state: LayerState) -> LayerId {
        let id = LayerId(self.layers.len());
        self.layers.push(state);
        id
    }
}

struct PaintSceneBuilder<'a> {
    tree: &'a mut UiTree,
    previous: Option<&'a PaintScene>,
    scene: PaintScene,
    clip_stack: Vec<ClipId>,
    transform_stack: Vec<TransformId>,
    opacity_stack: Vec<OpacityId>,
    accumulated_opacity: f32,
}

impl PaintSceneBuilder<'_> {
    fn paint_node(&mut self, node: NodeId) {
        let Some(node_ref) = self.tree.node(node) else {
            return;
        };
        if self.can_replay_subtree(node) {
            self.replay_node(node);
            return;
        }

        let subtree_start = self.scene.command_len();
        let self_start = subtree_start;
        let mut pushed_clip = false;
        let mut pushed_opacity = false;

        if node_ref.tag != UiNodeTag::Fragment && self.is_display_none(node) {
            self.update_ranges(node, self_start, self_start, subtree_start, subtree_start);
            return;
        }

        if node_ref.tag != UiNodeTag::Fragment {
            pushed_clip = self.push_node_clip(node);
            pushed_opacity = self.push_node_opacity(node);
            self.push_node_primitives(node);
        }
        let self_end = self.scene.command_len();

        let children = self.tree.children(node).to_vec();
        for child in children {
            self.paint_node(child);
        }

        if pushed_opacity {
            self.pop_node_opacity();
        }
        if pushed_clip {
            self.pop_node_clip();
        }

        let subtree_end = self.scene.command_len();
        self.update_ranges(node, self_start, self_end, subtree_start, subtree_end);
    }

    fn can_replay_subtree(&self, node: NodeId) -> bool {
        if self.previous.is_none() || self.subtree_has_render_dirty(node) {
            return false;
        }
        self.tree
            .render_states
            .get(node)
            .map(|state| state.has_cached_subtree())
            .unwrap_or(false)
    }

    fn replay_node(&mut self, node: NodeId) {
        let state = self
            .tree
            .render_states
            .get(node)
            .cloned()
            .unwrap_or_default();
        let start = self.scene.command_len();
        self.scene
            .replay_range(self.previous.unwrap(), state.subtree_commands);
        let end = self.scene.command_len();
        let render_state = self.tree.render_states.entry(node).unwrap().or_default();
        render_state.subtree_commands = start..end;
        render_state.self_commands = start..start;
        render_state.paint_dirty = false;
        render_state.subtree_paint_dirty = false;
    }

    fn subtree_has_render_dirty(&self, node: NodeId) -> bool {
        let Some(node_ref) = self.tree.node(node) else {
            return false;
        };
        if node_ref.dirty.intersects(
            DirtyFlags::STRUCTURE
                | DirtyFlags::STYLE
                | DirtyFlags::PRE_PAINT
                | DirtyFlags::PAINT
                | DirtyFlags::COMPOSITE
                | DirtyFlags::TEXT,
        ) {
            return true;
        }
        self.tree
            .children(node)
            .iter()
            .any(|child| self.subtree_has_render_dirty(*child))
    }

    fn push_node_clip(&mut self, node: NodeId) -> bool {
        let Some(overflow) = self.tree.overflow_styles.get(node) else {
            return false;
        };
        if overflow.x == Overflow::Visible && overflow.y == Overflow::Visible {
            return false;
        }
        let Some(rect) = self.tree.layout_rect(node) else {
            return false;
        };
        let clip = self.scene.push_clip(ClipState { rect });
        self.scene.commands.push(PaintCommand::PushClip(clip));
        self.clip_stack.push(clip);
        true
    }

    fn pop_node_clip(&mut self) {
        self.clip_stack.pop();
        self.scene.commands.push(PaintCommand::PopClip);
    }

    fn push_node_opacity(&mut self, node: NodeId) -> bool {
        let Some(opacity) = self
            .tree
            .effect_styles
            .get(node)
            .and_then(|style| style.opacity)
            .filter(|opacity| *opacity < 1.0)
        else {
            return false;
        };
        let opacity = opacity.clamp(0.0, 1.0);
        let state = self.scene.push_opacity(OpacityState { opacity });
        self.scene.commands.push(PaintCommand::PushOpacity(state));
        self.opacity_stack.push(state);
        self.accumulated_opacity *= opacity;
        true
    }

    fn pop_node_opacity(&mut self) {
        if let Some(opacity) = self.opacity_stack.pop() {
            let value = self.scene.opacities[opacity.0].opacity.max(0.0001);
            self.accumulated_opacity /= value;
        }
        self.scene.commands.push(PaintCommand::PopOpacity);
    }

    fn push_node_primitives(&mut self, node: NodeId) {
        self.push_rect(node);
        self.push_text(node);
        self.push_image(node);
        self.push_surface(node);
    }

    fn push_rect(&mut self, node: NodeId) {
        let Some(rect) = self.valid_rect(node) else {
            return;
        };
        let tag = self.tree.node(node).map(|node| node.tag);
        let fill = self
            .tree
            .background_styles
            .get(node)
            .and_then(|style| style.color)
            .or_else(|| (tag == Some(UiNodeTag::Button)).then_some(Color::rgb_u8(52, 58, 70)))
            .unwrap_or_else(Color::transparent);
        let mut border = self
            .tree
            .border_styles
            .get(node)
            .copied()
            .unwrap_or_default();
        if tag == Some(UiNodeTag::Button) {
            border.color.get_or_insert(Color::rgb_u8(88, 98, 118));
            if border.width == Edges::default() {
                border.width = Edges::all(1.0);
            }
            if border.radius == Corners::default() {
                border.radius = Corners::all(4.0);
            }
        }
        if fill.a <= 0.0 && border.color.map(|color| color.a).unwrap_or(0.0) <= 0.0 {
            return;
        }

        let id = self.scene.push_rect(RectPrimitive {
            node,
            rect,
            fill,
            border_color: border.color.unwrap_or_else(Color::transparent),
            border_width: border.width,
            radius: border.radius,
            opacity: self.accumulated_opacity,
        });
        self.scene.commands.push(PaintCommand::DrawRect(id));
    }

    fn push_text(&mut self, node: NodeId) {
        let Some(text) = self.tree.text(node) else {
            return;
        };
        if text.is_empty() {
            return;
        }
        let Some(rect) = self.valid_rect(node) else {
            return;
        };
        let style = self.tree.text_styles.get(node);
        let color = style
            .and_then(|style| style.color)
            .unwrap_or_else(|| Color::rgb_u8(235, 238, 245));
        let font_size = style.and_then(|style| style.font_size).unwrap_or(14.0);

        let id = self.scene.push_text(TextPrimitive {
            node,
            rect,
            text: text.to_string(),
            color,
            font_size,
            opacity: self.accumulated_opacity,
        });
        self.scene.commands.push(PaintCommand::DrawText(id));
    }

    fn push_image(&mut self, node: NodeId) {
        if self.tree.image_states.get(node).is_none() {
            return;
        }
        let Some(rect) = self.valid_rect(node) else {
            return;
        };
        let id = self.scene.push_image(ImagePrimitive {
            node,
            rect,
            opacity: self.accumulated_opacity,
        });
        self.scene.commands.push(PaintCommand::DrawImage(id));
    }

    fn push_surface(&mut self, node: NodeId) {
        let Some(surface) = self.tree.surface(node) else {
            return;
        };
        let Some(rect) = self.valid_rect(node) else {
            return;
        };
        let id = self.scene.push_surface(SurfacePrimitive {
            node,
            source: surface.source,
            rect,
            clip: self.clip_stack.last().copied(),
            transform: self.transform_stack.last().copied(),
            color_space: surface.color_space,
        });
        self.scene.commands.push(PaintCommand::DrawSurface(id));
    }

    fn valid_rect(&self, node: NodeId) -> Option<LayoutRect> {
        let rect = self.tree.layout_rect(node)?;
        (rect.width > 0.0 && rect.height > 0.0).then_some(rect)
    }

    fn is_display_none(&self, node: NodeId) -> bool {
        self.tree
            .flex_styles
            .get(node)
            .map(|style| style.display == crate::ui_tree::Display::None)
            .unwrap_or(false)
    }

    fn update_ranges(
        &mut self,
        node: NodeId,
        self_start: usize,
        self_end: usize,
        subtree_start: usize,
        subtree_end: usize,
    ) {
        let render_state = self.tree.render_states.entry(node).unwrap().or_default();
        render_state.paint_dirty = false;
        render_state.subtree_paint_dirty = false;
        render_state.self_commands = self_start..self_end;
        render_state.subtree_commands = subtree_start..subtree_end;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui_tree::{Color, Overflow, Style, SurfaceSource};

    #[test]
    fn scene_preserves_mixed_paint_order() {
        let mut tree = UiTree::new();
        let root_child = tree.create_node(UiNodeTag::Div);
        let text = tree.create_node(UiNodeTag::Text);
        let overlay = tree.create_node(UiNodeTag::Div);
        tree.append_child(tree.root(), root_child);
        tree.append_child(root_child, text);
        tree.append_child(root_child, overlay);
        tree.apply_style(
            root_child,
            Style::new().w(300.0).h(160.0).bg(Color::rgb_u8(1, 2, 3)),
        );
        tree.apply_style(text, Style::new().w(120.0).h(40.0));
        tree.set_text(text, "under");
        tree.apply_style(
            overlay,
            Style::new().w(120.0).h(40.0).bg(Color::rgb_u8(4, 5, 6)),
        );
        tree.compute_layout(800.0, 600.0);

        let scene = PaintScene::build(&mut tree, None);
        let kinds = scene
            .commands
            .iter()
            .filter_map(|command| match command {
                PaintCommand::DrawRect(_) => Some("rect"),
                PaintCommand::DrawText(_) => Some("text"),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(kinds, vec!["rect", "text", "rect"]);
    }

    #[test]
    fn overflow_pushes_clip_commands() {
        let mut tree = UiTree::new();
        let node = tree.create_node(UiNodeTag::Div);
        tree.append_child(tree.root(), node);
        tree.apply_style(
            node,
            Style::new()
                .w(100.0)
                .h(80.0)
                .overflow(Overflow::Hidden)
                .bg(Color::rgb_u8(1, 2, 3)),
        );
        tree.compute_layout(800.0, 600.0);

        let scene = PaintScene::build(&mut tree, None);

        assert!(matches!(scene.commands[0], PaintCommand::PushClip(_)));
        assert!(matches!(scene.commands.last(), Some(PaintCommand::PopClip)));
    }

    #[test]
    fn fragment_is_transparent_to_scene() {
        let mut tree = UiTree::new();
        let fragment = tree.create_node(UiNodeTag::Fragment);
        let child = tree.create_node(UiNodeTag::Div);
        tree.append_child(tree.root(), fragment);
        tree.append_child(fragment, child);
        tree.apply_style(
            child,
            Style::new().w(100.0).h(80.0).bg(Color::rgb_u8(1, 2, 3)),
        );
        tree.compute_layout(800.0, 600.0);

        let scene = PaintScene::build(&mut tree, None);

        assert_eq!(scene.rects.len(), 1);
        assert_eq!(scene.rects[0].node, child);
    }

    #[test]
    fn replay_updates_cached_subtree_range() {
        let mut tree = UiTree::new();
        let node = tree.create_node(UiNodeTag::Div);
        tree.append_child(tree.root(), node);
        tree.apply_style(
            node,
            Style::new().w(100.0).h(80.0).bg(Color::rgb_u8(1, 2, 3)),
        );
        tree.compute_layout(800.0, 600.0);
        let previous = PaintScene::build(&mut tree, None);
        tree.clear_dirty_flags(
            DirtyFlags::STRUCTURE
                | DirtyFlags::STYLE
                | DirtyFlags::PRE_PAINT
                | DirtyFlags::PAINT
                | DirtyFlags::COMPOSITE
                | DirtyFlags::TEXT,
        );

        let next = PaintScene::build(&mut tree, Some(&previous));

        assert_eq!(next.commands, previous.commands);
        assert_eq!(tree.render_states[node].subtree_commands, 0..1);
    }

    #[test]
    fn surface_primitive_enters_scene() {
        let mut tree = UiTree::new();
        let node = tree.create_node(UiNodeTag::Surface);
        tree.append_child(tree.root(), node);
        tree.set_surface(
            node,
            crate::ui_tree::SurfaceState {
                source: SurfaceSource::ExternalTexture(crate::ui_tree::ExternalSurfaceId(7)),
                color_space: SurfaceColorSpace::Srgb,
            },
        );
        tree.apply_style(node, Style::new().w(320.0).h(180.0));
        tree.compute_layout(800.0, 600.0);

        let scene = PaintScene::build(&mut tree, None);

        assert_eq!(scene.surfaces.len(), 1);
        assert!(matches!(scene.commands[0], PaintCommand::DrawSurface(_)));
    }
}
