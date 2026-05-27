use bytemuck::{Pod, Zeroable};
use std::mem;
use wgpu::{
    BindGroup, BindGroupLayout, Buffer, Device, PipelineLayout, RenderPipeline, ShaderModule,
};

// Quad instance configuration parameters
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct QuadInstance {
    pub border_width: f32,    // Border width
    pub shadow_blur: f32,     // Shadow blur radius
    pub position: [f32; 2],   // Quad position (x, y)
    pub size: [f32; 2],       // Quad size (width, height)
    pub shadow_offset: [f32; 2], // Shadow offset (x, y)
    pub radius: [f32; 4],     // Corner radii (top-left, top-right, bottom-right, bottom-left)
    pub background_color: [f32; 4], // Background color (rgba)
    pub border_color: [f32; 4],    // Border color (rgba)
    pub shadow_color: [f32; 4],    // Shadow color (rgba)
}

// Quad render pipeline wrapper
pub struct QuadPipeline {
    pub render_pipeline: RenderPipeline,
    pub instance_buffer: Buffer,
    pub bind_group_layout: BindGroupLayout,
    pub bind_group: BindGroup,
}

impl QuadPipeline {
    // Create new quad render pipeline
    pub fn new(device: &Device) -> Self {
        // Embed shader code directly
        let shader_code = include_str!("quad.wgsl");

        // Create shader module
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("quad_shader"),
            source: wgpu::ShaderSource::Wgsl(shader_code.into()),
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[],
            label: Some("quad_bind_group_layout"),
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("quad_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create render pipeline
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("quad_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: mem::size_of::<QuadInstance>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32,
                        1 => Float32,
                        2 => Float32x2,
                        3 => Float32x2,
                        4 => Float32x2,
                        5 => Float32x4,
                        6 => Float32x4,
                        7 => Float32x4,
                        8 => Float32x4
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        // Create instance buffer with initial capacity for 1024 instances
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("quad_instance_buffer"),
            size: (mem::size_of::<QuadInstance>() * 1024) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("quad_bind_group"),
            layout: &bind_group_layout,
            entries: &[],
        });

        Self {
            render_pipeline,
            instance_buffer,
            bind_group_layout,
            bind_group,
        }
    }

    // Render quads with provided instance data
    pub fn render(
        &self,
        render_pass: &mut wgpu::RenderPass,
        queue: &wgpu::Queue,
        instances: &[QuadInstance],
    ) {
        // Update instance buffer
        self.update_instances(queue, instances);

        // Set pipeline and bind group
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
        
        // Draw 6 vertices per instance (two triangles)
        render_pass.draw(0..6, 0..instances.len() as u32);
    }

    // Update instance buffer with new data
    pub fn update_instances(&self, queue: &wgpu::Queue, instances: &[QuadInstance]) {
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(instances));
    }
}
