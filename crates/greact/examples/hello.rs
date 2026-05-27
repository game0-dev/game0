//! Window MVP demo: click counter + scroll container.
//!
//! Run with:
//!   cargo run -p greact --example hello

use greact::*;

#[derive(Default)]
struct CounterProps {
    init_value: i32,
}

fn counter(cx: &Cx, props: CounterProps) -> NodeId {
    let (count, set_count) = cx.create_signal(props.init_value);

    view! { cx,
        <div style={style! {
            padding: 10.0,
            gap: 8.0,
            width: pct(100.0),
            background_color: rgb(0.95, 0.95, 0.95),
            border_radius: 6.0,
            border_width: 1.0,
            border_color: rgb(0.82, 0.82, 0.82),
        }}>
            {move || format!("Count: {}", count.get())}
            <button on_click={move || set_count.update(|n| *n += 1)} style={style! {
                padding: 8.0,
                background_color: rgb(0.18, 0.47, 0.95),
                border_radius: 10.0,
                border_width: 1.0,
                border_color: rgb(0.12, 0.35, 0.80),
                box_shadow: BoxShadow {
                    offset_x: 0.0,
                    offset_y: 4.0,
                    blur: 8.0,
                    spread: 0.0,
                    color: rgba(0.0, 0.0, 0.0, 0.28),
                },
                color: rgb(1.0, 1.0, 1.0),
            }}>
                "Increment"
            </button>
            <button on_click={move || set_count.update(|n| *n -= 1)} style={style! {
                padding: 8.0,
                background_color: rgb(0.84, 0.20, 0.20),
                border_radius: 10.0,
                border_width: 1.0,
                border_color: rgb(0.70, 0.16, 0.16),
                box_shadow: BoxShadow {
                    offset_x: 0.0,
                    offset_y: 4.0,
                    blur: 8.0,
                    spread: 0.0,
                    color: rgba(0.0, 0.0, 0.0, 0.26),
                },
                color: rgb(1.0, 1.0, 1.0),
            }}>
                "Decrement"
            </button>
            <button on_click={move || batch(|| {
                set_count.update(|n| *n += 10);
                set_count.update(|n| *n -= 3);
            })} style={style! {
                padding: 8.0,
                background_color: rgb(0.18, 0.68, 0.48),
                border_radius: 10.0,
                border_width: 1.0,
                border_color: rgb(0.12, 0.52, 0.36),
                color: rgb(1.0, 1.0, 1.0),
            }}>
                "Batch +7"
            </button>
        </div>
    }
}

fn icon_row(cx: &Cx) -> NodeId {
    view! { cx,
        <div style={style! {
            width: pct(100.0),
            padding: 10.0,
            gap: 10.0,
            align_items: Align::Center,
            background_color: rgb(0.98, 0.99, 1.0),
            border_width: 1.0,
            border_color: rgb(0.84, 0.89, 0.96),
            border_radius: 8.0,
        }}>
            <icon icon={IconName::Search} style={style! {
                font_size: 20.0,
                color: rgb(0.16, 0.40, 0.96),
            }} />
            <div style={style! { font_size: 16.0 }}>
                "Search"
            </div>
            <icon icon={IconName::Warning} style={style! {
                font_size: 20.0,
                color: rgb(0.90, 0.46, 0.06),
            }} />
            <div style={style! { font_size: 16.0 }}>
                "Warning"
            </div>
            <icon icon={IconName::Check} style={style! {
                font_size: 20.0,
                color: rgb(0.12, 0.65, 0.36),
            }} />
            <div style={style! { font_size: 16.0 }}>
                "Done"
            </div>
        </div>
    }
}

fn image_panel(cx: &Cx) -> NodeId {
    view! { cx,
        <div style={style! {
            width: pct(100.0),
            flex_direction: FlexDir::Column,
            gap: 8.0,
            padding: 10.0,
            background_color: rgb(0.97, 0.98, 1.0),
            border_width: 1.0,
            border_color: rgb(0.84, 0.88, 0.95),
            border_radius: 8.0,
        }}>
            <div style={style! { font_size: 16.0 }}>
                "Image (placeholder path source)"
            </div>
            <image src={"assets/hero.png"} style={style! {
                width: px(280.0),
                height: px(140.0),
                border_radius: 10.0,
            }} />
        </div>
    }
}

fn scroll_panel(cx: &Cx) -> NodeId {
    view! { cx,
        <div style={style! {
            width: pct(100.0),
            height: px(220.0),
            flex_direction: FlexDir::Column,
            padding: 8.0,
            gap: 6.0,
            overflow: Overflow::Scroll,
            background_color: rgb(0.97, 0.97, 0.99),
            border_width: 1.0,
            border_color: rgb(0.84, 0.84, 0.9),
            border_radius: 6.0,
        }}>
            {for i in 0..20 {
                <div style={style! {
                    width: pct(100.0),
                    padding: 6.0,
                    background_color: if i % 2 == 0 { rgb(1.0, 1.0, 1.0) } else { rgb(0.94, 0.96, 1.0) },
                    border_radius: 4.0,
                }}>
                    {format!("Scrollable item #{i}")}
                </div>
            }}
        </div>
    }
}

fn app(cx: &Cx) -> NodeId {
    view! { cx,
        <div style={style! {
            width: pct(100.0),
            height: pct(100.0),
            flex_direction: FlexDir::Column,
            gap: 12.0,
            padding: 20.0,
            background_color: rgb(0.92, 0.94, 0.97),
        }}>
            <div style={style! { padding: 4.0, font_size: 22.0 }}>
                "Hello, greact window MVP"
            </div>
            <Counter init_value={3} />
            <IconRow />
            <ImagePanel />
            <ScrollPanel />
        </div>
    }
}

fn main() {
    App::run(|application| {
        application.app_context.create_window(WindowSpec::default(), app);
    });
}
