//! The Prism: a mesh-gradient card inside a liquid shell inside a glass
//! shell, rendered on wgpu at parity with the three.js build it is ported
//! from (lyle-klyne `builds/2026-10-11-prism`).
//!
//! The passes are the web build's, shader for shader: three's generated
//! WGSL is kept close to verbatim under `shaders/`, with the uniforms it was
//! fed (`uniforms`), the environment it sampled (`assets/`) and the meshes it
//! drew (`geometry`). A frame is
//!
//! ```text
//! card -> copy to backdrop -> liquid -> copy -> glass -> display -> output
//! ```
//!
//! where the output pass is the composite (the shape cut out of the scene's
//! alpha) folded into a box downscale, so a supersampled render comes out
//! antialiased at the size asked for.
//!
//! The web build's paint post passes (tensor, blurs, flow, strokes) weigh
//! nothing in the Prism preset and are not ported.

mod block;
mod geometry;
mod params;
mod pipelines;
mod resources;
mod targets;
mod tumble;
mod uniforms;

pub use params::{
    Camera, Coverage, Environment, Gradient, Highlight, Light, Pose, PrismParams, Rim, Shell,
};

pub use tumble::Tumble;

use pipelines::Pipelines;
use resources::Resources;
use targets::{Targets, Uniforms};
use uniforms::Frame;

/// The web build's shell subdivision.
pub const SHELL_SEGMENTS: u32 = 80;

/// How the output's colour relates to its alpha.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alpha {
    /// Colour already multiplied by alpha, as the web canvas holds it.
    Premultiplied,
    /// Colour not multiplied by alpha, as an image file holds it.
    Straight,
}

/// What a [`PrismRenderer`] draws into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrismOptions {
    /// The output's edge in pixels.
    pub size: u32,
    /// Scene pixels per output pixel, on each axis. 1 is the web build
    /// exactly; 3 or 4 antialiases the shape's edge at avatar sizes.
    pub supersample: u32,
    /// The output texture's format. Its values are the web build's canvas
    /// values, which it shows as sRGB-encoded: write them to a non-sRGB
    /// format (`Rgba8Unorm`, `Bgra8Unorm`) and show them as sRGB.
    pub format: wgpu::TextureFormat,
    /// Whether the output's colour is premultiplied.
    pub alpha: Alpha,
    /// Subdivisions per side of each face of the shells' rounded box. The
    /// web build's 80 matters only at large sizes; at avatar sizes 24 looks
    /// the same and draws a tenth of the triangles.
    pub shell_segments: u32,
}

impl PrismOptions {
    /// `size` pixels square at `supersample`x, premultiplied `Rgba8Unorm`.
    pub fn new(size: u32, supersample: u32) -> Self {
        Self {
            size,
            supersample,
            format: wgpu::TextureFormat::Rgba8Unorm,
            alpha: Alpha::Premultiplied,
            shell_segments: SHELL_SEGMENTS,
        }
    }
}

/// Where in the frame [`PrismRenderer::render_through`] stops.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    /// The gradient card alone (scene target, rgba16float).
    Card,
    /// The card under the liquid shell (scene target).
    Liquid,
    /// The card under both shells (scene target).
    Glass,
    /// The graded scene over the paper (display target, rgba16float,
    /// straight alpha).
    Display,
    /// The composite at the output size, in the output format.
    Output,
}

/// Renders Prism frames into its own output texture.
///
/// Each render writes its uniforms with `queue.write_buffer`, which lands
/// at the next submit: submit the encoder before rendering another frame.
#[derive(Debug)]
pub struct PrismRenderer {
    options: PrismOptions,
    params: PrismParams,
    pipelines: Pipelines,
    resources: Resources,
    uniforms: Uniforms,
    targets: Targets,
}

impl PrismRenderer {
    /// Builds the pipelines, uploads the meshes and tables, and makes the
    /// targets for `options`. Compiling the shell shaders is the slow part.
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, options: PrismOptions) -> Self {
        let options = PrismOptions {
            size: options.size.max(1),
            supersample: options.supersample.max(1),
            shell_segments: options.shell_segments.max(1),
            ..options
        };
        let params = PrismParams::default();
        let pipelines = Pipelines::new(device, options.format);
        let resources = Resources::new(device, queue, &params, options.shell_segments);
        let uniforms = Uniforms::new(device);
        let targets = Targets::new(
            device,
            &pipelines,
            &resources,
            &uniforms,
            (
                options.size * options.supersample,
                options.size,
                options.format,
            ),
        );
        Self {
            options,
            params,
            pipelines,
            resources,
            uniforms,
            targets,
        }
    }

    /// The options it renders with.
    pub fn options(&self) -> PrismOptions {
        self.options
    }

    /// The look. Changes apply from the next render, except the card's size
    /// and squircleness, which are fixed when the renderer is built.
    pub fn params_mut(&mut self) -> &mut PrismParams {
        &mut self.params
    }

    /// Records the frame `t` seconds in and returns the output texture.
    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        t: f32,
    ) -> &wgpu::Texture {
        self.render_through(encoder, queue, t, Stage::Output)
    }

    /// Records the frame `t` seconds in up to `stage`, and returns the
    /// texture that holds that stage's result.
    pub fn render_through(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        t: f32,
        stage: Stage,
    ) -> &wgpu::Texture {
        self.upload(queue, t);
        let t = &self.targets;
        let g = &t.groups;
        let r = &self.resources;
        let p = &self.pipelines;
        scene_pass(encoder, t, true, &p.card, &g.card, &r.card, [0, 1]);
        if stage >= Stage::Liquid {
            copy_backdrop(encoder, t);
            // Three gave the liquid's vertex shader the normal first.
            scene_pass(encoder, t, false, &p.liquid, &g.liquid, &r.shell, [1, 0]);
        }
        if stage >= Stage::Glass {
            copy_backdrop(encoder, t);
            scene_pass(encoder, t, false, &p.glass, &g.glass, &r.shell, [0, 1]);
        }
        if stage >= Stage::Display {
            let mut pass = begin(
                encoder,
                "prism-display",
                &t.display_view,
                Some(wgpu::Color::BLACK),
                None,
            );
            pass.set_pipeline(&p.display);
            pass.set_bind_group(0, &g.display, &[]);
            pass.set_vertex_buffer(0, r.post_uv.slice(..));
            pass.draw(0..3, 0..1);
        }
        if stage >= Stage::Output {
            let mut pass = begin(
                encoder,
                "prism-output",
                &t.output_view,
                Some(wgpu::Color::TRANSPARENT),
                None,
            );
            pass.set_pipeline(&p.output);
            pass.set_bind_group(0, &g.output, &[]);
            pass.draw(0..3, 0..1);
        }
        match stage {
            Stage::Card | Stage::Liquid | Stage::Glass => &self.targets.scene,
            Stage::Display => &self.targets.display,
            Stage::Output => &self.targets.output,
        }
    }

    fn upload(&mut self, queue: &wgpu::Queue, t: f32) {
        let f = Frame {
            t,
            size: (self.options.size * self.options.supersample) as f32,
        };
        let p = &self.params;
        let u = &mut self.uniforms;
        uniforms::card_render(&mut u.card_render.block, p);
        uniforms::card_object(&mut u.card_object.block, &mut u.card_colors.block, p, f);
        uniforms::liquid_render(&mut u.liquid_render.block, p, f);
        uniforms::liquid_object(&mut u.liquid_object.block, p, f);
        uniforms::glass_render(&mut u.glass_render.block, p, f);
        uniforms::glass_object(&mut u.glass_object.block, p, f);
        uniforms::display(&mut u.display.block, p);
        for uniform in [
            &u.card_render,
            &u.card_object,
            &u.card_colors,
            &u.liquid_render,
            &u.liquid_object,
            &u.glass_render,
            &u.glass_object,
            &u.display,
        ] {
            uniform.upload(queue);
        }
        let output: [u32; 4] = [
            p.coverage.threshold.to_bits(),
            p.coverage.softness.to_bits(),
            self.options.supersample,
            u32::from(self.options.alpha == Alpha::Straight),
        ];
        queue.write_buffer(&u.output, 0, bytemuck::cast_slice(&output));
    }

    /// The packed bytes of each uniform block, by three's pipeline and
    /// binding name, for checking against a capture of the web build.
    #[doc(hidden)]
    pub fn uniform_bytes(&mut self, queue: &wgpu::Queue, t: f32) -> Vec<(&'static str, Vec<u8>)> {
        self.upload(queue, t);
        let u = &self.uniforms;
        [
            ("00-MeshBasicNodeMaterial-g0-b0", &u.card_render),
            ("00-MeshBasicNodeMaterial-g1-b0", &u.card_object),
            ("00-MeshBasicNodeMaterial-g1-b1", &u.card_colors),
            ("01-ShellLiquid-g0-b0", &u.liquid_render),
            ("01-ShellLiquid-g1-b0", &u.liquid_object),
            ("02-ShellGlass-g0-b0", &u.glass_render),
            ("02-ShellGlass-g1-b0", &u.glass_object),
            ("03-RTT-g0-b2", &u.display),
        ]
        .into_iter()
        .map(|(name, uniform)| (name, uniform.block.bytes().to_vec()))
        .collect()
    }
}

fn begin<'e>(
    encoder: &'e mut wgpu::CommandEncoder,
    label: &str,
    view: &wgpu::TextureView,
    clear: Option<wgpu::Color>,
    depth: Option<(&wgpu::TextureView, bool)>,
) -> wgpu::RenderPass<'e> {
    let load = clear.map_or(wgpu::LoadOp::Load, wgpu::LoadOp::Clear);
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: depth.map(|(view, clear)| {
            wgpu::RenderPassDepthStencilAttachment {
                view,
                depth_ops: Some(wgpu::Operations {
                    load: if clear {
                        wgpu::LoadOp::Clear(1.0)
                    } else {
                        wgpu::LoadOp::Load
                    },
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }
        }),
        ..wgpu::RenderPassDescriptor::default()
    })
}

/// One mesh into the scene target, its attribute buffers bound in `slots`
/// order. The card's pass clears colour and depth.
fn scene_pass(
    encoder: &mut wgpu::CommandEncoder,
    targets: &Targets,
    clear: bool,
    pipeline: &wgpu::RenderPipeline,
    groups: &[wgpu::BindGroup; 2],
    mesh: &resources::Mesh,
    slots: [usize; 2],
) {
    let color = clear.then_some(wgpu::Color::TRANSPARENT);
    let mut pass = begin(
        encoder,
        "prism-scene",
        &targets.scene_view,
        color,
        Some((&targets.depth, clear)),
    );
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, &groups[0], &[]);
    pass.set_bind_group(1, &groups[1], &[]);
    pass.set_vertex_buffer(0, mesh.slots[slots[0]].slice(..));
    pass.set_vertex_buffer(1, mesh.slots[slots[1]].slice(..));
    pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
    pass.draw_indexed(0..mesh.count, 0, 0..1);
}

/// The shells refract a copy of what is drawn so far.
fn copy_backdrop(encoder: &mut wgpu::CommandEncoder, targets: &Targets) {
    encoder.copy_texture_to_texture(
        targets.scene.as_image_copy(),
        targets.backdrop.as_image_copy(),
        targets.scene.size(),
    );
}
