//! Render pipelines, layouts and the shared sampler, built once per
//! compositor.

use crate::gpu_types::{FRAME_UNIFORMS_SIZE, QuadInstance, ShapeInstance};

/// WGSL source for all pipelines.
pub(crate) const SHADER_SOURCE: &str = include_str!("shaders/canvas.wgsl");

/// Long-lived GPU objects that do not depend on scene content.
#[derive(Debug)]
pub(crate) struct Pipelines {
    pub(crate) grid: wgpu::RenderPipeline,
    pub(crate) quad: wgpu::RenderPipeline,
    pub(crate) shape: wgpu::RenderPipeline,
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
        let grid_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("grid-pipeline-layout"),
            bind_group_layouts: &[Some(&frame_layout)],
            immediate_size: 0,
        });
        let quad_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("quad-pipeline-layout"),
            bind_group_layouts: &[Some(&frame_layout), Some(&texture_layout)],
            immediate_size: 0,
        });
        let shape_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shape-pipeline-layout"),
            bind_group_layouts: &[Some(&frame_layout)],
            immediate_size: 0,
        });
        let instance_layout = wgpu::VertexBufferLayout {
            array_stride: size_of::<QuadInstance>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &QuadInstance::ATTRIBUTES,
        };
        let grid = render_pipeline(
            device,
            &PipelineSpec {
                label: "grid-pipeline",
                layout: &grid_layout,
                shader: &shader,
                vertex_entry: "vs_grid",
                fragment_entry: "fs_grid",
                buffers: &[],
                topology: wgpu::PrimitiveTopology::TriangleList,
                blend: None,
                target_format,
            },
        );
        let quad = render_pipeline(
            device,
            &PipelineSpec {
                label: "quad-pipeline",
                layout: &quad_layout,
                shader: &shader,
                vertex_entry: "vs_quad",
                fragment_entry: "fs_quad",
                buffers: &[Some(instance_layout)],
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                target_format,
            },
        );
        let shape = render_pipeline(
            device,
            &PipelineSpec {
                label: "shape-pipeline",
                layout: &shape_layout,
                shader: &shader,
                vertex_entry: "vs_shape",
                fragment_entry: "fs_shape",
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: size_of::<ShapeInstance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &ShapeInstance::ATTRIBUTES,
                })],
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                target_format,
            },
        );
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("page-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..wgpu::SamplerDescriptor::default()
        });
        Self {
            grid,
            quad,
            shape,
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

struct PipelineSpec<'a> {
    label: &'static str,
    layout: &'a wgpu::PipelineLayout,
    shader: &'a wgpu::ShaderModule,
    vertex_entry: &'static str,
    fragment_entry: &'static str,
    buffers: &'a [Option<wgpu::VertexBufferLayout<'a>>],
    topology: wgpu::PrimitiveTopology,
    blend: Option<wgpu::BlendState>,
    target_format: wgpu::TextureFormat,
}

fn render_pipeline(device: &wgpu::Device, spec: &PipelineSpec<'_>) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(spec.label),
        layout: Some(spec.layout),
        vertex: wgpu::VertexState {
            module: spec.shader,
            entry_point: Some(spec.vertex_entry),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: spec.buffers,
        },
        primitive: wgpu::PrimitiveState {
            topology: spec.topology,
            ..wgpu::PrimitiveState::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: spec.shader,
            entry_point: Some(spec.fragment_entry),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: spec.target_format,
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

#[cfg(test)]
mod tests {
    use wgpu::naga;

    use super::SHADER_SOURCE;

    #[test]
    fn canvas_shader_parses_and_validates() {
        let module = naga::front::wgsl::parse_str(SHADER_SOURCE).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap();
    }
}
