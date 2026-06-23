slotmap::new_key_type! {
    pub struct OwnerId;
}

#[derive(Default)]
pub(crate) struct Owner {
    pub(crate) parent: Option<OwnerId>,
    pub(crate) children: Vec<OwnerId>,
    pub(crate) computations: Vec<crate::reactive::runtime::ComputationId>,
    pub(crate) signals: Vec<crate::reactive::runtime::SignalId>,
    pub(crate) disposed: bool,
}

impl Owner {
    pub(crate) fn new(parent: Option<OwnerId>) -> Self {
        Self {
            parent,
            ..Self::default()
        }
    }
}
