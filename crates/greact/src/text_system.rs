use crate::render::text::{GlyphUpload, ShapedText, TextEngine};

pub struct SharedTextSystem {
    engine: TextEngine,
}

impl SharedTextSystem {
    pub fn new(atlas_page_size: i32) -> Self {
        Self {
            engine: TextEngine::new(atlas_page_size),
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
        self.engine
            .shape_text(text, font_size, line_height, max_width, origin_x, origin_y)
    }

    pub fn drain_uploads(&mut self) -> Vec<GlyphUpload> {
        self.engine.drain_uploads()
    }

    pub fn atlas_page_size(&self) -> u32 {
        self.engine.atlas_page_size()
    }

    pub fn atlas_page_count(&self) -> usize {
        self.engine.atlas_page_count()
    }
}
