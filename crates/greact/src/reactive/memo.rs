use crate::reactive::signal::{create_signal, ReadSignal};
use crate::reactive::effect::create_effect;

/// Create a memoised derived value.  The closure runs inside an effect; when
/// its dependencies change the memo is re-evaluated, but downstream effects
/// are only triggered if the new value differs from the previous one.
///
/// Note: we track the previous value in a local variable rather than via
/// `read.get()` to avoid self-subscribing (which would cause infinite loops).
pub fn create_memo<T: Clone + PartialEq + 'static>(
    mut f: impl FnMut() -> T + 'static,
) -> ReadSignal<T> {
    let initial = f();
    let (read, write) = create_signal(initial.clone());
    let mut prev = initial;
    create_effect(move || {
        let new_val = f();
        if new_val != prev {
            prev = new_val.clone();
            write.set(new_val);
        }
    });
    read
}
