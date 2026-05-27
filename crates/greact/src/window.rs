use std::cell::{Cell, RefCell};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Weak};
use std::time::{Duration, Instant};

use arboard::Clipboard;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent as WinitWindowEvent};
use winit::event_loop::EventLoopProxy;
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Window, WindowAttributes, WindowId};

use crate::application::{AppContext, UserEvent};
use crate::cx::Cx;
use crate::gpu_runtime::GpuRuntime;
use crate::input_edit;
use crate::layout::compute_layout;
use crate::reactive::effect::flush_effects;
use crate::render::RenderListBuilder;
use crate::render_tree::node::{DirtyFlags, ElementTag, NodeId};
use crate::render_tree::{CanvasPointerEvent, CanvasWheelEvent, InputState};
use crate::renderer::{GpuRenderer, GridDrawParams};
use crate::text_system::SharedTextSystem;
use crate::with_render_tree;

const BLINK_INTERVAL: Duration = Duration::from_millis(530);
const INPUT_TEXT_INSET_X: f32 = 6.0;
const DOUBLE_CLICK_WINDOW: Duration = Duration::from_millis(300);

pub type WindowRootFactory = Box<dyn FnOnce(&Cx) -> NodeId + Send + 'static>;
pub type WindowUiCallback = Box<dyn FnOnce(&mut GreactWindow) + Send + 'static>;

thread_local! {
    static CURRENT_WINDOW_ID: Cell<Option<WindowId>> = const { Cell::new(None) };
    static CURRENT_APP_CONTEXT: RefCell<Option<Arc<AppContext>>> = const { RefCell::new(None) };
}

pub fn current_window_id() -> Option<WindowId> {
    CURRENT_WINDOW_ID.with(|id| id.get())
}

pub fn current_app_context() -> Option<Arc<AppContext>> {
    CURRENT_APP_CONTEXT.with(|ctx| ctx.borrow().clone())
}

#[derive(Debug, Clone)]
pub struct WindowSpec {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub transparent: bool,
}

impl Default for WindowSpec {
    fn default() -> Self {
        Self {
            title: "greact".to_string(),
            width: 1280,
            height: 800,
            transparent: false,
        }
    }
}

impl WindowSpec {
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn with_inner_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn with_transparent(mut self, transparent: bool) -> Self {
        self.transparent = transparent;
        self
    }

    pub(crate) fn to_attributes(&self) -> WindowAttributes {
        WindowAttributes::default()
            .with_title(self.title.clone())
            .with_inner_size(PhysicalSize::new(self.width.max(1), self.height.max(1)))
            .with_transparent(self.transparent)
    }
}

pub enum WindowThreadEvent {
    PlatformEvent(WinitWindowEvent),
    RunCallback(WindowUiCallback),
    Close,
}

pub struct WindowContext {
    pub(crate) app_context: Weak<AppContext>,
    pub(crate) id: WindowId,
    pub(crate) winit_window: Arc<Window>,
    thread_sender: mpsc::Sender<WindowThreadEvent>,
    running: AtomicBool,
}

impl WindowContext {
    pub(crate) fn create_and_start(
        app_context: Weak<AppContext>,
        app_proxy: EventLoopProxy<UserEvent>,
        runtime: Arc<GpuRuntime>,
        shared_text_system: Arc<std::sync::Mutex<SharedTextSystem>>,
        winit_window: Arc<Window>,
        root_factory: WindowRootFactory,
    ) -> Arc<Self> {
        let (sender, receiver) = mpsc::channel();
        let context = Arc::new(Self {
            app_context,
            id: winit_window.id(),
            winit_window,
            thread_sender: sender,
            running: AtomicBool::new(true),
        });

        GreactWindow::start_ui_thread(
            context.id,
            Arc::clone(&context),
            receiver,
            runtime,
            shared_text_system,
            root_factory,
            app_proxy,
        );

        context
    }

    pub(crate) fn send_ui_thread(
        &self,
        event: WindowThreadEvent,
    ) -> Result<(), mpsc::SendError<WindowThreadEvent>> {
        self.thread_sender.send(event)
    }

    pub(crate) fn get_physical_size(&self) -> PhysicalSize<u32> {
        self.winit_window.inner_size()
    }
}

pub struct GreactWindow {
    id: WindowId,
    context: Arc<WindowContext>,
    app_context: Arc<AppContext>,
    runtime: Arc<GpuRuntime>,
    renderer: GpuRenderer,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    scale_factor: f32,
    root: NodeId,
    render_builder: RenderListBuilder,
    mouse_pos: (f32, f32),
    pressed_click_target: Option<NodeId>,
    pending_resize: Option<PhysicalSize<u32>>,
    modifiers: ModifiersState,
    clipboard: Option<Clipboard>,
    captured_canvas: Option<NodeId>,
    last_click_hit: Option<NodeId>,
    last_click_at: Option<Instant>,
}

impl GreactWindow {
    fn start_ui_thread(
        id: WindowId,
        context: Arc<WindowContext>,
        receiver: mpsc::Receiver<WindowThreadEvent>,
        runtime: Arc<GpuRuntime>,
        shared_text_system: Arc<std::sync::Mutex<SharedTextSystem>>,
        root_factory: WindowRootFactory,
        app_proxy: EventLoopProxy<UserEvent>,
    ) {
        std::thread::Builder::new()
            .name(format!("greact-ui-{id:?}"))
            .spawn(move || {
                CURRENT_WINDOW_ID.with(|slot| slot.set(Some(id)));

                let app_context = context
                    .app_context
                    .upgrade()
                    .expect("app context dropped before window thread start");
                CURRENT_APP_CONTEXT.with(|ctx| {
                    *ctx.borrow_mut() = Some(Arc::clone(&app_context));
                });

                let cx = Cx;
                let root = root_factory(&cx);
                with_render_tree(|tree| {
                    tree.set_root(root);
                    tree.mark_dirty(root, DirtyFlags::LAYOUT | DirtyFlags::PAINT);
                });

                let mut renderer = GpuRenderer::from_runtime(runtime.as_ref(), 2048);
                let (surface, config) = runtime.create_surface(Arc::clone(&context.winit_window));
                let scale_factor = context.winit_window.scale_factor().max(0.1) as f32;
                let logical_size =
                    PhysicalSize::new(config.width, config.height).to_logical::<f32>(scale_factor as f64);
                renderer.update_viewport(
                    config.width,
                    config.height,
                    logical_size.width,
                    logical_size.height,
                    scale_factor,
                );

                let mut window = Self {
                    id,
                    context: Arc::clone(&context),
                    app_context,
                    runtime,
                    renderer,
                    surface,
                    config,
                    scale_factor,
                    root,
                    render_builder: RenderListBuilder::with_text_system(shared_text_system),
                    mouse_pos: (0.0, 0.0),
                    pressed_click_target: None,
                    pending_resize: None,
                    modifiers: ModifiersState::empty(),
                    clipboard: None,
                    captured_canvas: None,
                    last_click_hit: None,
                    last_click_at: None,
                };

                window.request_redraw();

                while context.running.load(Ordering::Relaxed) {
                    let next_blink = update_blink_state(&mut window);
                    let received = if let Some(next) = next_blink {
                        let now = Instant::now();
                        if next <= now {
                            continue;
                        }
                        match receiver.recv_timeout(next.saturating_duration_since(now)) {
                            Ok(event) => Some(event),
                            Err(mpsc::RecvTimeoutError::Timeout) => None,
                            Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        }
                    } else {
                        match receiver.recv() {
                            Ok(event) => Some(event),
                            Err(_) => break,
                        }
                    };

                    let Some(event) = received else {
                        continue;
                    };

                    match event {
                        WindowThreadEvent::PlatformEvent(window_event) => {
                            window.handle_event(window_event);
                        }
                        WindowThreadEvent::RunCallback(callback) => {
                            callback(&mut window);
                            flush_effects();
                            window.request_redraw();
                        }
                        WindowThreadEvent::Close => {
                            context.running.store(false, Ordering::Relaxed);
                        }
                    }
                }

                CURRENT_WINDOW_ID.with(|slot| slot.set(None));
                CURRENT_APP_CONTEXT.with(|ctx| {
                    *ctx.borrow_mut() = None;
                });
                let _ = app_proxy.send_event(UserEvent::WindowExited { window_id: id });
            })
            .expect("failed to spawn greact ui thread");
    }

    pub fn id(&self) -> WindowId {
        self.id
    }

    pub fn window(&self) -> &Window {
        self.context.winit_window.as_ref()
    }

    pub fn app_context(&self) -> Arc<AppContext> {
        Arc::clone(&self.app_context)
    }

    pub fn request_redraw(&self) {
        self.context.winit_window.request_redraw();
    }

    pub fn set_title(&self, title: &str) {
        self.context.winit_window.set_title(title);
    }

    pub fn close(&mut self) {
        self.context.running.store(false, Ordering::Relaxed);
    }

    fn handle_event(&mut self, event: WinitWindowEvent) {
        match event {
            WinitWindowEvent::CloseRequested => {
                self.context.running.store(false, Ordering::Relaxed);
            }
            WinitWindowEvent::Resized(new_size) => {
                self.pending_resize = Some(new_size);
                self.request_redraw();
            }
            WinitWindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale_factor = scale_factor.max(0.1) as f32;
                self.pending_resize = Some(self.context.get_physical_size());
                self.request_redraw();
            }
            WinitWindowEvent::CursorMoved { position, .. } => {
                let logical = position.to_logical::<f32>(self.scale_factor as f64);
                self.mouse_pos = (logical.x, logical.y);
                dispatch_canvas_move(self);
            }
            WinitWindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WinitWindowEvent::MouseInput {
                state: button_state,
                button: MouseButton::Left,
                ..
            } => {
                handle_mouse_left(self, button_state);
            }
            WinitWindowEvent::MouseWheel { delta, .. } => {
                if handle_canvas_wheel(self, delta) {
                    self.request_redraw();
                    return;
                }

                let scroll_delta = match delta {
                    MouseScrollDelta::LineDelta(_, y) => -y * 24.0,
                    MouseScrollDelta::PixelDelta(pos) => {
                        (pos.y as f32) / self.scale_factor.max(0.1)
                    }
                };
                let maybe_scroller = with_render_tree(|tree| {
                    tree.hit_test(self.root, self.mouse_pos.0, self.mouse_pos.1)
                        .and_then(|id| tree.nearest_scrollable_ancestor(id))
                });
                if let Some(scroller) = maybe_scroller {
                    with_render_tree(|tree| {
                        let current = tree.get_scroll_offset(scroller);
                        let max = tree.max_scroll_offset_y(scroller);
                        let next = (current + scroll_delta).clamp(0.0, max);
                        tree.set_scroll_offset(scroller, next);
                        tree.mark_dirty(scroller, DirtyFlags::PAINT);
                    });
                    self.request_redraw();
                }
            }
            WinitWindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    if handle_keyboard_input(self, &event) {
                        self.request_redraw();
                        update_ime_cursor_area(self);
                    }
                }
            }
            WinitWindowEvent::Ime(ime) => {
                if handle_ime_event(ime) {
                    self.request_redraw();
                    update_ime_cursor_area(self);
                }
            }
            WinitWindowEvent::Focused(false) => {
                handle_input_focus_change(self, None);
                self.captured_canvas = None;
            }
            WinitWindowEvent::RedrawRequested => {
                render_frame(self);
            }
            _ => {}
        }
    }
}

fn update_blink_state(state: &mut GreactWindow) -> Option<Instant> {
    let mut next_blink = None;
    let mut toggled = false;
    with_render_tree(|tree| {
        if let Some(id) = tree.focused_input() {
            let mut should_mark = false;
            if let Some(input) = tree.get_input_state_mut(id) {
                let now = Instant::now();
                if now >= input.next_blink_at {
                    input.blink_visible = !input.blink_visible;
                    input.next_blink_at = now + BLINK_INTERVAL;
                    should_mark = true;
                    toggled = true;
                }
                next_blink = Some(input.next_blink_at);
            }
            if should_mark {
                tree.mark_dirty(id, DirtyFlags::PAINT);
            }
        }
    });

    if toggled {
        state.request_redraw();
    }

    next_blink
}

fn handle_mouse_left(state: &mut GreactWindow, button_state: ElementState) {
    let hit = with_render_tree(|tree| tree.hit_test(state.root, state.mouse_pos.0, state.mouse_pos.1));
    let click_target = with_render_tree(|tree| hit.and_then(|id| tree.nearest_clickable_ancestor(id)));
    let input_target = with_render_tree(|tree| hit.and_then(|id| tree.nearest_input_ancestor(id)));
    let canvas_target = with_render_tree(|tree| hit.and_then(|id| tree.nearest_canvas_ancestor(id)));

    match button_state {
        ElementState::Pressed => {
            state.pressed_click_target = click_target;
            handle_input_focus_change(state, input_target);

            if let Some(id) = click_target {
                with_render_tree(|tree| {
                    tree.set_pressed(id, true);
                    tree.mark_dirty(id, DirtyFlags::PAINT);
                });
            }

            let click_count = compute_click_count(state, hit);
            if let Some(canvas_id) = canvas_target {
                with_render_tree(|tree| {
                    tree.ensure_canvas_state(canvas_id).captured_pointer = true;
                });
                state.captured_canvas = Some(canvas_id);
                dispatch_canvas_pointer(
                    state,
                    canvas_id,
                    CanvasPointerEvent {
                        x: state.mouse_pos.0,
                        y: state.mouse_pos.1,
                        hit,
                        click_count,
                        shift: state.modifiers.shift_key(),
                        ctrl: state.modifiers.control_key(),
                        alt: state.modifiers.alt_key(),
                        meta: state.modifiers.super_key(),
                    },
                    CanvasDispatchKind::Down,
                );
            }

            state.context.winit_window.request_redraw();
        }
        ElementState::Released => {
            if let Some(canvas_id) = state.captured_canvas {
                dispatch_canvas_pointer(
                    state,
                    canvas_id,
                    CanvasPointerEvent {
                        x: state.mouse_pos.0,
                        y: state.mouse_pos.1,
                        hit,
                        click_count: 1,
                        shift: state.modifiers.shift_key(),
                        ctrl: state.modifiers.control_key(),
                        alt: state.modifiers.alt_key(),
                        meta: state.modifiers.super_key(),
                    },
                    CanvasDispatchKind::Up,
                );
                with_render_tree(|tree| {
                    if let Some(canvas_state) = tree.get_canvas_state_mut(canvas_id) {
                        canvas_state.captured_pointer = false;
                    }
                });
                state.captured_canvas = None;
            }

            let pressed_target = state.pressed_click_target.take();
            if let Some(id) = pressed_target {
                with_render_tree(|tree| {
                    tree.set_pressed(id, false);
                    tree.mark_dirty(id, DirtyFlags::PAINT);
                });
            }

            if pressed_target.is_some() && pressed_target == click_target {
                let handler = with_render_tree(|tree| {
                    click_target.and_then(|node_id| {
                        tree.get_handler(node_id)
                            .and_then(|h| h.on_click.clone())
                    })
                });
                if let Some(cb) = handler {
                    cb();
                    flush_effects();
                }
            }

            with_render_tree(|tree| {
                tree.mark_dirty(state.root, DirtyFlags::PAINT | DirtyFlags::LAYOUT);
            });
            state.context.winit_window.request_redraw();
        }
    }
}

fn compute_click_count(state: &mut GreactWindow, hit: Option<NodeId>) -> u8 {
    let now = Instant::now();
    let click_count = if state.last_click_hit == hit
        && state
            .last_click_at
            .map(|prev| now.saturating_duration_since(prev) <= DOUBLE_CLICK_WINDOW)
            .unwrap_or(false)
    {
        2
    } else {
        1
    };
    state.last_click_hit = hit;
    state.last_click_at = Some(now);
    click_count
}

fn dispatch_canvas_move(state: &mut GreactWindow) {
    let hit = with_render_tree(|tree| tree.hit_test(state.root, state.mouse_pos.0, state.mouse_pos.1));
    let canvas_target = if let Some(captured) = state.captured_canvas {
        Some(captured)
    } else {
        with_render_tree(|tree| hit.and_then(|id| tree.nearest_canvas_ancestor(id)))
    };

    with_render_tree(|tree| {
        let canvas_ids: Vec<NodeId> = tree
            .nodes
            .iter()
            .filter_map(|(id, node)| (node.tag == ElementTag::Canvas).then_some(id))
            .collect();
        for id in canvas_ids {
            if let Some(canvas_state) = tree.get_canvas_state_mut(id) {
                canvas_state.hovered = Some(id) == canvas_target;
            }
        }
    });

    if let Some(canvas_id) = canvas_target {
        dispatch_canvas_pointer(
            state,
            canvas_id,
            CanvasPointerEvent {
                x: state.mouse_pos.0,
                y: state.mouse_pos.1,
                hit,
                click_count: 1,
                shift: state.modifiers.shift_key(),
                ctrl: state.modifiers.control_key(),
                alt: state.modifiers.alt_key(),
                meta: state.modifiers.super_key(),
            },
            CanvasDispatchKind::Move,
        );
        state.context.winit_window.request_redraw();
    }
}

enum CanvasDispatchKind {
    Down,
    Move,
    Up,
}

fn dispatch_canvas_pointer(
    _state: &mut GreactWindow,
    canvas_id: NodeId,
    event: CanvasPointerEvent,
    kind: CanvasDispatchKind,
) {
    let callback = with_render_tree(|tree| {
        tree.get_canvas_handlers(canvas_id).and_then(|h| match kind {
            CanvasDispatchKind::Down => h.on_pointer_down.clone(),
            CanvasDispatchKind::Move => h.on_pointer_move.clone(),
            CanvasDispatchKind::Up => h.on_pointer_up.clone(),
        })
    });

    if let Some(cb) = callback {
        cb(event);
        flush_effects();
    }
}

fn handle_canvas_wheel(state: &mut GreactWindow, delta: MouseScrollDelta) -> bool {
    let hit = with_render_tree(|tree| tree.hit_test(state.root, state.mouse_pos.0, state.mouse_pos.1));
    let canvas_target = with_render_tree(|tree| hit.and_then(|id| tree.nearest_canvas_ancestor(id)));
    let Some(canvas_id) = canvas_target else {
        return false;
    };

    let (dx, dy) = match delta {
        MouseScrollDelta::LineDelta(x, y) => (x * 24.0, y * 24.0),
        MouseScrollDelta::PixelDelta(pos) => (
            (pos.x as f32) / state.scale_factor.max(0.1),
            (pos.y as f32) / state.scale_factor.max(0.1),
        ),
    };

    let callback = with_render_tree(|tree| {
        tree.get_canvas_handlers(canvas_id)
            .and_then(|h| h.on_wheel.clone())
    });

    if let Some(cb) = callback {
        cb(CanvasWheelEvent {
            x: state.mouse_pos.0,
            y: state.mouse_pos.1,
            delta_x: dx,
            delta_y: dy,
            shift: state.modifiers.shift_key(),
            ctrl: state.modifiers.control_key(),
            alt: state.modifiers.alt_key(),
            meta: state.modifiers.super_key(),
        });
        flush_effects();
        true
    } else {
        false
    }
}

fn render_frame(state: &mut GreactWindow) {
    if let Some(new_size) = state.pending_resize.take() {
        state.config.width = new_size.width.max(1);
        state.config.height = new_size.height.max(1);
        state
            .surface
            .configure(state.runtime.device(), &state.config);
        state
            .renderer
            .update_viewport(
                state.config.width,
                state.config.height,
                logical_width(state),
                logical_height(state),
                state.scale_factor,
            );
        with_render_tree(|tree| {
            tree.mark_dirty(state.root, DirtyFlags::LAYOUT | DirtyFlags::PAINT);
        });
    }

    let layout_start = Instant::now();
    with_render_tree(|tree| {
        let _ = compute_layout(
            tree,
            state.root,
            logical_width(state),
            logical_height(state),
        );
    });
    let _layout_ms = layout_start.elapsed().as_secs_f32() * 1000.0;

    let grid_params = with_render_tree(|tree| {
        let mut params = None;
        let canvas_ids: Vec<NodeId> = tree
            .nodes
            .iter()
            .filter_map(|(id, node)| (node.tag == ElementTag::Canvas).then_some(id))
            .collect();
        for id in canvas_ids {
            if let Some(rect) = tree.get_layout_rect(id) {
                let canvas_state = tree.ensure_canvas_state(id);
                canvas_state.viewport_rect = Some(rect);
                if canvas_state.show_dot_grid && params.is_none() {
                    params = Some(GridDrawParams {
                        zoom: canvas_state.zoom,
                        pan_world: canvas_state.pan_world,
                        base_world_step: canvas_state.base_world_step,
                        dot_radius_px: canvas_state.dot_radius_px,
                        target_screen_step_px: canvas_state.target_screen_step_px,
                        background_color: [0.985, 0.987, 0.993, 1.0],
                        dot_color: [0.78, 0.80, 0.86, 1.0],
                        scissor: Some((
                            rect.x.max(0.0) as u32,
                            rect.y.max(0.0) as u32,
                            rect.width.max(1.0) as u32,
                            rect.height.max(1.0) as u32,
                        )),
                    });
                }
            }
        }
        params
    });
    state.renderer.set_grid_params(grid_params);

    let focused_input = with_render_tree(|tree| tree.focused_input().is_some());
    state.context.winit_window.set_ime_allowed(focused_input);
    update_ime_cursor_area(state);

    let build_start = Instant::now();
    state
        .render_builder
        .set_viewport(
            logical_width(state).ceil().max(1.0) as u32,
            logical_height(state).ceil().max(1.0) as u32,
        );
    let render_list = with_render_tree(|tree| state.render_builder.build(tree, state.root));
    let _build_ms = build_start.elapsed().as_secs_f32() * 1000.0;

    let upload_start = Instant::now();
    state
        .renderer
        .ensure_text_pages(state.render_builder.atlas_page_count());
    let uploads = state.render_builder.drain_glyph_uploads();
    let _glyph_upload_count = uploads.len();
    for upload in uploads {
        state.renderer.upload_text_glyph(
            upload.page,
            upload.x,
            upload.y,
            upload.width,
            upload.height,
            &upload.alpha,
        );
    }
    let _upload_ms = upload_start.elapsed().as_secs_f32() * 1000.0;

    let draw_start = Instant::now();
    let surface_tex = match state.surface.get_current_texture() {
        Ok(tex) => tex,
        Err(_) => {
            state
                .surface
                .configure(state.runtime.device(), &state.config);
            match state.surface.get_current_texture() {
                Ok(tex) => tex,
                Err(err) => {
                    eprintln!("failed to acquire surface texture: {err}");
                    return;
                }
            }
        }
    };

    let view = surface_tex
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());

    state.renderer.render(&view, &render_list);
    surface_tex.present();
    let _draw_ms = draw_start.elapsed().as_secs_f32() * 1000.0;


    with_render_tree(|tree| {
        let dirty = tree.take_dirty();
        for id in dirty {
            tree.clear_dirty(id);
        }
    });
}

fn handle_input_focus_change(state: &mut GreactWindow, next: Option<NodeId>) {
    let mut blur_cb = None;
    let mut focus_cb = None;
    let mut changed = false;

    with_render_tree(|tree| {
        let prev = tree.focused_input();
        if prev == next {
            return;
        }

        changed = true;

        if let Some(id) = prev {
            if let Some(input) = tree.get_input_state_mut(id) {
                input.is_composing = false;
                input.preedit.clear();
                input.preedit_range = None;
                input.blink_visible = false;
            }
            blur_cb = tree.get_handler(id).and_then(|h| h.on_blur.clone());
            tree.mark_dirty(id, DirtyFlags::PAINT);
            tree.clear_focused_input();
        }

        if let Some(id) = next {
            tree.set_focused_input(id);
            if let Some(input) = tree.get_input_state_mut(id) {
                reset_blink(input);
            }
            focus_cb = tree.get_handler(id).and_then(|h| h.on_focus.clone());
            tree.mark_dirty(id, DirtyFlags::PAINT);
        }
    });

    if !changed {
        return;
    }

    state.context.winit_window.set_ime_allowed(next.is_some());
    if let Some(cb) = blur_cb {
        cb();
        flush_effects();
    }
    if let Some(cb) = focus_cb {
        cb();
        flush_effects();
    }
    update_ime_cursor_area(state);
    state.context.winit_window.request_redraw();
}

fn handle_keyboard_input(state: &mut GreactWindow, event: &winit::event::KeyEvent) -> bool {
    let focused = with_render_tree(|tree| tree.focused_input());
    let Some(node_id) = focused else {
        return false;
    };

    let primary_shortcut = state.modifiers.control_key() || state.modifiers.super_key();
    let shift = state.modifiers.shift_key();

    let mut changed = false;
    let mut emitted_input = None;
    let mut emitted_submit = None;

    with_render_tree(|tree| {
        let mut value_changed = false;
        let mut submit = false;

        {
            let Some(input) = tree.get_input_state_mut(node_id) else {
                return;
            };

            let mut handled_shortcut = false;
            if primary_shortcut {
                if let Some(key) = character_key(&event.logical_key) {
                    handled_shortcut = true;
                    match key {
                        'a' => {
                            let (start, end) = input_edit::select_all(input.value.len());
                            input.selection_anchor = start;
                            input.cursor = end;
                            reset_blink(input);
                            changed = true;
                        }
                        'c' => {
                            if let Some(sel) = selected_text(&input.value, input.cursor, input.selection_anchor) {
                                clipboard_set_text(state, &sel);
                            }
                        }
                        'x' => {
                            if let Some(sel) = selected_text(&input.value, input.cursor, input.selection_anchor) {
                                clipboard_set_text(state, &sel);
                                if input_edit::replace_selection(
                                    &mut input.value,
                                    &mut input.cursor,
                                    &mut input.selection_anchor,
                                    "",
                                ) {
                                    value_changed = true;
                                    reset_blink(input);
                                    changed = true;
                                }
                            }
                        }
                        'v' => {
                            if let Some(paste) = clipboard_get_text(state) {
                                let filtered: String = paste.chars().filter(|c| !c.is_control()).collect();
                                if !filtered.is_empty()
                                    && input_edit::insert_text(
                                        &mut input.value,
                                        &mut input.cursor,
                                        &mut input.selection_anchor,
                                        &filtered,
                                    )
                                {
                                    value_changed = true;
                                    reset_blink(input);
                                    changed = true;
                                }
                            }
                        }
                        _ => {
                            handled_shortcut = false;
                        }
                    }
                }
            }

            if !handled_shortcut {
                match &event.logical_key {
                    Key::Named(NamedKey::ArrowLeft) => {
                        let next = input_edit::move_cursor_left(&input.value, input.cursor);
                        input.cursor = next;
                        if !shift {
                            input.selection_anchor = next;
                        }
                        reset_blink(input);
                        changed = true;
                    }
                    Key::Named(NamedKey::ArrowRight) => {
                        let next = input_edit::move_cursor_right(&input.value, input.cursor);
                        input.cursor = next;
                        if !shift {
                            input.selection_anchor = next;
                        }
                        reset_blink(input);
                        changed = true;
                    }
                    Key::Named(NamedKey::Home) => {
                        let next = input_edit::move_cursor_home(&input.value);
                        input.cursor = next;
                        if !shift {
                            input.selection_anchor = next;
                        }
                        reset_blink(input);
                        changed = true;
                    }
                    Key::Named(NamedKey::End) => {
                        let next = input_edit::move_cursor_end(&input.value);
                        input.cursor = next;
                        if !shift {
                            input.selection_anchor = next;
                        }
                        reset_blink(input);
                        changed = true;
                    }
                    Key::Named(NamedKey::Backspace) => {
                        if input_edit::delete_backward(
                            &mut input.value,
                            &mut input.cursor,
                            &mut input.selection_anchor,
                        ) {
                            value_changed = true;
                            reset_blink(input);
                            changed = true;
                        }
                    }
                    Key::Named(NamedKey::Delete) => {
                        if input_edit::delete_forward(
                            &mut input.value,
                            &mut input.cursor,
                            &mut input.selection_anchor,
                        ) {
                            value_changed = true;
                            reset_blink(input);
                            changed = true;
                        }
                    }
                    Key::Named(NamedKey::Enter) => {
                        submit = true;
                        reset_blink(input);
                        changed = true;
                    }
                    _ => {
                        if !primary_shortcut {
                            if let Some(text) = &event.text {
                                let filtered: String = text.chars().filter(|c| !c.is_control()).collect();
                                if !filtered.is_empty()
                                    && input_edit::insert_text(
                                        &mut input.value,
                                        &mut input.cursor,
                                        &mut input.selection_anchor,
                                        &filtered,
                                    )
                                {
                                    value_changed = true;
                                    reset_blink(input);
                                    changed = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        let current_value = tree
            .get_input_state(node_id)
            .map(|s| s.value.clone())
            .unwrap_or_default();

        if value_changed {
            emitted_input = tree
                .get_handler(node_id)
                .and_then(|h| h.on_input.clone())
                .map(|cb| (cb, current_value.clone()));
        }
        if submit {
            emitted_submit = tree
                .get_handler(node_id)
                .and_then(|h| h.on_submit.clone())
                .map(|cb| (cb, current_value));
        }

        if changed {
            tree.mark_dirty(node_id, DirtyFlags::PAINT);
        }
    });

    if let Some((cb, text)) = emitted_input {
        cb(text);
        flush_effects();
    }
    if let Some((cb, text)) = emitted_submit {
        cb(text);
        flush_effects();
    }

    changed
}

fn handle_ime_event(ime: Ime) -> bool {
    let focused = with_render_tree(|tree| tree.focused_input());
    let Some(node_id) = focused else {
        return false;
    };

    let mut changed = false;
    let mut emitted_input = None;

    with_render_tree(|tree| {
        let mut emit_input_value: Option<String> = None;
        let Some(input) = tree.get_input_state_mut(node_id) else {
            return;
        };

        match ime {
            Ime::Enabled => {
                input.is_composing = true;
                reset_blink(input);
                changed = true;
            }
            Ime::Preedit(text, range) => {
                input.is_composing = true;
                input.preedit = text;
                input.preedit_range = range;
                reset_blink(input);
                changed = true;
            }
            Ime::Commit(text) => {
                if input_edit::insert_text(
                    &mut input.value,
                    &mut input.cursor,
                    &mut input.selection_anchor,
                    &text,
                ) {
                    emit_input_value = Some(input.value.clone());
                }
                input.preedit.clear();
                input.preedit_range = None;
                input.is_composing = false;
                reset_blink(input);
                changed = true;
            }
            Ime::Disabled => {
                input.is_composing = false;
                input.preedit.clear();
                input.preedit_range = None;
                reset_blink(input);
                changed = true;
            }
        }

        if changed {
            tree.mark_dirty(node_id, DirtyFlags::PAINT);
        }
        if let Some(value) = emit_input_value {
            emitted_input = tree
                .get_handler(node_id)
                .and_then(|h| h.on_input.clone())
                .map(|cb| (cb, value));
        }
    });

    if let Some((cb, text)) = emitted_input {
        cb(text);
        flush_effects();
    }

    changed
}

fn reset_blink(input: &mut InputState) {
    input.blink_visible = true;
    input.next_blink_at = Instant::now() + BLINK_INTERVAL;
}

fn character_key(key: &Key) -> Option<char> {
    match key {
        Key::Character(s) => s.chars().next().map(|c| c.to_ascii_lowercase()),
        _ => None,
    }
}

fn normalize_range(a: usize, b: usize) -> Option<(usize, usize)> {
    if a == b {
        return None;
    }
    Some((a.min(b), a.max(b)))
}

fn selected_text(value: &str, cursor: usize, anchor: usize) -> Option<String> {
    let (start, end) = normalize_range(cursor, anchor)?;
    if end > value.len() {
        return None;
    }
    value.get(start..end).map(ToOwned::to_owned)
}

fn clipboard_get_text(state: &mut GreactWindow) -> Option<String> {
    if state.clipboard.is_none() {
        state.clipboard = Clipboard::new().ok();
    }
    let Some(clipboard) = state.clipboard.as_mut() else {
        eprintln!("clipboard unavailable");
        return None;
    };
    match clipboard.get_text() {
        Ok(text) => Some(text),
        Err(err) => {
            eprintln!("clipboard get_text failed: {err}");
            None
        }
    }
}

fn clipboard_set_text(state: &mut GreactWindow, text: &str) {
    if state.clipboard.is_none() {
        state.clipboard = Clipboard::new().ok();
    }
    let Some(clipboard) = state.clipboard.as_mut() else {
        eprintln!("clipboard unavailable");
        return;
    };
    if let Err(err) = clipboard.set_text(text.to_string()) {
        eprintln!("clipboard set_text failed: {err}");
    }
}

fn update_ime_cursor_area(state: &GreactWindow) {
    let cursor_area = with_render_tree(|tree| {
        let id = tree.focused_input()?;
        let rect = tree.get_layout_rect(id)?;
        let text_style = tree
            .text_style_groups
            .get(id)
            .cloned()
            .unwrap_or_default();
        let input = tree.get_input_state(id)?;

        let cursor_chars = count_chars_to_boundary(&input.value, input.cursor);
        let advance = text_style.font_size * 0.6;
        let caret_x = rect.x + INPUT_TEXT_INSET_X + cursor_chars as f32 * advance;
        let caret_y = rect.y + ((rect.height - text_style.font_size) * 0.5).max(0.0);
        let caret_h = text_style.font_size.max(10.0);

        Some((caret_x, caret_y, 2.0f32, caret_h))
    });

    if let Some((x, y, w, h)) = cursor_area {
        let scale = state.scale_factor.max(0.1);
        state.context.winit_window.set_ime_cursor_area(
            PhysicalPosition::new(
                (x.max(0.0) * scale).round() as i32,
                (y.max(0.0) * scale).round() as i32,
            ),
            PhysicalSize::new(
                (w.max(1.0) * scale).round().max(1.0) as u32,
                (h.max(1.0) * scale).round().max(1.0) as u32,
            ),
        );
    }
}

fn logical_width(state: &GreactWindow) -> f32 {
    (state.config.width as f32 / state.scale_factor.max(0.1)).max(1.0)
}

fn logical_height(state: &GreactWindow) -> f32 {
    (state.config.height as f32 / state.scale_factor.max(0.1)).max(1.0)
}

fn count_chars_to_boundary(s: &str, idx: usize) -> usize {
    let idx = idx.min(s.len());
    if idx == s.len() {
        return s.chars().count();
    }

    let mut count = 0usize;
    for (i, _) in s.char_indices() {
        if i >= idx {
            break;
        }
        count += 1;
    }
    count
}
