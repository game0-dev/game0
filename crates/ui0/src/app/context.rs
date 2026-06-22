use std::future::Future;
use std::marker::PhantomData;

use winit::event_loop::ActiveEventLoop;

use super::runtime::{AppHandle, RuntimeState};
use super::Application;
use crate::element::IntoElement;
use crate::window::{WindowDesc, WindowHandle, WindowId};

pub struct AppCx<'a, A: Application> {
    pub(crate) state: &'a mut RuntimeState<A>,
    pub(crate) event_loop: &'a ActiveEventLoop,
}

impl<'a, A: Application> AppCx<'a, A> {
    pub fn app_handle(&self) -> AppHandle<A> {
        self.state.handle()
    }

    pub fn handle(&self) -> AppHandle<A> {
        self.app_handle()
    }

    pub fn open_window<F>(&mut self, desc: WindowDesc, build: F) -> WindowHandle<A>
    where
        F: FnOnce(&mut WindowCx<A>) + 'static,
    {
        self.state.open_window(self.event_loop, desc, build)
    }

    pub fn request_redraw(&mut self, window: WindowId) {
        self.state.request_redraw(window);
    }

    pub fn close_window(&mut self, window: WindowId) {
        self.state.close_window(window);
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

pub struct WindowCx<'a, A: Application> {
    pub(crate) state: &'a mut RuntimeState<A>,
    pub(crate) event_loop: &'a ActiveEventLoop,
    pub(crate) window: WindowId,
}

impl<'a, A: Application> WindowCx<'a, A> {
    pub fn id(&self) -> WindowId {
        self.window
    }

    pub fn app_handle(&self) -> AppHandle<A> {
        self.state.handle()
    }

    pub fn window_handle(&self) -> WindowHandle<A> {
        WindowHandle {
            id: self.window,
            app: self.state.handle(),
        }
    }

    pub fn handle(&self) -> WindowHandle<A> {
        self.window_handle()
    }

    pub fn open_window<F>(&mut self, desc: WindowDesc, build: F) -> WindowHandle<A>
    where
        F: FnOnce(&mut WindowCx<A>) + 'static,
    {
        self.state.open_window(self.event_loop, desc, build)
    }

    pub fn request_redraw(&mut self) {
        self.state.request_redraw(self.window);
    }

    pub fn mount<E>(&mut self, view: E)
    where
        E: IntoElement,
    {
        let Some(runtime) = self.state.windows.get_mut(self.window) else {
            return;
        };
        runtime.mount(view.into_element());
        self.state.request_redraw(self.window);
    }

    pub fn set_title(&mut self, title: &str) {
        if let Some(window) = self.state.windows.get(self.window) {
            window.window.set_title(title);
        }
    }

    pub fn close(self) {
        self.state.close_window(self.window);
        if self.state.windows.is_empty() {
            self.event_loop.exit();
        }
    }
}

pub struct EventCx<'a> {
    pub(crate) window: WindowId,
    pub(crate) _marker: PhantomData<&'a mut ()>,
}

impl<'a> EventCx<'a> {
    pub fn id(&self) -> WindowId {
        self.window
    }
}
