use std::sync::Arc;

use greact::*;

fn app(cx: &Cx) -> NodeId {
    let (value, set_value) = cx.create_signal(String::new());
    let (submitted, set_submitted) = cx.create_signal(String::new());

    view! { cx,
        <div style={style! {
            width: pct(100.0),
            height: pct(100.0),
            flex_direction: FlexDir::Column,
            gap: 12.0,
            padding: 20.0,
            background_color: rgb(0.95, 0.96, 0.98),
        }}>
            <div style={style! { font_size: 22.0 }}>
                "Input Basic"
            </div>

            <input
                value={value.get()}
                placeholder={"Type here (Ctrl/Cmd+A/C/X/V, Enter submit)"}
                on_input={move |v| set_value.set(v)}
                on_submit={move |v| set_submitted.set(v)}
                style={style! {
                    width: px(560.0),
                    height: px(40.0),
                    padding: 8.0,
                    background_color: rgb(1.0, 1.0, 1.0),
                    border_width: 1.0,
                    border_color: rgb(0.72, 0.75, 0.82),
                    border_radius: 8.0,
                    font_size: 16.0,
                    color: rgb(0.10, 0.12, 0.18),
                }}
            />

            <div style={style! { font_size: 15.0, color: rgb(0.2, 0.25, 0.35) }}>
                {move || format!("on_input value: {}", value.get())}
            </div>
            <div style={style! { font_size: 15.0, color: rgb(0.12, 0.44, 0.80) }}>
                {move || format!("on_submit value: {}", submitted.get())}
            </div>
            <button
                on_click={move || {
                    let Some(window_id) = current_window_id() else {
                        return;
                    };
                    let Some(app_context) = current_app_context() else {
                        return;
                    };

                    let io_context = Arc::clone(&app_context);
                    let ui_context = Arc::clone(&app_context);
                    io_context.spawn_io(async move {
                        ui_context.spawn_ui(window_id, move |_window| {
                            set_submitted.set("updated via spawn_io -> spawn_ui".to_string());
                        });
                    });
                }}
                style={style! {
                    width: px(280.0),
                    height: px(36.0),
                    border_width: 1.0,
                    border_color: rgb(0.25, 0.39, 0.72),
                    border_radius: 8.0,
                    background_color: rgb(0.17, 0.38, 0.88),
                    color: rgb(1.0, 1.0, 1.0),
                }}
            >
                "Trigger spawn_io -> spawn_ui"
            </button>
        </div>
    }
}

fn main() {
    App::run(|application| {
        application.app_context.create_window(WindowSpec::default(), app);
    });
}
