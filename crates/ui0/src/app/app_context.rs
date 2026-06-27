use std::future::Future;

use winit::event_loop::ActiveEventLoop;

use super::app_runtime::{AppHandle, RuntimeState};
use super::Application;
use crate::window::{WindowCx, WindowDesc, WindowHandle, WindowId};

pub struct AppCx<'a, A: Application> {
    pub(crate) state: &'a mut RuntimeState<A>,
    pub(crate) event_loop: &'a ActiveEventLoop,
}

impl<'a, A: Application> AppCx<'a, A> {
    pub fn app_handle(&self) -> AppHandle<A> {
        self.state.handle()
    }

    pub fn open_window<F>(&mut self, desc: WindowDesc, build: F) -> WindowHandle<A>
    where
        F: FnOnce(&mut WindowCx<A>) + 'static,
    {
        self.state.open_window(self.event_loop, desc, build)
    }

    pub fn request_redraw(&mut self, window_id: WindowId) {
        self.state.request_redraw(window_id);
    }

    pub fn close_window(&mut self, window_id: WindowId) {
        self.state.close_window(window_id);
        if self.state.windows.is_empty() {
            self.event_loop.exit();
        }
    }

    pub fn quit(&mut self) {
        self.state.running = false;
        self.event_loop.exit();
    }

    pub fn spawn_io<Fut, T, Then>(&mut self, fut: Fut, then: Then)
    where
        Fut: Future<Output = T> + Send + 'static,
        T: Send + 'static,
        Then: FnOnce(&mut A, &mut AppCx<A>, T) + Send + 'static,
    {
        self.state.async_runtime.spawn_io(fut, then);
    }

    pub fn spawn_blocking<F, T, Then>(&mut self, job: F, then: Then)
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
        Then: FnOnce(&mut A, &mut AppCx<A>, T) + Send + 'static,
    {
        self.state.async_runtime.spawn_blocking(job, then);
    }
}
