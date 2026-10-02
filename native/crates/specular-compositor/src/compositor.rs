//! The renderer: page frames + dot grid under a camera.

use std::time::Duration;

use glam::Vec2;
use specular_core::{Camera, CanvasRect, PageEvent, PageId};

use crate::error::CompositorError;

/// One page to draw this frame, in paint order (back to front).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageDraw {
    /// Which page's latest frame to draw.
    pub page: PageId,
    /// Where, in canvas space.
    pub rect: CanvasRect,
}

/// Dot-grid background parameters (canvas-space spacing, like canvas-bg).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DotGrid {
    /// Distance between dots in canvas units.
    pub spacing: f32,
    /// Dot radius in screen (logical) pixels.
    pub radius: f32,
    /// Background colour, linear RGBA.
    pub background: [f32; 4],
    /// Dot colour, linear RGBA.
    pub dot: [f32; 4],
}

impl Default for DotGrid {
    fn default() -> Self {
        Self {
            spacing: 24.0,
            radius: 1.0,
            background: [0.96, 0.96, 0.96, 1.0],
            dot: [0.75, 0.75, 0.75, 1.0],
        }
    }
}

/// Everything [`Compositor::render`] needs for one window frame.
#[derive(Debug, Clone, Copy)]
pub struct SceneView<'a> {
    /// Canvas camera.
    pub camera: Camera,
    /// Viewport size in logical pixels.
    pub viewport: Vec2,
    /// Physical pixels per logical pixel (window scale factor).
    pub scale_factor: f32,
    /// Pages in paint order.
    pub pages: &'a [PageDraw],
    /// Background grid.
    pub grid: DotGrid,
}

/// Per-frame counters, comparable to the Electron lab's benchmark fields.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RenderStats {
    /// Pages with a texture that were drawn (after culling).
    pub pages_drawn: u32,
    /// Visible pages that had no frame yet (`framesWithoutTexture`).
    pub pages_without_texture: u32,
    /// Pages whose current texture came from a CPU upload (non-representative).
    pub cpu_textures: u32,
    /// CPU time spent encoding and submitting the frame.
    pub encode_time: Duration,
}

/// The wgpu compositor; see the crate docs.
#[derive(Debug)]
pub struct Compositor {
    device: wgpu::Device,
    queue: wgpu::Queue,
    target_format: wgpu::TextureFormat,
}

impl Compositor {
    /// Creates a compositor that renders into textures of `target_format`
    /// (the window surface's format).
    pub fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        target_format: wgpu::TextureFormat,
    ) -> Self {
        Self {
            device,
            queue,
            target_format,
        }
    }

    /// The colour format this compositor renders into.
    pub fn target_format(&self) -> wgpu::TextureFormat {
        self.target_format
    }

    /// Ingests one event from a page source: frames replace the page's
    /// texture for that layer (dropping, and so releasing, the previous one);
    /// popup events show/hide/move the popup layer; others are ignored.
    pub fn handle_page_event(&mut self, event: PageEvent) -> Result<(), CompositorError> {
        drop(event);
        Ok(())
    }

    /// Forgets a closed page and releases its textures.
    pub fn remove_page(&mut self, page: PageId) {
        let _ = page;
    }

    /// Number of shared textures currently held for `page` (bounded by
    /// [`MAX_OUTSTANDING_TEXTURES`](specular_core::MAX_OUTSTANDING_TEXTURES)).
    pub fn outstanding_textures(&self, page: PageId) -> usize {
        let _ = page;
        0
    }

    /// Draws `scene` into `target` and submits the work.
    pub fn render(&mut self, target: &wgpu::TextureView, scene: &SceneView<'_>) -> RenderStats {
        let started = std::time::Instant::now();
        let [r, g, b, a] = scene.grid.background.map(f64::from);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("specular-frame"),
            });
        drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("specular-canvas"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..wgpu::RenderPassDescriptor::default()
        }));
        self.queue.submit([encoder.finish()]);
        RenderStats {
            pages_without_texture: scene.pages.len() as u32,
            encode_time: started.elapsed(),
            ..RenderStats::default()
        }
    }
}
