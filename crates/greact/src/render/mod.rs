pub mod atlas;
pub mod text;

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::layout::LayoutRect;
use crate::render::text::GlyphUpload;
use crate::render_tree::node::{ElementTag, NodeId, StyleFlags};
use crate::render_tree::RenderTree;
use crate::style::groups::TextStyleGroup;
use crate::text_system::SharedTextSystem;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum PipelineKind {
    RectSdf,
    Text,
    Image,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScissorKey {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct SdfRectParams {
    pub radius: [f32; 4],
    pub border_width: f32,
    pub shadow_blur: f32,
    pub shadow_spread: f32,
}

#[derive(Debug, Clone, Copy)]
pub enum PaintPrimitive {
    Rect {
        color: [f32; 4],
        border_color: [f32; 4],
        shadow_color: [f32; 4],
        shadow_offset: [f32; 2],
        sdf: SdfRectParams,
    },
    Glyph {
        atlas_page: u16,
        atlas_x: u16,
        atlas_y: u16,
        color: [f32; 4],
    },
    Image {
        image_id: u32,
    },
}

#[derive(Debug, Clone)]
pub struct PaintItem {
    pub node_id: NodeId,
    pub layer_id: u16,
    pub z_index: i32,
    pub pipeline: PipelineKind,
    pub scissor: ScissorKey,
    pub rect: LayoutRect,
    pub primitive: PaintPrimitive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BatchKey {
    pub layer_id: u16,
    pub pipeline: PipelineKind,
    pub z_bucket: i32,
    pub texture_page: u16,
    pub scissor: ScissorKey,
}

#[derive(Debug, Clone)]
pub struct RenderBatch {
    pub key: BatchKey,
    pub start: usize,
    pub count: usize,
}

#[derive(Debug, Default)]
pub struct RenderList {
    pub items: Vec<PaintItem>,
    pub batches: Vec<RenderBatch>,
}

pub struct RenderListBuilder {
    next_layer: u16,
    text: Arc<Mutex<SharedTextSystem>>,
    viewport: (u32, u32),
}

impl RenderListBuilder {
    pub fn new() -> Self {
        Self::with_text_system(Arc::new(Mutex::new(SharedTextSystem::new(2048))))
    }

    pub fn with_text_system(text_system: Arc<Mutex<SharedTextSystem>>) -> Self {
        Self {
            next_layer: 1,
            text: text_system,
            viewport: (1024, 768),
        }
    }

    pub fn set_viewport(&mut self, width: u32, height: u32) {
        self.viewport = (width.max(1), height.max(1));
    }

    pub fn build(&mut self, tree: &RenderTree, root: NodeId) -> RenderList {
        self.next_layer = 1;
        let mut items = Vec::new();
        let root_scissor = default_scissor(tree, root, self.viewport);
        self.walk(tree, root, 0, 0, 0.0, root_scissor, &mut items);
        if debug_enabled() {
            append_layer_debug_borders(&mut items);
            debug_log_items(&items, "build");
        }
        let batches = build_batches(&items);
        if debug_enabled() {
            eprintln!(
                "[render] batches={} items={} atlas_pages={}",
                batches.len(),
                items.len(),
                self.atlas_page_count()
            );
        }
        RenderList { items, batches }
    }

    pub fn drain_glyph_uploads(&mut self) -> Vec<GlyphUpload> {
        self.text
            .lock()
            .expect("shared text lock poisoned")
            .drain_uploads()
    }

    pub fn atlas_page_size(&self) -> u32 {
        self.text
            .lock()
            .expect("shared text lock poisoned")
            .atlas_page_size()
    }

    pub fn atlas_page_count(&self) -> usize {
        self.text
            .lock()
            .expect("shared text lock poisoned")
            .atlas_page_count()
    }

    fn walk(
        &mut self,
        tree: &RenderTree,
        node_id: NodeId,
        current_layer: u16,
        z: i32,
        inherited_scroll_y: f32,
        current_scissor: ScissorKey,
        out: &mut Vec<PaintItem>,
    ) {
        let Some(node) = tree.get(node_id) else { return };
        if node.display == crate::Display::None {
            return;
        }

        let mut rect = tree.get_layout_rect(node_id).unwrap_or_default();
        rect.y -= inherited_scroll_y;

        let node_scissor = clip_for_node(tree, node_id, rect, current_scissor);
        let layer_id = if requires_own_layer(tree, node_id) {
            let l = self.next_layer;
            self.next_layer = self.next_layer.saturating_add(1);
            l
        } else {
            current_layer
        };

        if node
            .style_flags
            .intersects(StyleFlags::BACKGROUND | StyleFlags::BORDER | StyleFlags::EFFECT)
        {
            let bg = tree.background_styles.get(node_id).map(|s| s.color).unwrap_or([0.0; 4]);
            let border = tree.border_styles.get(node_id).cloned().unwrap_or_default();
            let (shadow_blur, shadow_spread, shadow_offset, shadow_color) = tree
                .effect_styles
                .get(node_id)
                .and_then(|fx| fx.box_shadow)
                .map(|s| (s.blur, s.spread, [s.offset_x, s.offset_y], s.color.to_rgba()))
                .unwrap_or((0.0, 0.0, [0.0, 0.0], [0.0, 0.0, 0.0, 0.0]));

            out.push(PaintItem {
                node_id,
                layer_id,
                z_index: z,
                pipeline: PipelineKind::RectSdf,
                scissor: node_scissor,
                rect,
                primitive: PaintPrimitive::Rect {
                    color: bg,
                    border_color: border.color,
                    shadow_color,
                    shadow_offset,
                    sdf: SdfRectParams {
                        radius: border.radius,
                        border_width: border.width,
                        shadow_blur,
                        shadow_spread,
                    },
                },
            });
        }

        if node.tag == ElementTag::Image {
            if let Some(src) = tree.get_image_source(node_id) {
                if !node.style_flags.contains(StyleFlags::BACKGROUND) {
                    out.push(PaintItem {
                        node_id,
                        layer_id,
                        z_index: z,
                        pipeline: PipelineKind::RectSdf,
                        scissor: node_scissor,
                        rect,
                        primitive: PaintPrimitive::Rect {
                            color: image_placeholder_color(src),
                            border_color: [0.0, 0.0, 0.0, 0.12],
                            shadow_color: [0.0, 0.0, 0.0, 0.0],
                            shadow_offset: [0.0, 0.0],
                            sdf: SdfRectParams {
                                radius: [8.0; 4],
                                border_width: 1.0,
                                shadow_blur: 0.0,
                                shadow_spread: 0.0,
                            },
                        },
                    });
                }

                out.push(PaintItem {
                    node_id,
                    layer_id,
                    z_index: z + 1,
                    pipeline: PipelineKind::Image,
                    scissor: node_scissor,
                    rect,
                    primitive: PaintPrimitive::Image {
                        image_id: stable_image_id(src),
                    },
                });
            }
        }

        let mut rendered_input = false;
        if node.tag == ElementTag::Input {
            if let Some(input) = tree.get_input_state(node_id) {
                rendered_input = true;
                let focused = tree.focused_input() == Some(node_id);
                let text_style = tree
                    .text_style_groups
                    .get(node_id)
                    .cloned()
                    .unwrap_or_else(TextStyleGroup::default);
                let text_x = rect.x + 6.0;
                let text_y = rect.y + ((rect.height - text_style.font_size) * 0.5).max(0.0);
                let char_advance = (text_style.font_size * 0.6).max(1.0);
                let text_max_width = if node.style_flags.contains(StyleFlags::SIZE) {
                    (rect.width - 12.0).max(1.0)
                } else {
                    self.viewport.0 as f32
                };

                if focused {
                    if let Some((start, end)) = normalize_range(input.cursor, input.selection_anchor) {
                        let start_chars = count_chars_to_byte(&input.value, start);
                        let end_chars = count_chars_to_byte(&input.value, end);
                        let sx = text_x + start_chars as f32 * char_advance;
                        let sw = ((end_chars.saturating_sub(start_chars)) as f32 * char_advance).max(1.0);
                        out.push(PaintItem {
                            node_id,
                            layer_id,
                            z_index: z + 1,
                            pipeline: PipelineKind::RectSdf,
                            scissor: node_scissor,
                            rect: LayoutRect {
                                x: sx,
                                y: text_y,
                                width: sw,
                                height: text_style.font_size.max(10.0),
                            },
                            primitive: PaintPrimitive::Rect {
                                color: [0.22, 0.47, 0.93, 0.30],
                                border_color: [0.0, 0.0, 0.0, 0.0],
                                shadow_color: [0.0, 0.0, 0.0, 0.0],
                                shadow_offset: [0.0, 0.0],
                                sdf: SdfRectParams {
                                    radius: [2.0; 4],
                                    border_width: 0.0,
                                    shadow_blur: 0.0,
                                    shadow_spread: 0.0,
                                },
                            },
                        });
                    }

                    if !input.preedit.is_empty() {
                        let cursor_chars = count_chars_to_byte(&input.value, input.cursor);
                        let (range_start, range_end) = input
                            .preedit_range
                            .and_then(|(a, b)| normalize_range(a, b))
                            .unwrap_or((0, input.preedit.chars().count()));
                        let sx = text_x + (cursor_chars + range_start) as f32 * char_advance;
                        let sw = ((range_end.saturating_sub(range_start)) as f32 * char_advance).max(1.0);
                        out.push(PaintItem {
                            node_id,
                            layer_id,
                            z_index: z + 1,
                            pipeline: PipelineKind::RectSdf,
                            scissor: node_scissor,
                            rect: LayoutRect {
                                x: sx,
                                y: text_y,
                                width: sw,
                                height: text_style.font_size.max(10.0),
                            },
                            primitive: PaintPrimitive::Rect {
                                color: [0.95, 0.72, 0.18, 0.28],
                                border_color: [0.0, 0.0, 0.0, 0.0],
                                shadow_color: [0.0, 0.0, 0.0, 0.0],
                                shadow_offset: [0.0, 0.0],
                                sdf: SdfRectParams {
                                    radius: [2.0; 4],
                                    border_width: 0.0,
                                    shadow_blur: 0.0,
                                    shadow_spread: 0.0,
                                },
                            },
                        });
                    }
                }

                let is_placeholder = !focused && input.value.is_empty() && input.preedit.is_empty();
                let display_text = if is_placeholder {
                    input.placeholder.clone()
                } else {
                    compose_input_text(&input.value, input.cursor, &input.preedit)
                };
                if !display_text.is_empty() {
                    let glyph_color = if is_placeholder {
                        [
                            text_style.color[0],
                            text_style.color[1],
                            text_style.color[2],
                            text_style.color[3] * 0.55,
                        ]
                    } else {
                        text_style.color
                    };
                    let shaped = self
                        .text
                        .lock()
                        .expect("shared text lock poisoned")
                        .shape_text(
                        &display_text,
                        text_style.font_size,
                        text_style.line_height * text_style.font_size,
                        text_max_width,
                        text_x,
                        text_y,
                    );
                    for glyph in shaped.glyphs {
                        out.push(PaintItem {
                            node_id,
                            layer_id,
                            z_index: z + 2,
                            pipeline: PipelineKind::Text,
                            scissor: node_scissor,
                            rect: LayoutRect {
                                x: glyph.x,
                                y: glyph.y,
                                width: glyph.width,
                                height: glyph.height,
                            },
                            primitive: PaintPrimitive::Glyph {
                                atlas_page: glyph.atlas_page,
                                atlas_x: glyph.atlas_x,
                                atlas_y: glyph.atlas_y,
                                color: glyph_color,
                            },
                        });
                    }
                }

                if focused && input.blink_visible {
                    let cursor_chars = count_chars_to_byte(&input.value, input.cursor);
                    let caret_x = text_x + cursor_chars as f32 * char_advance;
                    out.push(PaintItem {
                        node_id,
                        layer_id,
                        z_index: z + 3,
                        pipeline: PipelineKind::RectSdf,
                        scissor: node_scissor,
                        rect: LayoutRect {
                            x: caret_x,
                            y: text_y,
                            width: 1.5,
                            height: text_style.font_size.max(10.0),
                        },
                        primitive: PaintPrimitive::Rect {
                            color: text_style.color,
                            border_color: [0.0, 0.0, 0.0, 0.0],
                            shadow_color: [0.0, 0.0, 0.0, 0.0],
                            shadow_offset: [0.0, 0.0],
                            sdf: SdfRectParams {
                                radius: [0.0; 4],
                                border_width: 0.0,
                                shadow_blur: 0.0,
                                shadow_spread: 0.0,
                            },
                        },
                    });
                }
            }
        }

        if !rendered_input {
            if let Some(text) = tree.get_text(node_id) {
                if !text.is_empty() {
                    let text_style = tree
                        .text_style_groups
                        .get(node_id)
                        .cloned()
                        .unwrap_or_else(TextStyleGroup::default);
                    let text_max_width = if node.style_flags.contains(StyleFlags::SIZE) {
                        rect.width.max(1.0)
                    } else {
                        self.viewport.0 as f32
                    };
                    let shaped = self
                        .text
                        .lock()
                        .expect("shared text lock poisoned")
                        .shape_text(
                        text,
                        text_style.font_size,
                        text_style.line_height * text_style.font_size,
                        text_max_width,
                        rect.x,
                        rect.y,
                    );
                    if debug_enabled() {
                        eprintln!(
                            "[text-shape] node={:?} text='{}' rect=({:.1},{:.1},{:.1},{:.1}) max_w={:.1} glyphs={}",
                            node_id,
                            text,
                            rect.x,
                            rect.y,
                            rect.width,
                            rect.height,
                            text_max_width,
                            shaped.glyphs.len()
                        );
                    }

                    for glyph in shaped.glyphs {
                        out.push(PaintItem {
                            node_id,
                            layer_id,
                            z_index: z + 1,
                            pipeline: PipelineKind::Text,
                            scissor: node_scissor,
                            rect: LayoutRect {
                                x: glyph.x,
                                y: glyph.y,
                                width: glyph.width,
                                height: glyph.height,
                            },
                            primitive: PaintPrimitive::Glyph {
                                atlas_page: glyph.atlas_page,
                                atlas_x: glyph.atlas_x,
                                atlas_y: glyph.atlas_y,
                                color: text_style.color,
                            },
                        });
                    }
                }
            }
        }

        let next_scroll_y = inherited_scroll_y + tree.get_scroll_offset(node_id);
        for &child in &node.children {
            self.walk(tree, child, layer_id, z + 1, next_scroll_y, node_scissor, out);
        }
    }
}

fn requires_own_layer(tree: &RenderTree, id: NodeId) -> bool {
    if let Some(ov) = tree.overflow_styles.get(id) {
        if !matches!(ov.overflow_x, crate::style::types::Overflow::Visible)
            || !matches!(ov.overflow_y, crate::style::types::Overflow::Visible)
        {
            return true;
        }
    }
    if let Some(fx) = tree.effect_styles.get(id) {
        if fx.opacity < 1.0 {
            return true;
        }
    }
    false
}

fn clip_for_node(tree: &RenderTree, id: NodeId, rect: LayoutRect, parent: ScissorKey) -> ScissorKey {
    let clips = tree
        .overflow_styles
        .get(id)
        .map(|ov| {
            !matches!(ov.overflow_x, crate::style::types::Overflow::Visible)
                || !matches!(ov.overflow_y, crate::style::types::Overflow::Visible)
        })
        .unwrap_or(false);

    if !clips {
        return parent;
    }

    intersect_scissor(
        parent,
        ScissorKey {
            x: rect.x.max(0.0).floor() as u32,
            y: rect.y.max(0.0).floor() as u32,
            width: rect.width.max(0.0).ceil() as u32,
            height: rect.height.max(0.0).ceil() as u32,
        },
    )
}

fn default_scissor(tree: &RenderTree, root: NodeId, viewport: (u32, u32)) -> ScissorKey {
    let _ = (tree, root);
    ScissorKey {
        x: 0,
        y: 0,
        width: viewport.0.max(1),
        height: viewport.1.max(1),
    }
}

fn intersect_scissor(a: ScissorKey, b: ScissorKey) -> ScissorKey {
    let x1 = a.x.max(b.x);
    let y1 = a.y.max(b.y);
    let x2 = a.x.saturating_add(a.width).min(b.x.saturating_add(b.width));
    let y2 = a.y.saturating_add(a.height).min(b.y.saturating_add(b.height));
    ScissorKey {
        x: x1,
        y: y1,
        width: x2.saturating_sub(x1),
        height: y2.saturating_sub(y1),
    }
}

fn build_batches(items: &[PaintItem]) -> Vec<RenderBatch> {
    if items.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::new();
    let mut start = 0usize;
    let mut current = key_for_item(&items[0]);

    for (idx, item) in items.iter().enumerate().skip(1) {
        let key = key_for_item(item);
        if key != current {
            out.push(RenderBatch {
                key: current,
                start,
                count: idx - start,
            });
            start = idx;
            current = key;
        }
    }

    out.push(RenderBatch {
        key: current,
        start,
        count: items.len() - start,
    });
    out
}

fn key_for_item(item: &PaintItem) -> BatchKey {
    BatchKey {
        layer_id: item.layer_id,
        pipeline: item.pipeline,
        z_bucket: item.z_index,
        texture_page: texture_page(item),
        scissor: item.scissor,
    }
}

fn texture_page(item: &PaintItem) -> u16 {
    match item.primitive {
        PaintPrimitive::Glyph { atlas_page, .. } => atlas_page,
        _ => 0,
    }
}

pub fn print_render_list_stats(list: &RenderList) {
    let mut by_pipeline = HashMap::<PipelineKind, usize>::new();
    for item in &list.items {
        *by_pipeline.entry(item.pipeline).or_default() += 1;
    }

    println!("=== Render List ===");
    println!("items: {}", list.items.len());
    println!("batches: {}", list.batches.len());
    for (pipeline, count) in by_pipeline {
        println!("  {:?}: {}", pipeline, count);
    }
}

fn debug_enabled() -> bool {
    std::env::var("GREACT_DEBUG_RENDER")
        .map(|v| v != "0")
        .unwrap_or(false)
}

fn debug_layer_color(layer: u16) -> [f32; 4] {
    let t = (layer as f32 * 0.618_034) % 1.0;
    let r = (0.5 + 0.5 * (t * 6.283).sin()).clamp(0.15, 1.0);
    let g = (0.5 + 0.5 * ((t + 0.33) * 6.283).sin()).clamp(0.15, 1.0);
    let b = (0.5 + 0.5 * ((t + 0.66) * 6.283).sin()).clamp(0.15, 1.0);
    [r, g, b, 1.0]
}

fn append_layer_debug_borders(items: &mut Vec<PaintItem>) {
    let mut bounds: HashMap<u16, LayoutRect> = HashMap::new();
    for item in items.iter() {
        let entry = bounds.entry(item.layer_id).or_insert(item.rect);
        *entry = union_rect(*entry, item.rect);
    }

    let template = items.first().cloned();
    for (layer_id, rect) in bounds {
        let Some(sample) = &template else { break };
        items.push(PaintItem {
            node_id: sample.node_id,
            layer_id,
            z_index: i32::MAX / 2,
            pipeline: PipelineKind::RectSdf,
            scissor: sample.scissor,
            rect,
            primitive: PaintPrimitive::Rect {
                color: [0.0, 0.0, 0.0, 0.0],
                border_color: debug_layer_color(layer_id),
                shadow_color: [0.0, 0.0, 0.0, 0.0],
                shadow_offset: [0.0, 0.0],
                sdf: SdfRectParams {
                    radius: [0.0; 4],
                    border_width: 2.0,
                    shadow_blur: 0.0,
                    shadow_spread: 0.0,
                },
            },
        });
    }
}

fn union_rect(a: LayoutRect, b: LayoutRect) -> LayoutRect {
    let x1 = a.x.min(b.x);
    let y1 = a.y.min(b.y);
    let x2 = (a.x + a.width).max(b.x + b.width);
    let y2 = (a.y + a.height).max(b.y + b.height);
    LayoutRect {
        x: x1,
        y: y1,
        width: (x2 - x1).max(0.0),
        height: (y2 - y1).max(0.0),
    }
}

fn debug_log_items(items: &[PaintItem], stage: &str) {
    static FRAME: AtomicUsize = AtomicUsize::new(0);
    let frame = FRAME.fetch_add(1, Ordering::Relaxed);
    if frame > 5 {
        return;
    }
    eprintln!("[render:{stage}] frame={frame} item_count={}", items.len());
    for (i, item) in items.iter().take(40).enumerate() {
        match item.primitive {
            PaintPrimitive::Glyph {
                atlas_page,
                atlas_x,
                atlas_y,
                ..
            } => eprintln!(
                "  [{i}] L{} Z{} TEXT rect=({:.1},{:.1},{:.1},{:.1}) sc=({},{} {}x{}) atlas=p{}@({}, {})",
                item.layer_id,
                item.z_index,
                item.rect.x,
                item.rect.y,
                item.rect.width,
                item.rect.height,
                item.scissor.x,
                item.scissor.y,
                item.scissor.width,
                item.scissor.height,
                atlas_page,
                atlas_x,
                atlas_y
            ),
            PaintPrimitive::Rect { .. } => eprintln!(
                "  [{i}] L{} Z{} RECT rect=({:.1},{:.1},{:.1},{:.1}) sc=({},{} {}x{})",
                item.layer_id,
                item.z_index,
                item.rect.x,
                item.rect.y,
                item.rect.width,
                item.rect.height,
                item.scissor.x,
                item.scissor.y,
                item.scissor.width,
                item.scissor.height
            ),
            PaintPrimitive::Image { image_id } => eprintln!(
                "  [{i}] L{} Z{} IMAGE#{} rect=({:.1},{:.1},{:.1},{:.1})",
                item.layer_id, item.z_index, image_id, item.rect.x, item.rect.y, item.rect.width, item.rect.height
            ),
        }
    }
}

fn stable_image_id(src: &str) -> u32 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    src.hash(&mut hasher);
    (hasher.finish() & 0xFFFF_FFFF) as u32
}

fn image_placeholder_color(src: &str) -> [f32; 4] {
    let id = stable_image_id(src);
    let r = ((id & 0xFF) as f32) / 255.0;
    let g = (((id >> 8) & 0xFF) as f32) / 255.0;
    let b = (((id >> 16) & 0xFF) as f32) / 255.0;
    [0.35 + r * 0.45, 0.35 + g * 0.45, 0.35 + b * 0.45, 1.0]
}

fn normalize_range(a: usize, b: usize) -> Option<(usize, usize)> {
    if a == b {
        return None;
    }
    Some((a.min(b), a.max(b)))
}

fn count_chars_to_byte(text: &str, idx: usize) -> usize {
    let idx = idx.min(text.len());
    if idx == text.len() {
        return text.chars().count();
    }
    let mut count = 0usize;
    for (i, _) in text.char_indices() {
        if i >= idx {
            break;
        }
        count += 1;
    }
    count
}

fn compose_input_text(value: &str, cursor: usize, preedit: &str) -> String {
    let cursor = cursor.min(value.len());
    let mut text = String::with_capacity(value.len() + preedit.len());
    text.push_str(&value[..cursor]);
    text.push_str(preedit);
    text.push_str(&value[cursor..]);
    text
}
