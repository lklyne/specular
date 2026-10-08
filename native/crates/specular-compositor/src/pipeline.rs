//! Render pipelines, layouts and the shared sampler, built once per
//! compositor.

use crate::gpu_types::{FRAME_UNIFORMS_SIZE, MeshVertex, QuadInstance, ShapeInstance};

/// WGSL source for all pipelines.
pub(crate) const SHADER_SOURCE: &str = include_str!("shaders/canvas.wgsl");

/// Samples per pixel of the scene pass. Tessellated edges have no
/// antialiasing of their own (ADR 0039).
pub(crate) const SCENE_SAMPLES: u32 = 4;

/// The pipelines of one pass, all built for the same sample count.
#[derive(Debug)]
pub(crate) struct PassPipelines {
    pub(crate) grid: wgpu::RenderPipeline,
    pub(crate) quad: wgpu::RenderPipeline,
    pub(crate) shape: wgpu::RenderPipeline,
    pub(crate) mesh: wgpu::RenderPipeline,
    /// Meshes multiplied into the target ([`specular_scene::Blend::Multiply`]).
    pub(crate) mesh_multiply: wgpu::RenderPipeline,
}

/// Long-lived GPU objects that do not depend on scene content.
#[derive(Debug)]
pub(crate) struct Pipelines {
    /// For the multisampled scene pass.
    pub(crate) multisampled: PassPipelines,
    pub(crate) frame_layout: wgpu::BindGroupLayout,
    pub(crate) texture_layout: wgpu::BindGroupLayout,
    pub(crate) sampler: wgpu::Sampler,
}

impl Pipelines {
    pub(crate) fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("canvas-shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SOURCE.into()),
        });
        let frame_layout = frame_layout(device);
        let texture_layout = texture_layout(device);
        let untextured = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("untextured-pipeline-layout"),
            bind_group_layouts: &[Some(&frame_layout)],
            immediate_size: 0,
        });
        let textured = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("textured-pipeline-layout"),
            bind_group_layouts: &[Some(&frame_layout), Some(&texture_layout)],
            immediate_size: 0,
        });
        let pass = |samples| {
            PassPipelines::new(
                device,
                &PassSpec {
                    shader: &shader,
                    untextured: &untextured,
                    textured: &textured,
                    target_format,
                    samples,
                },
            )
        };
        let multisampled = pass(SCENE_SAMPLES);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("page-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            // Images carry mip levels; page textures have one and ignore it.
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..wgpu::SamplerDescriptor::default()
        });
        Self {
            multisampled,
            frame_layout,
            texture_layout,
            sampler,
        }
    }

    /// A bind group sampling `texture` with the shared sampler.
    pub(crate) fn texture_bind_group(
        &self,
        device: &wgpu::Device,
        texture: &wgpu::Texture,
    ) -> wgpu::BindGroup {
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("page-texture"),
            layout: &self.texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }
}

/// What every pipeline of one pass shares.
struct PassSpec<'a> {
    shader: &'a wgpu::ShaderModule,
    untextured: &'a wgpu::PipelineLayout,
    textured: &'a wgpu::PipelineLayout,
    target_format: wgpu::TextureFormat,
    samples: u32,
}

impl PassPipelines {
    fn new(device: &wgpu::Device, pass: &PassSpec<'_>) -> Self {
        let instanced = |stride: usize, attributes| wgpu::VertexBufferLayout {
            array_stride: stride as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes,
        };
        let blend = Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING);
        let strip = wgpu::PrimitiveTopology::TriangleStrip;
        let list = wgpu::PrimitiveTopology::TriangleList;
        let grid = PipelineSpec {
            label: "grid-pipeline",
            layout: pass.untextured,
            entries: ("vs_grid", "fs_grid"),
            buffers: &[],
            topology: list,
            blend: None,
        };
        let quad = PipelineSpec {
            label: "quad-pipeline",
            layout: pass.textured,
            entries: ("vs_quad", "fs_quad"),
            buffers: &[Some(instanced(
                size_of::<QuadInstance>(),
                &QuadInstance::ATTRIBUTES,
            ))],
            topology: strip,
            blend,
        };
        let shape = PipelineSpec {
            label: "shape-pipeline",
            layout: pass.untextured,
            entries: ("vs_shape", "fs_shape"),
            buffers: &[Some(instanced(
                size_of::<ShapeInstance>(),
                &ShapeInstance::ATTRIBUTES,
            ))],
            topology: strip,
            blend,
        };
        let mesh = PipelineSpec {
            label: "mesh-pipeline",
            layout: pass.untextured,
            entries: ("vs_mesh", "fs_mesh"),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: size_of::<MeshVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &MeshVertex::ATTRIBUTES,
            })],
            topology: list,
            blend,
        };
        // The target's colour times the fragment's, and its alpha kept.
        let multiply = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Dst,
                dst_factor: wgpu::BlendFactor::Zero,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Zero,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        };
        let mesh_multiply = PipelineSpec {
            label: "mesh-multiply-pipeline",
            entries: ("vs_mesh", "fs_mesh_multiply"),
            blend: Some(multiply),
            ..mesh
        };
        Self {
            grid: render_pipeline(device, pass, &grid),
            quad: render_pipeline(device, pass, &quad),
            shape: render_pipeline(device, pass, &shape),
            mesh: render_pipeline(device, pass, &mesh),
            mesh_multiply: render_pipeline(device, pass, &mesh_multiply),
        }
    }
}

#[derive(Clone, Copy)]
struct PipelineSpec<'a> {
    label: &'static str,
    layout: &'a wgpu::PipelineLayout,
    /// Vertex and fragment entry points.
    entries: (&'static str, &'static str),
    buffers: &'a [Option<wgpu::VertexBufferLayout<'a>>],
    topology: wgpu::PrimitiveTopology,
    blend: Option<wgpu::BlendState>,
}

fn render_pipeline(
    device: &wgpu::Device,
    pass: &PassSpec<'_>,
    spec: &PipelineSpec<'_>,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(spec.label),
        layout: Some(spec.layout),
        vertex: wgpu::VertexState {
            module: pass.shader,
            entry_point: Some(spec.entries.0),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: spec.buffers,
        },
        primitive: wgpu::PrimitiveState {
            topology: spec.topology,
            ..wgpu::PrimitiveState::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: pass.samples,
            ..wgpu::MultisampleState::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: pass.shader,
            entry_point: Some(spec.entries.1),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: pass.target_format,
                blend: spec.blend,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

/// Group 0: the per-frame uniform block.
fn frame_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("frame-layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(FRAME_UNIFORMS_SIZE),
            },
            count: None,
        }],
    })
}

/// Group 1: one page layer's texture and the shared sampler.
fn texture_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("page-texture-layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}
