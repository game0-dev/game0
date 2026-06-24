# ui0 Event and Input API Review

This note records the current API boundary after adding the first reactive examples.

## Event API

Current user-facing shape:

```rust
button().on_click(move |cx| {
    let window = cx.id();
});
```

Current implementation status:

- `Element::on_click` stores a click handler in `EventHandlers`.
- Mounting an element with a click handler marks the node with `EventFlags::CLICK`.
- `EventCx` currently exposes the owning `WindowId` through `id()`.
- The runtime does not yet perform pointer hit testing, click dispatch, focus dispatch, keyboard dispatch, or event propagation.

Recommended first runtime step:

- Track pointer position from `WindowEvent::CursorMoved`.
- On pointer release, hit-test the window `UiTree` against `LayoutRect` once layout exists.
- Invoke the nearest target node with a click handler.
- Flush the window reactive runtime after the handler, then request redraw if anything changed.

Recommended first `EventCx` additions:

```rust
cx.window_id()
cx.request_redraw()
cx.stop_propagation()
cx.target()
```

`id()` can remain as a short alias for `window_id()`.

## Input API

Target controlled input shape:

```rust
text_input()
    .value(name.clone())
    .on_value_change(move |value| name.set(value))
```

This should mean:

- `value(...)` reads from a `Signal<String>`, `Memo<String>`, static `String`, or closure.
- `on_value_change(...)` is the only write path for controlled input.
- The input node owns `TextInputState`, focus state, and text editing state in `UiTree`.
- Runtime text events update the internal editing state, call `on_value_change`, flush reactivity, and request redraw.

Do not create new signals inside event handlers. Signals should be created while mounting inside a reactive owner; handlers should update existing signals.

## Example Coverage

- `reactive_counter.rs` covers `window.mount(|| ...)`, `signal`, `memo`, dynamic `text`, and intended `on_click` usage.
- `control_flow.rs` covers `show`, `child_fn`, `for_each`, keyed row reuse, and row-local signals.

The examples intentionally compile against the current API without assuming click dispatch or text input behavior is implemented.
