mod grid_pipeline;
mod rect_pipeline;
mod text_pipeline;

use std::sync::Arc;

use crate::gpu_runtime::GpuRuntime;
use crate::render::{PaintPrimitive, PipelineKind, RenderList};
pub use rect_pipeline::RectInstance;
pub use text_pipeline::GlyphInstance;

#[derive(Debug, Clone, Copy)]
pub struct GridDrawParams {
    pub zoom: f32,
    pub pan_world: [f32; 2],
    pub base_world_step: f32,
    pub dot_radius_px: f32,
    pub target_screen_step_px: f32,
    pub background_color: [f32; 4],
    pub dot_color: [f32; 4],
    pub scissor: Option<(u32, u32, u32, u32)>,
}

pub struct GpuRenderer {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    grid: grid_pipeline::GridPipeline,
    rect: rect_pipeline::RectPipeline,
    text: text_pipeline::TextPipeline,
    target_format: wgpu::TextureFormat,
    viewport: (u32, u32),
    logical_viewport: (f32, f32),
    scale_factor: f32,
    grid_params: Option<GridDrawParams>,
}

impl GpuRenderer {
    pub fn from_runtime(runtime: &GpuRuntime, atlas_page_size: u32) -> Self {
        let device = runtime.device_arc();
        let queue = runtime.queue_arc();
        let target_format = runtime.target_format();

        let grid = grid_pipeline::GridPipeline::new(device.as_ref(), target_format);
        let rect = rect_pipeline::RectPipeline::new(device.as_ref(), target_format);
        let text =
            text_pipeline::TextPipeline::new(device.as_ref(), target_format, atlas_page_size);

        Self {
            device,
            queue,
            grid,
            rect,
            text,
            target_format,
            viewport: (1, 1),
            logical_viewport: (1.0, 1.0),
            scale_factor: 1.0,
            grid_params: None,
        }
    }

    pub fn device(&self) -> &wgpu::Device {
        self.device.as_ref()
    }

    pub fn target_format(&self) -> wgpu::TextureFormat {
        self.target_format
    }

    pub fn update_viewport(
        &mut self,
        physical_width: u32,
        physical_height: u32,
        logical_width: f32,
        logical_height: f32,
        scale_factor: f32,
    ) {
        self.viewport = (physical_width.max(1), physical_height.max(1));
        self.logical_viewport = (logical_width.max(1.0), logical_height.max(1.0));
        self.scale_factor = scale_factor.max(0.1);
        self.rect
            .update_viewport(self.queue.as_ref(), self.logical_viewport.0, self.logical_viewport.1);
        self.text
            .update_viewport(self.queue.as_ref(), self.logical_viewport.0, self.logical_viewport.1);
    }

    pub fn set_grid_params(&mut self, params: Option<GridDrawParams>) {
        self.grid_params = params;
    }

    pub fn ensure_text_pages(&mut self, page_count: usize) {
        self.text
            .ensure_page_count(self.device.as_ref(), page_count);
    }

    pub fn upload_text_glyph(
        &mut self,
        page: u16,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
        alpha: &[u8],
    ) {
        self.text
            .upload_glyph(self.queue.as_ref(), page, x, y, width, height, alpha);
    }

    pub fn render(
        &mut self,
        surface_view: &wgpu::TextureView,
        render_list: &RenderList,
    ) {
        let debug_enabled = std::env::var("GREACT_DEBUG_RENDER")
            .map(|v| v != "0")
            .unwrap_or(false);
        let disable_scissor = std::env::var("GREACT_DISABLE_SCISSOR")
            .map(|v| v != "0")
            .unwrap_or(false);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("render_encoder") });

        let mut rect_instances = Vec::<RectInstance>::new();
        let mut text_instances = Vec::<GlyphInstance>::new();
        let mut rect_ranges: Vec<Option<(usize, usize)>> = vec![None; render_list.batches.len()];
        let mut text_ranges: Vec<Option<(usize, usize)>> = vec![None; render_list.batches.len()];

        for (batch_idx, batch) in render_list.batches.iter().enumerate() {
            let slice = &render_list.items[batch.start..batch.start + batch.count];
            match batch.key.pipeline {
                PipelineKind::RectSdf => {
                    let start = rect_instances.len();
                    for item in slice {
                        if let PaintPrimitive::Rect {
                            color,
                            border_color,
                            shadow_color,
                            shadow_offset,
                            sdf,
                        } = item.primitive
                        {
                            let shadow_pad = (sdf.shadow_blur * 2.0)
                                + sdf.shadow_spread.abs()
                                + shadow_offset[0].abs().max(shadow_offset[1].abs());
                            let shadow_pad = shadow_pad.max(0.0);
                            let draw_x = item.rect.x - shadow_pad;
                            let draw_y = item.rect.y - shadow_pad;
                            let draw_w = item.rect.width.max(1.0) + shadow_pad * 2.0;
                            let draw_h = item.rect.height.max(1.0) + shadow_pad * 2.0;

                            rect_instances.push(RectInstance {
                                border_width: sdf.border_width,
                                shadow_blur: sdf.shadow_blur,
                                shadow_spread: sdf.shadow_spread,
                                _pad0: 0.0,
                                position: [draw_x, draw_y],
                                size: [draw_w, draw_h],
                                shape_origin: [shadow_pad, shadow_pad],
                                shape_size: [item.rect.width.max(1.0), item.rect.height.max(1.0)],
                                shadow_offset,
                                radius: sdf.radius,
                                background_color: color,
                                border_color,
                                shadow_color,
                            });
                        }
                    }
                    let count = rect_instances.len() - start;
                    if count > 0 {
                        rect_ranges[batch_idx] = Some((start, count));
                    }
                }
                PipelineKind::Text => {
                    let start = text_instances.len();
                    for item in slice {
                        if let PaintPrimitive::Glyph {
                            atlas_x,
                            atlas_y,
                            color,
                            ..
                        } = item.primitive
                        {
                            text_instances.push(GlyphInstance {
                                position: [item.rect.x, item.rect.y],
                                size: [item.rect.width.max(1.0), item.rect.height.max(1.0)],
                                atlas_origin: [atlas_x as f32, atlas_y as f32],
                                atlas_size: [item.rect.width.max(1.0), item.rect.height.max(1.0)],
                                color,
                            });
                        }
                    }
                    let count = text_instances.len() - start;
                    if count > 0 {
                        text_ranges[batch_idx] = Some((start, count));
                    }
                }
                PipelineKind::Image => {}
            }
        }

        if !rect_instances.is_empty() {
            self.rect
                .upload_instances(self.device.as_ref(), self.queue.as_ref(), &rect_instances);
        }
        if !text_instances.is_empty() {
            self.text
                .upload_instances(self.device.as_ref(), self.queue.as_ref(), &text_instances);
        }

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("main_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: surface_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        if let Some(grid) = self.grid_params {
            if let Some((x, y, w, h)) = grid.scissor {
                let (sx, sy, sw, sh) = scale_scissor((x, y, w, h), self.scale_factor, self.viewport);
                pass.set_scissor_rect(sx, sy, sw, sh);
            } else {
                pass.set_scissor_rect(0, 0, self.viewport.0.max(1), self.viewport.1.max(1));
            }
            self.grid.update_uniforms(
                self.queue.as_ref(),
                &grid_pipeline::GridUniforms {
                    zoom: grid.zoom,
                    pan_world_x: grid.pan_world[0],
                    pan_world_y: grid.pan_world[1],
                    viewport_width: self.logical_viewport.0,
                    viewport_height: self.logical_viewport.1,
                    base_world_step: grid.base_world_step,
                    dot_radius_px: grid.dot_radius_px,
                    target_screen_step_px: grid.target_screen_step_px,
                    background_color: grid.background_color,
                    dot_color: grid.dot_color,
                },
            );
            self.grid.render(&mut pass);
        }

        for (batch_idx, batch) in render_list.batches.iter().enumerate() {
            if debug_enabled {
                eprintln!(
                    "[gpu] batch L{} Z{} {:?} tex={} scissor=({},{} {}x{}) count={}",
                    batch.key.layer_id,
                    batch.key.z_bucket,
                    batch.key.pipeline,
                    batch.key.texture_page,
                    batch.key.scissor.x,
                    batch.key.scissor.y,
                    batch.key.scissor.width,
                    batch.key.scissor.height,
                    batch.count
                );
            }
            if !disable_scissor {
                let (sx, sy, sw, sh) = scale_scissor(
                    (
                        batch.key.scissor.x,
                        batch.key.scissor.y,
                        batch.key.scissor.width,
                        batch.key.scissor.height,
                    ),
                    self.scale_factor,
                    self.viewport,
                );
                pass.set_scissor_rect(
                    sx,
                    sy,
                    sw,
                    sh,
                );
            }
            match batch.key.pipeline {
                PipelineKind::RectSdf => {
                    if let Some((start, count)) = rect_ranges[batch_idx] {
                        self.rect.draw(&mut pass, start, count);
                    }
                }
                PipelineKind::Text => {
                    if let Some((start, count)) = text_ranges[batch_idx] {
                        self.text
                            .draw(&mut pass, batch.key.texture_page, start, count);
                    }
                }
                PipelineKind::Image => {}
            }
        }

        drop(pass);
        self.queue.as_ref().submit([encoder.finish()]);
    }
}

fn scale_scissor(
    scissor: (u32, u32, u32, u32),
    scale_factor: f32,
    viewport: (u32, u32),
) -> (u32, u32, u32, u32) {
    let scale = scale_factor.max(0.1);
    let max_x = viewport.0.saturating_sub(1);
    let max_y = viewport.1.saturating_sub(1);
    let x = (((scissor.0 as f32) * scale).floor().max(0.0) as u32).min(max_x);
    let y = (((scissor.1 as f32) * scale).floor().max(0.0) as u32).min(max_y);
    let w = ((scissor.2 as f32) * scale).ceil().max(1.0) as u32;
    let h = ((scissor.3 as f32) * scale).ceil().max(1.0) as u32;

    let max_w = viewport.0.saturating_sub(x).max(1);
    let max_h = viewport.1.saturating_sub(y).max(1);
    (x, y, w.min(max_w), h.min(max_h))
}
