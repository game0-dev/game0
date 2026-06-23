use crate::ui_tree::NodeId;

use super::OwnerId;

#[derive(Default)]
pub(crate) struct RegionState {
    pub(crate) parent: Option<NodeId>,
    pub(crate) nodes: Vec<NodeId>,
    pub(crate) owner: Option<OwnerId>,
}

impl RegionState {
    pub(crate) fn new(parent: NodeId) -> Self {
        Self {
            parent: Some(parent),
            nodes: Vec::new(),
            owner: None,
        }
    }
}
