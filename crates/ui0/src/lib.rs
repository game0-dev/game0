//! Minimal app/window runtime for `ui0`.
//!
//! This crate intentionally starts with only the outer runtime contract:
//! app lifecycle, window lifecycle, cross-thread handles, and background tasks.
//! UI tree, reactive state, layout, and rendering are layered on top later.

pub mod app;
pub mod window;

pub type Result<T = ()> = anyhow::Result<T>;

pub use app::{
    run, run_with, AppCx, AppEvent, AppHandle, AppOptions, Application, TaskOptions, WindowCx,
};
pub use window::{WindowDesc, WindowHandle, WindowId};
