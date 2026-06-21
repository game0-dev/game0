use ui0::{AppCx, Application, WindowDesc, WindowHandle};

#[derive(Default)]
struct HelloApp {
    main: Option<WindowHandle<Self>>,
}

impl Application for HelloApp {
    fn handle_init(&mut self, cx: &mut AppCx<Self>) {
        self.main = Some(
            cx.open_window(WindowDesc::new("ui0 hello").size(640, 420), |window| {
                window.set_title("ui0 hello");
            }),
        );
    }
}

fn main() -> ui0::Result<()> {
    ui0::run(HelloApp::default())
}
