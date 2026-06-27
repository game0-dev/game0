//! App runtime public API.

mod app_context;
pub(crate) mod app_runtime;
mod async_runtime_tokio;

use winit::event_loop::EventLoop;

pub use app_context::AppCx;
pub use app_runtime::AppHandle;

pub trait Application: Sized + 'static {
    fn handle_init(&mut self, _app: &mut AppCx<Self>) {}

    fn handle_shutdown(&mut self, _app: &mut AppCx<Self>) {}
}

pub fn run<A: Application>(app: A) -> crate::Result<()> {
    let event_loop = EventLoop::<app_runtime::RuntimeMsg<A>>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    let mut runtime = app_runtime::AppRuntime::new(app, proxy)?;
    event_loop.run_app(&mut runtime)?;
    Ok(())
}
