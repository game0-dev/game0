use ui0::{button, div, memo, signal, text, AppCx, Application, Element, WindowDesc, WindowHandle};

#[derive(Default)]
struct ReactiveCounterApp {
    main: Option<WindowHandle<Self>>,
}

impl Application for ReactiveCounterApp {
    fn handle_init(&mut self, cx: &mut AppCx<Self>) {
        self.main = Some(cx.open_window(
            WindowDesc::new("ui0 reactive counter").size(640, 420),
            |window| {
                window.mount(counter_view);
            },
        ));
    }
}

fn counter_view() -> Element {
    let count = signal(0);
    let doubled = {
        let count = count.clone();
        memo(move || count.get() * 2)
    };

    let count_text = count.clone();
    let doubled_text = doubled.clone();
    let increment = count.clone();
    let decrement = count.clone();
    let reset = count.clone();

    div()
        .w(360.0)
        .h(220.0)
        .child("Counter")
        .child(text(move || format!("value: {}", count_text.get())))
        .child(text(move || format!("doubled: {}", doubled_text.get())))
        .child(button().child("-").on_click(move |_| {
            decrement.update(|value| *value -= 1);
        }))
        .child(button().child("+").on_click(move |_| {
            increment.update(|value| *value += 1);
        }))
        .child(button().child("Reset").on_click(move |_| {
            reset.set(0);
        }))
}

fn main() -> ui0::Result<()> {
    ui0::run(ReactiveCounterApp::default())
}
