pub(crate) mod batch;
pub(crate) mod cache;
mod gpu;
pub(crate) mod scene;
mod text;

use std::sync::Arc;

use glyphon::{
    Attrs, Buffer, Cache, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache, TextArea,
    TextAtlas, TextBounds, TextRenderer, Viewport,
};
use slotmap::SecondaryMap;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BlendState, Buffer as WgpuBuffer, BufferBindingType,
    BufferDescriptor, BufferUsages, ColorTargetState, ColorWrites, CommandEncoderDescriptor,
    CompositeAlphaMode, CurrentSurfaceTexture, Device, DeviceDescriptor, FragmentState, Instance,
    InstanceDescriptor, LoadOp, MultisampleState, Operations, PipelineLayoutDescriptor,
    PresentMode, PrimitiveState, Queue, RenderPass, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions,
    ShaderModuleDescriptor, ShaderSource, StoreOp, Surface, SurfaceConfiguration, TextureUsages,
    TextureViewDescriptor, VertexState,
};
use winit::window::Window;

use self::batch::{BatchCompiler, PrimitiveRange};
use self::gpu::{
    color_to_array, preferred_surface_format, srgb_to_linear, RectInstance, RectUniform,
};
use self::scene::{PaintScene, RectPrimitive, TextPrimitive};
use self::text::{glyph_color, TextNodeState};
use crate::ui_tree::{DirtyFlags, LayoutRect, NodeId, UiTree};

pub struct UiRenderer {
    device: Device,
    queue: Queue,
    surface: Surface<'static>,
    config: SurfaceConfiguration,
    scale_factor: f32,
    rect_pipeline: RenderPipeline,
    rect_uniform_buffer: WgpuBuffer,
    rect_bind_group: BindGroup,
    rect_instance_buffer: WgpuBuffer,
    rect_instance_capacity: usize,

    font_system: FontSystem,
    swash_cache: SwashCache,
    text_viewport: Viewport,
    text_atlas: TextAtlas,
    text_renderer: TextRenderer,

    previous_scene: Option<PaintScene>,
    text_states: SecondaryMap<NodeId, TextNodeState>,
}

impl UiRenderer {
    pub(crate) fn new(window: Arc<Window>) -> Self {
        pollster::block_on(Self::new_async(window))
    }

    async fn new_async(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let scale_factor = window.scale_factor() as f32;
        let instance = Instance::new(InstanceDescriptor::new_without_display_handle());
        let surface = instance
            .create_surface(window)
            .expect("failed to create wgpu surface");
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..RequestAdapterOptions::default()
            })
            .await
            .expect("failed to request wgpu adapter");
        let (device, queue) = adapter
            .request_device(&DeviceDescriptor::default())
            .await
            .expect("failed to request wgpu device");

        let caps = surface.get_capabilities(&adapter);
        let format = preferred_surface_format(&caps.formats);
        let present_mode = if caps.present_modes.contains(&PresentMode::Fifo) {
            PresentMode::Fifo
        } else {
            caps.present_modes[0]
        };
        let alpha_mode = caps
            .alpha_modes
            .iter()
            .copied()
            .find(|mode| *mode == CompositeAlphaMode::Opaque)
            .unwrap_or(caps.alpha_modes[0]);
        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let rect_uniform_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("ui0 rect uniform buffer"),
            size: std::mem::size_of::<RectUniform>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let rect_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("ui0 rect bind group layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let rect_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("ui0 rect bind group"),
            layout: &rect_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: rect_uniform_buffer.as_entire_binding(),
            }],
        });
        let rect_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("ui0 rect shader"),
            source: ShaderSource::Wgsl(include_str!("renderer/rect.wgsl").into()),
        });
        let rect_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("ui0 rect pipeline layout"),
            bind_group_layouts: &[Some(&rect_bind_group_layout)],
            immediate_size: 0,
        });
        let rect_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("ui0 rect pipeline"),
            layout: Some(&rect_pipeline_layout),
            vertex: VertexState {
                module: &rect_shader,
                entry_point: Some("vs_main"),
                buffers: &[RectInstance::layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &rect_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let rect_instance_capacity = 256;
        let rect_instance_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("ui0 rect instance buffer"),
            size: (rect_instance_capacity * std::mem::size_of::<RectInstance>()) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let text_cache = Cache::new(&device);
        let text_viewport = Viewport::new(&device, &text_cache);
        let mut text_atlas = TextAtlas::new(&device, &queue, &text_cache, format);
        let text_renderer =
            TextRenderer::new(&mut text_atlas, &device, MultisampleState::default(), None);

        Self {
            device,
            queue,
            surface,
            config,
            scale_factor,
            rect_pipeline,
            rect_uniform_buffer,
            rect_bind_group,
            rect_instance_buffer,
            rect_instance_capacity,
            font_system,
            swash_cache,
            text_viewport,
            text_atlas,
            text_renderer,
            previous_scene: None,
            text_states: SecondaryMap::new(),
        }
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        if self.config.width == width && self.config.height == height {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    pub(crate) fn set_scale_factor(&mut self, scale_factor: f32) {
        self.scale_factor = scale_factor.max(0.01);
    }

    pub(crate) fn render(&mut self, tree: &mut UiTree) {
        if self.config.width == 0 || self.config.height == 0 {
            return;
        }

        let scene = PaintScene::build(tree, self.previous_scene.as_ref());
        let batches = BatchCompiler::compile(&scene);
        tree.clear_dirty_flags(
            DirtyFlags::STRUCTURE
                | DirtyFlags::STYLE
                | DirtyFlags::PRE_PAINT
                | DirtyFlags::PAINT
                | DirtyFlags::COMPOSITE
                | DirtyFlags::TEXT,
        );

        let frame = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(frame) | CurrentSurfaceTexture::Suboptimal(frame) => {
                frame
            }
            CurrentSurfaceTexture::Lost | CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            CurrentSurfaceTexture::Timeout | CurrentSurfaceTexture::Occluded => return,
            CurrentSurfaceTexture::Validation => {
                panic!("ui0 renderer surface acquisition failed validation")
            }
        };
        let view = frame.texture.create_view(&TextureViewDescriptor::default());

        self.queue.write_buffer(
            &self.rect_uniform_buffer,
            0,
            bytemuck::bytes_of(&RectUniform {
                screen_size: [self.config.width as f32, self.config.height as f32],
                _pad: [0.0; 2],
            }),
        );
        self.text_viewport.update(
            &self.queue,
            Resolution {
                width: self.config.width,
                height: self.config.height,
            },
        );

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("ui0 render encoder"),
            });
        self.upload_rect_instances(&scene.rects);
        self.prepare_text_renderer(&scene.texts);

        let color_attachments = [Some(RenderPassColorAttachment {
            view: &view,
            depth_slice: None,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(wgpu::Color {
                    r: srgb_to_linear(18.0 / 255.0) as f64,
                    g: srgb_to_linear(20.0 / 255.0) as f64,
                    b: srgb_to_linear(25.0 / 255.0) as f64,
                    a: 1.0,
                }),
                store: StoreOp::Store,
            },
        })];
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("ui0 render pass"),
            color_attachments: &color_attachments,
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        for batch in &batches.batches {
            match &batch.range {
                PrimitiveRange::Rects(range) => {
                    let scissor = batch.key.clip.map(|clip| scene.clips[clip.0].rect);
                    self.draw_rect_batch(range.clone(), scissor, &mut pass);
                }
                PrimitiveRange::Texts(_) => {}
                PrimitiveRange::Images(_) => {}
                PrimitiveRange::Surfaces(_) => {}
            }
        }
        self.draw_prepared_texts(&mut pass);
        drop(pass);

        self.queue.submit(Some(encoder.finish()));
        frame.present();
        self.text_atlas.trim();
        self.previous_scene = Some(scene);
    }

    fn upload_rect_instances(&mut self, rects: &[RectPrimitive]) {
        if rects.is_empty() {
            return;
        }
        let rect_instances = rects
            .iter()
            .map(|rect| RectInstance {
                rect: [
                    rect.rect.x * self.scale_factor,
                    rect.rect.y * self.scale_factor,
                    rect.rect.width * self.scale_factor,
                    rect.rect.height * self.scale_factor,
                ],
                fill: color_to_array(rect.fill, rect.opacity),
                border_color: color_to_array(rect.border_color, rect.opacity),
                border_width: [
                    rect.border_width.left * self.scale_factor,
                    rect.border_width.right * self.scale_factor,
                    rect.border_width.top * self.scale_factor,
                    rect.border_width.bottom * self.scale_factor,
                ],
                radius: [
                    rect.radius.top_left * self.scale_factor,
                    rect.radius.top_right * self.scale_factor,
                    rect.radius.bottom_right * self.scale_factor,
                    rect.radius.bottom_left * self.scale_factor,
                ],
            })
            .collect::<Vec<_>>();
        self.ensure_rect_capacity(rect_instances.len());
        self.queue.write_buffer(
            &self.rect_instance_buffer,
            0,
            bytemuck::cast_slice(&rect_instances),
        );
    }

    fn draw_rect_batch(
        &mut self,
        range: std::ops::Range<usize>,
        scissor: Option<LayoutRect>,
        pass: &mut RenderPass<'_>,
    ) {
        if range.is_empty() {
            return;
        }
        let Some(scissor) = self.physical_scissor(scissor) else {
            return;
        };
        pass.set_scissor_rect(scissor.x, scissor.y, scissor.width, scissor.height);
        pass.set_pipeline(&self.rect_pipeline);
        pass.set_bind_group(0, &self.rect_bind_group, &[]);
        let stride = std::mem::size_of::<RectInstance>() as u64;
        let start = range.start as u64 * stride;
        let end = range.end as u64 * stride;
        pass.set_vertex_buffer(0, self.rect_instance_buffer.slice(start..end));
        pass.draw(0..6, 0..(range.end - range.start) as u32);
    }

    fn prepare_text_renderer(&mut self, texts: &[TextPrimitive]) {
        if texts.is_empty() {
            return;
        }
        self.prepare_text_buffers(texts);
        let text_states = &self.text_states;
        let text_areas = texts
            .iter()
            .filter_map(|text| {
                let state = text_states.get(text.node)?;
                Some(TextArea {
                    buffer: &state.buffer,
                    left: text.rect.x * self.scale_factor,
                    top: text.rect.y * self.scale_factor,
                    scale: 1.0,
                    bounds: TextBounds {
                        left: (text.rect.x * self.scale_factor).floor() as i32,
                        top: (text.rect.y * self.scale_factor).floor() as i32,
                        right: ((text.rect.x + text.rect.width) * self.scale_factor).ceil() as i32,
                        bottom: ((text.rect.y + text.rect.height) * self.scale_factor).ceil()
                            as i32,
                    },
                    default_color: glyph_color(text.color, text.opacity),
                    custom_glyphs: &[],
                })
            })
            .collect::<Vec<_>>();

        self.text_renderer
            .prepare(
                &self.device,
                &self.queue,
                &mut self.font_system,
                &mut self.text_atlas,
                &self.text_viewport,
                text_areas,
                &mut self.swash_cache,
            )
            .expect("failed to prepare ui0 text");
    }

    fn draw_prepared_texts(&mut self, pass: &mut RenderPass<'_>) {
        pass.set_scissor_rect(0, 0, self.config.width, self.config.height);
        self.text_renderer
            .render(&self.text_atlas, &self.text_viewport, pass)
            .expect("failed to render ui0 text");
    }

    fn physical_scissor(&self, rect: Option<LayoutRect>) -> Option<PhysicalScissor> {
        let Some(rect) = rect else {
            return Some(PhysicalScissor {
                x: 0,
                y: 0,
                width: self.config.width,
                height: self.config.height,
            });
        };

        let x0 = (rect.x * self.scale_factor)
            .floor()
            .clamp(0.0, self.config.width as f32);
        let y0 = (rect.y * self.scale_factor)
            .floor()
            .clamp(0.0, self.config.height as f32);
        let x1 = ((rect.x + rect.width) * self.scale_factor)
            .ceil()
            .clamp(0.0, self.config.width as f32);
        let y1 = ((rect.y + rect.height) * self.scale_factor)
            .ceil()
            .clamp(0.0, self.config.height as f32);
        let width = (x1 - x0).max(0.0) as u32;
        let height = (y1 - y0).max(0.0) as u32;
        (width > 0 && height > 0).then_some(PhysicalScissor {
            x: x0 as u32,
            y: y0 as u32,
            width,
            height,
        })
    }

    fn prepare_text_buffers(&mut self, texts: &[TextPrimitive]) {
        for text in texts {
            let font_size = text.font_size.max(1.0);
            let physical_font_size = (font_size * self.scale_factor).max(1.0);
            let width = (text.rect.width * self.scale_factor).max(1.0);
            let height = (text.rect.height * self.scale_factor).max(physical_font_size * 1.2);
            let should_insert = !self.text_states.contains_key(text.node);
            if should_insert {
                let buffer = Buffer::new(
                    &mut self.font_system,
                    Metrics::new(font_size, font_size * 1.2),
                );
                self.text_states.insert(
                    text.node,
                    TextNodeState {
                        buffer,
                        text: String::new(),
                        font_size: 0.0,
                        width: 0.0,
                        height: 0.0,
                    },
                );
            }

            let state = self.text_states.get_mut(text.node).unwrap();
            if state.font_size != physical_font_size {
                state.buffer.set_metrics(
                    &mut self.font_system,
                    Metrics::new(physical_font_size, physical_font_size * 1.2),
                );
                state.font_size = physical_font_size;
            }
            if state.width != width || state.height != height {
                state
                    .buffer
                    .set_size(&mut self.font_system, Some(width), Some(height));
                state.width = width;
                state.height = height;
            }
            if state.text != text.text {
                state.buffer.set_text(
                    &mut self.font_system,
                    &text.text,
                    &Attrs::new().family(Family::SansSerif),
                    Shaping::Advanced,
                    None,
                );
                state.text = text.text.clone();
            }
            state
                .buffer
                .shape_until_scroll(&mut self.font_system, false);
        }
    }

    fn ensure_rect_capacity(&mut self, len: usize) {
        if len <= self.rect_instance_capacity {
            return;
        }
        self.rect_instance_capacity = len.next_power_of_two();
        self.rect_instance_buffer = self.device.create_buffer(&BufferDescriptor {
            label: Some("ui0 rect instance buffer"),
            size: (self.rect_instance_capacity * std::mem::size_of::<RectInstance>()) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
    }
}

struct PhysicalScissor {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}
