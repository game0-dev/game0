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
cargo run -p ui0 --example render_gallery
cargo run -p ui0 --example interactive_smoke
```

## ui0 Renderer Baseline

`render_gallery` is the fixed manual acceptance sample for the current renderer
slice:

```bash
cargo run -p ui0 --example render_gallery
```

When accepting renderer changes, the sample must visibly show:

- A dark window background with no uninitialized or flickering surface content.
- Three rows of cards laid out consistently after first paint.
- Rounded rectangles with visibly different radii: 0, 4, 8, and 16 px.
- Borders with visibly different widths: 1, 2, and 4 px.
- Text rendered through the GPU text path at small, medium, and large sizes.
- Correct HiDPI scaling: text, borders, and radii should remain crisp on scaled displays.
- Correct window resize handling: resizing the window should not panic, distort the scene, or leave stale frame contents.

The current renderer baseline includes single-pass GPU rect rendering, SDF
rounded corners, border rendering, glyphon text rendering, rect paint-order
batching, opacity accumulation, and overflow clip scissor for rect batches.
Text is prepared once per frame and rendered after rects in this baseline;
strict interleaved text paint order is future work.

Images and external/game surfaces are represented in `PaintScene` and batch
compilation, but their GPU draw paths are intentionally not part of this
baseline yet.

## Validation

```bash
cargo fmt -p ui0 --check
cargo check -p ui0 --examples
cargo test -p ui0
```
