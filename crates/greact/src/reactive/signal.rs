use std::marker::PhantomData;

use super::runtime::{EffectId, RawSignal, SignalId};
use super::effect::flush_effects;
use crate::RUNTIME;

/// Read-only handle to a signal.  `Copy` so it can be freely captured by
/// closures.
#[derive(Debug)]
pub struct ReadSignal<T> {
    pub(crate) id: SignalId,
    _marker: PhantomData<T>,
}

// Manual Copy/Clone so there is no `T: Copy` / `T: Clone` bound.
impl<T> Copy for ReadSignal<T> {}
impl<T> Clone for ReadSignal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

/// Write-only handle to a signal.  Also `Copy`.
#[derive(Debug)]
pub struct WriteSignal<T> {
    pub(crate) id: SignalId,
    _marker: PhantomData<T>,
}

impl<T> Copy for WriteSignal<T> {}
impl<T> Clone for WriteSignal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

/// Create a new signal with an initial value.  Returns a `(ReadSignal,
/// WriteSignal)` pair.
pub fn create_signal<T: 'static>(value: T) -> (ReadSignal<T>, WriteSignal<T>) {
    let id = RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        rt.signals.insert(RawSignal {
            value: Box::new(value),
            subscribers: Default::default(),
        })
    });
    (
        ReadSignal {
            id,
            _marker: PhantomData,
        },
        WriteSignal {
            id,
            _marker: PhantomData,
        },
    )
}

impl<T: Clone + 'static> ReadSignal<T> {
    /// Read the current value.  If called inside an effect, the effect is
    /// automatically registered as a subscriber of this signal.
    pub fn get(&self) -> T {
        RUNTIME.with(|rt| {
            let mut rt = rt.borrow_mut();
            track_current_observer(&mut rt, self.id);
            rt.signals[self.id]
                .value
                .downcast_ref::<T>()
                .unwrap()
                .clone()
        })
    }
}

impl<T: 'static> ReadSignal<T> {
    /// Read the current value by reference and avoid cloning large data.
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        RUNTIME.with(|rt| {
            let mut rt = rt.borrow_mut();
            track_current_observer(&mut rt, self.id);
            let value = rt.signals[self.id].value.downcast_ref::<T>().unwrap();
            f(value)
        })
    }
}

impl<T: 'static> WriteSignal<T> {
    /// Overwrite the signal value and schedule subscriber effects.
    pub fn set(&self, value: T) {
        let should_flush = RUNTIME.with(|rt| {
            let mut rt = rt.borrow_mut();
            rt.signals[self.id].value = Box::new(value);
            schedule_subscribers(&mut rt, self.id);
            rt.batch_depth == 0
        });
        if should_flush {
            flush_effects();
        }
    }

    /// Modify the signal value in-place and schedule subscriber effects.
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        let should_flush = RUNTIME.with(|rt| {
            let mut rt = rt.borrow_mut();
            let val = rt.signals[self.id]
                .value
                .downcast_mut::<T>()
                .unwrap();
            f(val);
            schedule_subscribers(&mut rt, self.id);
            rt.batch_depth == 0
        });
        if should_flush {
            flush_effects();
        }
    }
}

impl<T: PartialEq + 'static> WriteSignal<T> {
    /// Set only when value changed (`PartialEq`), reducing downstream work.
    pub fn set_if_changed(&self, value: T) {
        let should_flush = RUNTIME.with(|rt| {
            let mut rt = rt.borrow_mut();
            let current = rt.signals[self.id].value.downcast_ref::<T>().unwrap();
            if *current == value {
                return false;
            }
            rt.signals[self.id].value = Box::new(value);
            schedule_subscribers(&mut rt, self.id);
            rt.batch_depth == 0
        });
        if should_flush {
            flush_effects();
        }
    }
}

fn track_current_observer(rt: &mut super::runtime::ReactiveRuntime, sid: SignalId) {
    if let Some(&eid) = rt.observer_stack.last() {
        rt.signals[sid].subscribers.insert(eid);
        if let Some(effect) = rt.effects.get_mut(eid) {
            effect.deps.insert(sid);
        }
    }
}

fn schedule_subscribers(rt: &mut super::runtime::ReactiveRuntime, sid: SignalId) {
    let pending: Vec<EffectId> = rt.signals[sid].subscribers.iter().copied().collect();
    for eid in pending {
        rt.schedule_effect(eid);
    }
}
