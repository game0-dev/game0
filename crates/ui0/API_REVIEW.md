# ui0 Event and Input API Review

This note records the current API boundary after adding the first reactive examples.

## Event API

Current user-facing shape:

```rust
button().on_click(move |cx| {
    let window = cx.id();
});

div()
    .on_pointer_down(|cx, event| {
        let target = cx.target();
        let position = event.position;
    })
    .on_pointer_move(|cx, event| {
        cx.stop_propagation();
    });
```

Current implementation status:

- `Element::on_click` stores a click handler in `EventHandlers`.
- `Element::on_pointer_down`, `on_pointer_up`, `on_pointer_move`, `on_pointer_enter`, and `on_pointer_leave` store typed pointer handlers.
- Mounting an element with handlers marks the node with the corresponding `EventFlags`.
- `UiTree::hit_test` uses the computed layout rects, reverse child order, and transparent fragments to return `HitTestResult { target, path }`.
- The runtime tracks cursor position, pointer buttons, modifiers, hover path, and pressed target.
- Pointer move dispatches enter/leave transitions and pointer move.
- Pointer down/up dispatches pointer handlers; primary down/up on the same target synthesizes click.
- Dispatch uses target-to-root bubbling and supports `stop_propagation`.
- `EventCx` exposes `id()`, `window_id()`, `target()`, `current_target()`, `phase()`, `request_redraw()`, and propagation state.

Remaining runtime gaps:

- Focus dispatch.
- Keyboard dispatch.
- Scroll/wheel dispatch.
- Pointer capture.
- Text input editing behavior.
- Clipping, scrolling, transforms, and z-index in hit testing.

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

- `reactive_counter.rs` covers `window.mount(|| ...)`, `signal`, `memo`, dynamic `text`, and `on_click` usage.
- `control_flow.rs` covers `show`, `child_fn`, `for_each`, keyed row reuse, and row-local signals.

The examples intentionally compile against the current API without assuming text input behavior is implemented.
