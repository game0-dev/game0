use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, OnceLock, Weak};

use winit::application::ApplicationHandler;
use winit::event::WindowEvent as WinitWindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::WindowId;

use crate::cx::Cx;
use crate::gpu_runtime::GpuRuntime;
use crate::render_tree::node::NodeId;
use crate::text_system::SharedTextSystem;
use crate::window::{GreactWindow, WindowContext, WindowRootFactory, WindowSpec, WindowThreadEvent};

pub enum UserEvent {
    ExecuteApp(Box<dyn FnOnce(&mut App, &ActiveEventLoop) + Send + 'static>),
    WindowExited { window_id: WindowId },
}

pub struct App {
    windows: HashMap<WindowId, Arc<WindowContext>>,
    pub app_context: Arc<AppContext>,
}

pub struct AppContext {
    me: Weak<Self>,
    winit_event_proxy: winit::event_loop::EventLoopProxy<UserEvent>,
    executor: tokio::runtime::Runtime,
    pub(crate) shared_runtime: OnceLock<Arc<GpuRuntime>>,
    pub(crate) shared_text_system: Arc<std::sync::Mutex<SharedTextSystem>>,
}

impl App {
    pub fn run(pre_run: impl FnOnce(&mut Self) + Send + 'static) {
        let executor = tokio::runtime::Runtime::new().expect("failed to create runtime");
        let winit_event_loop = EventLoop::<UserEvent>::with_user_event()
            .build()
            .expect("failed to create event loop");
        let winit_event_proxy = winit_event_loop.create_proxy();

        let app_context = Arc::new_cyclic(|me| AppContext {
            me: me.clone(),
            winit_event_proxy,
            executor,
            shared_runtime: OnceLock::new(),
            shared_text_system: Arc::new(std::sync::Mutex::new(SharedTextSystem::new(2048))),
        });

        let mut app = Self {
            windows: HashMap::new(),
            app_context,
        };

        pre_run(&mut app);

        let _ = winit_event_loop.run_app(&mut app);
    }

    pub fn spawn_ui<F>(&self, window_id: WindowId, func: F)
    where
        F: FnOnce(&mut GreactWindow) + Send + 'static,
    {
        let Some(window) = self.windows.get(&window_id) else {
            eprintln!("window not found: {window_id:?}");
            return;
        };

        if let Err(err) = window.send_ui_thread(WindowThreadEvent::RunCallback(Box::new(func))) {
            eprintln!("failed to send callback to window thread: {err}");
        }
    }

    pub fn send_to_ui_thread(&self, window_id: WindowId, event: WindowThreadEvent) {
        let Some(window) = self.windows.get(&window_id) else {
            eprintln!("window not found: {window_id:?}");
            return;
        };

        if let Err(err) = window.send_ui_thread(event) {
            eprintln!("failed to send event to window thread: {err}");
        }
    }

    fn create_window_internal(
        &mut self,
        event_loop: &ActiveEventLoop,
        spec: WindowSpec,
        root_factory: WindowRootFactory,
    ) {
        let winit_window = Arc::new(
            event_loop
                .create_window(spec.to_attributes())
                .expect("failed to create window"),
        );

        let runtime = Arc::clone(self.app_context.shared_runtime.get_or_init(|| {
            Arc::new(pollster::block_on(GpuRuntime::new(winit_window.as_ref())))
        }));

        let context = WindowContext::create_and_start(
            Arc::downgrade(&self.app_context),
            self.app_context.winit_event_proxy.clone(),
            runtime,
            Arc::clone(&self.app_context.shared_text_system),
            winit_window,
            root_factory,
        );

        self.windows.insert(context.id, context);
    }

    fn close_window_internal(&mut self, window_id: WindowId) {
        if let Some(window) = self.windows.get(&window_id) {
            let _ = window.send_ui_thread(WindowThreadEvent::Close);
        }
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::ExecuteApp(func) => {
                func(self, event_loop);
            }
            UserEvent::WindowExited { window_id } => {
                self.windows.remove(&window_id);
                if self.windows.is_empty() {
                    event_loop.exit();
                }
            }
        }
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        id: WindowId,
        event: WinitWindowEvent,
    ) {
        self.send_to_ui_thread(id, WindowThreadEvent::PlatformEvent(event));
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Wait);
    }
}

impl AppContext {
    pub fn spawn_main(
        &self,
        func: impl FnOnce(&mut App, &ActiveEventLoop) + Send + 'static,
    ) {
        let _ = self
            .winit_event_proxy
            .send_event(UserEvent::ExecuteApp(Box::new(func)));
    }

    pub fn spawn_io<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.executor.spawn(future);
    }

    pub fn spawn_io_blocking<F: Future>(&self, future: F) -> F::Output {
        self.executor.block_on(future)
    }

    pub fn spawn_background<F>(&self, func: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.executor.spawn_blocking(func);
    }

    pub fn spawn_ui<F>(&self, window_id: WindowId, func: F)
    where
        F: FnOnce(&mut GreactWindow) + Send + 'static,
    {
        self.spawn_main(move |app, _event_loop| {
            app.spawn_ui(window_id, func);
        });
    }

    pub fn create_window<F>(&self, spec: WindowSpec, root_factory: F)
    where
        F: FnOnce(&Cx) -> NodeId + Send + 'static,
    {
        self.spawn_main(move |app, event_loop| {
            app.create_window_internal(event_loop, spec, Box::new(root_factory));
        });
    }

    pub fn close_window(&self, window_id: WindowId) {
        self.spawn_main(move |app, _event_loop| {
            app.close_window_internal(window_id);
        });
    }

    pub fn me(&self) -> Arc<Self> {
        self.me.upgrade().expect("app context dropped")
    }
}
