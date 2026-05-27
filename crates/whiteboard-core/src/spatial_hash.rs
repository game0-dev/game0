use std::collections::{HashMap, HashSet};

use smallvec::SmallVec;

use crate::object::ObjectId;

#[derive(Debug, Clone)]
pub struct SpatialHash {
    cell_size: f32,
    cells: HashMap<(i32, i32), SmallVec<[ObjectId; 8]>>,
}

impl SpatialHash {
    pub fn new(cell_size: f32) -> Self {
        Self {
            cell_size: cell_size.max(1.0),
            cells: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.cells.clear();
    }

    pub fn insert_aabb(&mut self, id: ObjectId, x: f32, y: f32, width: f32, height: f32) {
        let min_x = (x / self.cell_size).floor() as i32;
        let min_y = (y / self.cell_size).floor() as i32;
        let max_x = ((x + width) / self.cell_size).floor() as i32;
        let max_y = ((y + height) / self.cell_size).floor() as i32;

        for cy in min_y..=max_y {
            for cx in min_x..=max_x {
                self.cells.entry((cx, cy)).or_default().push(id);
            }
        }
    }

    pub fn query_point(&self, x: f32, y: f32) -> Vec<ObjectId> {
        let cx = (x / self.cell_size).floor() as i32;
        let cy = (y / self.cell_size).floor() as i32;
        self.cells
            .get(&(cx, cy))
            .map(|v| v.iter().copied().collect())
            .unwrap_or_default()
    }

    pub fn query_aabb(&self, x: f32, y: f32, width: f32, height: f32) -> Vec<ObjectId> {
        let min_x = (x / self.cell_size).floor() as i32;
        let min_y = (y / self.cell_size).floor() as i32;
        let max_x = ((x + width) / self.cell_size).floor() as i32;
        let max_y = ((y + height) / self.cell_size).floor() as i32;

        let mut out = HashSet::new();
        for cy in min_y..=max_y {
            for cx in min_x..=max_x {
                if let Some(bucket) = self.cells.get(&(cx, cy)) {
                    for id in bucket {
                        out.insert(*id);
                    }
                }
            }
        }
        out.into_iter().collect()
    }
}
