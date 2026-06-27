use std::marker::PhantomData;
use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::{MouseScrollDelta, TouchPhase, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy};

use super::app_context::AppCx;
use super::async_runtime_tokio::AsyncRuntime;
use super::Application;
use crate::ui_tree::Point;
use crate::window::{key_modifiers, WindowCx, WindowDesc, WindowHandle, WindowId, WindowRuntime};

pub struct AppHandle<A: Application> {
    pub(crate) proxy: EventLoopProxy<RuntimeMsg<A>>,
    pub(crate) _marker: PhantomData<fn() -> A>,
}

impl<A: Application> Clone for AppHandle<A> {
    fn clone(&self) -> Self {
        Self {
            proxy: self.proxy.clone(),
            _marker: PhantomData,
        }
    }
}

impl<A: Application> AppHandle<A> {
    pub fn run_on_ui<F>(&self, f: F)
    where
        F: FnOnce(&mut A, &mut AppCx<A>) + Send + 'static,
    {
        let _ = self.proxy.send_event(RuntimeMsg::RunApp(Box::new(f)));
    }

    pub fn request_redraw(&self, window: WindowId) {
        let _ = self.proxy.send_event(RuntimeMsg::RequestRedraw(window));
    }

    pub fn close_window(&self, window: WindowId) {
        let _ = self.proxy.send_event(RuntimeMsg::CloseWindow(window));
    }

    pub fn wake(&self) {
        let _ = self.proxy.send_event(RuntimeMsg::Wake);
    }

    pub(crate) fn send_window<F>(&self, window: WindowId, f: F)
    where
        F: FnOnce(&mut A, &mut WindowCx<A>) + Send + 'static,
    {
        let _ = self.proxy.send_event(RuntimeMsg::RunWindow {
            window,
            task: Box::new(f),
        });
    }
}

pub(crate) type AppTask<A> = Box<dyn FnOnce(&mut A, &mut AppCx<A>) + Send + 'static>;
pub(crate) type WindowTask<A> = Box<dyn FnOnce(&mut A, &mut WindowCx<A>) + Send + 'static>;

pub(crate) enum RuntimeMsg<A: Application> {
    RunApp(AppTask<A>),
    RunWindow {
        window: WindowId,
        task: WindowTask<A>,
    },
    RequestRedraw(WindowId),
    CloseWindow(WindowId),
    Wake,
}

pub(crate) struct RuntimeState<A: Application> {
    pub(crate) proxy: EventLoopProxy<RuntimeMsg<A>>,
    pub(crate) async_runtime: AsyncRuntime<A>,
    pub(crate) windows: Vec<(WindowId, WindowRuntime)>,
    pub(crate) running: bool,
}

impl<A: Application> RuntimeState<A> {
    pub(crate) fn handle(&self) -> AppHandle<A> {
        AppHandle {
            proxy: self.proxy.clone(),
            _marker: PhantomData,
        }
    }

    pub(crate) fn open_window<F>(
        &mut self,
        event_loop: &ActiveEventLoop,
        desc: WindowDesc,
        build: F,
    ) -> WindowHandle<A>
    where
        F: FnOnce(&mut WindowCx<A>) + 'static,
    {
        let window = Arc::new(
            event_loop
                .create_window(desc.to_attributes())
                .expect("failed to create window"),
        );
        let window_id = window.id();
        self.windows.push((
            window_id,
            WindowRuntime::new(window_id, Arc::clone(&window)),
        ));

        let mut cx = WindowCx {
            state: self,
            event_loop,
            window: window_id,
        };
        build(&mut cx);

        WindowHandle {
            id: window_id,
            app: self.handle(),
        }
    }

    pub(crate) fn request_redraw(&mut self, window: WindowId) {
        let Some(runtime) = self.window_mut(window) else {
            return;
        };
        runtime.request_redraw();
    }

    pub(crate) fn request_redraw_all(&mut self) {
        for (_, runtime) in &mut self.windows {
            runtime.request_redraw();
        }
    }

    pub(crate) fn close_window(&mut self, window: WindowId) -> bool {
        let Some(index) = self.window_index(window) else {
            return false;
        };
        let (_, runtime) = self.windows.remove(index);
        runtime.dispose();
        true
    }

    pub(crate) fn has_window(&self, window: WindowId) -> bool {
        self.windows.iter().any(|(id, _)| *id == window)
    }

    pub(crate) fn window(&self, window: WindowId) -> Option<&WindowRuntime> {
        self.windows
            .iter()
            .find(|(id, _)| *id == window)
            .map(|(_, runtime)| runtime)
    }

    pub(crate) fn window_mut(&mut self, window: WindowId) -> Option<&mut WindowRuntime> {
        self.windows
            .iter_mut()
            .find(|(id, _)| *id == window)
            .map(|(_, runtime)| runtime)
    }

    fn window_index(&self, window: WindowId) -> Option<usize> {
        self.windows.iter().position(|(id, _)| *id == window)
    }
}

pub(crate) struct AppRuntime<A: Application> {
    app: A,
    state: RuntimeState<A>,
    initialized: bool,
    shutting_down: bool,
}

impl<A: Application> ApplicationHandler<RuntimeMsg<A>> for AppRuntime<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if !self.initialized {
            self.initialized = true;
            let mut cx = AppCx {
                state: &mut self.state,
                event_loop,
            };
            self.app.handle_init(&mut cx);
        }

        if self.state.windows.is_empty() {
            event_loop.exit();
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {}

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: RuntimeMsg<A>) {
        match event {
            RuntimeMsg::RunApp(task) => {
                let mut cx = AppCx {
                    state: &mut self.state,
                    event_loop,
                };
                task(&mut self.app, &mut cx);
                self.state.request_redraw_all();
            }
            RuntimeMsg::RunWindow { window, task } => {
                if self.state.has_window(window) {
                    let mut cx = WindowCx {
                        state: &mut self.state,
                        event_loop,
                        window,
                    };
                    task(&mut self.app, &mut cx);
                    self.state.request_redraw(window);
                }
            }
            RuntimeMsg::RequestRedraw(window) => {
                self.state.request_redraw(window);
            }
            RuntimeMsg::CloseWindow(window) => {
                self.close_window(event_loop, window);
            }
            RuntimeMsg::Wake => {}
        }

        if !self.state.running || self.state.windows.is_empty() {
            event_loop.exit();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window: WindowId, event: WindowEvent) {
        if !self.state.has_window(window) {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                self.close_window(event_loop, window);
            }
            WindowEvent::Resized(size) => {
                if let Some(runtime) = self.state.window_mut(window) {
                    if runtime.resize(size.width, size.height) {
                        self.state.request_redraw(window);
                    }
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(runtime) = self.state.window_mut(window) {
                    runtime.set_scale_factor(scale_factor as f32);
                    self.state.request_redraw(window);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(runtime) = self.state.window_mut(window) {
                    runtime.clear_redraw_requested();
                    runtime.render();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let point = self
                    .state
                    .window(window)
                    .map(|runtime| runtime.physical_point_to_logical(position.x, position.y))
                    .unwrap_or(Point { x: 0.0, y: 0.0 });
                self.dispatch_window_pointer(window, |runtime| runtime.pointer_moved(point));
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.dispatch_window_pointer(window, |runtime| runtime.mouse_input(state, button));
            }
            WindowEvent::MouseWheel { delta, phase, .. } => {
                if phase != TouchPhase::Cancelled
                    && !matches!(delta, MouseScrollDelta::LineDelta(0.0, 0.0))
                {
                    // Wheel dispatch is intentionally left for the scroll/focus phase. Keep this
                    // arm so the runtime has one place for pointer-family events.
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                if let Some(runtime) = self.state.window_mut(window) {
                    runtime.set_key_modifiers(key_modifiers(modifiers.state()));
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Wait);
        if !self.state.running {
            event_loop.exit();
        }
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        if self.shutting_down {
            return;
        }
        self.shutting_down = true;
        let mut cx = AppCx {
            state: &mut self.state,
            event_loop,
        };
        self.app.handle_shutdown(&mut cx);
    }
}

impl<A: Application> AppRuntime<A> {
    pub(crate) fn new(app: A, proxy: EventLoopProxy<RuntimeMsg<A>>) -> crate::Result<Self> {
        let handle = AppHandle {
            proxy: proxy.clone(),
            _marker: PhantomData,
        };
        let async_runtime = AsyncRuntime::new(handle)?;
        Ok(Self {
            app,
            state: RuntimeState {
                proxy,
                async_runtime,
                windows: Vec::new(),
                running: true,
            },
            initialized: false,
            shutting_down: false,
        })
    }

    fn close_window(&mut self, event_loop: &ActiveEventLoop, window: crate::window::WindowId) {
        self.state.close_window(window);
        if self.state.windows.is_empty() {
            event_loop.exit();
        }
    }

    fn dispatch_window_pointer<F>(&mut self, window: crate::window::WindowId, f: F)
    where
        F: FnOnce(&mut WindowRuntime) -> bool,
    {
        let Some(runtime) = self.state.window_mut(window) else {
            return;
        };
        let redraw_requested = f(runtime);
        if redraw_requested {
            self.state.request_redraw(window);
        }
    }
}
