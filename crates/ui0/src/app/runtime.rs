use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

use slotmap::SlotMap;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::window::{Window, WindowId as WinitWindowId};

use super::async_runtime::AsyncRuntime;
use super::{AppCx, Application, WindowCx};
use crate::ui_tree::UiTree;
use crate::window::{WindowDesc, WindowHandle, WindowId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEvent {
    Resumed,
    Suspended,
    WindowRedrawRequested(WindowId),
    WindowResized {
        window: WindowId,
        width: u32,
        height: u32,
    },
    WindowCloseRequested(WindowId),
    WindowClosed(WindowId),
}

#[derive(Debug, Clone, Default)]
pub struct AppOptions {
    pub task: TaskOptions,
}

#[derive(Debug, Clone, Default)]
pub struct TaskOptions {
    pub worker_threads: Option<usize>,
}

pub fn run<A: Application>(app: A) -> crate::Result<()> {
    run_with(app, AppOptions::default())
}

pub fn run_with<A: Application>(app: A, options: AppOptions) -> crate::Result<()> {
    let event_loop = EventLoop::<RuntimeMsg<A>>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    let handle = AppHandle {
        proxy: proxy.clone(),
        _marker: PhantomData,
    };
    let async_runtime = AsyncRuntime::new(&options.task, handle)?;
    let mut runtime = AppRuntime {
        app,
        state: RuntimeState {
            proxy,
            async_runtime,
            windows: SlotMap::with_key(),
            winit_to_ui0: HashMap::new(),
            running: true,
        },
        initialized: false,
        shutting_down: false,
    };
    event_loop.run_app(&mut runtime)?;
    Ok(())
}

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

struct AppRuntime<A: Application> {
    app: A,
    state: RuntimeState<A>,
    initialized: bool,
    shutting_down: bool,
}

pub(crate) struct RuntimeState<A: Application> {
    pub(crate) proxy: EventLoopProxy<RuntimeMsg<A>>,
    pub(crate) async_runtime: AsyncRuntime<A>,
    pub(crate) windows: SlotMap<WindowId, WindowRuntime<A>>,
    pub(crate) winit_to_ui0: HashMap<WinitWindowId, WindowId>,
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
        let winit_id = window.id();
        let ui0_id = self
            .windows
            .insert_with_key(|id| WindowRuntime::new(id, Arc::clone(&window)));
        self.winit_to_ui0.insert(winit_id, ui0_id);

        let mut cx = WindowCx {
            state: self,
            event_loop,
            window: ui0_id,
        };
        build(&mut cx);

        WindowHandle {
            id: ui0_id,
            app: self.handle(),
        }
    }

    pub(crate) fn request_redraw(&mut self, window: WindowId) {
        let Some(runtime) = self.windows.get_mut(window) else {
            return;
        };
        if runtime.redraw_requested {
            return;
        }
        runtime.redraw_requested = true;
        runtime.window.request_redraw();
    }

    pub(crate) fn close_window(&mut self, window: WindowId) -> bool {
        let Some(runtime) = self.windows.remove(window) else {
            return false;
        };
        self.winit_to_ui0.remove(&runtime.window.id());
        true
    }
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

        self.emit_app_event(event_loop, AppEvent::Resumed);
        if self.state.windows.is_empty() {
            event_loop.exit();
        }
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        self.emit_app_event(event_loop, AppEvent::Suspended);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: RuntimeMsg<A>) {
        match event {
            RuntimeMsg::RunApp(task) => {
                let mut cx = AppCx {
                    state: &mut self.state,
                    event_loop,
                };
                task(&mut self.app, &mut cx);
            }
            RuntimeMsg::RunWindow { window, task } => {
                if self.state.windows.contains_key(window) {
                    let mut cx = WindowCx {
                        state: &mut self.state,
                        event_loop,
                        window,
                    };
                    task(&mut self.app, &mut cx);
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

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        id: WinitWindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.state.winit_to_ui0.get(&id).copied() else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => {
                self.emit_app_event(event_loop, AppEvent::WindowCloseRequested(window));
                self.close_window(event_loop, window);
            }
            WindowEvent::Resized(size) => {
                self.emit_app_event(
                    event_loop,
                    AppEvent::WindowResized {
                        window,
                        width: size.width,
                        height: size.height,
                    },
                );
            }
            WindowEvent::RedrawRequested => {
                if let Some(runtime) = self.state.windows.get_mut(window) {
                    runtime.redraw_requested = false;
                }
                self.emit_app_event(event_loop, AppEvent::WindowRedrawRequested(window));
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
    fn emit_app_event(&mut self, event_loop: &ActiveEventLoop, event: AppEvent) {
        let mut cx = AppCx {
            state: &mut self.state,
            event_loop,
        };
        self.app.handle_event(&mut cx, event);
    }

    fn close_window(&mut self, event_loop: &ActiveEventLoop, window: WindowId) {
        if self.state.close_window(window) {
            self.emit_app_event(event_loop, AppEvent::WindowClosed(window));
        }
        if self.state.windows.is_empty() {
            event_loop.exit();
        }
    }
}

pub(crate) struct WindowRuntime<A: Application> {
    #[allow(dead_code)]
    pub(crate) id: WindowId,
    pub(crate) window: Arc<Window>,
    #[allow(dead_code)]
    tree: UiTree<A>,
    pub(crate) redraw_requested: bool,
}

impl<A: Application> WindowRuntime<A> {
    fn new(id: WindowId, window: Arc<Window>) -> Self {
        Self {
            id,
            window,
            tree: UiTree::new(),
            redraw_requested: false,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn tree(&self) -> &UiTree<A> {
        &self.tree
    }

    #[allow(dead_code)]
    pub(crate) fn tree_mut(&mut self) -> &mut UiTree<A> {
        &mut self.tree
    }
}
