use super::runtime::{EffectData, EffectId};
use crate::RUNTIME;
use std::collections::HashSet;

/// Create an effect.  The closure is run immediately (to collect initial
/// dependencies) and then re-run whenever any signal it reads changes.
pub fn create_effect(f: impl FnMut() + 'static) -> EffectId {
    let eid = RUNTIME.with(|rt| {
        rt.borrow_mut().effects.insert(EffectData {
            f: Some(Box::new(f)),
            deps: HashSet::new(),
        })
    });
    run_effect(eid);
    eid
}

/// Dispose an effect and unsubscribe it from all current dependencies.
pub fn dispose_effect(eid: EffectId) {
    RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let Some(effect) = rt.effects.remove(eid) else { return };
        for sid in effect.deps {
            if let Some(sig) = rt.signals.get_mut(sid) {
                sig.subscribers.remove(&eid);
            }
        }
        rt.pending_effect_set.remove(&eid);
        rt.pending_effects.retain(|queued| *queued != eid);
    });
}

/// Execute a single effect.
///
/// Borrow-safe design: we *take* the closure out of the runtime, so
/// `signal.get()` / `with_render_tree()` inside the closure can freely
/// borrow the thread-local singletons without conflicting with the
/// runtime borrow.
pub(crate) fn run_effect(eid: EffectId) {
    // 1. Take the closure out + clear old deps
    let took: Option<Box<dyn FnMut()>> = RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();

        // First, grab the effect's closure and deps while we hold a mutable
        // borrow to that effect only.
        let (old_deps, took) = {
            let effect = rt.effects.get_mut(eid)?;
            let old_deps = std::mem::take(&mut effect.deps);
            let took = effect.f.take();
            (old_deps, took)
        };

        // Now that the mutable borrow to `effect` has ended (scope above),
        // it's safe to also touch `rt.signals`.
        for sid in &old_deps {
            if let Some(sig) = rt.signals.get_mut(*sid) {
                sig.subscribers.remove(&eid);
            }
        }

        took
    });

    let Some(mut f) = took else { return };

    // 2. Push this effect onto the observer stack so signal reads track it
    RUNTIME.with(|rt| rt.borrow_mut().observer_stack.push(eid));

    // 3. Run the closure -- signal.get() calls will register dependencies
    f();

    // 4. Pop observer and put the closure back
    RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        rt.observer_stack.pop();
        if let Some(effect) = rt.effects.get_mut(eid) {
            effect.f = Some(f);
        }
    });
}

/// Drain and run all pending effects.
pub fn flush_effects() {
    loop {
        let eid = RUNTIME.with(|rt| rt.borrow_mut().take_next_scheduled());
        match eid {
            Some(eid) => run_effect(eid),
            None => break,
        }
    }
}

struct BatchGuard;

impl Drop for BatchGuard {
    fn drop(&mut self) {
        let should_flush = RUNTIME.with(|rt| {
            let mut rt = rt.borrow_mut();
            rt.batch_depth = rt.batch_depth.saturating_sub(1);
            rt.batch_depth == 0
        });
        if should_flush {
            flush_effects();
        }
    }
}

/// Batch multiple signal writes into a single effect flush.
pub fn batch<R>(f: impl FnOnce() -> R) -> R {
    RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        rt.batch_depth = rt.batch_depth.saturating_add(1);
    });
    let _guard = BatchGuard;
    f()
}

/// Run a closure without dependency tracking from the current effect.
pub fn untrack<R>(f: impl FnOnce() -> R) -> R {
    let prev = RUNTIME.with(|rt| rt.borrow_mut().observer_stack.pop());
    let result = f();
    if let Some(eid) = prev {
        RUNTIME.with(|rt| rt.borrow_mut().observer_stack.push(eid));
    }
    result
}
