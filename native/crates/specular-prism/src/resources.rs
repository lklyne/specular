//! What every frame reads and nothing writes: the meshes, the baked
//! environment and BRDF tables, and the samplers.

use wgpu::util::DeviceExt as _;

use crate::geometry;
use crate::params::PrismParams;

/// The environment as the shaders sample it: three's PMREM cube-UV atlas
/// of the build's HDR, 336 x 256 rgba16float, baked from the web build.
const PMREM: &[u8] = include_bytes!("../assets/pmrem-cubeuv-mip0.336x256.rgba16f");
const PMREM_SIZE: (u32, u32) = (336, 256);
/// Three's DFG table for the split-sum specular, 16 x 16 rg16float.
const DFG: &[u8] = include_bytes!("../assets/dfg-lut-mip0.16x16.rg16f");
const DFG_SIZE: u32 = 16;

/// The edge of the unit shell and its subdivision, as the build makes it.
const SHELL_SIZE: f64 = 1.2;
const SHELL_SQUIRCLENESS: f64 = 4.0;
const SHELL_SEGMENTS: u32 = 80;
const CARD_SEGMENTS: u32 = 256;

/// The full-screen triangle's UVs; three's post passes put `uv.y = 0` at
/// the top.
const POST_UV: [f32; 6] = [0.0, -1.0, 0.0, 1.0, 2.0, 1.0];

/// A mesh on the GPU: one buffer per vertex attribute and the indices.
#[derive(Debug)]
pub(crate) struct Mesh {
    pub(crate) slots: [wgpu::Buffer; 2],
    pub(crate) indices: wgpu::Buffer,
    pub(crate) count: u32,
}

/// The frame-independent GPU resources.
#[derive(Debug)]
pub(crate) struct Resources {
    pub(crate) card: Mesh,
    pub(crate) shell: Mesh,
    pub(crate) post_uv: wgpu::Buffer,
    pub(crate) pmrem: wgpu::TextureView,
    pub(crate) dfg: wgpu::TextureView,
    pub(crate) brush: wgpu::TextureView,
    /// Linear, clamped, nearest mip: every sampler but the brush's.
    pub(crate) clamp: wgpu::Sampler,
    /// Linear, repeating, linear mip: the brush's.
    pub(crate) repeat: wgpu::Sampler,
}

fn buffer(
    device: &wgpu::Device,
    label: &str,
    bytes: &[u8],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: bytes,
        usage,
    })
}

fn texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    (width, height): (u32, u32),
    format: wgpu::TextureFormat,
    bytes: &[u8],
) -> wgpu::TextureView {
    let texture = device.create_texture_with_data(
        queue,
        &wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        },
        wgpu::util::TextureDataOrder::LayerMajor,
        bytes,
    );
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

impl Resources {
    #[expect(
        clippy::too_many_lines,
        reason = "one descriptor per resource, read top to bottom"
    )]
    pub(crate) fn new(device: &wgpu::Device, queue: &wgpu::Queue, params: &PrismParams) -> Self {
        let vertex = wgpu::BufferUsages::VERTEX;
        let index = wgpu::BufferUsages::INDEX;
        let card = geometry::card(
            f64::from(params.gradient.size),
            f64::from(params.gradient.squircleness),
            CARD_SEGMENTS,
        );
        let shell = geometry::shell(SHELL_SIZE, SHELL_SQUIRCLENESS, SHELL_SEGMENTS);
        let sampler = |label, address, mipmap| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some(label),
                address_mode_u: address,
                address_mode_v: address,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: mipmap,
                ..wgpu::SamplerDescriptor::default()
            })
        };
        Self {
            card: Mesh {
                slots: [
                    buffer(
                        device,
                        "prism-card-uv",
                        bytemuck::cast_slice(&card.uvs),
                        vertex,
                    ),
                    buffer(
                        device,
                        "prism-card-position",
                        bytemuck::cast_slice(&card.positions),
                        vertex,
                    ),
                ],
                indices: buffer(
                    device,
                    "prism-card-index",
                    bytemuck::cast_slice(&card.indices),
                    index,
                ),
                count: card.indices.len() as u32,
            },
            shell: Mesh {
                slots: [
                    buffer(
                        device,
                        "prism-shell-position",
                        bytemuck::cast_slice(&shell.positions),
                        vertex,
                    ),
                    buffer(
                        device,
                        "prism-shell-normal",
                        bytemuck::cast_slice(&shell.normals),
                        vertex,
                    ),
                ],
                indices: buffer(
                    device,
                    "prism-shell-index",
                    bytemuck::cast_slice(&shell.indices),
                    index,
                ),
                count: shell.indices.len() as u32,
            },
            post_uv: buffer(
                device,
                "prism-post-uv",
                bytemuck::cast_slice(&POST_UV),
                vertex,
            ),
            pmrem: texture(
                device,
                queue,
                "prism-pmrem",
                PMREM_SIZE,
                wgpu::TextureFormat::Rgba16Float,
                PMREM,
            ),
            dfg: texture(
                device,
                queue,
                "prism-dfg",
                (DFG_SIZE, DFG_SIZE),
                wgpu::TextureFormat::Rg16Float,
                DFG,
            ),
            // The painted normal is mixed in at strength 0: any texture does.
            brush: texture(
                device,
                queue,
                "prism-brush",
                (1, 1),
                wgpu::TextureFormat::Rgba8Unorm,
                &[128, 128, 255, 255],
            ),
            clamp: sampler(
                "prism-clamp",
                wgpu::AddressMode::ClampToEdge,
                wgpu::MipmapFilterMode::Nearest,
            ),
            repeat: sampler(
                "prism-repeat",
                wgpu::AddressMode::Repeat,
                wgpu::MipmapFilterMode::Linear,
            ),
        }
    }
}
