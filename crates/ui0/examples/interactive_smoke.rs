use ui0::{
    button, div, signal, text, AlignItems, AppCx, Application, Color, Element, JustifyContent,
    WindowDesc, WindowHandle,
};

#[derive(Default)]
struct InteractiveSmokeApp {
    main: Option<WindowHandle<Self>>,
}

impl Application for InteractiveSmokeApp {
    fn handle_init(&mut self, cx: &mut AppCx<Self>) {
        self.main = Some(cx.open_window(
            WindowDesc::new("ui0 interactive smoke").size(680, 460),
            |window| {
                window.mount(interactive_smoke);
            },
        ));
    }
}

fn interactive_smoke() -> Element {
    let count = signal(0);
    let hue = signal(false);

    let count_text = count.clone();
    let increment = count.clone();
    let decrement = count.clone();
    let accent = hue.clone();
    let toggle_accent = hue.clone();

    div()
        .w(680.0)
        .h(460.0)
        .padding(30.0)
        .column()
        .gap(24.0)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center)
        .bg(Color::rgb_u8(18, 20, 25))
        .child(
            text("Interactive Smoke")
                .font_size(28.0)
                .text_color(Color::rgb_u8(245, 247, 250)),
        )
        .child(
            div()
                .w(420.0)
                .h(180.0)
                .padding(24.0)
                .gap(22.0)
                .column()
                .align_items(AlignItems::Center)
                .bg(move || {
                    if accent.get() {
                        Color::rgb_u8(33, 48, 58)
                    } else {
                        Color::rgb_u8(30, 33, 42)
                    }
                })
                .border_color(Color::rgb_u8(72, 84, 104))
                .border_width(1.0)
                .radius(12.0)
                .child(
                    text(move || format!("count: {}", count_text.get()))
                        .font_size(34.0)
                        .text_color(Color::rgb_u8(255, 255, 255)),
                )
                .child(
                    div()
                        .row()
                        .gap(14.0)
                        .child(action_button("-").on_click(move |_| {
                            decrement.update(|value| *value -= 1);
                        }))
                        .child(action_button("+").on_click(move |_| {
                            increment.update(|value| *value += 1);
                        }))
                        .child(action_button("theme").on_click(move |_| {
                            toggle_accent.update(|value| *value = !*value);
                        })),
                ),
        )
}

fn action_button(label: &'static str) -> Element {
    button()
        .w(86.0)
        .h(44.0)
        .row()
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center)
        .bg(Color::rgb_u8(63, 80, 110))
        .border_color(Color::rgb_u8(111, 130, 164))
        .border_width(1.0)
        .radius(8.0)
        .child(
            text(label)
                .font_size(16.0)
                .text_color(Color::rgb_u8(255, 255, 255)),
        )
}

fn main() -> ui0::Result<()> {
    ui0::run(InteractiveSmokeApp::default())
}
