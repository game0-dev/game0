use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use cosmic_text::{
    Attrs, Buffer, Color as CosmicColor, FontSystem, Metrics, Shaping, SwashCache, SwashContent,
};

use crate::render::atlas::{GlyphAtlas, GlyphCacheKey};

#[derive(Debug, Clone)]
pub struct GlyphUpload {
    pub page: u16,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub alpha: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]
pub struct GlyphQuad {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub atlas_page: u16,
    pub atlas_x: u16,
    pub atlas_y: u16,
}

#[derive(Debug, Clone)]
pub struct ShapedText {
    pub glyphs: Vec<GlyphQuad>,
}

pub struct TextEngine {
    font_system: FontSystem,
    swash: SwashCache,
    atlas: GlyphAtlas,
    pending_uploads: Vec<GlyphUpload>,
}

impl TextEngine {
    pub fn new(atlas_page_size: i32) -> Self {
        Self {
            font_system: FontSystem::new(),
            swash: SwashCache::new(),
            atlas: GlyphAtlas::new(atlas_page_size),
            pending_uploads: Vec::new(),
        }
    }

    pub fn shape_text(
        &mut self,
        text: &str,
        font_size: f32,
        line_height: f32,
        max_width: f32,
        origin_x: f32,
        origin_y: f32,
    ) -> ShapedText {
        let metrics = Metrics::new(font_size.max(1.0), line_height.max(font_size.max(1.0)));
        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        buffer.set_size(&mut self.font_system, Some(max_width.max(1.0)), None);
        buffer.set_text(&mut self.font_system, text, Attrs::new(), Shaping::Advanced);
        buffer.shape_until_scroll(&mut self.font_system, false);

        let mut glyphs = Vec::new();

        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                let physical = glyph.physical((0.0, 0.0), 1.0);
                let Some(image) = self
                    .swash
                    .get_image(&mut self.font_system, physical.cache_key)
                    .as_ref()
                else {
                    continue;
                };

                let width = image.placement.width.max(1) as u16;
                let height = image.placement.height.max(1) as u16;
                let cache_key = hash_glyph_key(physical.cache_key);
                let (region, inserted) = self.atlas.ensure_region(cache_key, width, height);
                if inserted {
                    let alpha = image_to_alpha(image.content, &image.data);
                    let expected_len = usize::from(width) * usize::from(height);
                    if alpha.len() >= expected_len && expected_len > 0 {
                        self.pending_uploads.push(GlyphUpload {
                            page: region.page,
                            x: region.x,
                            y: region.y,
                            width,
                            height,
                            alpha,
                        });
                    }
                }

                let gx = origin_x + physical.x as f32 + image.placement.left as f32;
                let gy = origin_y + run.line_y + physical.y as f32 - image.placement.top as f32;
                glyphs.push(GlyphQuad {
                    x: gx,
                    y: gy,
                    width: width as f32,
                    height: height as f32,
                    atlas_page: region.page,
                    atlas_x: region.x,
                    atlas_y: region.y,
                });
            }
        }

        ShapedText { glyphs }
    }

    pub fn drain_uploads(&mut self) -> Vec<GlyphUpload> {
        std::mem::take(&mut self.pending_uploads)
    }

    pub fn atlas_page_size(&self) -> u32 {
        self.atlas.page_size()
    }

    pub fn atlas_page_count(&self) -> usize {
        self.atlas.page_count()
    }
}

fn image_to_alpha(content: SwashContent, data: &[u8]) -> Vec<u8> {
    match content {
        SwashContent::Mask => data.to_vec(),
        SwashContent::Color => {
            let mut out = Vec::with_capacity(data.len() / 4);
            for px in data.chunks_exact(4) {
                out.push(px[3]);
            }
            out
        }
        SwashContent::SubpixelMask => {
            let mut out = Vec::with_capacity(data.len() / 3);
            for px in data.chunks_exact(3) {
                let avg = ((u16::from(px[0]) + u16::from(px[1]) + u16::from(px[2])) / 3) as u8;
                out.push(avg);
            }
            out
        }
    }
}

fn hash_glyph_key<T: Hash>(key: T) -> GlyphCacheKey {
    let mut h1 = DefaultHasher::new();
    key.hash(&mut h1);
    let v1 = h1.finish();

    let mut h2 = DefaultHasher::new();
    (v1 ^ 0x9E37_79B9_7F4A_7C15).hash(&mut h2);
    let v2 = h2.finish();

    GlyphCacheKey(((v1 as u128) << 64) | u128::from(v2))
}

#[allow(dead_code)]
fn _to_cosmic_color(c: [f32; 4]) -> CosmicColor {
    CosmicColor::rgba(
        (c[0] * 255.0) as u8,
        (c[1] * 255.0) as u8,
        (c[2] * 255.0) as u8,
        (c[3] * 255.0) as u8,
    )
}
