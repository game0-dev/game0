use greact::*;

fn app(cx: &Cx) -> NodeId {
    let (value, set_value) = cx.create_signal(String::new());
    let (submitted, set_submitted) = cx.create_signal(String::new());

    view! { cx,
        <div style={style! {
            width: pct(100.0),
            height: pct(100.0),
            flex_direction: FlexDir::Column,
            gap: 10.0,
            padding: 20.0,
            background_color: rgb(0.96, 0.97, 1.0),
        }}>
            <div style={style! { font_size: 22.0 }}>
                "Input IME"
            </div>
            <div style={style! { font_size: 14.0, color: rgb(0.28, 0.33, 0.45) }}>
                "Focus input and use IME. Preedit is inline with highlight. Enter to submit."
            </div>

            <input
                value={value.get()}
                placeholder={"Try Chinese/Japanese IME here"}
                on_input={move |v| set_value.set(v)}
                on_submit={move |v| set_submitted.set(v)}
                style={style! {
                    width: px(620.0),
                    height: px(42.0),
                    padding: 8.0,
                    background_color: rgb(1.0, 1.0, 1.0),
                    border_width: 1.0,
                    border_color: rgb(0.68, 0.74, 0.86),
                    border_radius: 9.0,
                    font_size: 17.0,
                    color: rgb(0.10, 0.12, 0.18),
                }}
            />

            <div style={style! { font_size: 14.0, color: rgb(0.20, 0.25, 0.36) }}>
                {move || format!("committed value: {}", value.get())}
            </div>
            <div style={style! { font_size: 14.0, color: rgb(0.10, 0.47, 0.28) }}>
                {move || format!("submitted: {}", submitted.get())}
            </div>
        </div>
    }
}

fn main() {
    App::run(|application| {
        application.app_context.create_window(WindowSpec::default(), app);
    });
}
