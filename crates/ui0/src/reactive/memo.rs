use super::effect::effect;
use super::effect::untrack;
use super::signal::Signal;

pub fn memo<T, F>(f: F) -> Memo<T>
where
    T: Clone + PartialEq + 'static,
    F: FnMut() -> T + 'static,
{
    Memo::new(f)
}

pub struct Memo<T: Clone + PartialEq + 'static> {
    signal: Signal<T>,
}

impl<T: Clone + PartialEq + 'static> Clone for Memo<T> {
    fn clone(&self) -> Self {
        Self {
            signal: self.signal.clone(),
        }
    }
}

impl<T: Clone + PartialEq + 'static> Memo<T> {
    pub(crate) fn new<F>(mut f: F) -> Self
    where
        F: FnMut() -> T + 'static,
    {
        let initial = untrack(&mut f);
        let signal = Signal::new(initial);
        let signal_for_effect = signal.clone();
        effect(move || {
            signal_for_effect.set(f());
        });
        Self { signal }
    }

    pub fn get(&self) -> T {
        self.signal.get()
    }

    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        self.signal.with(f)
    }
}
