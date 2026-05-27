use std::collections::HashMap;

use crate::object::{ObjectId, WhiteboardObject};
use crate::spatial_hash::SpatialHash;

#[derive(Debug, Clone, Copy)]
pub struct Camera2D {
    pub pan_world: [f32; 2],
    pub zoom: f32,
}

impl Default for Camera2D {
    fn default() -> Self {
        Self {
            pan_world: [0.0, 0.0],
            zoom: 1.0,
        }
    }
}

pub struct WhiteboardDoc {
    pub objects: HashMap<ObjectId, WhiteboardObject>,
    pub z_order: Vec<ObjectId>,
    pub camera: Camera2D,
    pub selection: Vec<ObjectId>,
    pub next_id: ObjectId,
    index: SpatialHash,
}

impl WhiteboardDoc {
    pub fn new() -> Self {
        Self {
            objects: HashMap::new(),
            z_order: Vec::new(),
            camera: Camera2D::default(),
            selection: Vec::new(),
            next_id: 1,
            index: SpatialHash::new(256.0),
        }
    }

    pub fn alloc_id(&mut self) -> ObjectId {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        id
    }

    pub fn insert_object(&mut self, object: WhiteboardObject) {
        let id = object.id;
        self.objects.insert(id, object);
        if !self.z_order.contains(&id) {
            self.z_order.push(id);
        }
        self.rebuild_spatial_index();
    }

    pub fn remove_object(&mut self, id: ObjectId) -> Option<WhiteboardObject> {
        self.z_order.retain(|v| *v != id);
        self.selection.retain(|v| *v != id);
        let removed = self.objects.remove(&id);
        self.rebuild_spatial_index();
        removed
    }

    pub fn move_objects(&mut self, ids: &[ObjectId], dx: f32, dy: f32) {
        for id in ids {
            if let Some(object) = self.objects.get_mut(id) {
                object.x += dx;
                object.y += dy;
            }
        }
        self.rebuild_spatial_index();
    }

    pub fn set_selection(&mut self, selection: Vec<ObjectId>) {
        self.selection = selection;
    }

    pub fn clear_selection(&mut self) {
        self.selection.clear();
    }

    pub fn hit_test(&self, x: f32, y: f32) -> Option<ObjectId> {
        let mut candidates = self.index.query_point(x, y);
        candidates.sort_unstable();

        for id in self.z_order.iter().rev() {
            if !candidates.contains(id) {
                continue;
            }
            let Some(obj) = self.objects.get(id) else { continue };
            let (ox, oy, ow, oh) = obj.bounds();
            if x >= ox && x <= ox + ow && y >= oy && y <= oy + oh {
                return Some(*id);
            }
        }
        None
    }

    pub fn query_aabb(&self, x: f32, y: f32, width: f32, height: f32) -> Vec<ObjectId> {
        let mut out = Vec::new();
        let candidates = self.index.query_aabb(x, y, width, height);
        for id in candidates {
            let Some(obj) = self.objects.get(&id) else { continue };
            let (ox, oy, ow, oh) = obj.bounds();
            let intersects = ox <= x + width && ox + ow >= x && oy <= y + height && oy + oh >= y;
            if intersects {
                out.push(id);
            }
        }
        out.sort_by_key(|id| self.z_order.iter().position(|v| v == id).unwrap_or(usize::MAX));
        out
    }

    pub fn rebuild_spatial_index(&mut self) {
        self.index.clear();
        for (id, object) in &self.objects {
            let (x, y, w, h) = object.bounds();
            self.index.insert_aabb(*id, x, y, w, h);
        }
    }
}
