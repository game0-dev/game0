use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::element::{Element, IntoElement};

pub struct ShowBuilder<C>
where
    C: FnMut() -> bool + 'static,
{
    pub(crate) condition: C,
}

pub struct ShowElement {
    pub(crate) condition: Box<dyn FnMut() -> bool>,
    pub(crate) then_view: Box<dyn FnMut() -> Element>,
    pub(crate) fallback_view: Option<Box<dyn FnMut() -> Element>>,
}

pub fn show<C>(condition: C) -> ShowBuilder<C>
where
    C: FnMut() -> bool + 'static,
{
    ShowBuilder { condition }
}

impl<C> ShowBuilder<C>
where
    C: FnMut() -> bool + 'static,
{
    pub fn then<T, E>(self, then_view: T) -> ShowThenBuilder
    where
        T: FnMut() -> E + 'static,
        E: IntoElement + 'static,
    {
        let mut condition = self.condition;
        let mut then_view = then_view;
        ShowThenBuilder {
            condition: Box::new(move || condition()),
            then_view: Box::new(move || then_view().into_element()),
        }
    }
}

pub struct ShowThenBuilder {
    pub(crate) condition: Box<dyn FnMut() -> bool>,
    pub(crate) then_view: Box<dyn FnMut() -> Element>,
}

impl ShowThenBuilder {
    pub fn fallback<F, E>(self, fallback_view: F) -> ShowElement
    where
        F: FnMut() -> E + 'static,
        E: IntoElement + 'static,
    {
        let mut fallback_view = fallback_view;
        ShowElement {
            condition: self.condition,
            then_view: self.then_view,
            fallback_view: Some(Box::new(move || fallback_view().into_element())),
        }
    }

    pub fn build(self) -> ShowElement {
        ShowElement {
            condition: self.condition,
            then_view: self.then_view,
            fallback_view: None,
        }
    }
}

pub struct ForEachBuilder<T, S>
where
    S: FnMut() -> Vec<T> + 'static,
    T: 'static,
{
    pub(crate) source: S,
    pub(crate) _marker: std::marker::PhantomData<T>,
}

pub fn for_each<T, S>(source: S) -> ForEachBuilder<T, S>
where
    S: FnMut() -> Vec<T> + 'static,
    T: 'static,
{
    ForEachBuilder {
        source,
        _marker: std::marker::PhantomData,
    }
}

impl<T, S> ForEachBuilder<T, S>
where
    S: FnMut() -> Vec<T> + 'static,
    T: 'static,
{
    pub fn key<K, F>(self, key: F) -> ForEachKeyBuilder<T, S, F, K>
    where
        F: Fn(&T) -> K + 'static,
        K: ToString + 'static,
    {
        ForEachKeyBuilder {
            source: self.source,
            key,
            _marker: std::marker::PhantomData,
        }
    }
}

pub struct ForEachKeyBuilder<T, S, F, K>
where
    S: FnMut() -> Vec<T> + 'static,
    F: Fn(&T) -> K + 'static,
    K: ToString + 'static,
    T: 'static,
{
    source: S,
    key: F,
    _marker: std::marker::PhantomData<(T, K)>,
}

impl<T, S, F, K> ForEachKeyBuilder<T, S, F, K>
where
    S: FnMut() -> Vec<T> + 'static,
    F: Fn(&T) -> K + 'static,
    K: ToString + 'static,
    T: 'static,
{
    pub fn row<R, E>(self, row: R) -> ForEachElement
    where
        R: FnMut(T) -> E + 'static,
        E: IntoElement + 'static,
    {
        let empty_view: Rc<RefCell<Box<dyn FnMut() -> Element>>> =
            Rc::new(RefCell::new(Box::new(Element::fragment)));
        self.row_with_empty_view(row, empty_view)
    }

    fn row_with_empty_view<R, E>(
        self,
        row: R,
        empty_view: Rc<RefCell<Box<dyn FnMut() -> Element>>>,
    ) -> ForEachElement
    where
        R: FnMut(T) -> E + 'static,
        E: IntoElement + 'static,
    {
        let mut source = self.source;
        let key = self.key;
        let mut row = row;
        let empty_for_build = Rc::clone(&empty_view);
        ForEachElement {
            state: ForEachState::new(),
            empty_view,
            build: Box::new(move || {
                let items = source();
                if items.is_empty() {
                    return vec![ForEachItem::empty((empty_for_build.borrow_mut())())];
                }
                items
                    .into_iter()
                    .map(|item| {
                        let key = key(&item).to_string();
                        ForEachItem::row(key, row(item).into_element())
                    })
                    .collect()
            }),
        }
    }
}

pub struct ForEachElement {
    pub(crate) state: ForEachState,
    empty_view: Rc<RefCell<Box<dyn FnMut() -> Element>>>,
    pub(crate) build: Box<dyn FnMut() -> Vec<ForEachItem>>,
}

impl ForEachElement {
    pub fn empty<F, E>(self, empty: F) -> Self
    where
        F: FnMut() -> E + 'static,
        E: IntoElement + 'static,
    {
        let mut empty = empty;
        *self.empty_view.borrow_mut() = Box::new(move || empty().into_element());
        self
    }
}

pub(crate) struct ForEachItem {
    pub(crate) key: String,
    pub(crate) element: Element,
    pub(crate) empty: bool,
}

impl ForEachItem {
    fn row(key: String, element: Element) -> Self {
        Self {
            key,
            element,
            empty: false,
        }
    }

    fn empty(element: Element) -> Self {
        Self {
            key: "__ui0_empty__".to_string(),
            element,
            empty: true,
        }
    }
}

#[derive(Default)]
pub(crate) struct ForEachState {
    pub(crate) rows: HashMap<String, crate::reactive::RegionState>,
    pub(crate) empty: Option<crate::reactive::RegionState>,
}

impl ForEachState {
    fn new() -> Self {
        Self::default()
    }
}
