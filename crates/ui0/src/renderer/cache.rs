use std::ops::Range;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RenderNodeState {
    pub(crate) paint_dirty: bool,
    pub(crate) subtree_paint_dirty: bool,
    pub(crate) self_commands: Range<usize>,
    pub(crate) subtree_commands: Range<usize>,
    pub(crate) rect_instances: Range<usize>,
    pub(crate) text_runs: Range<usize>,
    pub(crate) surface_range: Range<usize>,
}

impl RenderNodeState {
    pub(crate) fn has_cached_subtree(&self) -> bool {
        self.subtree_commands.start < self.subtree_commands.end
    }
}
