//! App runtime public API.

mod async_runtime;
mod context;
mod runtime;

pub use context::{AppCx, EventCx, WindowCx};
pub use runtime::{run, run_with, AppEvent, AppHandle, AppOptions, TaskOptions};

pub trait Application: Sized + 'static {
    fn handle_init(&mut self, _app: &mut AppCx<Self>) {}

    fn handle_event(&mut self, _app: &mut AppCx<Self>, _event: AppEvent) {}

    fn handle_shutdown(&mut self, _app: &mut AppCx<Self>) {}
}
