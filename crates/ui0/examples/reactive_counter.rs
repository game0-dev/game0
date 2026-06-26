use ui0::{
    button, div, signal, text, AlignItems, AppCx, Application, Color, Element, JustifyContent,
    WindowDesc, WindowHandle,
};

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

    let count_text = count.clone();
    let increment = count.clone();
    let decrement = count.clone();

    div()
        .w(420.0)
        .h(240.0)
        .padding(24.0)
        .gap(28.0)
        .column()
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center)
        .bg(Color::rgb_u8(28, 31, 38))
        .border_color(Color::rgb_u8(72, 81, 96))
        .border_width(1.0)
        .radius(8.0)
        .child(
            text("Counter")
                .font_size(28.0)
                .text_color(Color::rgb_u8(245, 247, 250)),
        )
        .child(
            div()
                .w(300.0)
                .h(64.0)
                .gap(18.0)
                .row()
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::Center)
                .child(counter_button("-").on_click(move |_| {
                    decrement.update(|value| *value -= 1);
                }))
                .child(
                    text(move || count_text.get().to_string())
                        .w(110.0)
                        .h(56.0)
                        .font_size(36.0)
                        .text_color(Color::rgb_u8(245, 247, 250)),
                )
                .child(counter_button("+").on_click(move |_| {
                    increment.update(|value| *value += 1);
                })),
        )
}

fn counter_button(label: &'static str) -> Element {
    button()
        .w(64.0)
        .h(56.0)
        .row()
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center)
        .bg(Color::rgb_u8(66, 82, 110))
        .border_color(Color::rgb_u8(112, 129, 158))
        .border_width(1.0)
        .radius(8.0)
        .child(
            text(label)
                .font_size(30.0)
                .text_color(Color::rgb_u8(255, 255, 255)),
        )
}

fn main() -> ui0::Result<()> {
    ui0::run(ReactiveCounterApp::default())
}
