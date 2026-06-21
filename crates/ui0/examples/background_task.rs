use ui0::{AppCx, AppEvent, Application, WindowDesc, WindowHandle};

#[derive(Default)]
struct BackgroundTaskApp {
    main: Option<WindowHandle<Self>>,
    title: Option<String>,
}

impl Application for BackgroundTaskApp {
    fn handle_init(&mut self, cx: &mut AppCx<Self>) {
        let main = cx.open_window(WindowDesc::new("ui0 loading").size(640, 420), |_| {});
        self.main = Some(main.clone());

        cx.spawn_blocking(
            || "computed on background thread".to_string(),
            move |app, _cx, title| {
                app.title = Some(title.clone());
                main.run_on_ui(move |_app, window| {
                    window.set_title(&title);
                });
            },
        );
    }

    fn handle_event(&mut self, _cx: &mut AppCx<Self>, event: AppEvent) {
        if let AppEvent::WindowRedrawRequested(_window) = event {
            // Rendering is intentionally not part of this minimal app/window layer.
        }
    }
}

fn main() -> ui0::Result<()> {
    ui0::run(BackgroundTaskApp::default())
}
