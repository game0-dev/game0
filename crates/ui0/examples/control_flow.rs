use std::collections::HashMap;
use std::rc::Rc;

use ui0::{
    button, div, for_each, show, signal, text, AppCx, Application, Element, Signal, WindowDesc,
    WindowHandle,
};

#[derive(Default)]
struct ControlFlowApp {
    main: Option<WindowHandle<Self>>,
}

impl Application for ControlFlowApp {
    fn handle_init(&mut self, cx: &mut AppCx<Self>) {
        self.main = Some(cx.open_window(
            WindowDesc::new("ui0 control flow").size(720, 480),
            |window| {
                window.mount(control_flow_view);
            },
        ));
    }
}

fn control_flow_view() -> Element {
    let expanded = signal(true);
    let compact = signal(false);
    let task_order = signal(vec![1_u32, 2, 3]);
    let task_done = Rc::new(HashMap::from([
        (1_u32, signal(false)),
        (2_u32, signal(true)),
        (3_u32, signal(false)),
    ]));

    let show_expanded = expanded.clone();
    let toggle_expanded = expanded.clone();
    let dynamic_compact = compact.clone();
    let toggle_compact = compact.clone();
    let list_source = task_order.clone();
    let reverse_tasks = task_order.clone();
    let remove_first_task = task_order.clone();
    let done_by_id = Rc::clone(&task_done);

    div()
        .w(460.0)
        .child("Control flow")
        .child(button().child("Toggle details").on_click(move |_| {
            toggle_expanded.update(|value| *value = !*value);
        }))
        .child(button().child("Toggle dynamic child").on_click(move |_| {
            toggle_compact.update(|value| *value = !*value);
        }))
        .child(
            show(move || show_expanded.get())
                .then(|| details_panel())
                .fallback(|| text("Details are hidden")),
        )
        .child_fn(move || {
            if dynamic_compact.get() {
                text("child_fn selected the compact branch")
            } else {
                div()
                    .child("child_fn selected the expanded branch")
                    .child(text("This whole subtree is replaced as one dynamic region"))
            }
        })
        .child(button().child("Reverse tasks").on_click(move |_| {
            reverse_tasks.update(|items| items.reverse());
        }))
        .child(button().child("Remove first task").on_click(move |_| {
            remove_first_task.update(|items| {
                if !items.is_empty() {
                    items.remove(0);
                }
            });
        }))
        .child(
            for_each(move || list_source.get())
                .key(|id| *id)
                .row(move |id| {
                    let done = done_by_id
                        .get(&id)
                        .cloned()
                        .expect("task id should have a row signal");
                    task_row(id, done)
                })
                .empty(|| text("No tasks")),
        )
}

fn details_panel() -> Element {
    div()
        .child("Details")
        .child(text("show() owns this branch until the condition changes"))
}

fn task_row(id: u32, done: Signal<bool>) -> Element {
    let done_text = done.clone();
    let toggle_done = done.clone();

    div()
        .child(text(move || {
            let status = if done_text.get() { "done" } else { "open" };
            format!("task {id}: {status}")
        }))
        .child(button().child("Toggle").on_click(move |_| {
            toggle_done.update(|value| *value = !*value);
        }))
}

fn main() -> ui0::Result<()> {
    ui0::run(ControlFlowApp::default())
}
