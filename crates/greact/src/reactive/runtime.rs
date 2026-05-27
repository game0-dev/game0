use std::any::Any;
use std::collections::{HashSet, VecDeque};

use slotmap::SlotMap;

slotmap::new_key_type! {
    /// Handle to a signal inside the `ReactiveRuntime`.
    pub struct SignalId;
}
slotmap::new_key_type! {
    /// Handle to an effect inside the `ReactiveRuntime`.
    pub struct EffectId;
}

pub(crate) struct RawSignal {
    pub value: Box<dyn Any>,
    pub subscribers: HashSet<EffectId>,
}

pub(crate) struct EffectData {
    pub f: Option<Box<dyn FnMut()>>,
    pub deps: HashSet<SignalId>,
}

/// The reactive runtime owns all signals and effects on a single thread.
pub struct ReactiveRuntime {
    pub(crate) signals: SlotMap<SignalId, RawSignal>,
    pub(crate) effects: SlotMap<EffectId, EffectData>,
    /// Stack of effects currently being run (for auto-tracking).
    pub(crate) observer_stack: Vec<EffectId>,
    /// Effects whose dependencies changed and are waiting to be re-run.
    pub(crate) pending_effects: VecDeque<EffectId>,
    /// Fast de-dup set for `pending_effects`.
    pub(crate) pending_effect_set: HashSet<EffectId>,
    pub(crate) batch_depth: u32,
}

impl ReactiveRuntime {
    pub fn new() -> Self {
        Self {
            signals: SlotMap::with_key(),
            effects: SlotMap::with_key(),
            observer_stack: Vec::new(),
            pending_effects: VecDeque::new(),
            pending_effect_set: HashSet::new(),
            batch_depth: 0,
        }
    }

    pub(crate) fn schedule_effect(&mut self, eid: EffectId) {
        if self.pending_effect_set.insert(eid) {
            self.pending_effects.push_back(eid);
        }
    }

    pub(crate) fn take_next_scheduled(&mut self) -> Option<EffectId> {
        let next = self.pending_effects.pop_front()?;
        self.pending_effect_set.remove(&next);
        Some(next)
    }
}
