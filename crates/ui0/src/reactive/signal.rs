use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

use super::runtime::{ReactiveGraph, SignalId};

pub fn signal<T: 'static>(value: T) -> Signal<T> {
    Signal::new(value)
}

pub struct Signal<T: 'static> {
    id: SignalId,
    value: Rc<RefCell<T>>,
    graph: Rc<RefCell<ReactiveGraph>>,
    _not_send_sync: PhantomData<Rc<()>>,
}

impl<T: 'static> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            value: Rc::clone(&self.value),
            graph: Rc::clone(&self.graph),
            _not_send_sync: PhantomData,
        }
    }
}

impl<T: 'static> Signal<T> {
    pub(crate) fn new(value: T) -> Self {
        let graph = ReactiveGraph::current_runtime();
        let owner = ReactiveGraph::current_owner();
        let id = graph.borrow_mut().create_signal(owner);
        Self {
            id,
            value: Rc::new(RefCell::new(value)),
            graph,
            _not_send_sync: PhantomData,
        }
    }

    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.graph.borrow_mut().track_signal(self.id);
        self.value.borrow().clone()
    }

    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        self.graph.borrow_mut().track_signal(self.id);
        f(&self.value.borrow())
    }

    pub fn set(&self, value: T)
    where
        T: PartialEq,
    {
        let changed = {
            let mut current = self.value.borrow_mut();
            if *current == value {
                false
            } else {
                *current = value;
                true
            }
        };
        if changed {
            self.graph.borrow_mut().mark_signal_changed(self.id);
        }
    }

    pub fn update(&self, f: impl FnOnce(&mut T))
    where
        T: PartialEq + Clone,
    {
        let previous = self.value.borrow().clone();
        {
            let mut current = self.value.borrow_mut();
            f(&mut current);
        }
        if *self.value.borrow() != previous {
            self.graph.borrow_mut().mark_signal_changed(self.id);
        }
    }
}
