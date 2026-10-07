//! The wgpu half of candidate B that the libraries do not provide: two
//! pipelines, a frame uniform, and vertex buffers that grow.

use bakeoff_scene::Gpu;
use bytemuck::{Pod, Zeroable};

use crate::mesh;

pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
pub const SAMPLES: u32 = 4;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct FrameUniform {
    pub offset: [f32; 2],
    pub zoom: f32,
    pub _pad: f32,
    pub viewport: [f32; 2],
    pub _pad2: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct NoteInstance {
    /// Origin and size in canvas units.
    pub rect: [f32; 4],
    pub color: [u8; 4],
    pub radius: f32,
}

pub struct Pipelines {
    pub notes: wgpu::RenderPipeline,
    pub mesh: wgpu::RenderPipeline,
    pub frame: wgpu::Buffer,
    pub frame_bind: wgpu::BindGroup,
}

impl Pipelines {
    pub fn new(gpu: &Gpu) -> Self {
        let device = &gpu.device;
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        let frame = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frame"),
            size: size_of::<FrameUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frame"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let frame_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("frame"),
            layout: &bind_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: frame.as_entire_binding(),
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("canvas"),
            bind_group_layouts: &[Some(&bind_layout)],
            immediate_size: 0,
        });

        let pipeline = |label, vertex, fragment, topology, buffer: wgpu::VertexBufferLayout<'_>| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(vertex),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[Some(buffer)],
                },
                primitive: wgpu::PrimitiveState {
                    topology,
                    ..wgpu::PrimitiveState::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: SAMPLES,
                    ..Default::default()
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(fragment),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: FORMAT,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let notes = pipeline(
            "notes",
            "note_vertex",
            "note_fragment",
            wgpu::PrimitiveTopology::TriangleStrip,
            wgpu::VertexBufferLayout {
                array_stride: size_of::<NoteInstance>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array![0 => Float32x4, 1 => Unorm8x4, 2 => Float32],
            },
        );
        let mesh = pipeline(
            "mesh",
            "mesh_vertex",
            "mesh_fragment",
            wgpu::PrimitiveTopology::TriangleList,
            wgpu::VertexBufferLayout {
                array_stride: size_of::<mesh::Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Unorm8x4],
            },
        );
        Self {
            notes,
            mesh,
            frame,
            frame_bind,
        }
    }
}

/// A GPU buffer that is reallocated only when the data outgrows it.
pub struct GrowBuffer {
    pub buffer: wgpu::Buffer,
    usage: wgpu::BufferUsages,
}

impl GrowBuffer {
    pub fn new(gpu: &Gpu, usage: wgpu::BufferUsages) -> Self {
        let usage = usage | wgpu::BufferUsages::COPY_DST;
        Self {
            buffer: Self::allocate(gpu, usage, 1 << 16),
            usage,
        }
    }

    pub fn write<T: Pod>(&mut self, gpu: &Gpu, data: &[T]) {
        let bytes: &[u8] = bytemuck::cast_slice(data);
        if bytes.len() as u64 > self.buffer.size() {
            let size = (bytes.len() as u64).next_power_of_two();
            self.buffer = Self::allocate(gpu, self.usage, size);
        }
        gpu.queue.write_buffer(&self.buffer, 0, bytes);
    }

    fn allocate(gpu: &Gpu, usage: wgpu::BufferUsages, size: u64) -> wgpu::Buffer {
        gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("grow"),
            size,
            usage,
            mapped_at_creation: false,
        })
    }
}
