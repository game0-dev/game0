use super::{Display, HitTestResult, NodeId, Point, UiNodeTag, UiTree};

impl UiTree {
    pub fn hit_test(&self, point: Point) -> HitTestResult {
        self.hit_test_node(self.root, point).unwrap_or_default()
    }

    fn hit_test_node(&self, node: NodeId, point: Point) -> Option<HitTestResult> {
        let node_ref = self.node(node)?;

        if node_ref.tag == UiNodeTag::Fragment {
            return self.hit_test_children(node, point);
        }

        if self.is_display_none(node) || !self.contains_point(node, point) {
            return None;
        }

        if let Some(mut result) = self.hit_test_children(node, point) {
            result.path.insert(0, node);
            return Some(result);
        }

        Some(HitTestResult {
            target: Some(node),
            path: vec![node],
        })
    }

    fn hit_test_children(&self, node: NodeId, point: Point) -> Option<HitTestResult> {
        for child in self.children(node).iter().rev() {
            if let Some(result) = self.hit_test_node(*child, point) {
                return Some(result);
            }
        }
        None
    }

    fn is_display_none(&self, node: NodeId) -> bool {
        self.flex_styles
            .get(node)
            .map(|style| style.display == Display::None)
            .unwrap_or(false)
    }

    fn contains_point(&self, node: NodeId, point: Point) -> bool {
        let Some(rect) = self.layout_rect(node) else {
            return false;
        };
        point.x >= rect.x
            && point.y >= rect.y
            && point.x <= rect.x + rect.width
            && point.y <= rect.y + rect.height
    }
}
