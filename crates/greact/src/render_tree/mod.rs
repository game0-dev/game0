pub mod node;

use std::rc::Rc;
use std::time::{Duration, Instant};

use node::*;
use slotmap::{SecondaryMap, SlotMap};

use crate::style::{groups::*, Style};
use crate::layout::LayoutRect;

// ---------------------------------------------------------------------------
// EventHandlers -- stored in a SecondaryMap, only for nodes that need them.
// Uses Rc so handlers can be cheaply cloned out for invocation without
// holding a RenderTree borrow (which would conflict with reactive updates).
// ---------------------------------------------------------------------------

#[derive(Default, Clone)]
pub struct EventHandlers {
    pub on_click: Option<Rc<dyn Fn()>>,
    pub on_focus: Option<Rc<dyn Fn()>>,
    pub on_blur: Option<Rc<dyn Fn()>>,
    pub on_input: Option<Rc<dyn Fn(String)>>,
    pub on_submit: Option<Rc<dyn Fn(String)>>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CanvasPointerEvent {
    pub x: f32,
    pub y: f32,
    pub hit: Option<NodeId>,
    pub click_count: u8,
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CanvasWheelEvent {
    pub x: f32,
    pub y: f32,
    pub delta_x: f32,
    pub delta_y: f32,
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

#[derive(Default, Clone)]
pub struct CanvasHandlers {
    pub on_pointer_down: Option<Rc<dyn Fn(CanvasPointerEvent)>>,
    pub on_pointer_move: Option<Rc<dyn Fn(CanvasPointerEvent)>>,
    pub on_pointer_up: Option<Rc<dyn Fn(CanvasPointerEvent)>>,
    pub on_wheel: Option<Rc<dyn Fn(CanvasWheelEvent)>>,
}

#[derive(Debug, Clone, Copy)]
pub struct CanvasState {
    pub captured_pointer: bool,
    pub hovered: bool,
    pub viewport_rect: Option<LayoutRect>,
    pub zoom: f32,
    pub pan_world: [f32; 2],
    pub show_dot_grid: bool,
    pub base_world_step: f32,
    pub dot_radius_px: f32,
    pub target_screen_step_px: f32,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            captured_pointer: false,
            hovered: false,
            viewport_rect: None,
            zoom: 1.0,
            pan_world: [0.0, 0.0],
            show_dot_grid: false,
            base_world_step: 16.0,
            dot_radius_px: 1.0,
            target_screen_step_px: 24.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InputState {
    pub value: String,
    pub placeholder: String,
    pub cursor: usize,
    pub selection_anchor: usize,
    pub preedit: String,
    pub preedit_range: Option<(usize, usize)>,
    pub blink_visible: bool,
    pub next_blink_at: Instant,
    pub is_composing: bool,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            value: String::new(),
            placeholder: String::new(),
            cursor: 0,
            selection_anchor: 0,
            preedit: String::new(),
            preedit_range: None,
            blink_visible: true,
            next_blink_at: Instant::now() + Duration::from_millis(530),
            is_composing: false,
        }
    }
}

// ---------------------------------------------------------------------------
// RenderTree
// ---------------------------------------------------------------------------

pub struct RenderTree {
    // -- core tree ---------------------------------------------------------
    pub(crate) nodes: SlotMap<NodeId, RenderNode>,
    root: Option<NodeId>,
    dirty_nodes: Vec<NodeId>,

    // -- style SoA (9 groups) ----------------------------------------------
    pub(crate) size_styles: SecondaryMap<NodeId, SizeStyle>,
    pub(crate) spacing_styles: SecondaryMap<NodeId, SpacingStyle>,
    pub(crate) flex_styles: SecondaryMap<NodeId, FlexStyle>,
    pub(crate) background_styles: SecondaryMap<NodeId, BackgroundStyle>,
    pub(crate) border_styles: SecondaryMap<NodeId, BorderStyle>,
    pub(crate) text_style_groups: SecondaryMap<NodeId, TextStyleGroup>,
    pub(crate) position_styles: SecondaryMap<NodeId, PositionStyle>,
    pub(crate) effect_styles: SecondaryMap<NodeId, EffectStyle>,
    pub(crate) overflow_styles: SecondaryMap<NodeId, OverflowStyle>,

    // -- non-style SoA -----------------------------------------------------
    pub(crate) handlers: SecondaryMap<NodeId, EventHandlers>,
    pub(crate) canvas_handlers: SecondaryMap<NodeId, CanvasHandlers>,
    pub(crate) text_content: SecondaryMap<NodeId, String>,
    pub(crate) image_source: SecondaryMap<NodeId, String>,
    pub(crate) input_states: SecondaryMap<NodeId, InputState>,
    pub(crate) canvas_states: SecondaryMap<NodeId, CanvasState>,
    focused_input: Option<NodeId>,
    pub(crate) layout_rects: SecondaryMap<NodeId, LayoutRect>,
    pub(crate) hovered: SecondaryMap<NodeId, bool>,
    pub(crate) pressed: SecondaryMap<NodeId, bool>,
    pub(crate) focused: SecondaryMap<NodeId, bool>,
    pub(crate) scroll_offset_y: SecondaryMap<NodeId, f32>,
}

impl RenderTree {
    pub fn new() -> Self {
        Self {
            nodes: SlotMap::with_key(),
            root: None,
            dirty_nodes: Vec::new(),

            size_styles: SecondaryMap::new(),
            spacing_styles: SecondaryMap::new(),
            flex_styles: SecondaryMap::new(),
            background_styles: SecondaryMap::new(),
            border_styles: SecondaryMap::new(),
            text_style_groups: SecondaryMap::new(),
            position_styles: SecondaryMap::new(),
            effect_styles: SecondaryMap::new(),
            overflow_styles: SecondaryMap::new(),

            handlers: SecondaryMap::new(),
            canvas_handlers: SecondaryMap::new(),
            text_content: SecondaryMap::new(),
            image_source: SecondaryMap::new(),
            input_states: SecondaryMap::new(),
            canvas_states: SecondaryMap::new(),
            focused_input: None,
            layout_rects: SecondaryMap::new(),
            hovered: SecondaryMap::new(),
            pressed: SecondaryMap::new(),
            focused: SecondaryMap::new(),
            scroll_offset_y: SecondaryMap::new(),
        }
    }

    // -- node CRUD ---------------------------------------------------------

    pub fn create_node(&mut self, tag: ElementTag) -> NodeId {
        self.nodes.insert(RenderNode::new(tag))
    }

    pub fn append_child(&mut self, parent: NodeId, child: NodeId) {
        if let Some(node) = self.nodes.get_mut(child) {
            node.parent = Some(parent);
        }
        if let Some(node) = self.nodes.get_mut(parent) {
            node.children.push(child);
        }
    }

    pub fn remove_node(&mut self, id: NodeId) {
        // Remove from parent's children list
        if let Some(parent_id) = self.nodes.get(id).and_then(|n| n.parent) {
            if let Some(parent) = self.nodes.get_mut(parent_id) {
                parent.children.retain(|c| *c != id);
            }
        }
        // Recursively remove children
        let children: Vec<NodeId> = self
            .nodes
            .get(id)
            .map(|n| n.children.to_vec())
            .unwrap_or_default();
        for child in children {
            self.remove_node(child);
        }
        // Remove from all maps
        self.nodes.remove(id);
        self.size_styles.remove(id);
        self.spacing_styles.remove(id);
        self.flex_styles.remove(id);
        self.background_styles.remove(id);
        self.border_styles.remove(id);
        self.text_style_groups.remove(id);
        self.position_styles.remove(id);
        self.effect_styles.remove(id);
        self.overflow_styles.remove(id);
        self.handlers.remove(id);
        self.canvas_handlers.remove(id);
        self.text_content.remove(id);
        self.image_source.remove(id);
        self.input_states.remove(id);
        self.canvas_states.remove(id);
        if self.focused_input == Some(id) {
            self.focused_input = None;
        }
        self.layout_rects.remove(id);
        self.hovered.remove(id);
        self.pressed.remove(id);
        self.focused.remove(id);
        self.scroll_offset_y.remove(id);
    }

    pub fn get(&self, id: NodeId) -> Option<&RenderNode> {
        self.nodes.get(id)
    }

    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut RenderNode> {
        self.nodes.get_mut(id)
    }

    // -- root --------------------------------------------------------------

    pub fn root(&self) -> Option<NodeId> {
        self.root
    }

    pub fn set_root(&mut self, id: NodeId) {
        self.root = Some(id);
    }

    // -- text content ------------------------------------------------------

    pub fn set_text(&mut self, id: NodeId, text: String) -> bool {
        if let Some(existing) = self.text_content.get_mut(id) {
            if *existing == text {
                return false;
            }
            *existing = text;
            return true;
        }
        self.text_content.insert(id, text);
        true
    }

    pub fn get_text(&self, id: NodeId) -> Option<&str> {
        self.text_content.get(id).map(|s| s.as_str())
    }

    pub fn set_image_source(&mut self, id: NodeId, src: String) -> bool {
        if let Some(existing) = self.image_source.get_mut(id) {
            if *existing == src {
                return false;
            }
            *existing = src;
            return true;
        }
        self.image_source.insert(id, src);
        true
    }

    pub fn get_image_source(&self, id: NodeId) -> Option<&str> {
        self.image_source.get(id).map(|s| s.as_str())
    }

    pub fn ensure_input_state(&mut self, id: NodeId) -> &mut InputState {
        if !self.input_states.contains_key(id) {
            self.input_states.insert(id, InputState::default());
        }
        self.input_states.get_mut(id).unwrap()
    }

    pub fn get_input_state(&self, id: NodeId) -> Option<&InputState> {
        self.input_states.get(id)
    }

    pub fn get_input_state_mut(&mut self, id: NodeId) -> Option<&mut InputState> {
        self.input_states.get_mut(id)
    }

    pub fn set_input_value(&mut self, id: NodeId, value: String) -> bool {
        let state = self.ensure_input_state(id);
        if state.value == value {
            return false;
        }
        state.value = value;
        state.cursor = state.value.len();
        state.selection_anchor = state.cursor;
        true
    }

    pub fn get_input_value(&self, id: NodeId) -> Option<&str> {
        self.input_states.get(id).map(|s| s.value.as_str())
    }

    pub fn set_placeholder(&mut self, id: NodeId, placeholder: String) -> bool {
        let state = self.ensure_input_state(id);
        if state.placeholder == placeholder {
            return false;
        }
        state.placeholder = placeholder;
        true
    }

    pub fn get_placeholder(&self, id: NodeId) -> Option<&str> {
        self.input_states.get(id).map(|s| s.placeholder.as_str())
    }

    pub fn focused_input(&self) -> Option<NodeId> {
        self.focused_input
    }

    pub fn set_focused_input(&mut self, id: NodeId) {
        self.focused_input = Some(id);
        self.ensure_input_state(id);
    }

    pub fn clear_focused_input(&mut self) {
        self.focused_input = None;
    }

    pub fn ensure_canvas_state(&mut self, id: NodeId) -> &mut CanvasState {
        if !self.canvas_states.contains_key(id) {
            self.canvas_states.insert(id, CanvasState::default());
        }
        self.canvas_states.get_mut(id).unwrap()
    }

    pub fn get_canvas_state(&self, id: NodeId) -> Option<&CanvasState> {
        self.canvas_states.get(id)
    }

    pub fn get_canvas_state_mut(&mut self, id: NodeId) -> Option<&mut CanvasState> {
        self.canvas_states.get_mut(id)
    }

    pub fn set_canvas_camera(&mut self, id: NodeId, zoom: f32, pan_world: [f32; 2]) {
        let state = self.ensure_canvas_state(id);
        state.zoom = zoom.clamp(0.1, 8.0);
        state.pan_world = pan_world;
    }

    pub fn set_canvas_grid(&mut self, id: NodeId, show_dot_grid: bool) {
        self.ensure_canvas_state(id).show_dot_grid = show_dot_grid;
    }

    // -- layout ------------------------------------------------------------

    pub fn set_layout_rect(&mut self, id: NodeId, rect: LayoutRect) {
        self.layout_rects.insert(id, rect);
    }

    pub fn get_layout_rect(&self, id: NodeId) -> Option<LayoutRect> {
        self.layout_rects.get(id).copied()
    }

    pub fn set_hovered(&mut self, id: NodeId, hovered: bool) {
        self.hovered.insert(id, hovered);
    }

    pub fn is_hovered(&self, id: NodeId) -> bool {
        self.hovered.get(id).copied().unwrap_or(false)
    }

    pub fn set_pressed(&mut self, id: NodeId, pressed: bool) {
        self.pressed.insert(id, pressed);
    }

    pub fn is_pressed(&self, id: NodeId) -> bool {
        self.pressed.get(id).copied().unwrap_or(false)
    }

    pub fn set_focused(&mut self, id: NodeId, focused: bool) {
        self.focused.insert(id, focused);
    }

    pub fn is_focused(&self, id: NodeId) -> bool {
        self.focused.get(id).copied().unwrap_or(false)
    }

    pub fn set_scroll_offset(&mut self, id: NodeId, value: f32) {
        self.scroll_offset_y.insert(id, value.max(0.0));
    }

    pub fn get_scroll_offset(&self, id: NodeId) -> f32 {
        self.scroll_offset_y.get(id).copied().unwrap_or(0.0)
    }

    pub fn hit_test(&self, root: NodeId, x: f32, y: f32) -> Option<NodeId> {
        self.hit_test_node(root, x, y, 0.0, None)
    }

    fn hit_test_node(
        &self,
        id: NodeId,
        x: f32,
        y: f32,
        inherited_scroll_y: f32,
        parent_clip: Option<LayoutRect>,
    ) -> Option<NodeId> {
        let node = self.get(id)?;
        if node.display == crate::Display::None {
            return None;
        }
        let mut rect = self.get_layout_rect(id)?;
        rect.y -= inherited_scroll_y;

        if let Some(clip) = parent_clip {
            if !rects_overlap(rect, clip) {
                return None;
            }
        }

        let self_clips = self
            .overflow_styles
            .get(id)
            .map(|ov| {
                !matches!(ov.overflow_x, crate::style::types::Overflow::Visible)
                    || !matches!(ov.overflow_y, crate::style::types::Overflow::Visible)
            })
            .unwrap_or(false);
        let current_clip = if self_clips {
            Some(match parent_clip {
                Some(clip) => intersect_rect(clip, rect),
                None => rect,
            })
        } else {
            parent_clip
        };

        let next_scroll = inherited_scroll_y + self.get_scroll_offset(id);
        for &child in node.children.iter().rev() {
            if let Some(hit) = self.hit_test_node(child, x, y, next_scroll, current_clip) {
                return Some(hit);
            }
        }

        if point_in_rect(x, y, rect) {
            Some(id)
        } else {
            None
        }
    }

    pub fn nearest_scrollable_ancestor(&self, id: NodeId) -> Option<NodeId> {
        let mut current = Some(id);
        while let Some(node_id) = current {
            let is_scrollable = self
                .overflow_styles
                .get(node_id)
                .map(|ov| {
                    matches!(ov.overflow_y, crate::style::types::Overflow::Scroll)
                        || matches!(ov.overflow_x, crate::style::types::Overflow::Scroll)
                })
                .unwrap_or(false);
            if is_scrollable {
                return Some(node_id);
            }
            current = self.get(node_id).and_then(|n| n.parent);
        }
        None
    }

    pub fn nearest_clickable_ancestor(&self, id: NodeId) -> Option<NodeId> {
        let mut current = Some(id);
        while let Some(node_id) = current {
            let clickable = self
                .handlers
                .get(node_id)
                .and_then(|h| h.on_click.as_ref())
                .is_some();
            if clickable {
                return Some(node_id);
            }
            current = self.get(node_id).and_then(|n| n.parent);
        }
        None
    }

    pub fn nearest_input_ancestor(&self, id: NodeId) -> Option<NodeId> {
        let mut current = Some(id);
        while let Some(node_id) = current {
            if self
                .get(node_id)
                .map(|n| n.tag == ElementTag::Input)
                .unwrap_or(false)
            {
                return Some(node_id);
            }
            current = self.get(node_id).and_then(|n| n.parent);
        }
        None
    }

    pub fn nearest_canvas_ancestor(&self, id: NodeId) -> Option<NodeId> {
        let mut current = Some(id);
        while let Some(node_id) = current {
            if self
                .get(node_id)
                .map(|n| n.tag == ElementTag::Canvas)
                .unwrap_or(false)
            {
                return Some(node_id);
            }
            current = self.get(node_id).and_then(|n| n.parent);
        }
        None
    }

    pub fn max_scroll_offset_y(&self, id: NodeId) -> f32 {
        let Some(container) = self.get_layout_rect(id) else {
            return 0.0;
        };
        let Some(node) = self.get(id) else {
            return 0.0;
        };
        let mut max_bottom = container.y;
        for &child in &node.children {
            if let Some(r) = self.get_layout_rect(child) {
                max_bottom = max_bottom.max(r.y + r.height);
            }
        }
        (max_bottom - (container.y + container.height)).max(0.0)
    }

    // -- event handlers ----------------------------------------------------

    pub fn set_handler(&mut self, id: NodeId, h: EventHandlers) {
        self.handlers.insert(id, h);
    }

    pub fn get_handler(&self, id: NodeId) -> Option<&EventHandlers> {
        self.handlers.get(id)
    }

    /// Get or create the `EventHandlers` entry for a node.
    pub fn ensure_handlers(&mut self, id: NodeId) -> &mut EventHandlers {
        if !self.handlers.contains_key(id) {
            self.handlers.insert(id, EventHandlers::default());
        }
        self.handlers.get_mut(id).unwrap()
    }

    pub fn get_canvas_handlers(&self, id: NodeId) -> Option<&CanvasHandlers> {
        self.canvas_handlers.get(id)
    }

    pub fn ensure_canvas_handlers(&mut self, id: NodeId) -> &mut CanvasHandlers {
        if !self.canvas_handlers.contains_key(id) {
            self.canvas_handlers.insert(id, CanvasHandlers::default());
        }
        self.canvas_handlers.get_mut(id).unwrap()
    }

    // -- style application (builder → SecondaryMaps) -----------------------

    pub fn apply_style(&mut self, node_id: NodeId, style: Style) {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.display = style.display;
        }

        let mut flags = StyleFlags::empty();

        macro_rules! apply_or_clear_group {
            ($field:ident, $map:ident, $flag:expr) => {
                match style.$field {
                    Some(data) => {
                        self.$map.insert(node_id, data);
                        flags |= $flag;
                    }
                    None => {
                        self.$map.remove(node_id);
                    }
                }
            };
        }

        apply_or_clear_group!(size, size_styles, StyleFlags::SIZE);
        apply_or_clear_group!(spacing, spacing_styles, StyleFlags::SPACING);
        apply_or_clear_group!(flex, flex_styles, StyleFlags::FLEX);
        apply_or_clear_group!(background, background_styles, StyleFlags::BACKGROUND);
        apply_or_clear_group!(border, border_styles, StyleFlags::BORDER);
        apply_or_clear_group!(text_style, text_style_groups, StyleFlags::TEXT_STYLE);
        apply_or_clear_group!(position, position_styles, StyleFlags::POSITION);
        apply_or_clear_group!(effect, effect_styles, StyleFlags::EFFECT);
        apply_or_clear_group!(overflow, overflow_styles, StyleFlags::OVERFLOW);

        if let Some(node) = self.nodes.get_mut(node_id) {
            node.style_flags = flags;
        }
    }

    // -- bitfield queries --------------------------------------------------

    pub fn has_style(&self, id: NodeId, flag: StyleFlags) -> bool {
        self.nodes
            .get(id)
            .map_or(false, |n| n.style_flags.contains(flag))
    }

    // -- dirty management --------------------------------------------------

    pub fn mark_dirty(&mut self, id: NodeId, flags: DirtyFlags) {
        if let Some(node) = self.nodes.get_mut(id) {
            let was_clean = node.dirty.is_empty();
            node.dirty |= flags;
            if was_clean {
                self.dirty_nodes.push(id);
            }
        }
    }

    pub fn take_dirty(&mut self) -> Vec<NodeId> {
        std::mem::take(&mut self.dirty_nodes)
    }

    pub fn clear_dirty(&mut self, id: NodeId) {
        if let Some(node) = self.nodes.get_mut(id) {
            node.dirty = DirtyFlags::empty();
        }
    }

    // -- debug -------------------------------------------------------------

    /// Print a human-readable representation of the subtree rooted at `id`.
    pub fn debug_print(&self, id: NodeId, depth: usize) {
        let indent = "  ".repeat(depth);
        if let Some(node) = self.get(id) {
            let text = self.get_text(id).unwrap_or("");
            let has_handler = self.handlers.contains_key(id);
            let flags: Vec<&str> = {
                let mut v = Vec::new();
                if node.style_flags.contains(StyleFlags::SIZE) { v.push("size"); }
                if node.style_flags.contains(StyleFlags::SPACING) { v.push("spacing"); }
                if node.style_flags.contains(StyleFlags::FLEX) { v.push("flex"); }
                if node.style_flags.contains(StyleFlags::BACKGROUND) { v.push("bg"); }
                if node.style_flags.contains(StyleFlags::BORDER) { v.push("border"); }
                if node.style_flags.contains(StyleFlags::TEXT_STYLE) { v.push("text_style"); }
                if node.style_flags.contains(StyleFlags::POSITION) { v.push("pos"); }
                if node.style_flags.contains(StyleFlags::EFFECT) { v.push("fx"); }
                if node.style_flags.contains(StyleFlags::OVERFLOW) { v.push("overflow"); }
                v
            };

            let style_info = if flags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", flags.join(","))
            };
            let text_info = if text.is_empty() {
                String::new()
            } else {
                format!(" \"{}\"", text)
            };
            let image_info = self
                .get_image_source(id)
                .map(|src| format!(" src=\"{}\"", src))
                .unwrap_or_default();
            let handler_info = if has_handler { " (has handlers)" } else { "" };

            println!(
                "{}<{:?}>{}{}{}{}",
                indent, node.tag, text_info, image_info, style_info, handler_info
            );

            for &child in &node.children {
                self.debug_print(child, depth + 1);
            }
        }
    }
}

fn point_in_rect(x: f32, y: f32, rect: LayoutRect) -> bool {
    x >= rect.x && x <= rect.x + rect.width && y >= rect.y && y <= rect.y + rect.height
}

fn rects_overlap(a: LayoutRect, b: LayoutRect) -> bool {
    !(a.x + a.width < b.x
        || b.x + b.width < a.x
        || a.y + a.height < b.y
        || b.y + b.height < a.y)
}

fn intersect_rect(a: LayoutRect, b: LayoutRect) -> LayoutRect {
    let x1 = a.x.max(b.x);
    let y1 = a.y.max(b.y);
    let x2 = (a.x + a.width).min(b.x + b.width);
    let y2 = (a.y + a.height).min(b.y + b.height);
    LayoutRect {
        x: x1,
        y: y1,
        width: (x2 - x1).max(0.0),
        height: (y2 - y1).max(0.0),
    }
}
