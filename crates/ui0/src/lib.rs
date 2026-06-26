//! Minimal app/window runtime for `ui0`.
//!
//! This crate intentionally starts with only the outer runtime contract:
//! app lifecycle, window lifecycle, cross-thread handles, and background tasks.
//! UI tree, reactive state, layout, and rendering are layered on top later.

pub mod app;
pub mod element;
pub mod reactive;
mod renderer;
pub mod ui_tree;
pub mod window;

pub type Result<T = ()> = anyhow::Result<T>;

pub use app::{
    run, run_with, AppCx, AppEvent, AppHandle, AppOptions, Application, EventCx, TaskOptions,
    WindowCx,
};
pub use element::{button, div, img, span, surface_view, text, Element, IntoChild, IntoElement};
pub use reactive::{
    batch, effect, for_each, memo, show, signal, untrack, EffectHandle, ForEachBuilder,
    ForEachElement, Memo, ShowBuilder, ShowElement, Signal,
};
pub use ui_tree::{
    AlignItems, BackgroundStyle, BorderStyle, Color, Corners, DirtyFlags, Display, Edges,
    EffectStyle, EventFlags, EventHandlers, EventPhase, ExternalSurfaceId, FlexDirection,
    FlexStyle, HitTestResult, ImageSource, ImageState, InteractionState, JustifyContent,
    KeyModifiers, LayoutRect, Length, NodeId, Overflow, OverflowStyle, Point, PointerButton,
    PointerButtons, PointerEvent, PointerHandler, Position, PositionStyle, ScrollState, SizeStyle,
    SpacingStyle, Style, StyleFlags, SurfaceColorSpace, SurfaceSource, SurfaceState, TextContent,
    TextInputState, TextStyle, UiNode, UiNodeTag, UiTree,
};
pub use window::{WindowDesc, WindowHandle, WindowId};
