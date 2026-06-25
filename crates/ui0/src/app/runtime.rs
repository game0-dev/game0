use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

use slotmap::SlotMap;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton as WinitMouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::window::{Window, WindowId as WinitWindowId};

use super::async_runtime::AsyncRuntime;
use super::{AppCx, Application, WindowCx};
use crate::element::{mount_element, MountedRegion};
use crate::reactive::ReactiveRuntime;
use crate::ui_tree::{
    KeyModifiers, NodeId, Point, PointerButton, PointerButtons, PointerEvent, UiTree,
};
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
    pub(crate) windows: SlotMap<WindowId, WindowRuntime>,
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
        let winit_id = runtime.window.id();
        runtime.dispose();
        self.winit_to_ui0.remove(&winit_id);
        true
    }

    pub(crate) fn flush_reactive(&mut self, window: WindowId) {
        let Some(runtime) = self.windows.get_mut(window) else {
            return;
        };
        if runtime.flush_reactive() {
            self.request_redraw(window);
        }
    }

    pub(crate) fn flush_all_reactive(&mut self) {
        let windows = self.windows.keys().collect::<Vec<_>>();
        for window in windows {
            self.flush_reactive(window);
        }
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
                self.state.flush_all_reactive();
            }
            RuntimeMsg::RunWindow { window, task } => {
                if self.state.windows.contains_key(window) {
                    let mut cx = WindowCx {
                        state: &mut self.state,
                        event_loop,
                        window,
                    };
                    task(&mut self.app, &mut cx);
                    self.state.flush_reactive(window);
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
                if let Some(runtime) = self.state.windows.get_mut(window) {
                    if runtime.resize(size.width, size.height) {
                        self.state.request_redraw(window);
                    }
                }
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
            WindowEvent::CursorMoved { position, .. } => {
                let point = Point {
                    x: position.x as f32,
                    y: position.y as f32,
                };
                self.dispatch_window_pointer(window, |runtime| runtime.pointer_moved(point));
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.dispatch_window_pointer(window, |runtime| runtime.mouse_input(state, button));
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                if let Some(runtime) = self.state.windows.get_mut(window) {
                    runtime.modifiers = key_modifiers(modifiers.state());
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
    fn emit_app_event(&mut self, event_loop: &ActiveEventLoop, event: AppEvent) {
        let mut cx = AppCx {
            state: &mut self.state,
            event_loop,
        };
        self.app.handle_event(&mut cx, event);
        self.state.flush_all_reactive();
    }

    fn close_window(&mut self, event_loop: &ActiveEventLoop, window: WindowId) {
        if self.state.close_window(window) {
            self.emit_app_event(event_loop, AppEvent::WindowClosed(window));
        }
        if self.state.windows.is_empty() {
            event_loop.exit();
        }
    }

    fn dispatch_window_pointer<F>(&mut self, window: WindowId, f: F)
    where
        F: FnOnce(&mut WindowRuntime) -> bool,
    {
        let Some(runtime) = self.state.windows.get_mut(window) else {
            return;
        };
        let redraw_requested = f(runtime);
        self.state.flush_reactive(window);
        if redraw_requested {
            self.state.request_redraw(window);
        }
    }
}

pub(crate) struct WindowRuntime {
    #[allow(dead_code)]
    pub(crate) id: WindowId,
    #[allow(dead_code)]
    tree: UiTree,
    root_region: MountedRegion,
    reactive: ReactiveRuntime,
    viewport_width: f32,
    viewport_height: f32,
    cursor_position: Option<Point>,
    pointer_buttons: PointerButtons,
    modifiers: KeyModifiers,
    hover_path: Vec<NodeId>,
    pressed_target: Option<NodeId>,
    pub(crate) redraw_requested: bool,
    pub(crate) window: Arc<Window>,
}

impl WindowRuntime {
    fn new(id: WindowId, window: Arc<Window>) -> Self {
        let tree = UiTree::new();
        let root_region = MountedRegion::new(tree.root());
        let size = window.inner_size();
        Self {
            id,
            tree,
            root_region,
            reactive: ReactiveRuntime::new(),
            viewport_width: size.width as f32,
            viewport_height: size.height as f32,
            cursor_position: None,
            pointer_buttons: PointerButtons::empty(),
            modifiers: KeyModifiers::empty(),
            hover_path: Vec::new(),
            pressed_target: None,
            redraw_requested: false,
            window,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn tree(&self) -> &UiTree {
        &self.tree
    }

    #[allow(dead_code)]
    pub(crate) fn tree_mut(&mut self) -> &mut UiTree {
        &mut self.tree
    }

    pub(crate) fn mount<F, E>(&mut self, build: F)
    where
        F: FnOnce() -> E + 'static,
        E: crate::element::IntoElement,
    {
        self.reactive.dispose_all();
        for node in std::mem::take(&mut self.root_region.nodes) {
            self.tree.remove_subtree(node);
        }
        self.reactive = ReactiveRuntime::new();
        self.root_region = MountedRegion::new(self.tree.root());
        let root = self.reactive.root_owner();
        self.reactive.enter(root, || {
            let element = build().into_element();
            mount_element(
                &mut self.tree,
                &self.reactive,
                &mut self.root_region,
                element,
            );
        });
        self.flush_reactive();
    }

    pub(crate) fn flush_reactive(&mut self) -> bool {
        let reactive_changed = self.reactive.flush(&mut self.tree);
        let layout_changed = self.layout();
        reactive_changed || layout_changed
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32) -> bool {
        self.viewport_width = width as f32;
        self.viewport_height = height as f32;
        self.layout()
    }


    pub(crate) fn dispose(self) {
        self.reactive.dispose_all();
    }

    fn layout(&mut self) -> bool {
        self.tree
            .compute_layout(self.viewport_width, self.viewport_height)
    }


    fn pointer_moved(&mut self, position: Point) -> bool {
        self.cursor_position = Some(position);
        let event = self.pointer_event(position, None);
        let hit = self.tree.hit_test(position);
        let mut redraw_requested = self.dispatch_hover_changes(&event, &hit.path);
        if let Some(target) = hit.target {
            redraw_requested |=
                self.dispatch_pointer(target, &hit.path, &event, PointerDispatch::Move);
        }
        self.hover_path = hit.path;
        redraw_requested
    }

    fn mouse_input(&mut self, state: ElementState, button: WinitMouseButton) -> bool {
        let Some(position) = self.cursor_position else {
            return false;
        };

        let button = pointer_button(button);
        match state {
            ElementState::Pressed => self.pointer_buttons.insert(buttons_flag(button)),
            ElementState::Released => self.pointer_buttons.remove(buttons_flag(button)),
        }

        let event = self.pointer_event(position, Some(button));
        let hit = self.tree.hit_test(position);
        match state {
            ElementState::Pressed => {
                self.pressed_target = hit.target;
                hit.target
                    .map(|target| {
                        self.dispatch_pointer(target, &hit.path, &event, PointerDispatch::Down)
                    })
                    .unwrap_or(false)
            }
            ElementState::Released => {
                let mut redraw_requested = hit
                    .target
                    .map(|target| {
                        self.dispatch_pointer(target, &hit.path, &event, PointerDispatch::Up)
                    })
                    .unwrap_or(false);
                if hit.target.is_some()
                    && hit.target == self.pressed_target
                    && matches!(button, PointerButton::Primary)
                {
                    redraw_requested |= self.dispatch_click(hit.target.unwrap(), &hit.path);
                }
                if matches!(button, PointerButton::Primary) {
                    self.pressed_target = None;
                }
                redraw_requested
            }
        }
    }

    fn pointer_event(&self, position: Point, button: Option<PointerButton>) -> PointerEvent {
        PointerEvent {
            pointer_id: 0,
            position,
            button,
            buttons: self.pointer_buttons,
            modifiers: self.modifiers,
        }
    }

    fn dispatch_hover_changes(&mut self, event: &PointerEvent, new_path: &[NodeId]) -> bool {
        let common_prefix = self
            .hover_path
            .iter()
            .zip(new_path.iter())
            .take_while(|(old, new)| old == new)
            .count();
        let old_tail = self.hover_path[common_prefix..].to_vec();
        let new_tail = new_path[common_prefix..].to_vec();

        let mut redraw_requested = false;
        for node in old_tail.iter().rev() {
            redraw_requested |= self.dispatch_direct_pointer(*node, event, PointerDispatch::Leave);
        }
        for node in new_tail {
            redraw_requested |= self.dispatch_direct_pointer(node, event, PointerDispatch::Enter);
        }
        redraw_requested
    }

    fn dispatch_pointer(
        &mut self,
        target: NodeId,
        path: &[NodeId],
        event: &PointerEvent,
        dispatch: PointerDispatch,
    ) -> bool {
        let mut redraw_requested = false;
        for current_target in path.iter().rev().copied() {
            let Some(handlers) = self.tree.event_handlers.get_mut(current_target) else {
                continue;
            };
            let Some(handler) = pointer_handler(handlers, dispatch) else {
                continue;
            };
            let mut cx = crate::app::EventCx::new(self.id, target, current_target);
            handler(&mut cx, event);
            redraw_requested |= cx.redraw_requested;
            if cx.stopped {
                break;
            }
        }
        redraw_requested
    }

    fn dispatch_direct_pointer(
        &mut self,
        target: NodeId,
        event: &PointerEvent,
        dispatch: PointerDispatch,
    ) -> bool {
        let Some(handlers) = self.tree.event_handlers.get_mut(target) else {
            return false;
        };
        let Some(handler) = pointer_handler(handlers, dispatch) else {
            return false;
        };
        let mut cx = crate::app::EventCx::new(self.id, target, target);
        handler(&mut cx, event);
        cx.redraw_requested
    }

    fn dispatch_click(&mut self, target: NodeId, path: &[NodeId]) -> bool {
        let mut redraw_requested = false;
        for current_target in path.iter().rev().copied() {
            let Some(handlers) = self.tree.event_handlers.get_mut(current_target) else {
                continue;
            };
            let Some(handler) = handlers.click.as_mut() else {
                continue;
            };
            let mut cx = crate::app::EventCx::new(self.id, target, current_target);
            handler(&mut cx);
            redraw_requested |= cx.redraw_requested;
            if cx.stopped {
                break;
            }
        }
        redraw_requested
    }
}

#[derive(Debug, Clone, Copy)]
enum PointerDispatch {
    Down,
    Up,
    Move,
    Enter,
    Leave,
}

fn pointer_handler(
    handlers: &mut crate::ui_tree::EventHandlers,
    dispatch: PointerDispatch,
) -> Option<&mut crate::ui_tree::PointerHandler> {
    match dispatch {
        PointerDispatch::Down => handlers.pointer_down.as_mut(),
        PointerDispatch::Up => handlers.pointer_up.as_mut(),
        PointerDispatch::Move => handlers.pointer_move.as_mut(),
        PointerDispatch::Enter => handlers.pointer_enter.as_mut(),
        PointerDispatch::Leave => handlers.pointer_leave.as_mut(),
    }
}

fn pointer_button(button: WinitMouseButton) -> PointerButton {
    match button {
        WinitMouseButton::Left => PointerButton::Primary,
        WinitMouseButton::Right => PointerButton::Secondary,
        WinitMouseButton::Middle => PointerButton::Auxiliary,
        WinitMouseButton::Back => PointerButton::Back,
        WinitMouseButton::Forward => PointerButton::Forward,
        WinitMouseButton::Other(value) => PointerButton::Other(value),
    }
}

fn buttons_flag(button: PointerButton) -> PointerButtons {
    match button {
        PointerButton::Primary => PointerButtons::PRIMARY,
        PointerButton::Secondary => PointerButtons::SECONDARY,
        PointerButton::Auxiliary => PointerButtons::AUXILIARY,
        PointerButton::Back => PointerButtons::BACK,
        PointerButton::Forward => PointerButtons::FORWARD,
        PointerButton::Other(_) => PointerButtons::empty(),
    }
}

fn key_modifiers(modifiers: winit::keyboard::ModifiersState) -> KeyModifiers {
    let mut out = KeyModifiers::empty();
    if modifiers.shift_key() {
        out.insert(KeyModifiers::SHIFT);
    }
    if modifiers.control_key() {
        out.insert(KeyModifiers::CTRL);
    }
    if modifiers.alt_key() {
        out.insert(KeyModifiers::ALT);
    }
    if modifiers.super_key() {
        out.insert(KeyModifiers::SUPER);
    }
    out
}
