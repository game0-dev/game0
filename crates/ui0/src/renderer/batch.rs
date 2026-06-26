use std::ops::Range;

use super::scene::{ClipId, LayerId, OpacityId, PaintCommand, PaintScene, TransformId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BatchKind {
    Rect,
    Text,
    Image,
    Surface,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BatchKey {
    pub(crate) kind: BatchKind,
    pub(crate) clip: Option<ClipId>,
    pub(crate) transform: Option<TransformId>,
    pub(crate) opacity: Option<OpacityId>,
    pub(crate) layer: Option<LayerId>,
    pub(crate) texture: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PrimitiveRange {
    Rects(Range<usize>),
    Texts(Range<usize>),
    Images(Range<usize>),
    Surfaces(Range<usize>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RenderBatch {
    pub(crate) key: BatchKey,
    pub(crate) range: PrimitiveRange,
}

#[derive(Debug, Default)]
pub(crate) struct RenderBatchList {
    pub(crate) batches: Vec<RenderBatch>,
}

#[derive(Debug, Default)]
pub(crate) struct BatchCompiler {
    clip_stack: Vec<ClipId>,
    transform_stack: Vec<TransformId>,
    opacity_stack: Vec<OpacityId>,
    layer_stack: Vec<LayerId>,
    current: Option<OpenBatch>,
}

#[derive(Debug)]
struct OpenBatch {
    key: BatchKey,
    range: PrimitiveRange,
}

impl BatchCompiler {
    pub(crate) fn compile(scene: &PaintScene) -> RenderBatchList {
        let mut compiler = Self::default();
        let mut out = RenderBatchList::default();
        for command in &scene.commands {
            compiler.push_command(command, &mut out);
        }
        compiler.flush(&mut out);
        out
    }

    fn push_command(&mut self, command: &PaintCommand, out: &mut RenderBatchList) {
        match *command {
            PaintCommand::DrawRect(id) => self.push_primitive(BatchKind::Rect, id.0, out),
            PaintCommand::DrawText(id) => self.push_primitive(BatchKind::Text, id.0, out),
            PaintCommand::DrawImage(id) => self.push_primitive(BatchKind::Image, id.0, out),
            PaintCommand::DrawSurface(id) => {
                self.flush(out);
                let key = self.key(BatchKind::Surface);
                out.batches.push(RenderBatch {
                    key,
                    range: PrimitiveRange::Surfaces(id.0..id.0 + 1),
                });
            }
            PaintCommand::PushClip(id) => {
                self.flush(out);
                self.clip_stack.push(id);
            }
            PaintCommand::PopClip => {
                self.flush(out);
                self.clip_stack.pop();
            }
            PaintCommand::PushTransform(id) => {
                self.flush(out);
                self.transform_stack.push(id);
            }
            PaintCommand::PopTransform => {
                self.flush(out);
                self.transform_stack.pop();
            }
            PaintCommand::PushOpacity(id) => {
                self.flush(out);
                self.opacity_stack.push(id);
            }
            PaintCommand::PopOpacity => {
                self.flush(out);
                self.opacity_stack.pop();
            }
            PaintCommand::BeginLayer(id) => {
                self.flush(out);
                self.layer_stack.push(id);
            }
            PaintCommand::EndLayer => {
                self.flush(out);
                self.layer_stack.pop();
            }
        }
    }

    fn push_primitive(&mut self, kind: BatchKind, index: usize, out: &mut RenderBatchList) {
        let key = self.key(kind);
        let next_range = primitive_range(kind, index..index + 1);
        if let Some(current) = self.current.as_mut() {
            if current.key == key && extend_if_adjacent(&mut current.range, &next_range) {
                return;
            }
        }

        self.flush(out);
        self.current = Some(OpenBatch {
            key,
            range: next_range,
        });
    }

    fn key(&self, kind: BatchKind) -> BatchKey {
        BatchKey {
            kind,
            clip: self.clip_stack.last().copied(),
            transform: self.transform_stack.last().copied(),
            opacity: self.opacity_stack.last().copied(),
            layer: self.layer_stack.last().copied(),
            texture: None,
        }
    }

    fn flush(&mut self, out: &mut RenderBatchList) {
        if let Some(current) = self.current.take() {
            out.batches.push(RenderBatch {
                key: current.key,
                range: current.range,
            });
        }
    }
}

fn primitive_range(kind: BatchKind, range: Range<usize>) -> PrimitiveRange {
    match kind {
        BatchKind::Rect => PrimitiveRange::Rects(range),
        BatchKind::Text => PrimitiveRange::Texts(range),
        BatchKind::Image => PrimitiveRange::Images(range),
        BatchKind::Surface => PrimitiveRange::Surfaces(range),
    }
}

fn extend_if_adjacent(current: &mut PrimitiveRange, next: &PrimitiveRange) -> bool {
    match (current, next) {
        (PrimitiveRange::Rects(current), PrimitiveRange::Rects(next))
        | (PrimitiveRange::Texts(current), PrimitiveRange::Texts(next))
        | (PrimitiveRange::Images(current), PrimitiveRange::Images(next))
        | (PrimitiveRange::Surfaces(current), PrimitiveRange::Surfaces(next))
            if current.end == next.start =>
        {
            current.end = next.end;
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::scene::{
        ClipState, PaintCommand, PaintScene, RectPrimitive, TextPrimitive,
    };
    use crate::ui_tree::{Color, Corners, Edges, LayoutRect, NodeId};

    fn rect(node: NodeId) -> RectPrimitive {
        RectPrimitive {
            node,
            rect: LayoutRect {
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 10.0,
            },
            fill: Color::rgb_u8(1, 2, 3),
            border_color: Color::transparent(),
            border_width: Edges::default(),
            radius: Corners::default(),
            opacity: 1.0,
        }
    }

    fn text(node: NodeId) -> TextPrimitive {
        TextPrimitive {
            node,
            rect: LayoutRect {
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 10.0,
            },
            text: "x".to_string(),
            color: Color::rgb_u8(1, 2, 3),
            font_size: 12.0,
            opacity: 1.0,
        }
    }

    #[test]
    fn continuous_rects_batch_even_when_they_may_overlap() {
        let mut scene = PaintScene::default();
        let node = NodeId::default();
        let a = scene.push_rect(rect(node));
        let b = scene.push_rect(rect(node));
        scene.commands.push(PaintCommand::DrawRect(a));
        scene.commands.push(PaintCommand::DrawRect(b));

        let batches = BatchCompiler::compile(&scene);

        assert_eq!(batches.batches.len(), 1);
        assert_eq!(batches.batches[0].range, PrimitiveRange::Rects(0..2));
    }

    #[test]
    fn text_between_rects_prevents_rect_batch_merging() {
        let mut scene = PaintScene::default();
        let node = NodeId::default();
        let a = scene.push_rect(rect(node));
        let t = scene.push_text(text(node));
        let b = scene.push_rect(rect(node));
        scene.commands.push(PaintCommand::DrawRect(a));
        scene.commands.push(PaintCommand::DrawText(t));
        scene.commands.push(PaintCommand::DrawRect(b));

        let batches = BatchCompiler::compile(&scene);

        assert_eq!(batches.batches.len(), 3);
        assert_eq!(batches.batches[0].range, PrimitiveRange::Rects(0..1));
        assert_eq!(batches.batches[1].range, PrimitiveRange::Texts(0..1));
        assert_eq!(batches.batches[2].range, PrimitiveRange::Rects(1..2));
    }

    #[test]
    fn clip_is_batch_barrier() {
        let mut scene = PaintScene::default();
        let node = NodeId::default();
        let clip = scene.push_clip(ClipState {
            rect: LayoutRect {
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 10.0,
            },
        });
        let a = scene.push_rect(rect(node));
        let b = scene.push_rect(rect(node));
        scene.commands.push(PaintCommand::DrawRect(a));
        scene.commands.push(PaintCommand::PushClip(clip));
        scene.commands.push(PaintCommand::DrawRect(b));
        scene.commands.push(PaintCommand::PopClip);

        let batches = BatchCompiler::compile(&scene);

        assert_eq!(batches.batches.len(), 2);
        assert_eq!(batches.batches[0].range, PrimitiveRange::Rects(0..1));
        assert_eq!(batches.batches[1].range, PrimitiveRange::Rects(1..2));
    }
}
