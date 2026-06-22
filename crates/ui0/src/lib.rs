//! Minimal app/window runtime for `ui0`.
//!
//! This crate intentionally starts with only the outer runtime contract:
//! app lifecycle, window lifecycle, cross-thread handles, and background tasks.
//! UI tree, reactive state, layout, and rendering are layered on top later.

pub mod app;
pub mod ui_tree;
pub mod window;

pub type Result<T = ()> = anyhow::Result<T>;

pub use app::{
    run, run_with, AppCx, AppEvent, AppHandle, AppOptions, Application, TaskOptions, WindowCx,
};
pub use ui_tree::{
    AlignItems, BackgroundStyle, BorderStyle, Color, Corners, DirtyFlags, Display, Edges,
    EffectStyle, EventFlags, EventHandlers, FlexDirection, FlexStyle, ImageSource, ImageState,
    InteractionState, JustifyContent, LayoutRect, Length, NodeId, Overflow, OverflowStyle,
    Position, PositionStyle, ScrollState, SizeStyle, SpacingStyle, StyleFlags, TextContent,
    TextInputState, TextStyle, UiNode, UiNodeTag, UiTree,
};
pub use window::{WindowDesc, WindowHandle, WindowId};
