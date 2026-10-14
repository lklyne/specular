//! The five render pipelines, with the state the web build's pipelines had
//! (`reference/pipelines/*.json`). Layouts are derived from the shaders.

/// Three's generated WGSL, as dumped from the web build.
pub(crate) mod source {
    pub(crate) const CARD_VS: &str = include_str!("../shaders/card-vs.wgsl");
    pub(crate) const CARD_FS: &str = include_str!("../shaders/card-fs.wgsl");
    pub(crate) const LIQUID_VS: &str = include_str!("../shaders/liquid-vs.wgsl");
    pub(crate) const LIQUID_FS: &str = include_str!("../shaders/liquid-fs.wgsl");
    pub(crate) const GLASS_VS: &str = include_str!("../shaders/glass-vs.wgsl");
    pub(crate) const GLASS_FS: &str = include_str!("../shaders/glass-fs.wgsl");
    pub(crate) const DISPLAY_VS: &str = include_str!("../shaders/display-vs.wgsl");
    pub(crate) const DISPLAY_FS: &str = include_str!("../shaders/display-fs.wgsl");
    pub(crate) const OUTPUT: &str = include_str!("../shaders/output.wgsl");
}

/// The format of the scene and display targets.
pub(crate) const HDR: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
/// The scene's depth format.
pub(crate) const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

/// Every pipeline of a frame.
#[derive(Debug)]
pub(crate) struct Pipelines {
    pub(crate) card: wgpu::RenderPipeline,
    pub(crate) liquid: wgpu::RenderPipeline,
    pub(crate) glass: wgpu::RenderPipeline,
    pub(crate) display: wgpu::RenderPipeline,
    pub(crate) output: wgpu::RenderPipeline,
}

fn module(device: &wgpu::Device, label: &str, source: &str) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    })
}

fn float32(location: u32, format: wgpu::VertexFormat) -> [wgpu::VertexAttribute; 1] {
    [wgpu::VertexAttribute {
        format,
        offset: 0,
        shader_location: location,
    }]
}

fn layout(stride: u64, attributes: &[wgpu::VertexAttribute]) -> wgpu::VertexBufferLayout<'_> {
    wgpu::VertexBufferLayout {
        array_stride: stride,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes,
    }
}

/// The blend three uses for transparent meshes: straight alpha over, with
/// the alpha channel accumulating as premultiplied.
const OVER: wgpu::BlendState = wgpu::BlendState {
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
};

struct Spec<'a> {
    label: &'a str,
    vs: &'a str,
    fs: &'a str,
    buffers: &'a [Option<wgpu::VertexBufferLayout<'a>>],
    cull: Option<wgpu::Face>,
    depth: bool,
    blend: Option<wgpu::BlendState>,
    format: wgpu::TextureFormat,
}

fn pipeline(device: &wgpu::Device, spec: &Spec<'_>) -> wgpu::RenderPipeline {
    let vs = module(device, spec.label, spec.vs);
    let fs = module(device, spec.label, spec.fs);
    let (vs_entry, fs_entry) = if spec.vs == spec.fs {
        ("vs_main", "fs_main")
    } else {
        ("main", "main")
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(spec.label),
        layout: None,
        vertex: wgpu::VertexState {
            module: &vs,
            entry_point: Some(vs_entry),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: spec.buffers,
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: spec.cull,
            ..wgpu::PrimitiveState::default()
        },
        depth_stencil: spec.depth.then_some(wgpu::DepthStencilState {
            format: DEPTH,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &fs,
            entry_point: Some(fs_entry),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: spec.format,
                blend: spec.blend,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

impl Pipelines {
    pub(crate) fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Self {
        let card_uv = float32(0, wgpu::VertexFormat::Float32x2);
        let card_pos = float32(1, wgpu::VertexFormat::Float32x3);
        let card_buffers = [Some(layout(8, &card_uv)), Some(layout(12, &card_pos))];
        let shell_pos = float32(0, wgpu::VertexFormat::Float32x3);
        let shell_normal = float32(1, wgpu::VertexFormat::Float32x3);
        let shell_buffers = [
            Some(layout(12, &shell_pos)),
            Some(layout(12, &shell_normal)),
        ];
        let post_uv = float32(0, wgpu::VertexFormat::Float32x2);
        let post_buffers = [Some(layout(8, &post_uv))];
        let scene = |label, vs, fs, buffers, cull, blend| Spec {
            label,
            vs,
            fs,
            buffers,
            cull,
            depth: true,
            blend,
            format: HDR,
        };
        Self {
            card: pipeline(
                device,
                &scene(
                    "prism-card",
                    source::CARD_VS,
                    source::CARD_FS,
                    &card_buffers,
                    None,
                    None,
                ),
            ),
            liquid: pipeline(
                device,
                &scene(
                    "prism-liquid",
                    source::LIQUID_VS,
                    source::LIQUID_FS,
                    &shell_buffers,
                    None,
                    Some(OVER),
                ),
            ),
            glass: pipeline(
                device,
                &scene(
                    "prism-glass",
                    source::GLASS_VS,
                    source::GLASS_FS,
                    &shell_buffers,
                    Some(wgpu::Face::Back),
                    Some(OVER),
                ),
            ),
            display: pipeline(
                device,
                &Spec {
                    label: "prism-display",
                    vs: source::DISPLAY_VS,
                    fs: source::DISPLAY_FS,
                    buffers: &post_buffers,
                    cull: Some(wgpu::Face::Back),
                    depth: false,
                    blend: None,
                    format: HDR,
                },
            ),
            output: pipeline(
                device,
                &Spec {
                    label: "prism-output",
                    vs: source::OUTPUT,
                    fs: source::OUTPUT,
                    buffers: &[],
                    cull: None,
                    depth: false,
                    blend: None,
                    format: output_format,
                },
            ),
        }
    }
}
