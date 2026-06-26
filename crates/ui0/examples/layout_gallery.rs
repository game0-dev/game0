use ui0::{
    div, text, AlignItems, AppCx, Application, Color, Element, JustifyContent, WindowDesc,
    WindowHandle,
};

#[derive(Default)]
struct LayoutGalleryApp {
    main: Option<WindowHandle<Self>>,
}

impl Application for LayoutGalleryApp {
    fn handle_init(&mut self, cx: &mut AppCx<Self>) {
        self.main = Some(cx.open_window(
            WindowDesc::new("ui0 layout gallery").size(920, 620),
            |window| {
                window.mount(layout_gallery);
            },
        ));
    }
}

fn layout_gallery() -> Element {
    div()
        .w(920.0)
        .h(620.0)
        .padding(26.0)
        .gap(18.0)
        .column()
        .bg(Color::rgb_u8(17, 19, 24))
        .child(
            text("Layout Gallery")
                .font_size(30.0)
                .text_color(Color::rgb_u8(245, 247, 250)),
        )
        .child(
            div()
                .row()
                .gap(18.0)
                .child(stack_panel())
                .child(row_panel())
                .child(nested_panel()),
        )
}

fn stack_panel() -> Element {
    section("Column Stack")
        .child(tile("Inspector", Color::rgb_u8(65, 83, 118), 230.0, 56.0))
        .child(tile("Properties", Color::rgb_u8(43, 66, 86), 230.0, 88.0))
        .child(tile("Console", Color::rgb_u8(74, 58, 89), 230.0, 72.0))
}

fn row_panel() -> Element {
    section("Toolbar Row").child(
        div()
            .row()
            .gap(10.0)
            .align_items(AlignItems::Center)
            .child(tool("Move"))
            .child(tool("Rotate"))
            .child(tool("Scale"))
            .child(tool("Play")),
    )
}

fn nested_panel() -> Element {
    section("Nested Panels")
        .child(
            div()
                .w(250.0)
                .h(92.0)
                .padding(10.0)
                .gap(10.0)
                .row()
                .bg(Color::rgb_u8(31, 35, 44))
                .radius(8.0)
                .child(tile("A", Color::rgb_u8(75, 99, 140), 90.0, 68.0))
                .child(tile("B", Color::rgb_u8(100, 85, 132), 120.0, 68.0)),
        )
        .child(
            div()
                .w(250.0)
                .h(120.0)
                .padding(10.0)
                .column()
                .gap(8.0)
                .bg(Color::rgb_u8(31, 35, 44))
                .radius(8.0)
                .child(tile("Header", Color::rgb_u8(58, 74, 101), 230.0, 40.0))
                .child(tile("Body", Color::rgb_u8(43, 69, 66), 230.0, 52.0)),
        )
}

fn section(title: &'static str) -> Element {
    div()
        .w(280.0)
        .h(430.0)
        .padding(16.0)
        .gap(12.0)
        .column()
        .bg(Color::rgb_u8(24, 27, 34))
        .border_color(Color::rgb_u8(55, 63, 78))
        .border_width(1.0)
        .radius(10.0)
        .child(
            text(title)
                .font_size(18.0)
                .text_color(Color::rgb_u8(237, 241, 248)),
        )
}

fn tile(label: &'static str, color: Color, width: f32, height: f32) -> Element {
    div()
        .w(width)
        .h(height)
        .row()
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center)
        .bg(color)
        .radius(7.0)
        .child(
            text(label)
                .font_size(15.0)
                .text_color(Color::rgb_u8(255, 255, 255)),
        )
}

fn tool(label: &'static str) -> Element {
    div()
        .w(58.0)
        .h(42.0)
        .row()
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center)
        .bg(Color::rgb_u8(52, 61, 78))
        .border_color(Color::rgb_u8(78, 92, 116))
        .border_width(1.0)
        .radius(6.0)
        .child(
            text(label)
                .font_size(12.0)
                .text_color(Color::rgb_u8(238, 242, 248)),
        )
}

fn main() -> ui0::Result<()> {
    ui0::run(LayoutGalleryApp::default())
}
