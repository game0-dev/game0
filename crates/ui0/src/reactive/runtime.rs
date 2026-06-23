use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use slotmap::SlotMap;

use crate::ui_tree::UiTree;

use super::owner::{Owner, OwnerId};

slotmap::new_key_type! {
    pub struct SignalId;
    pub struct ComputationId;
}

thread_local! {
    static CURRENT_RUNTIME: RefCell<Option<Rc<RefCell<ReactiveGraph>>>> = const { RefCell::new(None) };
    static CURRENT_COMPUTATION: RefCell<Option<ComputationId>> = const { RefCell::new(None) };
    static CURRENT_OWNER: RefCell<Option<OwnerId>> = const { RefCell::new(None) };
}

pub(crate) struct ReactiveRuntime {
    graph: Rc<RefCell<ReactiveGraph>>,
}

impl Clone for ReactiveRuntime {
    fn clone(&self) -> Self {
        Self {
            graph: Rc::clone(&self.graph),
        }
    }
}

impl ReactiveRuntime {
    pub(crate) fn new() -> Self {
        Self {
            graph: Rc::new(RefCell::new(ReactiveGraph::new())),
        }
    }

    pub(crate) fn root_owner(&self) -> OwnerId {
        self.graph.borrow().root_owner
    }

    pub(crate) fn dispose_owner(&self, owner: OwnerId) {
        self.graph.borrow_mut().dispose_owner(owner);
    }

    pub(crate) fn dispose_all(&self) {
        let root = self.root_owner();
        self.dispose_owner(root);
    }

    pub(crate) fn create_child_owner(&self, parent: OwnerId) -> OwnerId {
        self.graph.borrow_mut().create_owner(Some(parent))
    }

    pub(crate) fn create_effect(
        &self,
        owner: OwnerId,
        callback: impl FnMut(&mut UiTree) + 'static,
    ) -> ComputationId {
        self.graph
            .borrow_mut()
            .create_computation(owner, Box::new(callback))
    }

    pub(crate) fn enter<R>(&self, owner: OwnerId, f: impl FnOnce() -> R) -> R {
        let previous_runtime =
            CURRENT_RUNTIME.with(|cell| cell.replace(Some(Rc::clone(&self.graph))));
        let previous_owner = CURRENT_OWNER.with(|cell| cell.replace(Some(owner)));
        let result = f();
        CURRENT_OWNER.with(|cell| {
            cell.replace(previous_owner);
        });
        CURRENT_RUNTIME.with(|cell| {
            cell.replace(previous_runtime);
        });
        result
    }

    pub(crate) fn flush(&self, tree: &mut UiTree) -> bool {
        let mut ran = false;
        loop {
            let computation = self.graph.borrow_mut().queue.pop_front();
            let Some(computation) = computation else {
                return ran;
            };
            if self.run_computation(computation, tree) {
                ran = true;
            }
        }
    }

    fn run_computation(&self, computation: ComputationId, tree: &mut UiTree) -> bool {
        let (owner, mut callback) = {
            let mut graph = self.graph.borrow_mut();
            let Some(computation_ref) = graph.computations.get_mut(computation) else {
                return false;
            };
            if computation_ref.disposed {
                return false;
            }
            computation_ref.queued = false;
            let owner = computation_ref.owner;
            let deps = std::mem::take(&mut computation_ref.deps);
            let Some(callback) = computation_ref.callback.take() else {
                return false;
            };
            for dep in deps {
                if let Some(signal) = graph.signals.get_mut(dep) {
                    signal
                        .subscribers
                        .retain(|subscriber| *subscriber != computation);
                }
            }
            (owner, callback)
        };

        let previous_computation = CURRENT_COMPUTATION.with(|cell| cell.replace(Some(computation)));
        let previous_owner = CURRENT_OWNER.with(|cell| cell.replace(Some(owner)));
        callback(tree);
        CURRENT_OWNER.with(|cell| {
            cell.replace(previous_owner);
        });
        CURRENT_COMPUTATION.with(|cell| {
            cell.replace(previous_computation);
        });

        let mut graph = self.graph.borrow_mut();
        if let Some(computation_ref) = graph.computations.get_mut(computation) {
            if !computation_ref.disposed {
                computation_ref.callback = Some(callback);
            }
        }
        true
    }
}

impl Default for ReactiveRuntime {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) struct ReactiveGraph {
    signals: SlotMap<SignalId, SignalNode>,
    computations: SlotMap<ComputationId, Computation>,
    owners: SlotMap<OwnerId, Owner>,
    root_owner: OwnerId,
    queue: VecDeque<ComputationId>,
    batch_depth: usize,
}

pub(crate) struct SignalNode {
    pub(crate) subscribers: Vec<ComputationId>,
}

pub(crate) type ComputationCallback = Box<dyn FnMut(&mut UiTree)>;

pub(crate) struct Computation {
    pub(crate) callback: Option<ComputationCallback>,
    pub(crate) deps: Vec<SignalId>,
    pub(crate) owner: OwnerId,
    pub(crate) queued: bool,
    pub(crate) disposed: bool,
}

impl ReactiveGraph {
    fn new() -> Self {
        let mut owners = SlotMap::with_key();
        let root_owner = owners.insert(Owner::new(None));
        Self {
            signals: SlotMap::with_key(),
            computations: SlotMap::with_key(),
            owners,
            root_owner,
            queue: VecDeque::new(),
            batch_depth: 0,
        }
    }

    pub(crate) fn current_runtime() -> Rc<RefCell<Self>> {
        CURRENT_RUNTIME.with(|cell| {
            cell.borrow()
                .as_ref()
                .cloned()
                .expect("ui0 reactive value was created outside a reactive runtime")
        })
    }

    pub(crate) fn current_owner() -> OwnerId {
        CURRENT_OWNER.with(|cell| {
            cell.borrow()
                .expect("ui0 reactive owner is not available during this operation")
        })
    }

    pub(crate) fn create_signal(&mut self, owner: OwnerId) -> SignalId {
        let signal = self.signals.insert(SignalNode {
            subscribers: Vec::new(),
        });
        if let Some(owner_ref) = self.owners.get_mut(owner) {
            owner_ref.signals.push(signal);
        }
        signal
    }

    pub(crate) fn track_signal(&mut self, signal: SignalId) {
        let Some(computation) = CURRENT_COMPUTATION.with(|cell| *cell.borrow()) else {
            return;
        };
        let Some(node) = self.signals.get_mut(signal) else {
            return;
        };
        if !node.subscribers.contains(&computation) {
            node.subscribers.push(computation);
        }
        if let Some(computation_ref) = self.computations.get_mut(computation) {
            if !computation_ref.deps.contains(&signal) {
                computation_ref.deps.push(signal);
            }
        }
    }

    pub(crate) fn mark_signal_changed(&mut self, signal: SignalId) {
        let subscribers = self
            .signals
            .get(signal)
            .map(|node| node.subscribers.clone())
            .unwrap_or_default();
        for computation in subscribers {
            self.enqueue(computation);
        }
    }

    pub(crate) fn create_computation(
        &mut self,
        owner: OwnerId,
        callback: ComputationCallback,
    ) -> ComputationId {
        let computation = self.computations.insert(Computation {
            callback: Some(callback),
            deps: Vec::new(),
            owner,
            queued: true,
            disposed: false,
        });
        if let Some(owner_ref) = self.owners.get_mut(owner) {
            owner_ref.computations.push(computation);
        }
        self.queue.push_back(computation);
        computation
    }

    pub(crate) fn create_owner(&mut self, parent: Option<OwnerId>) -> OwnerId {
        let owner = self.owners.insert(Owner::new(parent));
        if let Some(parent) = parent {
            if let Some(parent_ref) = self.owners.get_mut(parent) {
                parent_ref.children.push(owner);
            }
        }
        owner
    }

    pub(crate) fn dispose_owner(&mut self, owner: OwnerId) {
        let Some(owner_ref) = self.owners.get(owner) else {
            return;
        };
        let children = owner_ref.children.clone();
        let computations = owner_ref.computations.clone();
        let signals = owner_ref.signals.clone();
        let parent = owner_ref.parent;
        for child in children {
            self.dispose_owner(child);
        }
        for computation in computations {
            self.dispose_computation(computation);
        }
        for signal in signals {
            self.signals.remove(signal);
        }
        if let Some(parent) = parent {
            if let Some(parent_ref) = self.owners.get_mut(parent) {
                parent_ref.children.retain(|child| *child != owner);
            }
        }
        if let Some(owner_ref) = self.owners.get_mut(owner) {
            owner_ref.disposed = true;
            owner_ref.children.clear();
            owner_ref.computations.clear();
            owner_ref.signals.clear();
        }
        if owner != self.root_owner {
            self.owners.remove(owner);
        }
    }

    fn dispose_computation(&mut self, computation: ComputationId) {
        let deps = self
            .computations
            .get(computation)
            .map(|computation_ref| computation_ref.deps.clone())
            .unwrap_or_default();
        for dep in deps {
            if let Some(signal) = self.signals.get_mut(dep) {
                signal
                    .subscribers
                    .retain(|subscriber| *subscriber != computation);
            }
        }
        self.computations.remove(computation);
    }

    fn enqueue(&mut self, computation: ComputationId) {
        let Some(computation_ref) = self.computations.get_mut(computation) else {
            return;
        };
        if computation_ref.disposed || computation_ref.queued {
            return;
        }
        computation_ref.queued = true;
        self.queue.push_back(computation);
    }

    pub(crate) fn begin_batch(&mut self) {
        self.batch_depth += 1;
    }

    pub(crate) fn end_batch(&mut self) {
        self.batch_depth = self.batch_depth.saturating_sub(1);
    }
}

pub(crate) fn with_current_graph<R>(f: impl FnOnce(&Rc<RefCell<ReactiveGraph>>) -> R) -> R {
    CURRENT_RUNTIME.with(|cell| {
        let graph = cell
            .borrow()
            .as_ref()
            .cloned()
            .expect("ui0 reactive runtime is not available");
        f(&graph)
    })
}

pub(crate) fn with_untracked<R>(f: impl FnOnce() -> R) -> R {
    let previous = CURRENT_COMPUTATION.with(|cell| cell.replace(None));
    let result = f();
    CURRENT_COMPUTATION.with(|cell| {
        cell.replace(previous);
    });
    result
}
