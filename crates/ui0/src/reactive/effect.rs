use crate::ui_tree::UiTree;

use super::runtime::{with_current_graph, with_untracked, ComputationId, ReactiveGraph};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EffectHandle {
    pub(crate) id: ComputationId,
}

pub fn effect<F>(mut f: F) -> EffectHandle
where
    F: FnMut() + 'static,
{
    let id = with_current_graph(|graph| {
        let owner = ReactiveGraph::current_owner();
        graph.borrow_mut().create_computation(
            owner,
            Box::new(move |_tree: &mut UiTree| {
                f();
            }),
        )
    });
    EffectHandle { id }
}

pub fn batch<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    with_current_graph(|graph| graph.borrow_mut().begin_batch());
    let result = f();
    with_current_graph(|graph| graph.borrow_mut().end_batch());
    result
}

pub fn untrack<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    with_untracked(f)
}
