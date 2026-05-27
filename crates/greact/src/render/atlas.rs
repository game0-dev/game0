use std::collections::HashMap;

use etagere::{size2, Allocation, AtlasAllocator};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphCacheKey(pub u128);

#[derive(Debug, Clone, Copy)]
pub struct AtlasRegion {
    pub page: u16,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

struct AtlasPage {
    allocator: AtlasAllocator,
}

impl AtlasPage {
    fn new(size: i32) -> Self {
        Self {
            allocator: AtlasAllocator::new(size2(size, size)),
        }
    }

    fn allocate(&mut self, width: i32, height: i32) -> Option<Allocation> {
        self.allocator.allocate(size2(width.max(1), height.max(1)))
    }
}

pub struct GlyphAtlas {
    page_size: i32,
    pages: Vec<AtlasPage>,
    glyphs: HashMap<GlyphCacheKey, AtlasRegion>,
}

impl GlyphAtlas {
    pub fn new(page_size: i32) -> Self {
        let clamped_size = page_size.max(64);
        Self {
            page_size: clamped_size,
            pages: vec![AtlasPage::new(clamped_size)],
            glyphs: HashMap::new(),
        }
    }

    pub fn lookup(&self, key: GlyphCacheKey) -> Option<AtlasRegion> {
        self.glyphs.get(&key).copied()
    }

    pub fn ensure_region(
        &mut self,
        key: GlyphCacheKey,
        width: u16,
        height: u16,
    ) -> (AtlasRegion, bool) {
        if let Some(region) = self.lookup(key) {
            return (region, false);
        }

        let (alloc, page) = self.allocate_or_grow(width as i32, height as i32);
        let rect = alloc.rectangle;
        let region = AtlasRegion {
            page: page as u16,
            x: rect.min.x as u16,
            y: rect.min.y as u16,
            width,
            height,
        };
        self.glyphs.insert(key, region);
        (region, true)
    }

    pub fn page_size(&self) -> u32 {
        self.page_size as u32
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    fn allocate_or_grow(&mut self, width: i32, height: i32) -> (Allocation, usize) {
        for (idx, page) in self.pages.iter_mut().enumerate() {
            if let Some(alloc) = page.allocate(width, height) {
                return (alloc, idx);
            }
        }

        let mut new_page = AtlasPage::new(self.page_size);
        let alloc = new_page
            .allocate(width, height)
            .expect("glyph allocation too large for atlas page");
        self.pages.push(new_page);
        (alloc, self.pages.len() - 1)
    }
}
