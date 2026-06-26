use ui0::{
    div, text, AlignItems, AppCx, Application, Color, Element, JustifyContent, WindowDesc,
    WindowHandle,
};

#[derive(Default)]
struct RenderGalleryApp {
    main: Option<WindowHandle<Self>>,
}

impl Application for RenderGalleryApp {
    fn handle_init(&mut self, cx: &mut AppCx<Self>) {
        self.main = Some(cx.open_window(
            WindowDesc::new("ui0 render gallery").size(900, 640),
            |window| {
                window.mount(render_gallery);
            },
        ));
    }
}

fn render_gallery() -> Element {
    div()
        .w(900.0)
        .h(640.0)
        .padding(28.0)
        .gap(20.0)
        .column()
        .bg(Color::rgb_u8(18, 20, 25))
        .child(
            text("Render Gallery")
                .font_size(30.0)
                .text_color(Color::rgb_u8(245, 247, 250)),
        )
        .child(
            div()
                .row()
                .gap(18.0)
                .child(radius_card("Radius 0", 0.0, Color::rgb_u8(67, 92, 134)))
                .child(radius_card("Radius 4", 4.0, Color::rgb_u8(71, 112, 101)))
                .child(radius_card("Radius 8", 8.0, Color::rgb_u8(112, 88, 142)))
                .child(radius_card("Radius 16", 16.0, Color::rgb_u8(142, 92, 78))),
        )
        .child(
            div()
                .row()
                .gap(18.0)
                .child(border_card("1px Border", 1.0))
                .child(border_card("2px Border", 2.0))
                .child(border_card("4px Border", 4.0)),
        )
        .child(
            div()
                .row()
                .gap(18.0)
                .child(text_sample("Small text", 13.0))
                .child(text_sample("Medium text", 18.0))
                .child(text_sample("Large text", 28.0)),
        )
}

fn radius_card(label: &'static str, radius: f32, color: Color) -> Element {
    div()
        .w(190.0)
        .h(112.0)
        .padding(14.0)
        .column()
        .gap(10.0)
        .bg(color)
        .border_color(Color::rgb_u8(170, 180, 196))
        .border_width(1.0)
        .radius(radius)
        .child(
            text(label)
                .font_size(16.0)
                .text_color(Color::rgb_u8(255, 255, 255)),
        )
        .child(
            text("SDF AA")
                .font_size(13.0)
                .text_color(Color::rgb_u8(218, 224, 235)),
        )
}

fn border_card(label: &'static str, width: f32) -> Element {
    div()
        .w(250.0)
        .h(86.0)
        .padding(16.0)
        .row()
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center)
        .bg(Color::rgb_u8(34, 38, 48))
        .border_color(Color::rgb_u8(120, 146, 190))
        .border_width(width)
        .radius(10.0)
        .child(
            text(label)
                .font_size(17.0)
                .text_color(Color::rgb_u8(238, 242, 248)),
        )
}

fn text_sample(label: &'static str, size: f32) -> Element {
    div()
        .w(250.0)
        .h(110.0)
        .padding(14.0)
        .column()
        .gap(8.0)
        .bg(Color::rgb_u8(27, 31, 40))
        .border_color(Color::rgb_u8(58, 66, 82))
        .border_width(1.0)
        .radius(8.0)
        .child(
            text(label)
                .font_size(size)
                .text_color(Color::rgb_u8(245, 247, 250)),
        )
        .child(
            text("glyphon / cosmic-text")
                .font_size(13.0)
                .text_color(Color::rgb_u8(168, 178, 196)),
        )
}

fn main() -> ui0::Result<()> {
    ui0::run(RenderGalleryApp::default())
}
