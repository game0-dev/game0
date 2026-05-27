use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Instant;

use greact::*;
use whiteboard_core::{Command, History, ObjectId, ObjectKind, ObjectStyle, ToolKind, WhiteboardDoc, WhiteboardObject};

const TOOLBAR_WIDTH: f32 = 64.0;

#[derive(Clone)]
struct EditingState {
    object_id: ObjectId,
    before_text: String,
}

#[derive(Clone)]
struct DragState {
    ids: Vec<ObjectId>,
    start_world: [f32; 2],
    initial_positions: HashMap<ObjectId, [f32; 2]>,
}

#[derive(Clone)]
struct PanDragState {
    start_screen: [f32; 2],
    start_pan: [f32; 2],
}

#[derive(Clone)]
struct SelectionBoxState {
    start_world: [f32; 2],
    current_world: [f32; 2],
}

#[derive(Default)]
struct NodeRefs {
    canvas_id: Option<NodeId>,
    overlay_id: Option<NodeId>,
    input_id: Option<NodeId>,
    selection_rect_id: Option<NodeId>,
}

struct UiState {
    doc: WhiteboardDoc,
    history: History,
    tool: ToolKind,
    default_style: ObjectStyle,
    node_refs: NodeRefs,
    object_nodes: HashMap<ObjectId, NodeId>,
    node_to_object: HashMap<NodeId, ObjectId>,
    tool_buttons: HashMap<ToolKind, NodeId>,
    editing: Option<EditingState>,
    drag: Option<DragState>,
    pan_drag: Option<PanDragState>,
    selection_box: Option<SelectionBoxState>,
}

impl UiState {
    fn new() -> Self {
        Self {
            doc: WhiteboardDoc::new(),
            history: History::new(512),
            tool: ToolKind::Select,
            default_style: ObjectStyle::default(),
            node_refs: NodeRefs::default(),
            object_nodes: HashMap::new(),
            node_to_object: HashMap::new(),
            tool_buttons: HashMap::new(),
            editing: None,
            drag: None,
            pan_drag: None,
            selection_box: None,
        }
    }

    fn apply_command(&mut self, command: Command) {
        let mut history = std::mem::replace(&mut self.history, History::new(512));
        history.apply(&mut self.doc, command);
        self.history = history;
    }
}

fn main() {
    App::run(|application| {
        application.app_context.create_window(WindowSpec::default(), app);
    });
}

fn app(cx: &Cx) -> NodeId {
    let state = Rc::new(RefCell::new(UiState::new()));

    let toolbar = build_toolbar(cx, &state);
    let board_panel = build_board_panel(cx, &state);

    let root = cx.div(|b| {
        b.style(style! {
            width: pct(100.0),
            height: pct(100.0),
            flex_direction: FlexDir::Row,
            background_color: rgb(0.96, 0.97, 0.99),
        });
        b.child_node(toolbar);
        b.child_node(board_panel);
    });

    {
        let mut st = state.borrow_mut();
        sync_tool_button_styles(&mut st);
        sync_canvas_camera(&st);
    }

    root
}

fn build_toolbar(cx: &Cx, state: &Rc<RefCell<UiState>>) -> NodeId {
    let toolbar = cx.div(|b| {
        b.style(style! {
            width: px(TOOLBAR_WIDTH),
            height: pct(100.0),
            flex_direction: FlexDir::Column,
            gap: 6.0,
            padding: 8.0,
            background_color: rgb(0.93, 0.95, 0.98),
            border_width: 1.0,
            border_color: rgb(0.82, 0.85, 0.90),
        });
    });

    for tool in [
        ToolKind::Select,
        ToolKind::Pan,
        ToolKind::Rect,
        ToolKind::Ellipse,
        ToolKind::Line,
        ToolKind::Arrow,
        ToolKind::Text,
        ToolKind::Image,
    ] {
        let s = Rc::clone(state);
        let button = cx.button(|b| {
            b.style(style! {
                width: px(48.0),
                height: px(30.0),
                border_radius: 6.0,
                border_width: 1.0,
                border_color: rgb(0.72, 0.76, 0.84),
                background_color: rgb(1.0, 1.0, 1.0),
                font_size: 12.0,
                color: rgb(0.20, 0.25, 0.35),
            });
            b.on_click(move || {
                let mut st = s.borrow_mut();
                st.tool = tool;
                sync_tool_button_styles(&mut st);
            });
            b.child_text(tool.label());
        });

        with_render_tree(|tree| tree.append_child(toolbar, button));
        state.borrow_mut().tool_buttons.insert(tool, button);
    }

    let swatches = [
        [0.98, 0.28, 0.28, 1.0],
        [0.98, 0.64, 0.18, 1.0],
        [0.98, 0.88, 0.18, 1.0],
        [0.22, 0.76, 0.38, 1.0],
        [0.22, 0.56, 0.96, 1.0],
        [0.56, 0.42, 0.95, 1.0],
        [0.92, 0.28, 0.72, 1.0],
        [0.15, 0.18, 0.24, 1.0],
    ];

    for color in swatches {
        let s = Rc::clone(state);
        let swatch = cx.button(|b| {
            b.style(style! {
                width: px(24.0),
                height: px(24.0),
                border_radius: 12.0,
                border_width: 1.0,
                border_color: rgb(0.66, 0.70, 0.80),
                background_color: rgba(color[0], color[1], color[2], color[3]),
            });
            b.on_click(move || {
                let mut st = s.borrow_mut();
                st.default_style.fill = color;
                if st.doc.selection.is_empty() {
                    return;
                }
                let ids: Vec<ObjectId> = st.doc.selection.clone();
                for id in ids {
                    if let Some(obj) = st.doc.objects.get_mut(&id) {
                        obj.style.fill = color;
                    }
                    sync_object_node(&st, id);
                }
            });
        });
        with_render_tree(|tree| tree.append_child(toolbar, swatch));
    }

    toolbar
}

fn build_board_panel(cx: &Cx, state: &Rc<RefCell<UiState>>) -> NodeId {
    let panel = cx.div(|b| {
        b.style(style! {
            width: pct(100.0),
            height: pct(100.0),
            flex_direction: FlexDir::Column,
            position: Position::Relative,
            overflow: Overflow::Hidden,
            background_color: rgb(0.99, 0.99, 1.0),
        });
    });

    let down_state = Rc::clone(state);
    let move_state = Rc::clone(state);
    let up_state = Rc::clone(state);
    let wheel_state = Rc::clone(state);

    let canvas = cx.canvas(|b| {
        b.style(style! {
            width: pct(100.0),
            height: pct(100.0),
            position: Position::Absolute,
            left: px(0.0),
            top: px(0.0),
            background_color: rgb(1.0, 1.0, 1.0),
        });
        b.on_pointer_down(move |ev| {
            on_canvas_pointer_down(&down_state, ev);
        });
        b.on_pointer_move(move |ev| {
            on_canvas_pointer_move(&move_state, ev);
        });
        b.on_pointer_up(move |ev| {
            on_canvas_pointer_up(&up_state, ev);
        });
        b.on_wheel(move |ev| {
            on_canvas_wheel(&wheel_state, ev);
        });
    });

    let overlay = cx.div(|b| {
        b.style(style! {
            width: pct(100.0),
            height: pct(100.0),
            position: Position::Absolute,
            left: px(0.0),
            top: px(0.0),
            overflow: Overflow::Hidden,
            background_color: rgba(0.0, 0.0, 0.0, 0.0),
        });
    });

    let selection_rect = cx.div(|b| {
        b.style(style! {
            display: Display::None,
            position: Position::Absolute,
            left: px(0.0),
            top: px(0.0),
            width: px(1.0),
            height: px(1.0),
            border_width: 1.0,
            border_color: rgb(0.20, 0.50, 0.95),
            background_color: rgba(0.20, 0.50, 0.95, 0.10),
        });
    });

    let submit_state = Rc::clone(state);
    let blur_state = Rc::clone(state);
    let input = cx.build(ElementTag::Input, |b| {
        b.style(style! {
            display: Display::None,
            position: Position::Absolute,
            left: px(0.0),
            top: px(0.0),
            width: px(200.0),
            height: px(34.0),
            background_color: rgb(1.0, 1.0, 1.0),
            border_width: 1.0,
            border_color: rgb(0.30, 0.52, 0.95),
            border_radius: 6.0,
            font_size: 16.0,
            color: rgb(0.10, 0.12, 0.18),
        });
        b.on_submit(move |text| {
            commit_text_edit(&submit_state, text);
        });
        b.on_blur(move || {
            let value = current_input_value(&blur_state).unwrap_or_default();
            commit_text_edit(&blur_state, value);
        });
    });

    with_render_tree(|tree| {
        tree.append_child(overlay, selection_rect);
        tree.append_child(panel, canvas);
        tree.append_child(panel, overlay);
        tree.append_child(panel, input);
    });

    {
        let mut st = state.borrow_mut();
        st.node_refs.canvas_id = Some(canvas);
        st.node_refs.overlay_id = Some(overlay);
        st.node_refs.input_id = Some(input);
        st.node_refs.selection_rect_id = Some(selection_rect);
    }

    panel
}

fn on_canvas_pointer_down(state: &Rc<RefCell<UiState>>, ev: CanvasPointerEvent) {
    let mut st = state.borrow_mut();
    let (world_x, world_y) = screen_to_world(&st, ev.x, ev.y);

    if st.tool == ToolKind::Pan || ev.meta {
        st.pan_drag = Some(PanDragState {
            start_screen: [ev.x, ev.y],
            start_pan: st.doc.camera.pan_world,
        });
        return;
    }

    let hit_object = hit_object_from_event(&st, ev.hit).or_else(|| st.doc.hit_test(world_x, world_y));

    match st.tool {
        ToolKind::Select => {
            if let Some(id) = hit_object {
                if ev.click_count >= 2 {
                    if let Some(obj) = st.doc.objects.get(&id) {
                        if obj.kind == ObjectKind::Text {
                            open_text_editor(&mut st, id);
                            return;
                        }
                    }
                }

                if !ev.shift {
                    st.doc.set_selection(vec![id]);
                } else if !st.doc.selection.contains(&id) {
                    let mut next = st.doc.selection.clone();
                    next.push(id);
                    st.doc.set_selection(next);
                }

                let mut initial_positions = HashMap::new();
                for sel_id in &st.doc.selection {
                    if let Some(obj) = st.doc.objects.get(sel_id) {
                        initial_positions.insert(*sel_id, [obj.x, obj.y]);
                    }
                }
                st.drag = Some(DragState {
                    ids: st.doc.selection.clone(),
                    start_world: [world_x, world_y],
                    initial_positions,
                });
                sync_selected_styles(&st);
            } else {
                if !ev.shift {
                    st.doc.clear_selection();
                    sync_selected_styles(&st);
                }
                st.selection_box = Some(SelectionBoxState {
                    start_world: [world_x, world_y],
                    current_world: [world_x, world_y],
                });
                sync_selection_rect(&st);
            }
        }
        ToolKind::Rect
        | ToolKind::Ellipse
        | ToolKind::Line
        | ToolKind::Arrow
        | ToolKind::Text
        | ToolKind::Image => {
            let tool = st.tool;
            let default_style = st.default_style;
            let object = make_object_from_tool(&mut st, tool, world_x, world_y, default_style);
            st.apply_command(Command::CreateObject { object: object.clone() });
            st.doc.set_selection(vec![object.id]);
            sync_all_nodes(&mut st);

            if tool == ToolKind::Text {
                open_text_editor(&mut st, object.id);
            }
        }
        ToolKind::Pan => {}
    }
}

fn on_canvas_pointer_move(state: &Rc<RefCell<UiState>>, ev: CanvasPointerEvent) {
    let mut st = state.borrow_mut();

    if let Some(pan_drag) = st.pan_drag.clone() {
        let dx = ev.x - pan_drag.start_screen[0];
        let dy = ev.y - pan_drag.start_screen[1];
        st.doc.camera.pan_world[0] = pan_drag.start_pan[0] - dx / st.doc.camera.zoom;
        st.doc.camera.pan_world[1] = pan_drag.start_pan[1] - dy / st.doc.camera.zoom;
        sync_camera_and_nodes(&mut st);
        return;
    }

    let (world_x, world_y) = screen_to_world(&st, ev.x, ev.y);

    if let Some(drag) = st.drag.clone() {
        let dx = world_x - drag.start_world[0];
        let dy = world_y - drag.start_world[1];

        for id in &drag.ids {
            if let Some(start_pos) = drag.initial_positions.get(id) {
                if let Some(obj) = st.doc.objects.get_mut(id) {
                    obj.x = start_pos[0] + dx;
                    obj.y = start_pos[1] + dy;
                }
            }
        }
        st.doc.rebuild_spatial_index();
        for id in &drag.ids {
            sync_object_node(&st, *id);
        }
        return;
    }

    if let Some(selection) = st.selection_box.as_mut() {
        selection.current_world = [world_x, world_y];
        let (x, y, w, h) = box_from_points(selection.start_world, selection.current_world);
        let ids = st.doc.query_aabb(x, y, w, h);
        st.doc.set_selection(ids);
        sync_selected_styles(&st);
        sync_selection_rect(&st);
    }
}

fn on_canvas_pointer_up(state: &Rc<RefCell<UiState>>, _ev: CanvasPointerEvent) {
    let mut st = state.borrow_mut();

    if let Some(drag) = st.drag.take() {
        if let Some(first_id) = drag.ids.first().copied() {
            let mut delta = [0.0, 0.0];
            if let Some(start_pos) = drag.initial_positions.get(&first_id) {
                if let Some(obj) = st.doc.objects.get(&first_id) {
                    delta = [obj.x - start_pos[0], obj.y - start_pos[1]];
                }
            }

            for id in &drag.ids {
                if let Some(start_pos) = drag.initial_positions.get(id) {
                    if let Some(obj) = st.doc.objects.get_mut(id) {
                        obj.x = start_pos[0];
                        obj.y = start_pos[1];
                    }
                }
            }
            st.doc.rebuild_spatial_index();

            if delta[0].abs() > 0.0001 || delta[1].abs() > 0.0001 {
                let command = Command::MoveObjects {
                        ids: drag.ids.clone(),
                        dx: delta[0],
                        dy: delta[1],
                };
                st.apply_command(command);
            }
            for id in &drag.ids {
                sync_object_node(&st, *id);
            }
        }
    }

    st.pan_drag = None;
    st.selection_box = None;
    sync_selection_rect(&st);
}

fn on_canvas_wheel(state: &Rc<RefCell<UiState>>, ev: CanvasWheelEvent) {
    let mut st = state.borrow_mut();

    if ev.ctrl || ev.meta {
        let (world_before_x, world_before_y) = screen_to_world(&st, ev.x, ev.y);
        let zoom_factor = (1.0 - ev.delta_y * 0.0015).clamp(0.5, 1.5);
        st.doc.camera.zoom = (st.doc.camera.zoom * zoom_factor).clamp(0.1, 8.0);

        let canvas_rect = canvas_rect(&st).unwrap_or_default();
        let local_x = ev.x - canvas_rect.x;
        let local_y = ev.y - canvas_rect.y;

        st.doc.camera.pan_world[0] = world_before_x - local_x / st.doc.camera.zoom;
        st.doc.camera.pan_world[1] = world_before_y - local_y / st.doc.camera.zoom;
    } else {
        st.doc.camera.pan_world[1] += ev.delta_y / st.doc.camera.zoom;
        st.doc.camera.pan_world[0] += ev.delta_x / st.doc.camera.zoom;
    }

    sync_camera_and_nodes(&mut st);
}

fn make_object_from_tool(
    st: &mut UiState,
    tool: ToolKind,
    x: f32,
    y: f32,
    style: ObjectStyle,
) -> WhiteboardObject {
    let id = st.doc.alloc_id();
    match tool {
        ToolKind::Rect => WhiteboardObject {
            id,
            kind: ObjectKind::Rect,
            x,
            y,
            width: 180.0,
            height: 120.0,
            rotation: 0.0,
            text: String::new(),
            image_src: None,
            style,
        },
        ToolKind::Ellipse => WhiteboardObject {
            id,
            kind: ObjectKind::Ellipse,
            x,
            y,
            width: 180.0,
            height: 120.0,
            rotation: 0.0,
            text: String::new(),
            image_src: None,
            style,
        },
        ToolKind::Line => WhiteboardObject {
            id,
            kind: ObjectKind::Line,
            x,
            y,
            width: 180.0,
            height: 2.0,
            rotation: 0.0,
            text: String::new(),
            image_src: None,
            style,
        },
        ToolKind::Arrow => WhiteboardObject {
            id,
            kind: ObjectKind::Arrow,
            x,
            y,
            width: 200.0,
            height: 2.0,
            rotation: 0.0,
            text: "→".to_string(),
            image_src: None,
            style,
        },
        ToolKind::Text => WhiteboardObject {
            id,
            kind: ObjectKind::Text,
            x,
            y,
            width: 220.0,
            height: 34.0,
            rotation: 0.0,
            text: "Text".to_string(),
            image_src: None,
            style,
        },
        ToolKind::Image => WhiteboardObject {
            id,
            kind: ObjectKind::Image,
            x,
            y,
            width: 240.0,
            height: 140.0,
            rotation: 0.0,
            text: String::new(),
            image_src: Some("assets/hero.png".to_string()),
            style,
        },
        ToolKind::Select | ToolKind::Pan => WhiteboardObject {
            id,
            kind: ObjectKind::Rect,
            x,
            y,
            width: 120.0,
            height: 80.0,
            rotation: 0.0,
            text: String::new(),
            image_src: None,
            style,
        },
    }
}

fn sync_camera_and_nodes(st: &mut UiState) {
    sync_canvas_camera(st);
    let ids: Vec<ObjectId> = st.doc.z_order.clone();
    for id in ids {
        sync_object_node(st, id);
    }
    sync_selection_rect(st);
}

fn sync_canvas_camera(st: &UiState) {
    let Some(canvas_id) = st.node_refs.canvas_id else { return };
    with_render_tree(|tree| {
        tree.set_canvas_grid(canvas_id, true);
        tree.set_canvas_camera(canvas_id, st.doc.camera.zoom, st.doc.camera.pan_world);
        if let Some(canvas_state) = tree.get_canvas_state_mut(canvas_id) {
            canvas_state.base_world_step = 16.0;
            canvas_state.dot_radius_px = 1.0;
            canvas_state.target_screen_step_px = 24.0;
        }
        tree.mark_dirty(canvas_id, DirtyFlags::PAINT);
    });
}

fn sync_all_nodes(st: &mut UiState) {
    let existing: Vec<ObjectId> = st.object_nodes.keys().copied().collect();
    for id in existing {
        if st.doc.objects.contains_key(&id) {
            continue;
        }
        if let Some(node_id) = st.object_nodes.remove(&id) {
            st.node_to_object.remove(&node_id);
            with_render_tree(|tree| tree.remove_node(node_id));
        }
    }

    let ordered = st.doc.z_order.clone();
    for id in ordered {
        ensure_object_node(st, id);
        sync_object_node(st, id);
    }

    sync_selected_styles(st);
}

fn ensure_object_node(st: &mut UiState, id: ObjectId) {
    if st.object_nodes.contains_key(&id) {
        return;
    }
    let Some(overlay_id) = st.node_refs.overlay_id else { return };
    let Some(object) = st.doc.objects.get(&id).cloned() else { return };

    let node_id = with_render_tree(|tree| {
        let node_tag = if object.kind == ObjectKind::Image {
            ElementTag::Image
        } else {
            ElementTag::Div
        };
        let node_id = tree.create_node(node_tag);
        tree.append_child(overlay_id, node_id);
        node_id
    });

    st.object_nodes.insert(id, node_id);
    st.node_to_object.insert(node_id, id);
}

fn sync_object_node(st: &UiState, id: ObjectId) {
    let Some(node_id) = st.object_nodes.get(&id).copied() else { return };
    let Some(object) = st.doc.objects.get(&id) else { return };
    let Some(canvas_rect) = canvas_rect(st) else { return };

    let zoom = st.doc.camera.zoom;
    let local_x = (object.x - st.doc.camera.pan_world[0]) * zoom;
    let local_y = (object.y - st.doc.camera.pan_world[1]) * zoom;
    let width = (object.width * zoom).max(1.0);
    let height = (object.height * zoom).max(1.0);

    let selected = st.doc.selection.contains(&id);
    let mut border_color = object.style.stroke;
    let mut border_width = object.style.stroke_width.max(1.0);
    if selected {
        border_color = [0.17, 0.43, 0.95, 1.0];
        border_width = 2.0;
    }

    with_render_tree(|tree| {
        let mut style = Style::new()
            .position(Position::Absolute)
            .left(px(local_x))
            .top(px(local_y))
            .width(px(width))
            .height(px(height))
            .border_width(border_width)
            .border_color(rgba(
                border_color[0],
                border_color[1],
                border_color[2],
                border_color[3],
            ))
            .background_color(rgba(
                object.style.fill[0],
                object.style.fill[1],
                object.style.fill[2],
                object.style.fill[3] * object.style.opacity,
            ));

        match object.kind {
            ObjectKind::Ellipse => {
                style = style.border_radius((height * 0.5).max(8.0));
            }
            ObjectKind::Line | ObjectKind::Arrow => {
                style = style.height(px((2.0 * zoom).max(1.0))).border_width(0.0);
            }
            ObjectKind::Text => {
                style = style
                    .background_color(rgba(0.0, 0.0, 0.0, 0.0))
                    .border_width(if selected { 1.0 } else { 0.0 })
                    .color(rgb(0.12, 0.14, 0.20))
                    .font_size((16.0 * zoom.clamp(0.75, 1.75)).max(12.0));
            }
            ObjectKind::Image => {
                style = style.border_radius(8.0);
            }
            _ => {
                style = style.border_radius(6.0);
            }
        }

        tree.apply_style(node_id, style);
        match object.kind {
            ObjectKind::Text | ObjectKind::Arrow => {
                tree.set_text(node_id, object.text.clone());
            }
            ObjectKind::Image => {
                if let Some(src) = &object.image_src {
                    tree.set_image_source(node_id, src.clone());
                }
                tree.set_text(node_id, String::new());
            }
            _ => {
                tree.set_text(node_id, String::new());
            }
        }
        tree.mark_dirty(node_id, DirtyFlags::PAINT | DirtyFlags::LAYOUT);
        let _ = canvas_rect;
    });
}

fn sync_selected_styles(st: &UiState) {
    let ids: Vec<ObjectId> = st.doc.z_order.clone();
    for id in ids {
        sync_object_node(st, id);
    }
}

fn open_text_editor(st: &mut UiState, id: ObjectId) {
    let Some(input_id) = st.node_refs.input_id else { return };
    let Some(object) = st.doc.objects.get(&id).cloned() else { return };
    let Some(canvas_rect) = canvas_rect(st) else { return };

    let zoom = st.doc.camera.zoom;
    let local_x = (object.x - st.doc.camera.pan_world[0]) * zoom;
    let local_y = (object.y - st.doc.camera.pan_world[1]) * zoom;
    let width = (object.width * zoom).max(120.0);
    let height = (object.height * zoom).max(28.0);

    st.editing = Some(EditingState {
        object_id: id,
        before_text: object.text.clone(),
    });

    with_render_tree(|tree| {
        tree.apply_style(
            input_id,
            Style::new()
                .display(Display::Flex)
                .position(Position::Absolute)
                .left(px(local_x))
                .top(px(local_y))
                .width(px(width))
                .height(px(height))
                .background_color(rgb(1.0, 1.0, 1.0))
                .border_width(1.0)
                .border_color(rgb(0.20, 0.50, 0.95))
                .border_radius(6.0)
                .font_size((16.0 * zoom.clamp(0.8, 1.4)).max(12.0))
                .color(rgb(0.10, 0.12, 0.20)),
        );
        tree.set_input_value(input_id, object.text.clone());
        tree.set_placeholder(input_id, "Type text...".to_string());
        tree.set_focused_input(input_id);
        if let Some(input) = tree.get_input_state_mut(input_id) {
            input.cursor = input.value.len();
            input.selection_anchor = input.cursor;
            input.preedit.clear();
            input.preedit_range = None;
            input.blink_visible = true;
            input.next_blink_at = Instant::now() + std::time::Duration::from_millis(530);
        }
        tree.mark_dirty(input_id, DirtyFlags::PAINT | DirtyFlags::LAYOUT);
        let _ = canvas_rect;
    });
}

fn commit_text_edit(state: &Rc<RefCell<UiState>>, text: String) {
    let mut st = state.borrow_mut();
    let Some(editing) = st.editing.take() else { return };

    if let Some(obj) = st.doc.objects.get(&editing.object_id) {
        if obj.text != editing.before_text {
            // ignore stale edit state and continue with latest object baseline.
        }
    }

    let command = Command::UpdateText {
            id: editing.object_id,
            before: editing.before_text,
            after: text,
    };
    st.apply_command(command);

    if let Some(input_id) = st.node_refs.input_id {
        with_render_tree(|tree| {
            tree.apply_style(input_id, Style::new().display(Display::None));
            tree.clear_focused_input();
            tree.mark_dirty(input_id, DirtyFlags::PAINT | DirtyFlags::LAYOUT);
        });
    }

    sync_object_node(&st, editing.object_id);
}

fn current_input_value(state: &Rc<RefCell<UiState>>) -> Option<String> {
    let st = state.borrow();
    let input_id = st.node_refs.input_id?;
    with_render_tree(|tree| tree.get_input_value(input_id).map(|v| v.to_string()))
}

fn sync_selection_rect(st: &UiState) {
    let Some(rect_id) = st.node_refs.selection_rect_id else { return };
    let Some(canvas_rect) = canvas_rect(st) else { return };

    with_render_tree(|tree| {
        if let Some(sel) = &st.selection_box {
            let zoom = st.doc.camera.zoom;
            let (x, y, w, h) = box_from_points(sel.start_world, sel.current_world);
            let local_x = (x - st.doc.camera.pan_world[0]) * zoom;
            let local_y = (y - st.doc.camera.pan_world[1]) * zoom;
            let local_w = (w * zoom).max(1.0);
            let local_h = (h * zoom).max(1.0);
            tree.apply_style(
                rect_id,
                Style::new()
                    .display(Display::Flex)
                    .position(Position::Absolute)
                    .left(px(local_x))
                    .top(px(local_y))
                    .width(px(local_w))
                    .height(px(local_h))
                    .border_width(1.0)
                    .border_color(rgb(0.20, 0.50, 0.95))
                    .background_color(rgba(0.20, 0.50, 0.95, 0.10)),
            );
        } else {
            tree.apply_style(rect_id, Style::new().display(Display::None));
        }
        tree.mark_dirty(rect_id, DirtyFlags::PAINT | DirtyFlags::LAYOUT);
        let _ = canvas_rect;
    });
}

fn sync_tool_button_styles(st: &mut UiState) {
    let entries: Vec<(ToolKind, NodeId)> = st.tool_buttons.iter().map(|(k, v)| (*k, *v)).collect();
    with_render_tree(|tree| {
        for (tool, node_id) in entries {
            let active = tool == st.tool;
            let style = if active {
                Style::new()
                    .width(px(48.0))
                    .height(px(30.0))
                    .border_radius(6.0)
                    .border_width(1.0)
                    .border_color(rgb(0.15, 0.44, 0.95))
                    .background_color(rgb(0.85, 0.92, 1.0))
                    .font_size(12.0)
                    .color(rgb(0.10, 0.18, 0.34))
            } else {
                Style::new()
                    .width(px(48.0))
                    .height(px(30.0))
                    .border_radius(6.0)
                    .border_width(1.0)
                    .border_color(rgb(0.72, 0.76, 0.84))
                    .background_color(rgb(1.0, 1.0, 1.0))
                    .font_size(12.0)
                    .color(rgb(0.20, 0.25, 0.35))
            };
            tree.apply_style(node_id, style);
            tree.mark_dirty(node_id, DirtyFlags::PAINT | DirtyFlags::LAYOUT);
        }
    });
}

fn hit_object_from_event(st: &UiState, hit: Option<NodeId>) -> Option<ObjectId> {
    let mut current = hit;
    while let Some(node_id) = current {
        if let Some(id) = st.node_to_object.get(&node_id) {
            return Some(*id);
        }
        current = with_render_tree(|tree| tree.get(node_id).and_then(|n| n.parent));
    }
    None
}

fn canvas_rect(st: &UiState) -> Option<LayoutRect> {
    let canvas_id = st.node_refs.canvas_id?;
    with_render_tree(|tree| tree.get_layout_rect(canvas_id))
}

fn screen_to_world(st: &UiState, sx: f32, sy: f32) -> (f32, f32) {
    let rect = canvas_rect(st).unwrap_or_default();
    let local_x = sx - rect.x;
    let local_y = sy - rect.y;
    (
        local_x / st.doc.camera.zoom + st.doc.camera.pan_world[0],
        local_y / st.doc.camera.zoom + st.doc.camera.pan_world[1],
    )
}

fn box_from_points(a: [f32; 2], b: [f32; 2]) -> (f32, f32, f32, f32) {
    let x = a[0].min(b[0]);
    let y = a[1].min(b[1]);
    let w = (a[0] - b[0]).abs();
    let h = (a[1] - b[1]).abs();
    (x, y, w, h)
}
