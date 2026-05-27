use std::rc::Rc;

use crate::icon::IconInput;
use crate::reactive::runtime::EffectId;
use crate::render_tree::{CanvasPointerEvent, CanvasWheelEvent};
use crate::render_tree::node::{ElementTag, NodeId};
use crate::style::Style;
use crate::with_render_tree;

// ---------------------------------------------------------------------------
// Cx -- zero-sized component context.
//
// All state access goes through thread-local globals (RENDER_TREE and
// RUNTIME), so Cx carries no data.
// ---------------------------------------------------------------------------

pub struct Cx;

impl Cx {
    /// Create an element node.  The closure receives a `NodeBuilder` to
    /// configure attributes, styles, events, and children.
    pub fn build(&self, tag: ElementTag, f: impl FnOnce(&mut NodeBuilder)) -> NodeId {
        let node_id = with_render_tree(|tree| tree.create_node(tag));
        let mut builder = NodeBuilder { cx: self, node_id };
        f(&mut builder);
        node_id
    }

    // -- convenience constructors ------------------------------------------

    pub fn div(&self, f: impl FnOnce(&mut NodeBuilder)) -> NodeId {
        self.build(ElementTag::Div, f)
    }

    pub fn button(&self, f: impl FnOnce(&mut NodeBuilder)) -> NodeId {
        self.build(ElementTag::Button, f)
    }

    pub fn canvas(&self, f: impl FnOnce(&mut NodeBuilder)) -> NodeId {
        self.build(ElementTag::Canvas, f)
    }

    pub fn text(&self, content: impl Into<String>) -> NodeId {
        let content = content.into();
        with_render_tree(|tree| {
            let id = tree.create_node(ElementTag::Text);
            tree.set_text(id, content);
            id
        })
    }

    pub fn image(&self, src: impl Into<String>) -> NodeId {
        let src = src.into();
        with_render_tree(|tree| {
            let id = tree.create_node(ElementTag::Image);
            tree.set_image_source(id, src);
            id
        })
    }

    pub fn icon(&self, icon: impl Into<IconInput>) -> NodeId {
        let icon = crate::icon::resolve_icon(icon.into());
        with_render_tree(|tree| {
            let id = tree.create_node(ElementTag::Icon);
            tree.set_text(id, icon.as_glyph().to_string());
            id
        })
    }

    // -- reactive helpers --------------------------------------------------

    pub fn create_signal<T: 'static>(
        &self,
        value: T,
    ) -> (crate::ReadSignal<T>, crate::WriteSignal<T>) {
        crate::reactive::signal::create_signal(value)
    }

    pub fn create_effect(&self, f: impl FnMut() + 'static) {
        crate::reactive::effect::create_effect(f);
    }

    pub fn batch<R>(&self, f: impl FnOnce() -> R) -> R {
        crate::reactive::effect::batch(f)
    }

    pub fn untrack<R>(&self, f: impl FnOnce() -> R) -> R {
        crate::reactive::effect::untrack(f)
    }

    pub fn dispose_effect(&self, id: EffectId) {
        crate::reactive::effect::dispose_effect(id);
    }

    /// Create a reactive text node.  The closure is wrapped in an effect that
    /// re-runs whenever any signal it reads changes, directly updating this
    /// single text node.
    pub fn text_dyn(&self, f: impl Fn() -> String + 'static) -> NodeId {
        let node_id = with_render_tree(|tree| tree.create_node(ElementTag::Text));
        crate::reactive::effect::create_effect(move || {
            let text = f();
            with_render_tree(|tree| {
                if tree.set_text(node_id, text) {
                    tree.mark_dirty(node_id, crate::DirtyFlags::PAINT);
                }
            });
        });
        node_id
    }
}

// ---------------------------------------------------------------------------
// NodeBuilder -- fluent API used inside `Cx::build` closures.
// ---------------------------------------------------------------------------

pub struct NodeBuilder<'cx> {
    pub(crate) cx: &'cx Cx,
    pub(crate) node_id: NodeId,
}

impl<'cx> NodeBuilder<'cx> {
    /// Apply a `Style` to this node.
    pub fn style(&mut self, s: Style) -> &mut Self {
        with_render_tree(|tree| tree.apply_style(self.node_id, s));
        self
    }

    // -- events (stored as Rc so they can be cloned out for invocation) ----

    pub fn on_click(&mut self, f: impl Fn() + 'static) -> &mut Self {
        let f = Rc::new(f);
        let node_id = self.node_id;
        with_render_tree(|tree| {
            tree.ensure_handlers(node_id).on_click = Some(f);
        });
        self
    }

    pub fn on_focus(&mut self, f: impl Fn() + 'static) -> &mut Self {
        let f = Rc::new(f);
        let node_id = self.node_id;
        with_render_tree(|tree| {
            tree.ensure_handlers(node_id).on_focus = Some(f);
        });
        self
    }

    pub fn on_blur(&mut self, f: impl Fn() + 'static) -> &mut Self {
        let f = Rc::new(f);
        let node_id = self.node_id;
        with_render_tree(|tree| {
            tree.ensure_handlers(node_id).on_blur = Some(f);
        });
        self
    }

    pub fn on_input(&mut self, f: impl Fn(String) + 'static) -> &mut Self {
        let f = Rc::new(f);
        let node_id = self.node_id;
        with_render_tree(|tree| {
            tree.ensure_handlers(node_id).on_input = Some(f);
        });
        self
    }

    pub fn on_submit(&mut self, f: impl Fn(String) + 'static) -> &mut Self {
        let f = Rc::new(f);
        let node_id = self.node_id;
        with_render_tree(|tree| {
            tree.ensure_handlers(node_id).on_submit = Some(f);
        });
        self
    }

    pub fn on_pointer_down(&mut self, f: impl Fn(CanvasPointerEvent) + 'static) -> &mut Self {
        let f = Rc::new(f);
        let node_id = self.node_id;
        with_render_tree(|tree| {
            tree.ensure_canvas_handlers(node_id).on_pointer_down = Some(f);
        });
        self
    }

    pub fn on_pointer_move(&mut self, f: impl Fn(CanvasPointerEvent) + 'static) -> &mut Self {
        let f = Rc::new(f);
        let node_id = self.node_id;
        with_render_tree(|tree| {
            tree.ensure_canvas_handlers(node_id).on_pointer_move = Some(f);
        });
        self
    }

    pub fn on_pointer_up(&mut self, f: impl Fn(CanvasPointerEvent) + 'static) -> &mut Self {
        let f = Rc::new(f);
        let node_id = self.node_id;
        with_render_tree(|tree| {
            tree.ensure_canvas_handlers(node_id).on_pointer_up = Some(f);
        });
        self
    }

    pub fn on_wheel(&mut self, f: impl Fn(CanvasWheelEvent) + 'static) -> &mut Self {
        let f = Rc::new(f);
        let node_id = self.node_id;
        with_render_tree(|tree| {
            tree.ensure_canvas_handlers(node_id).on_wheel = Some(f);
        });
        self
    }

    // -- child nodes -------------------------------------------------------

    /// Add a child element configured via a builder closure.
    pub fn child(&mut self, tag: ElementTag, f: impl FnOnce(&mut NodeBuilder)) -> &mut Self {
        let child_id = self.cx.build(tag, f);
        with_render_tree(|tree| tree.append_child(self.node_id, child_id));
        self
    }

    /// Attach an already-created node as a child.
    pub fn child_node(&mut self, child_id: NodeId) -> &mut Self {
        with_render_tree(|tree| tree.append_child(self.node_id, child_id));
        self
    }

    /// Add a static text child.
    pub fn child_text(&mut self, content: impl Into<String>) -> &mut Self {
        let child_id = self.cx.text(content);
        with_render_tree(|tree| tree.append_child(self.node_id, child_id));
        self
    }

    pub fn child_image(&mut self, src: impl Into<String>) -> &mut Self {
        let child_id = self.cx.image(src);
        with_render_tree(|tree| tree.append_child(self.node_id, child_id));
        self
    }

    pub fn child_icon(&mut self, icon: impl Into<IconInput>) -> &mut Self {
        let child_id = self.cx.icon(icon);
        with_render_tree(|tree| tree.append_child(self.node_id, child_id));
        self
    }

    /// Add a reactive text child (re-evaluated whenever signals change).
    pub fn child_text_dyn(&mut self, f: impl Fn() -> String + 'static) -> &mut Self {
        let child_id = self.cx.text_dyn(f);
        with_render_tree(|tree| tree.append_child(self.node_id, child_id));
        self
    }

    /// Nest a child component (any `fn(&Cx) -> NodeId`).
    pub fn child_component(&mut self, f: impl FnOnce(&Cx) -> NodeId) -> &mut Self {
        let child_id = f(self.cx);
        with_render_tree(|tree| tree.append_child(self.node_id, child_id));
        self
    }

    /// Apply a reactive style that re-evaluates when signals change.
    pub fn reactive_style(&mut self, f: impl Fn() -> Style + 'static) -> &mut Self {
        let node_id = self.node_id;
        self.cx.create_effect(move || {
            let style = f();
            with_render_tree(|tree| {
                tree.apply_style(node_id, style);
                tree.mark_dirty(
                    node_id,
                    crate::DirtyFlags::STYLE | crate::DirtyFlags::LAYOUT,
                );
            });
        });
        self
    }

    // -- shorthand child builders ------------------------------------------

    pub fn div(&mut self, f: impl FnOnce(&mut NodeBuilder)) -> &mut Self {
        self.child(ElementTag::Div, f)
    }

    pub fn button(&mut self, f: impl FnOnce(&mut NodeBuilder)) -> &mut Self {
        self.child(ElementTag::Button, f)
    }

    pub fn input(&mut self, f: impl FnOnce(&mut NodeBuilder)) -> &mut Self {
        self.child(ElementTag::Input, f)
    }

    pub fn canvas(&mut self, f: impl FnOnce(&mut NodeBuilder)) -> &mut Self {
        self.child(ElementTag::Canvas, f)
    }

    /// Attribute helper for `<image src={...} />`.
    pub fn src(&mut self, src: impl Into<String>) -> &mut Self {
        with_render_tree(|tree| {
            if tree.set_image_source(self.node_id, src.into()) {
                tree.mark_dirty(self.node_id, crate::DirtyFlags::PAINT);
            }
        });
        self
    }

    /// Attribute helper for `<icon name={...} />` where the value can be an
    /// `IconName` or a `&str` parsed into one.
    pub fn name(&mut self, name: impl Into<IconInput>) -> &mut Self {
        let icon = crate::icon::resolve_icon(name.into());
        with_render_tree(|tree| {
            if tree.set_text(self.node_id, icon.as_glyph().to_string()) {
                tree.mark_dirty(self.node_id, crate::DirtyFlags::PAINT);
            }
        });
        self
    }

    pub fn icon(&mut self, icon: impl Into<IconInput>) -> &mut Self {
        let icon = crate::icon::resolve_icon(icon.into());
        with_render_tree(|tree| {
            if tree.set_text(self.node_id, icon.as_glyph().to_string()) {
                tree.mark_dirty(self.node_id, crate::DirtyFlags::PAINT);
            }
        });
        self
    }

    pub fn value(&mut self, value: impl Into<String>) -> &mut Self {
        with_render_tree(|tree| {
            if tree.set_input_value(self.node_id, value.into()) {
                tree.mark_dirty(self.node_id, crate::DirtyFlags::PAINT);
            }
        });
        self
    }

    pub fn placeholder(&mut self, placeholder: impl Into<String>) -> &mut Self {
        with_render_tree(|tree| {
            if tree.set_placeholder(self.node_id, placeholder.into()) {
                tree.mark_dirty(self.node_id, crate::DirtyFlags::PAINT);
            }
        });
        self
    }
}
