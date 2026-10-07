//! The panel half of the bake-off: an egui toolbar and sidebar painted in a
//! second pass over whatever the canvas renderer already drew into the target.
//!
//! There is no window here. egui is driven with a hand-built `RawInput`, which
//! is also how the shell would feed it winit events it has already routed.

use bakeoff_scene::{Gpu, camera::VIEWPORT};
use egui_wgpu::{Renderer, RendererOptions, ScreenDescriptor};
// Proves the winit bridge resolves against the workspace's winit; the shell
// would build its `RawInput` with this.
pub use egui_winit::State as WinitBridge;

const TOOLS: [&str; 7] = ["Select", "Hand", "Page", "Text", "Sticky", "Draw", "Arrow"];

pub struct Panels {
    context: egui::Context,
    renderer: Renderer,
    tool: usize,
}

impl Panels {
    pub fn new(gpu: &Gpu, format: wgpu::TextureFormat) -> Self {
        Self {
            context: egui::Context::default(),
            renderer: Renderer::new(&gpu.device, format, RendererOptions::default()),
            tool: 0,
        }
    }

    /// Lays out the panels and draws them over `target` without clearing it.
    pub fn paint(&mut self, gpu: &Gpu, target: &wgpu::TextureView, zoom: f32) {
        let screen = ScreenDescriptor {
            size_in_pixels: VIEWPORT,
            pixels_per_point: 1.0,
        };
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(VIEWPORT[0] as f32, VIEWPORT[1] as f32),
            )),
            ..egui::RawInput::default()
        };
        let tool = &mut self.tool;
        let output = self.context.run_ui(input, |ui| panels(ui, tool, zoom));
        let jobs = self
            .context
            .tessellate(output.shapes, screen.pixels_per_point);

        let mut textures = output.textures_delta;
        for (id, deltas) in &textures.set {
            for delta in deltas {
                self.renderer
                    .update_texture(&gpu.device, &gpu.queue, *id, delta);
            }
        }
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let mut commands =
            self.renderer
                .update_buffers(&gpu.device, &gpu.queue, &mut encoder, &jobs, &screen);
        {
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("panels"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: target,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    ..wgpu::RenderPassDescriptor::default()
                })
                .forget_lifetime();
            self.renderer.render(&mut pass, &jobs, &screen);
        }
        commands.push(encoder.finish());
        gpu.queue.submit(commands);
        for id in &textures.free {
            self.renderer.free_texture(id);
        }
        textures.clear();
    }
}

fn panels(ui: &mut egui::Ui, tool: &mut usize, zoom: f32) {
    egui::Panel::top("toolbar").show(ui, |ui| {
        ui.horizontal(|ui| {
            for (index, label) in TOOLS.iter().enumerate() {
                ui.selectable_value(tool, index, *label);
            }
            ui.separator();
            ui.label(format!("{:.0}%", zoom * 100.0));
        });
    });
    egui::Panel::left("sidebar").show(ui, |ui| {
        ui.heading("Spaces");
        for space in ["Landing page", "Checkout flow", "Design review"] {
            let _ = ui.button(space);
        }
        ui.separator();
        ui.label("Layers");
        egui::ScrollArea::vertical().show(ui, |ui| {
            for layer in 0..40 {
                ui.label(format!("Sticky note {layer}"));
            }
        });
    });
}
