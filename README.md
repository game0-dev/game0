# game0

This workspace currently contains `ui0`, a first-stage native UI runtime.

## ui0 Contract

- One winit UI thread owns the event loop.
- `Application` is the user-owned app object.
- `AppRuntime` owns all `WindowRuntime` instances.
- `WindowRuntime` owns one native window.
- Background work runs on Tokio and returns to the UI thread through internal
  app messages.
- `AppHandle` and `WindowHandle` are cross-thread handles.
- `AppCx` is the app-scope context.
- `WindowCx` is the window-scope context and does not own `AppCx`.
- UI tree, reactive state, layout, paint, and rendering are intentionally not
  part of this first app/window framework slice.

## API Review Scenarios

App initialization opens windows through `AppCx`:

```rust
impl Application for MyApp {
    fn handle_init(&mut self, cx: &mut AppCx<Self>) {
        cx.open_window(WindowDesc::new("Main"), |window| {
            window.set_title("Main");
        });
    }
}
```

Window-specific work goes through `WindowCx`:

```rust
window_handle.run_on_ui(|_app, window| {
    window.set_title("Updated");
    window.request_redraw();
});
```

Background work returns to the UI thread through `AppCx`:

```rust
cx.spawn_blocking(
    || "done".to_string(),
    |app, cx, value| {
        app.message = Some(value);
        let _ = cx;
    },
);
```

## Examples

```bash
cargo run -p ui0 --example hello_window
cargo run -p ui0 --example background_task
```

## Validation

```bash
cargo fmt -p ui0 --check
cargo check -p ui0 --examples
cargo test -p ui0
```
