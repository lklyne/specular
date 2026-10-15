//! The size-dependent textures of a frame and the bind groups over them.

use crate::block::Block;
use crate::pipelines::{DEPTH, HDR, Pipelines, source};
use crate::resources::Resources;

/// A uniform block with the buffer it is written to.
#[derive(Debug)]
pub(crate) struct Uniform {
    pub(crate) block: Block,
    pub(crate) buffer: wgpu::Buffer,
}

impl Uniform {
    fn new(device: &wgpu::Device, label: &str, block: Block) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: block.bytes().len() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self { block, buffer }
    }

    /// Queues the block's bytes into the buffer.
    pub(crate) fn upload(&self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.buffer, 0, self.block.bytes());
    }
}

/// Every uniform block of a frame.
#[derive(Debug)]
pub(crate) struct Uniforms {
    pub(crate) card_render: Uniform,
    pub(crate) card_object: Uniform,
    pub(crate) card_colors: Uniform,
    pub(crate) liquid_render: Uniform,
    pub(crate) liquid_object: Uniform,
    pub(crate) glass_render: Uniform,
    pub(crate) glass_object: Uniform,
    pub(crate) display: Uniform,
    pub(crate) output: wgpu::Buffer,
}

/// A shader that lacks a struct this crate fills is a broken build of the
/// crate itself, so it falls back to an empty block and draws wrong rather
/// than failing at run time.
fn block(source: &str, name: &str) -> Block {
    Block::from_wgsl(source, name).unwrap_or_else(|| {
        debug_assert!(false, "no struct {name} in a prism shader");
        Block::empty()
    })
}

impl Uniforms {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let u = |label, src, name| Uniform::new(device, label, block(src, name));
        Self {
            card_render: u("prism-card-render", source::CARD_FS, "renderStruct"),
            card_object: u("prism-card-object", source::CARD_FS, "objectStruct"),
            card_colors: u(
                "prism-card-colors",
                source::CARD_FS,
                "NodeBuffer_1401Struct",
            ),
            liquid_render: u("prism-liquid-render", source::LIQUID_FS, "renderStruct"),
            liquid_object: u("prism-liquid-object", source::LIQUID_FS, "objectStruct"),
            glass_render: u("prism-glass-render", source::GLASS_FS, "renderStruct"),
            glass_object: u("prism-glass-object", source::GLASS_FS, "objectStruct"),
            display: u("prism-display", source::DISPLAY_FS, "objectStruct"),
            output: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("prism-output"),
                size: 16,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
        }
    }
}

/// The textures one render size needs, and the bind groups that read them.
#[derive(Debug)]
pub(crate) struct Targets {
    /// The scene: card, then liquid, then glass, drawn into one target.
    pub(crate) scene: wgpu::Texture,
    pub(crate) scene_view: wgpu::TextureView,
    /// A copy of the scene so far, which a shell refracts.
    pub(crate) backdrop: wgpu::Texture,
    pub(crate) depth: wgpu::TextureView,
    /// The graded scene over the paper.
    pub(crate) display: wgpu::Texture,
    pub(crate) display_view: wgpu::TextureView,
    /// The composite at the output size.
    pub(crate) output: wgpu::Texture,
    pub(crate) output_view: wgpu::TextureView,
    pub(crate) groups: Groups,
}

/// Bind groups, per pipeline, in group order.
#[derive(Debug)]
pub(crate) struct Groups {
    pub(crate) card: [wgpu::BindGroup; 2],
    pub(crate) liquid: [wgpu::BindGroup; 2],
    pub(crate) glass: [wgpu::BindGroup; 2],
    pub(crate) display: wgpu::BindGroup,
    pub(crate) output: wgpu::BindGroup,
}

fn target(
    device: &wgpu::Device,
    label: &str,
    size: u32,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    })
}

fn group(
    device: &wgpu::Device,
    pipeline: &wgpu::RenderPipeline,
    index: u32,
    resources: &[wgpu::BindingResource<'_>],
) -> wgpu::BindGroup {
    let entries: Vec<wgpu::BindGroupEntry<'_>> = resources
        .iter()
        .enumerate()
        .map(|(binding, resource)| wgpu::BindGroupEntry {
            binding: binding as u32,
            resource: resource.clone(),
        })
        .collect();
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(index),
        entries: &entries,
    })
}

fn buf(u: &Uniform) -> wgpu::BindingResource<'_> {
    u.buffer.as_entire_binding()
}

impl Targets {
    #[expect(
        clippy::too_many_lines,
        reason = "every bind group in binding order, as the shaders declare them"
    )]
    pub(crate) fn new(
        device: &wgpu::Device,
        pipelines: &Pipelines,
        resources: &Resources,
        uniforms: &Uniforms,
        (internal, output_size, output_format): (u32, u32, wgpu::TextureFormat),
    ) -> Self {
        use wgpu::BindingResource as B;
        use wgpu::TextureUsages as U;
        let read = U::TEXTURE_BINDING | U::COPY_SRC;
        let scene = target(
            device,
            "prism-scene",
            internal,
            HDR,
            U::RENDER_ATTACHMENT | read,
        );
        let backdrop = target(
            device,
            "prism-backdrop",
            internal,
            HDR,
            U::TEXTURE_BINDING | U::COPY_DST,
        );
        let depth = target(device, "prism-depth", internal, DEPTH, U::RENDER_ATTACHMENT);
        let display = target(
            device,
            "prism-display",
            internal,
            HDR,
            U::RENDER_ATTACHMENT | read,
        );
        let output = target(
            device,
            "prism-output",
            output_size,
            output_format,
            U::RENDER_ATTACHMENT | read,
        );
        let view = |t: &wgpu::Texture| t.create_view(&wgpu::TextureViewDescriptor::default());
        let (scene_view, backdrop_view, display_view) =
            (view(&scene), view(&backdrop), view(&display));
        let clamp = B::Sampler(&resources.clamp);
        let groups = Groups {
            card: [
                group(device, &pipelines.card, 0, &[buf(&uniforms.card_render)]),
                group(
                    device,
                    &pipelines.card,
                    1,
                    &[buf(&uniforms.card_object), buf(&uniforms.card_colors)],
                ),
            ],
            liquid: [
                group(
                    device,
                    &pipelines.liquid,
                    0,
                    &[buf(&uniforms.liquid_render)],
                ),
                group(
                    device,
                    &pipelines.liquid,
                    1,
                    &[
                        buf(&uniforms.liquid_object),
                        clamp.clone(),
                        B::TextureView(&resources.dfg),
                        clamp.clone(),
                        B::TextureView(&resources.pmrem),
                        clamp.clone(),
                        B::TextureView(&backdrop_view),
                    ],
                ),
            ],
            glass: [
                group(device, &pipelines.glass, 0, &[buf(&uniforms.glass_render)]),
                group(
                    device,
                    &pipelines.glass,
                    1,
                    &[
                        buf(&uniforms.glass_object),
                        B::Sampler(&resources.repeat),
                        B::TextureView(&resources.brush),
                        clamp.clone(),
                        B::TextureView(&resources.dfg),
                        clamp.clone(),
                        B::TextureView(&resources.pmrem),
                        clamp.clone(),
                        B::TextureView(&backdrop_view),
                    ],
                ),
            ],
            display: group(
                device,
                &pipelines.display,
                0,
                &[
                    clamp.clone(),
                    B::TextureView(&scene_view),
                    buf(&uniforms.display),
                ],
            ),
            output: group(
                device,
                &pipelines.output,
                0,
                &[
                    B::TextureView(&display_view),
                    uniforms.output.as_entire_binding(),
                ],
            ),
        };
        Self {
            output_view: view(&output),
            depth: view(&depth),
            scene,
            scene_view,
            backdrop,
            display,
            display_view,
            output,
            groups,
        }
    }
}
