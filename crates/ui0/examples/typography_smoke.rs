use ui0::{
    div, text, AlignItems, AppCx, Application, Color, Element, JustifyContent, WindowDesc,
    WindowHandle,
};

#[derive(Default)]
struct TypographySmokeApp {
    main: Option<WindowHandle<Self>>,
}

impl Application for TypographySmokeApp {
    fn handle_init(&mut self, cx: &mut AppCx<Self>) {
        self.main = Some(cx.open_window(
            WindowDesc::new("ui0 typography smoke").size(820, 560),
            |window| {
                window.mount(typography_smoke);
            },
        ));
    }
}

fn typography_smoke() -> Element {
    div()
        .w(820.0)
        .h(560.0)
        .padding(30.0)
        .gap(18.0)
        .column()
        .bg(Color::rgb_u8(17, 19, 24))
        .child(
            text("Typography Smoke")
                .font_size(30.0)
                .text_color(Color::rgb_u8(245, 247, 250)),
        )
        .child(sample(
            "Latin",
            "The quick brown fox jumps over the lazy dog.",
            18.0,
        ))
        .child(sample("Numbers", "0123456789  -12.5  100%  x/y/z", 22.0))
        .child(sample(
            "CJK",
            "中文排版测试：界面、按钮、计数器、渲染。",
            21.0,
        ))
        .child(sample("Emoji", "Status: ✅  Warning: ⚠️  Rocket: 🚀", 21.0))
        .child(
            div()
                .w(720.0)
                .h(86.0)
                .padding(16.0)
                .row()
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::Center)
                .bg(Color::rgb_u8(32, 37, 47))
                .border_color(Color::rgb_u8(64, 74, 92))
                .border_width(1.0)
                .radius(10.0)
                .child(
                    text("Large glyphon text on a rounded panel")
                        .font_size(34.0)
                        .text_color(Color::rgb_u8(237, 241, 248)),
                ),
        )
}

fn sample(label: &'static str, value: &'static str, size: f32) -> Element {
    div()
        .w(720.0)
        .h(66.0)
        .padding(14.0)
        .row()
        .gap(18.0)
        .align_items(AlignItems::Center)
        .bg(Color::rgb_u8(26, 30, 38))
        .border_color(Color::rgb_u8(52, 61, 76))
        .border_width(1.0)
        .radius(8.0)
        .child(
            text(label)
                .w(110.0)
                .font_size(14.0)
                .text_color(Color::rgb_u8(150, 162, 184)),
        )
        .child(
            text(value)
                .font_size(size)
                .text_color(Color::rgb_u8(238, 242, 248)),
        )
}

fn main() -> ui0::Result<()> {
    ui0::run(TypographySmokeApp::default())
}
