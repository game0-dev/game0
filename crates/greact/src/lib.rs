use std::cell::RefCell;

// ---------------------------------------------------------------------------
// New architecture modules
// ---------------------------------------------------------------------------
pub mod application;
pub mod cx;
pub mod gpu_runtime;
pub mod icon;
pub mod input_edit;
pub mod layout;
pub mod reactive;
pub mod render;
pub mod render_tree;
pub mod renderer;
pub mod shared_services;
pub mod style;
pub mod text_system;
pub mod window;

// Re-exports for user convenience
pub use application::{App, AppContext, UserEvent};
pub use window::{current_app_context, current_window_id, GreactWindow, WindowSpec};
pub use cx::{Cx, NodeBuilder};
pub use gpu_runtime::GpuRuntime;
pub use greact_macros::view;
pub use icon::{icon_glyph, resolve_icon, IconInput, IconName};
pub use layout::{compute_layout, render_print, LayoutRect};
pub use reactive::{
    effect::{batch, create_effect, dispose_effect, untrack},
    memo::create_memo,
    signal::{create_signal, ReadSignal, WriteSignal},
    EffectId,
};
pub use render::{print_render_list_stats, RenderListBuilder};
pub use render_tree::node::{DirtyFlags, Display, ElementTag, NodeId, StyleFlags};
pub use render_tree::{
    CanvasHandlers, CanvasPointerEvent, CanvasState, CanvasWheelEvent, EventHandlers, RenderTree,
};
pub use shared_services::{FrameStats, SharedRenderServices};
pub use style::groups::*;
pub use style::types::*;
pub use style::Style;
pub use text_system::SharedTextSystem;

// ---------------------------------------------------------------------------
// Thread-local singletons
// ---------------------------------------------------------------------------

thread_local! {
    pub(crate) static RENDER_TREE: RefCell<RenderTree> = RefCell::new(RenderTree::new());
    pub(crate) static RUNTIME: RefCell<reactive::runtime::ReactiveRuntime> =
        RefCell::new(reactive::runtime::ReactiveRuntime::new());
}

/// Access the thread-local `RenderTree` inside a closure.
pub fn with_render_tree<R>(f: impl FnOnce(&mut RenderTree) -> R) -> R {
    RENDER_TREE.with(|rt| f(&mut rt.borrow_mut()))
}
